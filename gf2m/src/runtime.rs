//! Рантайм-слой: поле выбирается во время выполнения.
//!
//! [`GfRuntime`] хранит степень m и значение; все операции — чистые
//! функции от m. Умножение для m = 8 и m = 16 идёт через
//! [таблицы](crate::tables), для остальных степеней — сдвигами;
//! обращение — алгоритмом Ито–Цудзии. Значения в диапазоне
//! гарантируются конструктором, операции дополнительно приводят входы
//! по модулю образующего многочлена.
//!
//! # Пример
//! ```rust
//! use gf2m::{GfError, GfRuntime};
//!
//! let a = GfRuntime::new(8, 0x80).unwrap();
//! let b = GfRuntime::new(8, 0x02).unwrap();
//! assert_eq!(a * b, GfRuntime::new(8, 0x1D).unwrap()); // t^8 → 0x1D
//! assert_eq!(
//!     GfRuntime::new(8, 256),
//!     Err(GfError::ElementOutOfRange { m: 8, value: 256 })
//! );
//! ```

use core::fmt;
use core::ops::Neg;

use crate::error::GfError;
use crate::poly::poly_mod;
use crate::tables;

/// Стандартные примитивные многочлены для m = 2..=16
/// (индекс `m − 2`).
pub const POLYS: [u32; 15] = [
    0x7, 0xB, 0x13, 0x25, 0x43, 0x83, 0x11D, 0x211, 0x409, 0x805, 0x1053, 0x201B, 0x402B, 0x8003,
    0x1002D,
];

/// Элемент GF(2^m) с-runtime-выбором степени поля.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GfRuntime {
    m: u32,
    value: u16,
}

impl GfRuntime {
    /// Минимальная поддерживаемая степень расширения.
    pub const MIN_M: u32 = 2;
    /// Максимальная поддерживаемая степень расширения.
    pub const MAX_M: u32 = 16;

    /// Создаёт элемент `value` поля GF(2^m).
    ///
    /// # Ошибки
    /// [`GfError::UnsupportedDegree`] — если `m` вне 2..=16;
    /// [`GfError::ElementOutOfRange`] — если `value >= 2^m`.
    pub const fn new(m: u32, value: u16) -> Result<Self, GfError> {
        if m < Self::MIN_M || m > Self::MAX_M {
            return Err(GfError::UnsupportedDegree(m));
        }
        if (value as u32) >= (1u32 << m) {
            return Err(GfError::ElementOutOfRange { m, value });
        }
        Ok(Self { m, value })
    }

    /// Степень расширения поля элемента.
    #[must_use]
    pub const fn m(self) -> u32 {
        self.m
    }

