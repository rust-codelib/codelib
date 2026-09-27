//! Ошибки линейной алгебры.

use core::fmt;

/// Ошибка при создании матрицы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinalgError {
    /// Размеры матрицы недопустимы.
    InvalidDimensions {
        /// Число строк.
        rows: usize,
        /// Число столбцов.
        cols: usize,
    },
    /// Число элементов не соответствует размерам матрицы.
    ElementCountMismatch {
        /// Ожидаемое число элементов.
        expected: usize,
        /// Фактическое число элементов.
        actual: usize,
    },
}

impl fmt::Display for LinalgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { rows, cols } => {
                write!(f, "недопустимые размеры матрицы: {rows} × {cols}")
            }
            Self::ElementCountMismatch { expected, actual } => {
                write!(
                    f,
                    "неверное число элементов матрицы: ожидалось {expected}, получено {actual}"
                )
            }
        }
    }
}

impl std::error::Error for LinalgError {}
