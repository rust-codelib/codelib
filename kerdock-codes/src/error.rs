use std::fmt;

/// Parameter and single-block processing errors for Kerdock codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KerdockError {
    /// The codeword bit length is not 16, 64, 256, or 1024.
    UnsupportedBlockLength {
        /// The supplied codeword length in bits.
        bits: usize,
    },
    /// The message length is not `2m`.
    MessageLengthMismatch {
        /// The required number of bits.
        expected: usize,
        /// The supplied number of bits.
        actual: usize,
    },
    /// The word length is not `2^m`.
    WordLengthMismatch {
        /// The required number of bits.
        expected: usize,
        /// The supplied number of bits.
        actual: usize,
    },
    /// The word is not a codeword during exact message extraction.
    InvalidCodeword,
    /// No codeword lies within the correction radius.
    NoCodewordWithinRadius {
        /// The actual distance to the nearest codeword.
        nearest_distance: usize,
        /// The permitted correction radius.
        radius: usize,
    },
}

impl fmt::Display for KerdockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBlockLength { bits } => {
                write!(
                    f,
                    "unsupported block length {bits} bits: expected 16, 64, 256, or 1024 bits"
                )
            }
            Self::MessageLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "неверная длина сообщения: ожидалось {expected} битов, получено {actual}"
                )
            }
            Self::WordLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "неверная длина слова: ожидалось {expected} битов, получено {actual}"
                )
            }
            Self::InvalidCodeword => write!(f, "слово не принадлежит коду Кердока"),
            Self::NoCodewordWithinRadius {
                nearest_distance,
                radius,
            } => {
                write!(
                    f,
                    "ближайшее кодовое слово на расстоянии {nearest_distance}, радиус {radius}"
                )
            }
        }
    }
}

impl std::error::Error for KerdockError {}