    /// Целое представление элемента.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.value
    }

    /// Стандартный примитивный многочлен степени `m`.
    ///
    /// # Ошибки
    /// [`GfError::UnsupportedDegree`] — если `m` вне 2..=16.
    pub const fn standard_poly(m: u32) -> Result<u32, GfError> {
        if m < Self::MIN_M || m > Self::MAX_M {
            return Err(GfError::UnsupportedDegree(m));
        }
        Ok(POLYS[(m - 2) as usize])
    }

    /// Нулевой элемент поля GF(2^m).
    ///
    /// # Ошибки
    /// [`GfError::UnsupportedDegree`] — если `m` вне 2..=16.
    pub fn zero(m: u32) -> Result<Self, GfError> {
        Self::new(m, 0)
    }

    /// Единица поля GF(2^m).
    ///
    /// # Ошибки
    /// [`GfError::UnsupportedDegree`] — если `m` вне 2..=16.
    pub fn one(m: u32) -> Result<Self, GfError> {
        Self::new(m, 1)
    }

    /// Примитивный элемент α = t поля GF(2^m).
    ///
    /// # Ошибки
    /// [`GfError::UnsupportedDegree`] — если `m` вне 2..=16.
    pub fn alpha(m: u32) -> Result<Self, GfError> {
        Self::new(m, 2)
    }

    /// Сложение (= вычитание). Оба операнда приводятся к диапазону
    /// поля левого операнда.
    // Инherent-метод — первичный API; трейт Add делегирует ему.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn add(self, rhs: Self) -> Self {
        debug_assert_eq!(self.m, rhs.m, "сложение элементов разных полей");
        let b = Self::fit(self.m, rhs.value);
        Self {
            m: self.m,
            value: self.value ^ b,
        }
    }

    /// Умножение. Для m = 8 и m = 16 — таблицы, для остальных
    /// степеней — сдвигами с приведением.
    // Инherent-метод — первичный API; трейт Mul делегирует ему.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn mul(self, rhs: Self) -> Self {
        debug_assert_eq!(self.m, rhs.m, "умножение элементов разных полей");
        let b = Self::fit(self.m, rhs.value);
        Self {
            m: self.m,
            value: Self::mul_raw(self.m, self.value, b),
        }
    }

    /// Возведение в квадрат.
    #[must_use]
    pub fn sqr(self) -> Self {
        self.mul(self)
    }

    /// Обратный элемент алгоритмом Ито–Цудзии. Соглашение: `inv(0) = 0`.
    #[must_use]
    pub fn inv(self) -> Self {
        if self.value == 0 {
            return Self {
                m: self.m,
                value: 0,
            };
        }
        Self {
            m: self.m,
            value: Self::itoh_tsujii(self.m, self.value),
        }
    }

    /// Деление: `self / rhs`. Соглашение: `x / 0 = 0`.
    // Инherent-метод — первичный API; трейт Div делегирует ему.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn div(self, rhs: Self) -> Self {
        self.mul(rhs.inv())
    }

    /// Возведение в степень `k`. `0^0 = 1`; показатель ненулевых
    /// элементов приводится по модулю `2^m − 1`.
    #[must_use]
    pub fn pow(self, k: u32) -> Self {
        if self.value == 0 {
            return Self {
                m: self.m,
                value: if k == 0 { 1 } else { 0 },
            };
        }
        let order = (1u32 << self.m) - 1;
        let mut e = k % order;
        let mut r: u16 = 1;
        let mut base = self.value;
        while e > 0 {
            if e & 1 == 1 {
                r = Self::mul_raw(self.m, r, base);
            }
            base = Self::mul_raw(self.m, base, base);
            e >>= 1;
        }
        Self {
            m: self.m,
            value: r,
        }
    }

    /// Квадратный корень: возведение в степень `2^(m−1)`.
    #[must_use]
    pub fn sqrt(self) -> Self {
        let mut v = self.value;
        let mut i = self.m - 1;
        while i > 0 {
            v = Self::mul_raw(self.m, v, v);
            i -= 1;
        }
        Self {
            m: self.m,
            value: v,
        }
    }

    /// След: `Tr(x) = x + x^2 + … + x^(2^(m−1))`; результат — 0 или 1.
    /// Сложность O(m^2).
    #[must_use]
    pub fn trace(self) -> Self {
        let mut acc = self.value;
        let mut cur = self.value;
        let mut i: u32 = 1;
        while i < self.m {
            cur = Self::mul_raw(self.m, cur, cur);
            acc ^= cur;
            i += 1;
        }
        Self {
            m: self.m,
            value: acc,
        }
    }

    /// Цех-логарифм: интерпретирует `self` как показатель `k`
    /// и возвращает элемент со значением `Z(k)`, где
    /// `α^Z(k) = 1 + α^k`.
    ///
    /// `None`, если `k ≡ 0 (mod 2^m − 1)`: сумма `1 + 1 = 0`
    /// логарифма не имеет. Для m = 8 и m = 16 — O(1) по таблицам,
    /// для остальных степеней — O(N) линейным поиском.
    #[must_use]
    pub fn zech(self) -> Option<Self> {
        let order = (1u32 << self.m) - 1;
        let k = (self.value as u32) % order;
        if k == 0 {
            return None;
        }
        // y = 1 + α^k: не 0 (k != 0) и не 1 (α^k != 0)
        let x = Self {
            m: self.m,
            value: Self::pow_u32(self.m, 2, k),
        };
        let y = 1 ^ x.value;
        let z = match self.m {
            8 => tables::gf256::LOG[y as usize] as u32,
            #[cfg(not(target_pointer_width = "16"))]
            16 => tables::gf65536::LOG[y as usize] as u32,
            _ => Self::discrete_log(self.m, y),
        };
        Some(Self {
            m: self.m,
            value: z as u16,
        })
    }

    // ---- внутренние примитивы ----

    /// Приведение значения к диапазону поля GF(2^m)
    /// (полиномиальная редукция по модулю стандартного многочлена).
    fn fit(m: u32, v: u16) -> u16 {
        if (v as u32) >= (1u32 << m) {
            poly_mod(v as u32, POLYS[(m - 2) as usize]) as u16
        } else {
            v
        }
    }

    /// Умножение «сдвигом-и-ксором» с пошаговым приведением.
    fn mul_generic(m: u32, a: u16, b: u16) -> u16 {
        let poly = POLYS[(m - 2) as usize];
        let mut r: u32 = 0;
        let mut a: u32 = a as u32;
        let mut b: u32 = b as u32;
        while b != 0 {
            if b & 1 != 0 {
                r ^= a;
            }
            a <<= 1;
            if (a >> m) != 0 {
                a ^= poly;
            }
            b >>= 1;
        }
        r as u16
    }

    /// Диспетчер умножения: таблицы для ходовых степеней,
    /// универсальный путь — для остальных.
    fn mul_raw(m: u32, a: u16, b: u16) -> u16 {
        debug_assert!((Self::MIN_M..=Self::MAX_M).contains(&m));
        debug_assert!((a as u32) < (1u32 << m) && (b as u32) < (1u32 << m));
        match m {
            8 => tables::gf256::mul(a as u8, b as u8) as u16,
            #[cfg(not(target_pointer_width = "16"))]
            16 => tables::gf65536::mul(a, b),
            _ => Self::mul_generic(m, a, b),
        }
    }

    /// Возведение значения в степень (значение обязано быть в диапазоне).
    fn pow_u32(m: u32, base: u16, k: u32) -> u16 {
        if base == 0 {
            return if k == 0 { 1 } else { 0 };
        }
        let order = (1u32 << m) - 1;
        let mut e = k % order;
        let mut r: u16 = 1;
        let mut b = base;
        while e > 0 {
            if e & 1 == 1 {
                r = Self::mul_raw(m, r, b);
            }
            b = Self::mul_raw(m, b, b);
            e >>= 1;
        }
        r
    }

    /// Ито–Цудзии: `a^(2^(m−1) − 1)` цепочкой, затем квадрат —
    /// это `a^(2^m − 2) = a^(−1)` по Ферма.
    ///
    /// Контракт: `a != 0` (нуль отсекается в [`inv`](Self::inv));
    /// `m` в 2..=16. Цепочка поддерживает инвариант
    /// `r = a^(2^k − 1)`, удвоение `k → 2k` стоит `k + 1` умножений,
    /// итог — около `m + log2(m)` умножений вместо `2m` у наивного
    /// возведения в степень.
    fn itoh_tsujii(m: u32, a: u16) -> u16 {
        debug_assert!(a != 0, "itoh_tsujii: a = 0 — контракт нарушен");
        debug_assert!((Self::MIN_M..=Self::MAX_M).contains(&m));
        let n = m - 1; // показатель: 2^n − 1
        let mut r = a; // r = a^(2^1 − 1)
        let mut k: u32 = 1; // текущая длина цепочки
        let top = 31 - n.leading_zeros(); // n >= 1
        let mut bit = top;
        while bit > 0 {
            bit -= 1;
            // удвоение: r ← r^(2^k) · r, показатель 2^k − 1 → 2^(2k) − 1
            let mut s = r;
            let mut j = k;
            while j > 0 {
                s = Self::mul_raw(m, s, s);
                j -= 1;
            }
            r = Self::mul_raw(m, s, r);
            k *= 2;
            // добавление единицы: r ← r^2 · a, показатель → 2^(k+1) − 1
            if (n >> bit) & 1 == 1 {
                r = Self::mul_raw(m, Self::mul_raw(m, r, r), a);
                k += 1;
            }
        }
        // a^(2^n − 1) → квадрат даёт a^(2^(n+1) − 2) = a^(2^m − 2) = a^(−1)
        Self::mul_raw(m, r, r)
    }

    /// Дискретный логарифм по базе α линейным поиском.
    /// Контракт: `y != 0` и `y != 1` (тогда ответ в 1..2^m − 2).
    fn discrete_log(m: u32, y: u16) -> u32 {
        debug_assert!(y != 0 && y != 1, "discrete_log: y обязан быть не 0 и не 1");
        let mut cur: u16 = 2; // α^1
        let mut z: u32 = 1;
        while cur != y {
            cur = Self::mul_raw(m, cur, 2);
            z += 1;
            debug_assert!(
                z < (1u32 << m) - 1,
                "дискретный логарифм не найден — недостижимо"
            );
        }
        z
    }
}

impl fmt::Display for GfRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.value)
    }
}

macro_rules! impl_binop_rt {
    ($trait:ident, $method:ident, $inner:ident, $assign_trait:ident, $assign_method:ident) => {
        impl core::ops::$trait for GfRuntime {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Self) -> Self {
                Self::$inner(self, rhs)
            }
        }
        impl core::ops::$assign_trait for GfRuntime {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                *self = Self::$inner(*self, rhs);
            }
        }
    };
}

impl_binop_rt!(Add, add, add, AddAssign, add_assign);
impl_binop_rt!(Sub, sub, add, SubAssign, sub_assign); // в характеристике 2 разность = сумма
impl_binop_rt!(Mul, mul, mul, MulAssign, mul_assign);
impl_binop_rt!(Div, div, div, DivAssign, div_assign);

impl Neg for GfRuntime {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self // −x = x в характеристике 2
    }
}
