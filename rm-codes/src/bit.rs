//! Двоичный бит и простейшие операции над ним.

use crate::RmError;

/// Один двоичный символ.
///
/// Значение [`Bit::Zero`] соответствует `0`, [`Bit::One`] — `1`.
/// При преобразовании в `u8` получается `0` или `1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Bit {
    /// Нулевой бит.
    #[default]
    Zero,
    /// Единичный бит.
    One,
}

impl Bit {
    /// Возвращает противоположный бит.
    ///
    /// ```
    /// use rm_codes::Bit;
    ///
    /// assert_eq!(Bit::Zero.flip(), Bit::One);
    /// assert_eq!(Bit::One.flip(), Bit::Zero);
    /// ```
    #[must_use]
    pub const fn flip(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
        }
    }

    /// Возвращает `true`, если бит равен [`Bit::One`].
    #[must_use]
    pub const fn is_one(self) -> bool {
        matches!(self, Self::One)
    }

    /// Возвращает `true`, если бит равен [`Bit::Zero`].
    #[must_use]
    pub const fn is_zero(self) -> bool {
        matches!(self, Self::Zero)
    }

    /// Преобразует логическое значение в бит.
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::One
        } else {
            Self::Zero
        }
    }

    /// Преобразует бит в логическое значение.
    #[must_use]
    pub const fn to_bool(self) -> bool {
        self.is_one()
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

impl From<Bit> for bool {
    fn from(bit: Bit) -> Self {
        bit.to_bool()
    }
}

impl From<bool> for Bit {
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl TryFrom<u8> for Bit {
    type Error = RmError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            value => Err(RmError::InvalidBit { value }),
        }
    }
}

impl core::ops::Not for Bit {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.flip()
    }
}

impl core::ops::BitXor for Bit {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        if self == rhs {
            Self::Zero
        } else {
            Self::One
        }
    }
}

impl core::ops::BitXorAssign for Bit {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::Bit;

    #[test]
    fn flip_is_involution() {
        assert_eq!(Bit::Zero.flip().flip(), Bit::Zero);
        assert_eq!(Bit::One.flip().flip(), Bit::One);
    }

    #[test]
    fn conversion_round_trips() {
        for value in [Bit::Zero, Bit::One] {
            assert_eq!(Bit::try_from(u8::from(value)).unwrap(), value);
            assert_eq!(Bit::from(bool::from(value)), value);
        }
    }

    #[test]
    fn invalid_byte_is_rejected() {
        assert!(Bit::try_from(2).is_err());
    }

    #[test]
    fn xor_matches_bit_arithmetic() {
        assert_eq!(Bit::Zero ^ Bit::Zero, Bit::Zero);
        assert_eq!(Bit::Zero ^ Bit::One, Bit::One);
        assert_eq!(Bit::One ^ Bit::Zero, Bit::One);
        assert_eq!(Bit::One ^ Bit::One, Bit::Zero);
    }
}
