//! Ошибки полевого слоя GF(p^m).

use core::fmt;

/// Ошибка конструирования поля или элемента GF(p^m).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfError {
    /// Характеристика не является простым числом.
    NotPrime(u32),
    /// Степень расширения вне поддерживаемого диапазона 1..=64.
    UnsupportedDegree(u32),
    /// Порядок поля `p^m` превосходит 2^64.
    OrderTooLarge {
        /// Характеристика.
        p: u32,
        /// Степень расширения.
        m: u32,
    },
    /// Упакованный модуль нарушает структуру или запрещён:
    /// `poly >= p^m` (мономиальный многочлен степени m обязан
    /// упаковываться в m базис-p цифр), либо `poly = 0`
    /// (модуль `x^m` приводим при `m >= 2`, а при `m = 1`
    /// даёт нулевой `alpha()`).
    InvalidPolynomial {
        /// Характеристика.
        p: u32,
        /// Степень расширения.
        m: u32,
        /// Отклонённое упакованное значение модуля.
        poly: u128,
    },
    /// Упакованное значение элемента не меньше порядка поля.
    ElementOutOfRange {
        /// Отклонённое значение.
        value: u64,
        /// Порядок поля.
        order: u128,
    },
    /// Коэффициент элемента вне диапазона 0..p.
    CoefficientOutOfRange {
        /// Индекс коэффициента.
        index: usize,
        /// Отклонённое значение.
        coeff: u32,
        /// Характеристика.
        p: u32,
    },
    /// Несовместная длина среза коэффициентов (ожидалась `m`).
    InvalidSliceLen {
        /// Ожидаемая длина.
        expected: usize,
        /// Фактическая длина.
        got: usize,
    },
}

impl fmt::Display for GfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPrime(p) => {
                write!(f, "характеристика {p} не является простым числом")
            }
            Self::UnsupportedDegree(m) => {
                write!(
                    f,
                    "неподдерживаемая степень расширения: m = {m}, допустимо 1..=64"
                )
            }
            Self::OrderTooLarge { p, m } => {
                write!(f, "порядок поля {p}^{m} превосходит 2^64")
            }
            Self::InvalidPolynomial { p, m, poly } => {
                if *poly == 0 {
                    write!(
                        f,
                        "модуль 0 непригоден для GF({p}^{m}): при m >= 2 это x^{m} \
                         (приводимый), при m = 1 alpha() даёт нуль"
                    )
                } else {
                    write!(
                        f,
                        "модуль 0x{poly:x} не является мономиальным многочленом \
                         степени {m} над GF({p}): упакованное значение должно быть меньше {p}^{m}"
                    )
                }
            }
            Self::ElementOutOfRange { value, order } => {
                write!(
                    f,
                    "значение {value} не принадлежит полю: должно быть меньше {order}"
                )
            }
            Self::CoefficientOutOfRange { index, coeff, p } => {
                write!(
                    f,
                    "коэффициент[{index}] = {coeff} вне GF({p}): должен быть меньше {p}"
                )
            }
            Self::InvalidSliceLen { expected, got } => {
                write!(
                    f,
                    "неверная длина среза коэффициентов: ожидалась {expected}, получена {got}"
                )
            }
        }
    }
}

impl core::error::Error for GfError {}

#[cfg(test)]
mod tests {
    use super::GfError;

    // Display-строки проверяются в интеграционном тесте tests/runtime.rs;
    // здесь — только трейты и структура.

    #[test]
    fn error_is_copy_and_eq() {
        let e = GfError::NotPrime(4);
        let copy = e;
        assert_eq!(e, copy);
        assert_eq!(
            GfError::OrderTooLarge { p: 3, m: 41 },
            GfError::OrderTooLarge { p: 3, m: 41 }
        );
    }

    #[test]
    fn error_is_plain_enum() {
        assert!(matches!(GfError::NotPrime(4), GfError::NotPrime(4)));
        assert!(matches!(
            GfError::InvalidPolynomial {
                p: 3,
                m: 2,
                poly: 100
            },
            GfError::InvalidPolynomial { .. }
        ));
    }
}
