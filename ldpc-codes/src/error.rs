//! Ошибки библиотеки двоичных LDPC-кодов.

use core::fmt;

/// Ошибка при преобразовании или выполнении операции LDPC-кода.
#[derive(Debug, PartialEq, Eq)]
pub enum LdpcError {
    /// Значение не является двоичным битом.
    InvalidBit {
        /// Исходное значение, отличное от `0` и `1`.
        value: u8,
    },
    /// Элемент вектора не равен нулю или единице поля GF(2).
    InvalidFieldElement {
        /// Индекс некорректного элемента в векторе, начиная с нуля.
        index: usize,
        /// Упакованное представление элемента.
        value: u64,
    },
}

impl fmt::Display for LdpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBit { value } => {
                write!(f, "недопустимое значение бита: {value}")
            }
            Self::InvalidFieldElement { index, value } => {
                write!(
                    f,
                    "элемент вектора с индексом {index} не является битом GF(2): {value}"
                )
            }
        }
    }
}

impl std::error::Error for LdpcError {}
