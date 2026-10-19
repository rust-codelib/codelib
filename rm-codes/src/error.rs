//! Ошибки крейта кодов Рида–Маллера.

use core::fmt;

/// Ошибка параметров, входа или операции над кодом Рида–Маллера.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RmError {
    /// Значение не является двоичным битом.
    InvalidBit {
        /// Исходное значение, отличное от `0` и `1`.
        value: u8,
    },
    /// Параметры `(r, m)` не задают допустимый код.
    InvalidParameters {
        /// Порядок кода `r`.
        r: usize,
        /// Параметр длины `m`.
        m: usize,
    },
    /// Длина сообщения не совпадает с размерностью кода.
    MessageLengthMismatch {
        /// Ожидаемое число информационных битов.
        expected: usize,
        /// Фактическое число битов.
        actual: usize,
    },
    /// Длина слова не совпадает с длиной кода.
    WordLengthMismatch {
        /// Ожидаемая длина слова.
        expected: usize,
        /// Фактическая длина слова.
        actual: usize,
    },
    /// Длины слова и эталона для сравнения различаются.
    ReferenceLengthMismatch {
        /// Ожидаемая длина.
        expected: usize,
        /// Фактическая длина эталона.
        actual: usize,
    },
    /// Слово не принадлежит коду Рида–Маллера.
    InvalidCodeword,
    /// Размер внутреннего хранилища не представим типом `usize`.
    SizeOverflow,
}

impl fmt::Display for RmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBit { value } => {
                write!(f, "недопустимое значение бита: {value}")
            }
            Self::InvalidParameters { r, m } => write!(
                f,
                "недопустимые параметры кода Рида–Маллера: r = {r}, m = {m}; требуется 1 <= m <= 12 и 0 <= r <= m"
            ),
            Self::MessageLengthMismatch { expected, actual } => write!(
                f,
                "длина сообщения {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::WordLengthMismatch { expected, actual } => write!(
                f,
                "длина слова {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::ReferenceLengthMismatch { expected, actual } => write!(
                f,
                "длина эталона {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::InvalidCodeword => write!(f, "слово не принадлежит коду Рида–Маллера"),
            Self::SizeOverflow => write!(f, "переполнение размера внутреннего хранилища"),
        }
    }
}

impl std::error::Error for RmError {}
