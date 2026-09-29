//! Ошибки линейной алгебры.

use core::fmt;

/// Ошибка при создании матрицы или выполнении операции линейной алгебры.
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
    /// Длины складываемых векторов не совпадают.
    VectorLengthMismatch {
        /// Длина левого вектора.
        left: usize,
        /// Длина правого вектора.
        right: usize,
    },
    /// Запрошенная операция пока не реализована.
    NotImplemented {
        /// Название операции, доступное во время всей жизни программы.
        operation: &'static str,
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
            Self::VectorLengthMismatch { left, right } => {
                write!(
                    f,
                    "длины векторов не совпадают: слева {left}, справа {right}"
                )
            }
            Self::NotImplemented { operation } => {
                write!(f, "операция не реализована: {operation}")
            }
        }
    }
}

impl std::error::Error for LinalgError {}
