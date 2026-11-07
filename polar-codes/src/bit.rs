//! Тип двоичного бита и его преобразования.

use crate::PolarError;

/// Один бит двоичного значения.
///
/// При преобразовании в `u8` вариант [`Zero`](Self::Zero) становится `0`, а
/// [`One`](Self::One) — `1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bit {
    /// Нулевой бит.
    Zero,
    /// Единичный бит.
    One,
}

impl Bit {
    pub(crate) fn xor(self, other: Self) -> Self {
        match (self, other) {
            (Self::Zero, Self::One) | (Self::One, Self::Zero) => Self::One,
            _ => Self::Zero,
        }
    }
}

impl TryFrom<u8> for Bit {
    type Error = PolarError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            value => Err(PolarError::InvalidBit { value }),
        }
    }
}

impl From<Bit> for u8 {
    fn from(bit: Bit) -> Self {
        match bit {
            Bit::Zero => 0,
            Bit::One => 1,
        }
    }
}
