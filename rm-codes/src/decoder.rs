//! Мажоритарный декодер Рида для двоичных кодов Рида–Маллера.

use crate::layout::RmLayout;
use crate::{Bit, RmConfig, RmError};

/// Причина завершения декодирования.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmDecodeStatus {
    /// Входное слово уже было кодовым словом.
    Valid,
    /// Все мажоритарные голосования дали строгое большинство, слово изменено.
    Corrected,
    /// Хотя бы одно голосование закончилось ничьей.
    ///
    /// Возвращённый кандидат детерминирован, но не гарантирован.
    Ambiguous,
}

/// Результат декодирования одного блока.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RmDecodeResult {
    word: Vec<Bit>,
    message: Vec<Bit>,
    status: RmDecodeStatus,
    changed_bits: usize,
    corrected_positions: Vec<usize>,
    ambiguous_coefficients: Vec<usize>,
}

impl RmDecodeResult {
    /// Возвращает исправленное кодовое слово.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает восстановленное сообщение — коэффициенты АНФ.
    #[must_use]
    pub fn message(&self) -> &[Bit] {
        &self.message
    }

    /// Возвращает причину завершения декодирования.
    #[must_use]
    pub fn status(&self) -> RmDecodeStatus {
        self.status
    }

    /// Возвращает число позиций, изменившихся относительно входа.
    ///
    /// Это число изменений, а не обязательно число реально исправленных
    /// ошибок: без эталонного слова декодер не знает, какие изменения верны.
    #[must_use]
    pub fn changed_bits(&self) -> usize {
        self.changed_bits
    }

    /// Возвращает позиции, в которых итоговое слово отличается от входного.
    #[must_use]
    pub fn corrected_positions(&self) -> &[usize] {
        &self.corrected_positions
    }

    /// Возвращает индексы коэффициентов, для которых была ничья.
    #[must_use]
    pub fn ambiguous_coefficients(&self) -> &[usize] {
        &self.ambiguous_coefficients
    }
}

/// Событие синхронного наблюдения за декодированием.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmDecodeEvent {
    /// Декодер подготовил рабочее слово.
    Started {
        /// Длина кодового слова.
        codeword_len: usize,
    },
    /// Начата обработка коэффициентов указанной степени.
    DegreeStarted {
        /// Степень коэффициентов; `0` — свободный член.
        degree: usize,
    },
    /// Принято решение по одному коэффициенту.
    CoefficientDecided {
        /// Индекс коэффициента в порядке сообщения.
        index: usize,
        /// Степень коэффициента.
        degree: usize,
        /// Битовая маска коэффициента.
        mask: u32,
        /// Число голосов за единицу.
        ones: usize,
        /// Общее число голосов.
        votes: usize,
        /// Принятое значение.
        decision: Bit,
        /// Была ли ничья.
        ambiguous: bool,
    },
    /// Декодирование завершено.
    Finished {
        /// Причина завершения.
        status: RmDecodeStatus,
        /// Число позиций, изменённых относительно входа.
        changed_bits: usize,
    },
}

/// Получатель событий одного вызова декодера.
pub trait RmDecodeObserver {
    /// Обрабатывает событие до продолжения декодирования.
    fn on_event(&mut self, event: &RmDecodeEvent);
}

/// Общий контракт декодера одного двоичного блока.
pub trait Decoder {
    /// Возвращает ожидаемую длину кодового слова.
    fn codeword_len(&self) -> usize;

    /// Декодирует один блок и при необходимости синхронно отправляет события.
    ///
    /// Декодер работает с жёсткими решениями. При ошибке входа события не
    /// отправляются.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::WordLengthMismatch`] при неверной длине.
    fn decode(
        &self,
        word: &[Bit],
        observer: Option<&mut dyn RmDecodeObserver>,
    ) -> Result<RmDecodeResult, RmError>;
}

/// Мажоритарный декодер Рида для `RM(r, m)`.
#[derive(Clone, Debug)]
pub struct RmDecoder {
    config: RmConfig,
    layout: RmLayout,
}

impl RmDecoder {
    /// Создаёт декодер по проверенным параметрам.
    pub fn new(config: RmConfig) -> Self {
        let layout = RmLayout::new(config);
        Self { config, layout }
    }

    /// Возвращает параметры кода.
    #[must_use]
    pub fn config(&self) -> RmConfig {
        self.config
    }

