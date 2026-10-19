//! Кодер двоичного кода Рида–Маллера.

use crate::layout::RmLayout;
use crate::{Bit, RmConfig, RmError};

/// Общий контракт кодера двоичного блока.
///
/// Длина сообщения и кодового слова определяются конкретным кодом.
/// Методы читают входные срезы и не изменяют состояние кодера.
pub trait Encoder {
    /// Возвращает число информационных битов `k`.
    fn message_len(&self) -> usize;

    /// Возвращает длину кодового слова `n`.
    fn codeword_len(&self) -> usize;

    /// Кодирует сообщение длины `k` в слово длины `n`.
    ///
    /// Для `RM(r, m)` сообщение — коэффициенты булева полинома в порядке
    /// [`RmConfig`]: свободный член, коэффициенты степени 1, затем степени 2
    /// и так далее. Кодирование несистематическое.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::MessageLengthMismatch`], если длина сообщения
    /// не равна [`message_len`](Self::message_len).
    fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, RmError>;

    /// Извлекает коэффициенты сообщения из кодового слова длины `n`.
    ///
    /// Метод не исправляет ошибки. Если слово не принадлежит коду,
    /// возвращается [`RmError::InvalidCodeword`].
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::WordLengthMismatch`] при неверной длине и
    /// [`RmError::InvalidCodeword`], если слово не является кодовым.
    fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, RmError>;
}

/// Кодер кода `RM(r, m)`.
#[derive(Clone, Debug)]
pub struct RmEncoder {
    config: RmConfig,
    layout: RmLayout,
}

impl RmEncoder {
    /// Создаёт кодер по проверенным параметрам.
    pub fn new(config: RmConfig) -> Self {
        let layout = RmLayout::new(config);
        Self { config, layout }
    }

    /// Возвращает параметры кода.
    #[must_use]
    pub fn config(&self) -> RmConfig {
        self.config
    }

    /// Возвращает генераторную матрицу `G` размера `k × n`.
    ///
    /// Метод диагностический: для `m = 12, r = 12` матрица занимает около
    /// 16 МиБ (`Bit` хранится как отдельное значение). Внутри кодирования
    /// матрица не используется.
    #[must_use]
    pub fn generator_matrix(&self) -> Vec<Vec<Bit>> {
        self.layout.generator_rows()
    }
}

impl Encoder for RmEncoder {
    fn message_len(&self) -> usize {
        self.layout.k()
    }

    fn codeword_len(&self) -> usize {
        self.layout.n()
    }

    fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, RmError> {
        let expected = self.message_len();
        if message.len() != expected {
            return Err(RmError::MessageLengthMismatch {
                expected,
                actual: message.len(),
            });
        }

        Ok(self.layout.encode(message))
    }

    fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, RmError> {
        let expected = self.codeword_len();
        if word.len() != expected {
            return Err(RmError::WordLengthMismatch {
                expected,
                actual: word.len(),
            });
        }

        let message = self.layout.extract_coefficients(word);
        if self.encode(&message)? != word {
            return Err(RmError::InvalidCodeword);
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{Encoder, RmEncoder};
    use crate::{Bit, RmConfig, RmError};

    fn bits(text: &str) -> Vec<Bit> {
        text.bytes()
            .map(|byte| Bit::try_from(byte - b'0').unwrap())
            .collect()
    }

    #[test]
    fn encodes_first_pdf_example() {
        let encoder = RmEncoder::new(RmConfig::new(1, 4).unwrap());
        let message = bits("10011");

        let word = encoder.encode(&message).unwrap();

        assert_eq!(word, bits("1001100110011001"));
    }

    #[test]
    fn encodes_second_pdf_example() {
        let encoder = RmEncoder::new(RmConfig::new(2, 4).unwrap());
        let message = bits("11100011001");

        let word = encoder.encode(&message).unwrap();

        assert_eq!(word, bits("1110000101111000"));
    }

    #[test]
    fn extracts_messages_from_pdf_examples() {
        let encoder = RmEncoder::new(RmConfig::new(2, 4).unwrap());
        let word = bits("1110000101111000");

        assert_eq!(encoder.extract_message(&word).unwrap(), bits("11100011001"));
    }

    #[test]
    fn rejects_wrong_message_length() {
        let encoder = RmEncoder::new(RmConfig::new(1, 4).unwrap());

        assert_eq!(
            encoder.encode(&bits("1001")),
            Err(RmError::MessageLengthMismatch {
                expected: 5,
                actual: 4
            })
        );
    }

    #[test]
    fn rejects_wrong_word_length() {
        let encoder = RmEncoder::new(RmConfig::new(1, 4).unwrap());

        assert_eq!(
            encoder.extract_message(&bits("1001")),
            Err(RmError::WordLengthMismatch {
                expected: 16,
                actual: 4
            })
        );
    }

    #[test]
    fn rejects_word_outside_the_code() {
        let encoder = RmEncoder::new(RmConfig::new(1, 4).unwrap());
        let mut word = vec![Bit::Zero; encoder.codeword_len()];
        word[0] = Bit::One;

        assert_eq!(
            encoder.extract_message(&word),
            Err(RmError::InvalidCodeword)
        );
    }

    #[test]
    fn generator_matrix_has_expected_shape_and_rows() {
        let encoder = RmEncoder::new(RmConfig::new(1, 2).unwrap());
        let matrix = encoder.generator_matrix();

        assert_eq!(matrix.len(), 3);
        assert!(matrix.iter().all(|row| row.len() == 4));
        assert_eq!(matrix[0], bits("1111"));
        assert_eq!(matrix[1], bits("0011"));
        assert_eq!(matrix[2], bits("0101"));
    }

    #[test]
    fn full_code_accepts_every_word() {
        let encoder = RmEncoder::new(RmConfig::new(4, 4).unwrap());
        let word = bits("1010010111000010");

        assert_eq!(encoder.extract_message(&word).unwrap().len(), 16);
        assert_eq!(
            encoder
                .encode(&encoder.extract_message(&word).unwrap())
                .unwrap(),
            word
        );
    }
}
