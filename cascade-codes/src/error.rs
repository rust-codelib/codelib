//! Ошибки каскадных кодов.

use core::fmt;

use gf_linalg::LinalgError;

/// Ошибка построения кода, кодирования, восстановления данных или узла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeError {
    /// Параметры системы нарушают `1 <= k <= d < n`.
    InvalidSystem {
        /// Число узлов.
        n: usize,
        /// Число узлов, достаточное для восстановления данных.
        k: usize,
        /// Число помощников при восстановлении узла.
        d: usize,
    },
    /// Режим нарушает `1 <= mu <= k`.
    InvalidMode {
        /// Запрошенный режим.
        mu: usize,
        /// Параметр `k`.
        k: usize,
    },
    /// В поле меньше элементов, чем узлов: точки Вандермонда не различны.
    FieldTooSmall {
        /// Число узлов.
        n: usize,
        /// Порядок поля.
        order: u128,
    },
    /// Размер конструкции превышает предел крейта.
    SizeLimitExceeded {
        /// Что именно слишком велико.
        what: &'static str,
        /// Фактическое значение (насыщается при переполнении).
        actual: u128,
        /// Допустимый максимум.
        max: usize,
    },
    /// Длина блока сообщения не равна `F`.
    MessageLengthMismatch {
        /// Ожидаемое число символов.
        expected: usize,
        /// Фактическое число символов.
        actual: usize,
    },
    /// Байтовое сообщение длиннее допустимого.
    MessageTooLong {
        /// Допустимый максимум в байтах.
        max: usize,
        /// Фактическая длина в байтах.
        actual: usize,
    },
    /// Длина содержимого узла или пакета помощника неверна.
    ShareLengthMismatch {
        /// Номер узла.
        node: usize,
        /// Ожидаемое число символов.
        expected: usize,
        /// Фактическое число символов.
        actual: usize,
    },
    /// Номер узла вне диапазона `0..n`.
    NodeOutOfRange {
        /// Номер узла.
        node: usize,
        /// Число узлов.
        n: usize,
    },
    /// Узел указан повторно.
    DuplicateNode {
        /// Номер повторяющегося узла.
        node: usize,
    },
    /// Передано меньше узлов, чем нужно.
    NotEnoughShares {
        /// Нужное число узлов.
        required: usize,
        /// Переданное число узлов.
        actual: usize,
    },
    /// Число помощников не равно `d`.
    WrongHelperCount {
        /// Ожидаемое число помощников `d`.
        expected: usize,
        /// Переданное число помощников.
        actual: usize,
    },
    /// Отказавший узел указан среди помощников.
    HelperIsFailedNode {
        /// Номер узла.
        node: usize,
    },
    /// Символ не помещается в число бит, отведённых при упаковке байтов.
    InvalidPackedSymbol {
        /// Позиция символа в развёрнутой последовательности.
        position: usize,
    },
    /// Ошибка линейной алгебры (например, вырожденная подматрица кодера).
    Linalg(LinalgError),
    /// Нарушен внутренний инвариант алгоритма. Сообщите об ошибке.
    Internal(&'static str),
}

impl From<LinalgError> for CascadeError {
    fn from(error: LinalgError) -> Self {
        Self::Linalg(error)
    }
}

impl fmt::Display for CascadeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSystem { n, k, d } => write!(
                f,
                "параметры (n, k, d) = ({n}, {k}, {d}) нарушают 1 <= k <= d < n"
            ),
            Self::InvalidMode { mu, k } => write!(f, "режим mu = {mu} вне диапазона 1..={k}"),
            Self::FieldTooSmall { n, order } => {
                write!(f, "поле порядка {order} меньше числа узлов {n}")
            }
            Self::SizeLimitExceeded { what, actual, max } => {
                write!(f, "{what}: {actual} превышает предел {max}")
            }
            Self::MessageLengthMismatch { expected, actual } => write!(
                f,
                "длина блока сообщения {actual}, ожидалось {expected} символов"
            ),
            Self::MessageTooLong { max, actual } => {
                write!(f, "сообщение длиной {actual} байт длиннее {max} байт")
            }
            Self::ShareLengthMismatch {
                node,
                expected,
                actual,
            } => write!(
                f,
                "узел {node}: длина данных {actual}, ожидалось {expected} символов"
            ),
            Self::NodeOutOfRange { node, n } => write!(f, "номер узла {node} вне 0..{n}"),
            Self::DuplicateNode { node } => write!(f, "узел {node} указан повторно"),
            Self::NotEnoughShares { required, actual } => {
                write!(f, "передано {actual} узлов, нужно не меньше {required}")
            }
            Self::WrongHelperCount { expected, actual } => {
                write!(f, "передано {actual} помощников, нужно ровно {expected}")
            }
            Self::HelperIsFailedNode { node } => {
                write!(f, "отказавший узел {node} указан среди помощников")
            }
            Self::InvalidPackedSymbol { position } => write!(
                f,
                "символ {position} не помещается в разрядность упаковки байтов"
            ),
            Self::Linalg(error) => write!(f, "ошибка линейной алгебры: {error}"),
            Self::Internal(what) => write!(f, "нарушен внутренний инвариант: {what}"),
        }
    }
}

impl std::error::Error for CascadeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Linalg(error) => Some(error),
            _ => None,
        }
    }
}
