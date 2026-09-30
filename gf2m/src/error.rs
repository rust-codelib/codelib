//! Ошибки полевого слоя.

use core::fmt;

/// Ошибка конструирования элемента GF(2^m).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfError {
    /// Степень расширения m вне поддерживаемого диапазона 2..=16.
    UnsupportedDegree(u32),
    /// Значение не принадлежит полю GF(2^m): `value >= 2^m`.
    ElementOutOfRange {
        /// Степень расширения поля.
        m: u32,
        /// Отклонённое значение.
        value: u16,
    },
}

impl fmt::Display for GfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDegree(m) => {
                write!(
                    f,
                    "неподдерживаемая степень поля: m = {m}, допустимо 2..=16"
                )
            }
            Self::ElementOutOfRange { m, value } => {
                write!(
                    f,
                    "значение 0x{value:x} не принадлежит GF(2^{m}): должно быть меньше 2^{m}"
                )
            }
        }
    }
}

impl core::error::Error for GfError {}

#[cfg(test)]
mod tests {
    use super::GfError;

    // Display-строки проверяются в интеграционном тесте tests/runtime.rs.
    // Здесь — только трейты и структура.

    #[test]
    fn error_is_copy_and_eq() {
        let e = GfError::UnsupportedDegree(17);
        let copy = e;
        assert_eq!(e, copy);
        assert_eq!(
            GfError::ElementOutOfRange { m: 8, value: 256 },
            GfError::ElementOutOfRange { m: 8, value: 256 }
        );
    }

    #[test]
    fn error_is_plain_enum() {
        // Детальный разбор Display — в tests/runtime.rs; здесь — форма.
        assert!(matches!(
            GfError::UnsupportedDegree(17),
            GfError::UnsupportedDegree(17)
        ));
        assert!(matches!(
            GfError::ElementOutOfRange { m: 8, value: 256 },
            GfError::ElementOutOfRange { .. }
        ));
    }
}
