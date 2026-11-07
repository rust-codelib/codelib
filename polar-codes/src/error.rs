//! Ошибки библиотеки двоичных полярных кодов.

use core::fmt;

/// Ошибка при построении кода или выполнении кодирования и декодирования.
#[derive(Debug, PartialEq, Eq)]
pub enum PolarError {
    /// Значение не является двоичным битом.
    InvalidBit {
        /// Исходное значение, отличное от `0` и `1`.
        value: u8,
    },
    /// Длина блока не является степенью двойки из диапазона `1..=4096`.
    InvalidLength {
        /// Запрошенная длина.
        length: usize,
    },
    /// Индекс бита выходит за пределы блока.
    IndexOutOfBounds {
        /// Недопустимый индекс.
        index: usize,
        /// Длина блока.
        length: usize,
    },
    /// Информационная позиция указана повторно.
    DuplicateInformationIndex {
        /// Повторяющийся индекс.
        index: usize,
    },
    /// Число информационных битов больше длины блока.
    InvalidMessageLength {
        /// Запрошенное число информационных битов.
        message_len: usize,
        /// Длина блока.
        length: usize,
    },
    /// Вероятность стирания не лежит в отрезке `[0, 1]`.
    InvalidErasureProbability,
    /// Длина сообщения не совпадает с числом информационных позиций.
    MessageLengthMismatch {
        /// Ожидаемое число информационных битов.
        expected: usize,
        /// Фактическая длина сообщения.
        actual: usize,
    },
    /// Длина слова не совпадает с длиной блока.
    WordLengthMismatch {
        /// Ожидаемая длина.
        expected: usize,
        /// Фактическая длина.
        actual: usize,
    },
    /// После обратного преобразования замороженные биты не равны нулю.
    InvalidCodeword {
        /// Число замороженных позиций, равных единице.
        frozen_ones: usize,
    },
    /// Число LLR не совпадает с длиной блока.
    LlrLengthMismatch {
        /// Ожидаемое число LLR.
        expected: usize,
        /// Фактическое число LLR.
        actual: usize,
    },
    /// LLR с указанным индексом не является конечным числом.
    NonFiniteLlr {
        /// Индекс NaN или бесконечности.
        index: usize,
    },
}

impl fmt::Display for PolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBit { value } => {
                write!(f, "недопустимое значение бита: {value}")
            }
            Self::InvalidLength { length } => {
                write!(f, "недопустимая длина полярного блока: {length}")
            }
            Self::IndexOutOfBounds { index, length } => {
                write!(f, "индекс {index} выходит за границы блока длины {length}")
            }
            Self::DuplicateInformationIndex { index } => {
                write!(f, "информационная позиция {index} указана повторно")
            }
            Self::InvalidMessageLength {
                message_len,
                length,
            } => write!(
                f,
                "число информационных битов {message_len} больше длины блока {length}"
            ),
            Self::InvalidErasureProbability => {
                write!(f, "вероятность стирания должна лежать в отрезке [0, 1]")
            }
            Self::MessageLengthMismatch { expected, actual } => write!(
                f,
                "длина сообщения {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::WordLengthMismatch { expected, actual } => write!(
                f,
                "длина слова {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::InvalidCodeword { frozen_ones } => write!(
                f,
                "кодовое слово нарушает заморозку: единиц на замороженных позициях {frozen_ones}"
            ),
            Self::LlrLengthMismatch { expected, actual } => write!(
                f,
                "число LLR {actual} не совпадает с ожидаемым числом {expected}"
            ),
            Self::NonFiniteLlr { index } => {
                write!(f, "LLR с индексом {index} не является конечным числом")
            }
        }
    }
}

impl std::error::Error for PolarError {}
