use crate::KerdockError;

/// A supported codeword length in bits, not an exponent or byte count.
///
/// Use [`TryFrom<usize>`] to validate a bit length supplied at runtime.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KerdockSize {
    /// A 16-bit codeword carrying 8 message bits.
    Bits16,
    /// A 64-bit codeword carrying 12 message bits.
    Bits64,
    /// A 256-bit codeword carrying 16 message bits.
    Bits256,
    /// A 1024-bit codeword carrying 20 message bits.
    Bits1024,
}

impl KerdockSize {
    pub(crate) fn exponent(self) -> u32 {
        match self {
            Self::Bits16 => 4,
            Self::Bits64 => 6,
            Self::Bits256 => 8,
            Self::Bits1024 => 10,
        }
    }
}

impl TryFrom<usize> for KerdockSize {
    type Error = KerdockError;

    /// Converts an actual codeword bit length, not an exponent or byte count.
    ///
    /// # Errors
    /// Returns [`KerdockError::UnsupportedBlockLength`] unless `bits` is
    /// exactly 16, 64, 256, or 1024.
    fn try_from(bits: usize) -> Result<Self, Self::Error> {
        match bits {
            16 => Ok(Self::Bits16),
            64 => Ok(Self::Bits64),
            256 => Ok(Self::Bits256),
            1024 => Ok(Self::Bits1024),
            _ => Err(KerdockError::UnsupportedBlockLength { bits }),
        }
    }
}