    /// Декодирует слово мажоритарным алгоритмом Рида.
    ///
    /// Если хотя бы одно голосование закончилось ничьей, возвращается
    /// [`RmDecodeStatus::Ambiguous`] с детерминированным кандидатом.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::WordLengthMismatch`] при неверной длине.
    pub fn decode(
        &self,
        word: &[Bit],
        observer: Option<&mut dyn RmDecodeObserver>,
    ) -> Result<RmDecodeResult, RmError> {
        self.decode_impl(word, observer)
    }

    fn decode_impl(
        &self,
        word: &[Bit],
        observer: Option<&mut dyn RmDecodeObserver>,
    ) -> Result<RmDecodeResult, RmError> {
        let expected = self.layout.n();
        if word.len() != expected {
            return Err(RmError::WordLengthMismatch {
                expected,
                actual: word.len(),
            });
        }

        let mut observer = observer;
        emit(
            &mut observer,
            RmDecodeEvent::Started {
                codeword_len: expected,
            },
        );

        let mut work: Vec<u8> = word.iter().map(|&bit| u8::from(bit)).collect();
        let mut message = vec![Bit::Zero; self.layout.k()];
        let mut ambiguous = false;
        let mut ambiguous_coefficients = Vec::new();

        for degree in (1..=self.config.r()).rev() {
            emit(&mut observer, RmDecodeEvent::DegreeStarted { degree });

            for (index, slot) in message.iter_mut().enumerate() {
                if self.layout.degree(index) != degree {
                    continue;
                }

                let mask = self.layout.mask(index);
                let votes = 1usize << (self.config.m() - degree);
                let mut ones = 0usize;

                for base in self.layout.coset_bases(mask) {
                    let mut parity = 0u8;
                    for submask in RmLayout::submasks(mask) {
                        parity ^= work[(base | submask) as usize];
                    }
                    ones += usize::from(parity);
                }

                let tied = ones * 2 == votes;
                if tied {
                    ambiguous = true;
                    ambiguous_coefficients.push(index);
                }

                let decision = if ones * 2 > votes {
                    Bit::One
                } else {
                    Bit::Zero
                };
                *slot = decision;

                if decision.is_one() {
                    for (position, value) in work.iter_mut().enumerate() {
                        if RmLayout::contains(position as u32, mask) {
                            *value ^= 1;
                        }
                    }
                }

                emit(
                    &mut observer,
                    RmDecodeEvent::CoefficientDecided {
                        index,
                        degree,
                        mask,
                        ones,
                        votes,
                        decision,
                        ambiguous: tied,
                    },
                );
            }
        }

        emit(&mut observer, RmDecodeEvent::DegreeStarted { degree: 0 });
        let ones = work.iter().filter(|&&value| value == 1).count();
        let votes = expected;
        let tied = ones * 2 == votes;
        if tied {
            ambiguous = true;
            ambiguous_coefficients.push(0);
        }
        let decision = if ones * 2 > votes {
            Bit::One
        } else {
            Bit::Zero
        };
        message[0] = decision;
        emit(
            &mut observer,
            RmDecodeEvent::CoefficientDecided {
                index: 0,
                degree: 0,
                mask: 0,
                ones,
                votes,
                decision,
                ambiguous: tied,
            },
        );

        let corrected = self.layout.encode(&message);
        let mut changed_bits = 0usize;
        let mut corrected_positions = Vec::new();
        for (index, (&before, &after)) in word.iter().zip(&corrected).enumerate() {
            if before != after {
                changed_bits += 1;
                corrected_positions.push(index);
            }
        }

        let status = if changed_bits == 0 {
            RmDecodeStatus::Valid
        } else if ambiguous {
            RmDecodeStatus::Ambiguous
        } else {
            RmDecodeStatus::Corrected
        };

        emit(
            &mut observer,
            RmDecodeEvent::Finished {
                status,
                changed_bits,
            },
        );

        Ok(RmDecodeResult {
            word: corrected,
            message,
            status,
            changed_bits,
            corrected_positions,
            ambiguous_coefficients,
        })
    }
}

impl Decoder for RmDecoder {
    fn codeword_len(&self) -> usize {
        self.layout.n()
    }

    fn decode(
        &self,
        word: &[Bit],
        observer: Option<&mut dyn RmDecodeObserver>,
    ) -> Result<RmDecodeResult, RmError> {
        self.decode_impl(word, observer)
    }
}

fn emit(observer: &mut Option<&mut dyn RmDecodeObserver>, event: RmDecodeEvent) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&event);
    }
}

