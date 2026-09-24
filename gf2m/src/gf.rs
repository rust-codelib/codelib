//! Константно-генериковое ядро: [`Gf<N, POLY>`](Gf) — элемент поля
//! GF(2^m) с параметрами-константами.
//!
//! `N = 2^m` — порядок поля (m = 2..=16), `POLY` — образующий
//! многочлен степени m со старшим членом: `N <= POLY < 2N`.
//! Для ходовых степеней есть псевдонимы: [`Gf256`] (поле AES),
//! [`Gf16`], … Полный список — в корне крейта.
//!
//! Арифметика не паникует; значения вне диапазона поля приводятся
//! по модулю `POLY`. Соглашения нуля: `inv(0) = 0`, `x / 0 = 0`,
//! `0^0 = 1`.
//!
//! # Пример
//! ```rust
//! use gf2m::Gf256;
//!
//! let a = Gf256::new(0x57);
//! assert_eq!(a * Gf256::new(0x02), Gf256::new(0xAE)); // xtime из AES
//! assert_eq!(a.inv() * a, Gf256::new(1));
//! assert_eq!(a.sqrt().sqr(), a);
//! ```

use core::fmt;
use core::ops::Neg;

use crate::poly::{clmul, poly_mod};

/// Элемент поля GF(2^m) с константными параметрами `N = 2^m` и `POLY`.
///
/// Значение хранится в `u16`; инвариант: `value < N`. Нарушение
/// невозможно через [`new`](Self::new) (проверка в debug) и
/// бессмысленно через [`reduce`](Self::reduce) — все операции
/// дополнительно приводят входы по модулю `POLY`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Gf<const N: usize, const POLY: u32>(u16);

impl<const N: usize, const POLY: u32> Gf<N, POLY> {
    /// Степень расширения m (`N = 2^m`). Вычисление проверяет
    /// параметры типа при первом использовании.
    pub const M: u32 = {
        assert!(
            N >= 4 && N <= 65536 && N.is_power_of_two(),
            "N должен быть 2^m для m = 2..=16"
        );
        assert!(
            POLY >= N as u32 && POLY < 2 * N as u32,
            "POLY должен иметь степень m: N <= POLY < 2N"
        );
        N.trailing_zeros()
    };

    /// Элемент из целого значения. Контракт: `value < N`
    /// (проверяется в debug-сборках).
    #[must_use]
    pub const fn new(value: u16) -> Self {
        debug_assert!(
            (value as usize) < N,
            "значение вне поля: используйте Gf::reduce"
        );
        Self(value)
    }

    /// Элемент из произвольного целого: значение приводится
    /// по модулю `POLY` (полиномиальная редукция).
    #[must_use]
    pub const fn reduce(value: u16) -> Self {
        if (value as usize) >= N {
            Self(poly_mod(value as u32, POLY) as u16)
        } else {
            Self(value)
        }
    }

    /// Целое представление элемента (биты — коэффициенты многочлена).
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Нулевой элемент.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Единица.
    #[must_use]
    pub const fn one() -> Self {
        Self(1)
    }

    /// Примитивный элемент α = t (многочлен «x»): порядок группы равен
    /// `N − 1` для примитивного `POLY`.
    #[must_use]
    pub const fn alpha() -> Self {
        Self(2)
    }

    /// Проверка на ноль.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Приведение значения к диапазону поля (по модулю `POLY`).
    const fn fit(v: u16) -> u16 {
        if (v as usize) >= N {
            poly_mod(v as u32, POLY) as u16
        } else {
            v
        }
    }

    /// Сложение (= вычитание): побитовое XOR.
    #[must_use]
    pub const fn add(self, rhs: Self) -> Self {
        Self(Self::fit(self.0) ^ Self::fit(rhs.0))
    }

    /// Умножение: бесструктурное произведение с полиномиальной редукцией.
    #[must_use]
    pub const fn mul(self, rhs: Self) -> Self {
        let p = clmul(Self::fit(self.0) as u32, Self::fit(rhs.0) as u32);
        Self(poly_mod(p, POLY) as u16)
    }

    /// Возведение в квадрат.
    #[must_use]
    pub const fn sqr(self) -> Self {
        self.mul(self)
    }

    /// Обратный элемент по Ферма: `a^(N−2)`. Соглашение: `inv(0) = 0`.
    #[must_use]
    pub const fn inv(self) -> Self {
        let a = Self::fit(self.0);
        if a == 0 {
            return Self(0);
        }
        let mut r = Self(1);
        let mut base = Self(a);
        let mut e = (N as u32) - 2;
        while e > 0 {
            if e & 1 == 1 {
                r = r.mul(base);
            }
            base = base.sqr();
            e >>= 1;
        }
        r
    }

    /// Деление: `self / rhs`. Соглашение: `x / 0 = 0`.
    #[must_use]
    pub const fn div(self, rhs: Self) -> Self {
        self.mul(rhs.inv())
    }

