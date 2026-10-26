//! Ошибки, исходы и диагностические события BCH-кода.

use coding_core::SizeError;
use core::fmt;

/// Ошибка параметров, представления или ресурсов BCH-кода.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BchError {
    /// Параметры кода несовместимы между собой или с полем.
    InvalidParameters,
    /// Элемент `α = x` не порождает всю мультипликативную группу поля.
    NonPrimitiveField,
    /// Переданный порождающий многочлен не соответствует конфигурации.
    InvalidGenerator,
    /// Длина входного или выходного буфера не соответствует конфигурации.
    BufferLength {
        /// Требуемая длина в элементах.
        expected: usize,
        /// Полученная длина в элементах.
        actual: usize,
    },
    /// Рабочий буфер короче требуемого.
    WorkspaceTooSmall {
        /// Требуемое число элементов.
        required: usize,
        /// Полученное число элементов.
        actual: usize,
    },
    /// В неиспользуемых старших битах последнего байта находится единица.
    NonZeroPaddingBits,
    /// Размер сообщения нарушает ограничение унифицированного API.
    Size(SizeError),
}

impl fmt::Display for BchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameters => write!(f, "недопустимые параметры BCH-кода"),
            Self::NonPrimitiveField => write!(f, "полином поля не задаёт примитивный элемент α"),
            Self::InvalidGenerator => write!(f, "неверный порождающий многочлен BCH-кода"),
            Self::BufferLength { expected, actual } => write!(
                f,
                "неверная длина буфера: ожидалось {expected}, получено {actual}"
            ),
            Self::WorkspaceTooSmall { required, actual } => write!(
                f,
                "недостаточный рабочий буфер: требуется {required}, получено {actual}"
            ),
            Self::NonZeroPaddingBits => write!(f, "ненулевые неиспользуемые биты"),
            Self::Size(error) => write!(f, "недопустимый размер сообщения: {error:?}"),
        }
    }
}

impl core::error::Error for BchError {}

impl From<SizeError> for BchError {
    fn from(value: SizeError) -> Self {
        Self::Size(value)
    }
}

/// Проверки успешно полученного кандидата.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BchChecks {
    /// Были ли исходные синдромы нулевыми.
    pub initial_syndromes_zero: bool,
    /// Нулевые ли синдромы после исправления.
    pub final_syndromes_zero: bool,
    /// Число инвертированных битов.
    pub corrected_bits: usize,
}

/// Причина завершения корректного вызова без кандидата.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BchStopReason {
    /// Локатор имеет недопустимую степень или неверное число корней.
    InvalidLocator {
        /// Степень локатора.
        degree: usize,
        /// Число найденных корней.
        roots: usize,
    },
    /// Локатор указывает на позицию, удалённую при укорочении.
    ShortenedPosition {
        /// Полиномиальная позиция корня.
        position: usize,
    },
    /// После предполагаемого исправления синдромы не обнулились.
    VerificationFailed,
}

/// Результат низкоуровневого исправления слова.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BchCorrection {
    /// Получено проверенное кодовое слово.
    Candidate(BchChecks),
    /// Согласованный кандидат не найден.
    Stopped(BchStopReason),
}

/// Событие унифицированного кодека.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BchEvent {
    /// Кодирование завершено.
    Encoded,
    /// Синдромы вычислены.
    SyndromesComputed {
        /// Равны ли нулю все синдромы.
        all_zero: bool,
    },
    /// Попытка декодирования завершена.
    DecodingFinished {
        /// Число исправленных битов.
        corrected_bits: usize,
        /// Прошёл ли кандидат повторную проверку синдромов.
        verified: bool,
    },
}
