//! Публичный контракт декодирования и результат блока.

use crate::bit::{bit_distance, count_ones};
use crate::{Bit, DecodeInput, LdpcError, ParityCheckMatrix};

/// Причина завершения полного декодирования.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    /// Итоговое слово удовлетворяет всем проверкам `H`.
    ParitySatisfied,
    /// Бюджет итераций исчерпан до получения нулевого синдрома.
    IterationLimit,
}

/// Результат декодирования одного блока.
///
/// Результат владеет словом, итоговыми LLR и синдромом. Его данные не
/// изменяются при следующих вызовах декодера. Нулевой синдром означает, что
/// слово удовлетворяет проверочной матрице, но не доказывает совпадение с
/// переданным словом. SPA строит начальное жёсткое решение по LLR после
/// насыщения пределом конфигурации и применения стираний.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeResult {
    pub(crate) word: Vec<Bit>,
    pub(crate) posterior_llrs: Vec<f64>,
    pub(crate) syndrome: Vec<Bit>,
    pub(crate) status: DecodeStatus,
    pub(crate) iterations: usize,
    pub(crate) changed_bits: usize,
    pub(crate) initial_unsatisfied_checks: usize,
    pub(crate) final_unsatisfied_checks: usize,
}

impl DecodeResult {
    /// Создаёт проверяемый результат декодирования для реализации [`Decoder`].
    ///
    /// Итоговое жёсткое слово строится из знаков `posterior_llrs`: значение
    /// `>= 0.0`, включая оба нуля, становится [`Bit::Zero`], отрицательное —
    /// [`Bit::One`]. Синдром и все счётчики вычисляются по `checks`,
    /// `initial_word` и полученному слову. Результат забирает владение
    /// `posterior_llrs`; `initial_word` используется только для расчёта
    /// диагностики и не сохраняется.
    ///
    /// `initial_word` — заимствованное начальное жёсткое решение конкретного
    /// алгоритма. Для SPA это решение канальных LLR после насыщения и
    /// применения стираний.
    /// Предел `iteration_limit` задаётся вызывающим декодером и проверяется
    /// относительно `iterations` и `status`.
    ///
    /// Конструктор проверяет форму результата, конечность итоговых LLR,
    /// соответствие статуса синдрому и пределу итераций. Он не может подтвердить
    /// историю стороннего алгоритма: соответствие `initial_word` исходному
    /// каналу и факт выполнения каждой заявленной итерации остаются на
    /// ответственности реализации [`Decoder`]. При нуле итераций итоговое
    /// решение обязано совпадать с `initial_word`; конструктор проверяет это.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`] или
    /// [`LdpcError::LlrLengthMismatch`] для неверных длин,
    /// [`LdpcError::NonFiniteLlr`] для NaN или бесконечности,
    /// [`LdpcError::DecodeIterationsExceedLimit`] если итераций больше предела,
    /// и [`LdpcError::DecodeResultStatusMismatch`] если статус не согласуется с
    /// синдромом либо бюджетом. `ParitySatisfied` требует нулевой синдром;
    /// `IterationLimit` требует ненулевой синдром и `iterations == iteration_limit`.
    /// [`LdpcError::DecodeResultZeroIterationsMismatch`] возвращается, если
    /// итоговое слово изменилось при нуле итераций.
    pub fn try_new(
        checks: &ParityCheckMatrix,
        initial_word: &[Bit],
        posterior_llrs: Vec<f64>,
        status: DecodeStatus,
        iterations: usize,
        iteration_limit: usize,
    ) -> Result<Self, LdpcError> {
        let expected = checks.cols();
        if initial_word.len() != expected {
            return Err(LdpcError::WordLengthMismatch {
                expected,
                actual: initial_word.len(),
            });
        }
        if posterior_llrs.len() != expected {
            return Err(LdpcError::LlrLengthMismatch {
                expected,
                actual: posterior_llrs.len(),
            });
        }
        if let Some(index) = posterior_llrs.iter().position(|llr| !llr.is_finite()) {
            return Err(LdpcError::NonFiniteLlr { index });
        }
        if iterations > iteration_limit {
            return Err(LdpcError::DecodeIterationsExceedLimit {
                iterations,
                limit: iteration_limit,
            });
        }

        let word: Vec<_> = posterior_llrs
            .iter()
            .map(|&llr| if llr >= 0.0 { Bit::Zero } else { Bit::One })
            .collect();
        let syndrome = checks.syndrome(&word)?;
        let initial_syndrome = checks.syndrome(initial_word)?;
        let final_unsatisfied_checks = count_ones(&syndrome);
        let initial_unsatisfied_checks = count_ones(&initial_syndrome);

        if iterations == 0 && initial_word != word.as_slice() {
            return Err(LdpcError::DecodeResultZeroIterationsMismatch);
        }

        let status_is_consistent = match status {
            DecodeStatus::ParitySatisfied => final_unsatisfied_checks == 0,
            DecodeStatus::IterationLimit => {
                final_unsatisfied_checks != 0 && iterations == iteration_limit
            }
        };
        if !status_is_consistent {
            return Err(LdpcError::DecodeResultStatusMismatch);
        }

        let changed_bits = bit_distance(initial_word, &word);

        Ok(Self {
            word,
            posterior_llrs,
            syndrome,
            status,
            iterations,
            changed_bits,
            initial_unsatisfied_checks,
            final_unsatisfied_checks,
        })
    }