#[cfg(test)]
mod tests {
    use super::{RmDecodeEvent, RmDecodeObserver, RmDecodeStatus, RmDecoder};
    use crate::{Bit, RmConfig, RmError};

    fn bits(text: &str) -> Vec<Bit> {
        text.bytes()
            .map(|byte| Bit::try_from(byte - b'0').unwrap())
            .collect()
    }

    #[test]
    fn decodes_first_pdf_example() {
        let decoder = RmDecoder::new(RmConfig::new(1, 4).unwrap());
        let word = bits("1001100110011110");

        let result = decoder.decode(&word, None).unwrap();

        assert_eq!(result.message(), bits("10011"));
        assert_eq!(result.word(), bits("1001100110011001"));
        assert_eq!(result.status(), RmDecodeStatus::Corrected);
        assert_eq!(result.changed_bits(), 3);
        assert_eq!(result.corrected_positions(), &[13, 14, 15]);
    }

    #[test]
    fn decodes_second_pdf_example() {
        let decoder = RmDecoder::new(RmConfig::new(2, 4).unwrap());
        let word = bits("1100000101111000");

        let result = decoder.decode(&word, None).unwrap();

        assert_eq!(result.message(), bits("11100011001"));
        assert_eq!(result.word(), bits("1110000101111000"));
        assert_eq!(result.status(), RmDecodeStatus::Corrected);
        assert_eq!(result.changed_bits(), 1);
        assert_eq!(result.corrected_positions(), &[2]);
    }

    #[test]
    fn valid_word_is_reported_as_valid() {
        let decoder = RmDecoder::new(RmConfig::new(1, 4).unwrap());
        let word = bits("1001100110011001");

        let result = decoder.decode(&word, None).unwrap();

        assert_eq!(result.status(), RmDecodeStatus::Valid);
        assert_eq!(result.changed_bits(), 0);
        assert!(result.corrected_positions().is_empty());
    }

    #[test]
    fn tie_is_reported_as_ambiguous() {
        let decoder = RmDecoder::new(RmConfig::new(0, 1).unwrap());

        let result = decoder.decode(&bits("01"), None).unwrap();

        assert_eq!(result.status(), RmDecodeStatus::Ambiguous);
        assert_eq!(result.ambiguous_coefficients(), &[0]);
    }

    #[test]
    fn full_code_returns_input_unchanged() {
        let decoder = RmDecoder::new(RmConfig::new(4, 4).unwrap());
        let word = bits("1010010111000010");

        let result = decoder.decode(&word, None).unwrap();

        assert_eq!(result.status(), RmDecodeStatus::Valid);
        assert_eq!(result.word(), word);
        assert_eq!(
            decoder.decode(result.word(), None).unwrap().message(),
            result.message()
        );
    }

    #[test]
    fn rejects_wrong_word_length_without_events() {
        let decoder = RmDecoder::new(RmConfig::new(1, 4).unwrap());
        let mut observer = Recording::default();

        let error = decoder
            .decode(&bits("101"), Some(&mut observer))
            .unwrap_err();

        assert_eq!(
            error,
            RmError::WordLengthMismatch {
                expected: 16,
                actual: 3
            }
        );
        assert!(observer.events.is_empty());
    }

    #[test]
    fn observer_sees_all_coefficients_and_finish() {
        let decoder = RmDecoder::new(RmConfig::new(2, 4).unwrap());
        let mut observer = Recording::default();

        let result = decoder
            .decode(&bits("1100000101111000"), Some(&mut observer))
            .unwrap();

        assert!(matches!(
            observer.events.first(),
            Some(RmDecodeEvent::Started { codeword_len: 16 })
        ));
        assert!(matches!(
            observer.events.last(),
            Some(RmDecodeEvent::Finished {
                status: RmDecodeStatus::Corrected,
                changed_bits: 1
            })
        ));
        assert_eq!(
            observer
                .events
                .iter()
                .filter(|event| matches!(event, RmDecodeEvent::DegreeStarted { .. }))
                .count(),
            3
        );
        assert_eq!(
            observer
                .events
                .iter()
                .filter(|event| matches!(event, RmDecodeEvent::CoefficientDecided { .. }))
                .count(),
            11
        );
        assert_eq!(result.message().len(), 11);
    }

    #[derive(Default)]
    struct Recording {
        events: Vec<RmDecodeEvent>,
    }

    impl RmDecodeObserver for Recording {
        fn on_event(&mut self, event: &RmDecodeEvent) {
            self.events.push(*event);
        }
    }
}