    /// Возведение в степень `k`. `0^0 = 1`; показатель ненулевых
    /// элементов приводится по модулю `N − 1`.
    #[must_use]
    pub const fn pow(self, k: u32) -> Self {
        let a = Self::fit(self.0);
        if a == 0 {
            return if k == 0 { Self(1) } else { Self(0) };
        }
        let mut e = k % ((N as u32) - 1);
        let mut r = Self(1);
        let mut base = Self(a);
        while e > 0 {
            if e & 1 == 1 {
                r = r.mul(base);
            }
            base = base.sqr();
            e >>= 1;
        }
        r
    }

    /// Квадратный корень: `sqrt(x)^2 = x`. Реализовано возведением
    /// в степень `2^(m−1)` (возведение в квадрат — биекция в GF(2^m)).
    #[must_use]
    pub const fn sqrt(self) -> Self {
        let a = Self::fit(self.0);
        if a == 0 {
            return Self(0);
        }
        let mut v = Self(a);
        let mut i = Self::M - 1;
        while i > 0 {
            v = v.sqr();
            i -= 1;
        }
        v
    }

    /// След: `Tr(x) = x + x^2 + x^4 + … + x^(2^(m−1))`; результат —
    /// элемент GF(2) ⊂ GF(2^m), то есть 0 или 1.
    /// Сложность O(m^2).
    #[must_use]
    pub const fn trace(self) -> Self {
        let a = Self::fit(self.0);
        let mut acc = Self(a);
        let mut cur = Self(a);
        let mut i: u32 = 1;
        while i < Self::M {
            cur = cur.sqr();
            acc = acc.add(cur);
            i += 1;
        }
        acc
    }

    /// Цех-логарифм `Z(k)`: элемент `y` со значением `Z(k)`, где
    /// `α^Z(k) = 1 + α^k`.
    ///
    /// Возвращает `None`, если `k ≡ 0 (mod N − 1)`: для `α^0 = 1`
    /// сумма `1 + 1 = 0` логарифма не имеет.
    ///
    /// Сложность O(N): линейный поиск дискретного логарифма
    /// (в табличном слое — O(1)).
    ///
    /// # Пример
    /// ```rust
    /// use gf2m::Gf16;
    ///
    /// let z = Gf16::zech(1).unwrap();
    /// // определение: α^Z(1) = 1 + α
    /// assert_eq!(Gf16::alpha().pow(z.value() as u32), Gf16::new(1) + Gf16::alpha());
    /// assert_eq!(Gf16::zech(0), None);
    /// ```
    #[must_use]
    pub fn zech(k: u32) -> Option<Self> {
        let order = (N as u32) - 1;
        let k = k % order;
        if k == 0 {
            return None;
        }
        let x = Self::alpha().pow(k); // α^k ≠ 1
        let y = Self(1).add(x); // 1 + α^k ≠ 0 и ≠ 1
                                // линейный поиск Z: α^Z = y; Z ≥ 1, так как y ≠ 1
        let mut cur = Self::alpha();
        let mut z: u32 = 1;
        while cur != y {
            cur = cur.mul(Self::alpha());
            z += 1;
        }
        Some(Self(z as u16))
    }
}

impl<const N: usize, const POLY: u32> fmt::Display for Gf<N, POLY> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inner:ident, $assign_trait:ident, $assign_method:ident) => {
        impl<const N: usize, const POLY: u32> core::ops::$trait for Gf<N, POLY> {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Self) -> Self {
                Self::$inner(self, rhs)
            }
        }
        impl<const N: usize, const POLY: u32> core::ops::$assign_trait for Gf<N, POLY> {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                *self = Self::$inner(*self, rhs);
            }
        }
    };
}

impl_binop!(Add, add, add, AddAssign, add_assign);
impl_binop!(Sub, sub, add, SubAssign, sub_assign); // в характеристике 2 разность = сумма
impl_binop!(Mul, mul, mul, MulAssign, mul_assign);
impl_binop!(Div, div, div, DivAssign, div_assign);

impl<const N: usize, const POLY: u32> Neg for Gf<N, POLY> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self // −x = x в характеристике 2
    }
}

