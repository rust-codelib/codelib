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
    /// Размер проверочной матрицы выходит за допустимые пределы.
    InvalidDimensions {
        /// Число проверок (строк).
        rows: usize,
        /// Число битов (столбцов).
        cols: usize,
    },
    /// Индекс бита находится за пределами строки проверочной матрицы.
    BitIndexOutOfBounds {
        /// Номер проверки, начиная с нуля.
        check: usize,
        /// Номер недопустимого бита.
        bit: usize,
        /// Число битов в матрице.
        bits: usize,
    },
    /// Бит повторяется в одной проверке.
    DuplicateBitIndex {
        /// Номер проверки, начиная с нуля.
        check: usize,
        /// Номер повторяющегося бита.
        bit: usize,
    },
    /// Длина слова не совпадает с числом столбцов проверочной матрицы.
    WordLengthMismatch {
        /// Ожидаемое число битов.
        expected: usize,
        /// Фактическое число битов во входном слове.
        actual: usize,
    },
    /// Предел LLR не является конечным числом из диапазона `(0, 20]`.
    InvalidLlrLimit,
    /// Число LLR не совпадает с числом столбцов проверочной матрицы.
    LlrLengthMismatch {
        /// Ожидаемое число LLR.
        expected: usize,
        /// Фактическое число LLR во входном массиве.
        actual: usize,
    },
    /// LLR с указанным индексом не является конечным числом.
    NonFiniteLlr {
        /// Индекс NaN или бесконечности во входном массиве.
        index: usize,
    },
    /// Индекс стирания находится за пределами кодового слова.
    ErasureIndexOutOfBounds {
        /// Недопустимый индекс бита.
        bit: usize,
        /// Число битов в кодовом слове.
        bits: usize,
    },
    /// Индекс стирания повторяется во входном списке.
    DuplicateErasureIndex {
        /// Повторяющийся индекс бита.
        bit: usize,
    },
    /// Ранг проверочной матрицы не позволяет построить систематический кодер.
    InvalidEncoderRank {
        /// Ранг проверочной матрицы.
        rank: usize,
        /// Число столбцов проверочной матрицы.
        cols: usize,
    },
    /// Длина сообщения не совпадает с числом информационных позиций кодера.
    MessageLengthMismatch {
        /// Ожидаемое число информационных битов.
        expected: usize,
        /// Фактическое число битов во входном сообщении.
        actual: usize,
    },
    /// Длины слова и эталона для сравнения битовых ошибок различаются.
    ReferenceLengthMismatch {
        /// Ожидаемая длина слова.
        expected: usize,
        /// Фактическая длина эталона.
        actual: usize,
    },
    /// Слово правильной длины нарушает проверки исходной матрицы.
    InvalidCodeword {
        /// Число нарушенных строк исходной проверочной матрицы.
        unsatisfied_checks: usize,
    },
    /// Размер внутреннего хранилища не представим типом `usize`.
    SizeOverflow,
    /// Операция плотной линейной алгебры завершилась ошибкой.
    LinearAlgebra {
        /// Исходная ошибка крейта `gf-linalg`.
        source: gf_linalg::LinalgError,
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
            Self::InvalidDimensions { rows, cols } => write!(
                f,
                "недопустимый размер проверочной матрицы: {rows} × {cols}"
            ),
            Self::BitIndexOutOfBounds { check, bit, bits } => write!(
                f,
                "индекс бита {bit} в проверке {check} выходит за границы 0..{bits}"
            ),
            Self::DuplicateBitIndex { check, bit } => {
                write!(f, "бит {bit} повторяется в проверке {check}")
            }
            Self::WordLengthMismatch { expected, actual } => write!(
                f,
                "длина слова {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::InvalidLlrLimit => write!(
                f,
                "предел LLR должен быть конечным и находиться в диапазоне (0, 20]"
            ),
            Self::LlrLengthMismatch { expected, actual } => write!(
                f,
                "число LLR {actual} не совпадает с ожидаемым числом {expected}"
            ),
            Self::NonFiniteLlr { index } => {
                write!(f, "LLR с индексом {index} не является конечным числом")
            }
            Self::ErasureIndexOutOfBounds { bit, bits } => {
                write!(f, "индекс стирания {bit} выходит за границы 0..{bits}")
            }
            Self::DuplicateErasureIndex { bit } => {
                write!(f, "индекс стирания {bit} указан повторно")
            }
            Self::InvalidEncoderRank { rank, cols } => write!(
                f,
                "недопустимый ранг {rank} для систематического кодера с {cols} столбцами"
            ),
            Self::MessageLengthMismatch { expected, actual } => write!(
                f,
                "длина сообщения {actual} не совпадает с ожидаемой длиной {expected}"
            ),
            Self::ReferenceLengthMismatch { expected, actual } => write!(
                f,
                "длина эталона {actual} не совпадает с ожидаемой длиной слова {expected}"
            ),
            Self::InvalidCodeword { unsatisfied_checks } => {
                write!(f, "кодовое слово нарушает {unsatisfied_checks} проверки")
            }
            Self::SizeOverflow => write!(f, "переполнение размера внутреннего хранилища"),
            Self::LinearAlgebra { source } => write!(f, "ошибка линейной алгебры: {source}"),
        }
    }
}

impl From<gf_linalg::LinalgError> for LdpcError {
    fn from(source: gf_linalg::LinalgError) -> Self {
        Self::LinearAlgebra { source }
    }
}

impl std::error::Error for LdpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LinearAlgebra { source } => Some(source),
            _ => None,
        }
    }
}
