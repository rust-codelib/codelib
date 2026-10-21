//! Ошибки создания и чтения многочленов.

use core::fmt;

/// Ошибка создания или текстового ввода многочлена.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialError {
    /// Массив коэффициентов превышает допустимый объём памяти.
    CoefficientLimitExceeded {
        /// Число переданных коэффициентов, включая хвостовые нули.
        coefficients: usize,
        /// Максимальное число коэффициентов для этого типа поля.
        max_coefficients: usize,
    },
    /// Текст не начинается с заголовка `polynomial`.
    InvalidTextHeader,
    /// Число коэффициентов отсутствует, неверно записано или не входит в `usize`.
    InvalidTextLength,
    /// Число коэффициентов в тексте не совпадает с объявленным.
    TextCoefficientCountMismatch {
        /// Объявленное число коэффициентов.
        expected: usize,
        /// Фактическое число токенов коэффициентов.
        actual: usize,
    },
    /// Коэффициент имеет неверный формат или не принадлежит полю.
    InvalidTextCoefficient {
        /// Индекс коэффициента, начиная с нуля (также степень его монома).
        index: usize,
    },
    /// Исходное или восстановленное сообщение превышает предел 4096 байт.
    MessageTooLong {
        /// Длина сообщения в байтах.
        bytes: usize,
    },
    /// Число коэффициентов не кратно восьми для битового представления сообщения.
    InvalidMessageCoefficientCount {
        /// Число сохранённых коэффициентов.
        coefficients: usize,
    },
    /// Коэффициент битового представления отличается от нуля и единицы поля.
    InvalidMessageCoefficient {
        /// Индекс первого неверного коэффициента, начиная с нуля.
        index: usize,
    },
}

impl fmt::Display for PolynomialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoefficientLimitExceeded {
                coefficients,
                max_coefficients,
            } => write!(
                f,
                "превышен предел коэффициентов многочлена: {coefficients} > {max_coefficients}"
            ),
            Self::InvalidTextHeader => write!(
                f,
                "неверный заголовок текстового формата: ожидалось `polynomial`"
            ),
            Self::InvalidTextLength => write!(f, "неверное число коэффициентов в заголовке"),
            Self::TextCoefficientCountMismatch { expected, actual } => write!(
                f,
                "неверное число коэффициентов текста: ожидалось {expected}, получено {actual}"
            ),
            Self::InvalidTextCoefficient { index } => {
                write!(f, "неверный текстовый коэффициент с индексом {index}")
            }
            Self::MessageTooLong { bytes } => write!(
                f,
                "сообщение слишком длинное: {bytes} байт, предел {} байт",
                crate::MAX_MESSAGE_BYTES
            ),
            Self::InvalidMessageCoefficientCount { coefficients } => write!(
                f,
                "число коэффициентов битового сообщения не кратно восьми: {coefficients}"
            ),
            Self::InvalidMessageCoefficient { index } => write!(
                f,
                "коэффициент сообщения с индексом {index} не равен нулю или единице поля"
            ),
        }
    }
}

impl std::error::Error for PolynomialError {}