/// GF(2^2), POLY = 0x7 (x^2 + x + 1).
pub type Gf4 = Gf<4, 0x7>;
/// GF(2^3), POLY = 0xB (x^3 + x + 1).
pub type Gf8 = Gf<8, 0xB>;
/// GF(2^4), POLY = 0x13 (x^4 + x + 1).
pub type Gf16 = Gf<16, 0x13>;
/// GF(2^5), POLY = 0x25 (x^5 + x^2 + 1).
pub type Gf32 = Gf<32, 0x25>;
/// GF(2^6), POLY = 0x43 (x^6 + x + 1).
pub type Gf64 = Gf<64, 0x43>;
/// GF(2^7), POLY = 0x83 (x^7 + x + 1).
pub type Gf128 = Gf<128, 0x83>;
/// GF(2^8), POLY = 0x11D (x^8 + x^4 + x^3 + x^2 + 1) — поле AES.
pub type Gf256 = Gf<256, 0x11D>;
/// GF(2^9), POLY = 0x211 (x^9 + x^4 + 1).
pub type Gf512 = Gf<512, 0x211>;
/// GF(2^10), POLY = 0x409 (x^10 + x^3 + 1).
pub type Gf1024 = Gf<1024, 0x409>;
/// GF(2^11), POLY = 0x805 (x^11 + x^2 + 1).
pub type Gf2048 = Gf<2048, 0x805>;
/// GF(2^12), POLY = 0x1053 (x^12 + x^6 + x^4 + x + 1).
pub type Gf4096 = Gf<4096, 0x1053>;
/// GF(2^13), POLY = 0x201B (x^13 + x^4 + x^3 + x + 1).
pub type Gf8192 = Gf<8192, 0x201B>;
/// GF(2^14), POLY = 0x402B (x^14 + x^5 + x^3 + x + 1).
pub type Gf16384 = Gf<16384, 0x402B>;
/// GF(2^15), POLY = 0x8003 (x^15 + x + 1).
pub type Gf32768 = Gf<32768, 0x8003>;
/// GF(2^16), POLY = 0x1002D (x^16 + x^5 + x^3 + x^2 + 1).
///
/// Псевдоним доступен только на целях с указателем не менее 32 бит
/// (наряду с табличным слоем [`gf65536`](crate::tables::gf65536));
/// сам тип `Gf<65536, 0x1002D>` переносим на любые цели.
#[cfg(not(target_pointer_width = "16"))]
pub type Gf65536 = Gf<65536, 0x1002D>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables;

    #[test]
    fn mul_matches_const_mul() {
        // GF(256): табличное умножение совпадает с константным ядром
        // (полный перебор)
        for a in 0u16..256 {
            for b in 0u16..256 {
                let x = Gf256::new(a);
                let y = Gf256::new(b);
                assert_eq!(
                    u16::from(tables::gf256::mul(a as u8, b as u8)),
                    x.mul(y).value(),
                    "table({a:#x}, {b:#x}) != const mul"
                );
            }
        }
    }

    // Gf65536 и таблицы gf65536 существуют только на целях,
    // где usize адресует 65536 элементов (32/64 бита).
    #[cfg(not(target_pointer_width = "16"))]
    #[test]
    fn mul_matches_const_mul_gf65536() {
        // выборка пар детерминированным LCG
        let mut s: u32 = 0x1234_5678;
        let mut next = || {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 16) as u16
        };
        for _ in 0..4096 {
            let a = next();
            let b = next();
            assert_eq!(
                tables::gf65536::mul(a, b),
                Gf65536::new(a).mul(Gf65536::new(b)).value(),
                "table({a:#x}, {b:#x}) != const mul"
            );
        }
    }

    #[test]
    fn trace_properties() {
        // GF(16), полный перебор: значения в GF(2), линейность,
        // инвариантность относительно Фробениуса
        for av in 0u16..16 {
            let a = Gf16::new(av);
            let t = a.trace();
            assert!(t == Gf16::zero() || t == Gf16::one(), "trace({a}) = {t}");
            assert_eq!(a.sqr().trace(), t, "Tr(x^2) != Tr(x) для {a}");
            for bv in 0u16..16 {
                let b = Gf16::new(bv);
                assert_eq!(
                    a.add(b).trace(),
                    a.trace().add(b.trace()),
                    "Tr({a} + {b}) != Tr({a}) + Tr({b})"
                );
            }
        }
    }

    #[test]
    fn zech_definition_and_identity() {
        assert_eq!(Gf16::zech(0), None); // 1 + α^0 = 0 — логарифма нет
        for k in 1u32..15 {
            let z = Gf16::zech(k).expect("k != 0 mod 15");
            // определение: α^Z(k) = 1 + α^k
            let lhs = Gf16::alpha().pow(z.value() as u32);
            let rhs = Gf16::new(1) + Gf16::alpha().pow(k);
            assert_eq!(lhs, rhs, "Z({k}) = {}", z.value());
            // тождество: Z(k) = k + Z(−k) = k + Z(15 − k) (mod 15)
            let z_neg = Gf16::zech(15 - k).expect("15 − k != 0 для k < 15");
            assert_eq!(
                (k + z_neg.value() as u32) % 15,
                z.value() as u32,
                "Z({k}) != k + Z({})",
                15 - k
            );
        }
    }

    #[test]
    fn reduce_maps_out_of_range_into_field() {
        // 0x153 = x^8 + x^6 + x^4 + x + 1; бит 8 гасится XOR с 0x11D
        let x = Gf256::reduce(0x153);
        assert_eq!(x.value(), 0x153 ^ 0x11D);
        assert!(x.value() < 256);
        // значение в диапазоне не меняется
        assert_eq!(x, Gf256::reduce(x.value()));
    }

    #[test]
    fn operators_match_methods() {
        let a = Gf256::new(0x57);
        let b = Gf256::new(0x83);
        assert_eq!(a + b, a.add(b));
        assert_eq!(a - b, a.add(b)); // вычитание = сложение
        assert_eq!(a * b, a.mul(b));
        assert_eq!(a / b, a.div(b));
        assert_eq!(-a, a);
        let mut c = a;
        c += b;
        assert_eq!(c, a + b);
        c -= b;
        assert_eq!(c, a);
        c *= b;
        assert_eq!(c, a * b);
        c /= b;
        assert_eq!(c, a);
    }
}