    /// Возвращает итоговое жёсткое слово в порядке столбцов исходной `H`.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает итоговые LLR в порядке столбцов исходной `H`.
    ///
    /// Все значения конечны. SPA дополнительно ограничивает их пределом из
    /// [`crate::DecoderConfig`].
    #[must_use]
    pub fn posterior_llrs(&self) -> &[f64] {
        &self.posterior_llrs
    }

    /// Возвращает синдром итогового слова в исходном порядке проверок.
    #[must_use]
    pub fn syndrome(&self) -> &[Bit] {
        &self.syndrome
    }

    /// Возвращает причину остановки декодера.
    #[must_use]
    pub fn status(&self) -> DecodeStatus {
        self.status
    }

    /// Возвращает число полностью выполненных итераций, заявленное декодером.
    ///
    /// [`try_new`](Self::try_new) проверяет число относительно объявленного
    /// предела, но не может подтвердить историю выполнения стороннего алгоритма.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Возвращает число позиций, изменившихся от начального решения к итоговому.
    ///
    /// Для SPA начальное решение строится после насыщения LLR пределом
    /// конфигурации и зануления стёртых позиций. В стороннем декодере оно
    /// задаётся при создании результата. Это число не является числом
    /// исправленных ошибок: без эталонного слова декодер не знает, какие
    /// изменения верны.
    #[must_use]
    pub fn changed_bits(&self) -> usize {
        self.changed_bits
    }

    /// Возвращает число нарушенных проверок начального решения. Для SPA оно
    /// строится после насыщения LLR пределом конфигурации и применения стираний;
    /// сторонний декодер передаёт начальное решение в [`try_new`](Self::try_new).
    #[must_use]
    pub fn initial_unsatisfied_checks(&self) -> usize {
        self.initial_unsatisfied_checks
    }

    /// Возвращает число нарушенных проверок итогового жёсткого решения.
    #[must_use]
    pub fn final_unsatisfied_checks(&self) -> usize {
        self.final_unsatisfied_checks
    }
}

/// Событие синхронного наблюдения за одним вызовом декодера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeEvent {
    /// Декодер подготовил начальное жёсткое решение.
    Started {
        /// Число нарушенных проверок начального решения.
        unsatisfied_checks: usize,
    },
    /// Полностью завершена одна итерация SPA.
    IterationFinished {
        /// Номер итерации, начиная с единицы.
        iteration: usize,
        /// Число нарушенных проверок после этой итерации.
        unsatisfied_checks: usize,
        /// Расстояние от начального решения до слова после этой итерации.
        changed_bits: usize,
    },
    /// Декодирование остановлено, показатели совпадают с результатом.
    Finished {
        /// Число полностью выполненных итераций.
        iterations: usize,
        /// Причина остановки декодера.
        status: DecodeStatus,
        /// Число нарушенных проверок итогового слова.
        unsatisfied_checks: usize,
        /// Расстояние от начального решения до итогового слова.
        changed_bits: usize,
    },
}

/// Получатель событий одного вызова декодера.
pub trait DecodeObserver {
    /// Обрабатывает событие до продолжения декодирования.
    fn on_event(&mut self, event: &DecodeEvent);
}

/// Общий контракт декодера одного кодового блока.
pub trait Decoder {
    /// Возвращает число LLR, ожидаемое для одного входного блока.
    fn codeword_len(&self) -> usize;

    /// Декодирует один блок и при необходимости синхронно отправляет события.
    ///
    /// Перед жёстким решением конечные LLR насыщаются пределом конфигурации,
    /// а стёртые позиции заменяются нулём. Решение равно [`Bit::Zero`] при LLR
    /// `>= 0.0` (включая `+0.0` и `-0.0`) и [`Bit::One`] при отрицательном LLR.
    ///
    /// Для корректного входа события идут в порядке `Started`, по одному
    /// `IterationFinished` на полностью завершённую итерацию, затем `Finished`.
    /// При ошибке входа событий нет. Проверки выполняются в порядке: длина LLR,
    /// конечность значений, границы индексов стираний, повторы индексов.
    /// Исчерпание бюджета итераций является обычным [`DecodeStatus`] в результате.
    fn decode(
        &mut self,
        input: DecodeInput<'_>,
        observer: Option<&mut dyn DecodeObserver>,
    ) -> Result<DecodeResult, LdpcError>;
}
