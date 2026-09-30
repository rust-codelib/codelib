//! Рантайм-слой: p, m и модуль выбираются во время выполнения.
//!
//! [`FieldParams`] — проверенное описание поля (p простое,
//! `1 <= m <= 64`, `p^m <= 2^64`, структура модуля), [`GfRuntime`] —
//! элемент с копией описания. Арифметика — тот же школьный
//! алгоритм с пошаговой редукцией (через [`poly::mul_reduce`]),
//! обращение — по Ферма.
//!
//! Неприводимость модуля конструкторами **не** проверяется
//! (тест Рабина стоит O(m·log p) умножений и запускается явно):
//! контракт — модуль неприводим. Отдельно отклоняется `poly = 0`
//! как самая частая ошибка: при `m >= 2` это модуль `x^m` —
//! приводимый (поля не получается), а при `m = 1` — модуль `x`,
//! в котором `alpha()` нулевой. Удобный источник корректных
//! модулей — [`FieldParams::find`]: первый неприводимый степени m
//! над GF(p).
//!
//! Элемент хранит `[u32; 64]` независимо от m — это цена
//! рантайм-гибкости (~280 байт на элемент); для горячего кода
//! предназначен константно-генериковый [`Gf`](crate::Gf).
//!
//! # Пример
//! ```rust
//! use gfpm::{FieldParams, GfError, GfRuntime};
//!
//! let params = FieldParams::find(5, 3).unwrap(); // GF(125), модуль x^3 + x + 1
//! let a = GfRuntime::new(params, 7).unwrap(); // x + 2
//! let b = GfRuntime::new(params, 5).unwrap(); // x
//! assert_eq!(a * b, GfRuntime::new(params, 35).unwrap()); // x^2 + 2x
//! assert_eq!(
//!     GfRuntime::new(params, 125),
//!     Err(GfError::ElementOutOfRange { value: 125, order: 125 })
//! );
//! ```

use core::fmt;
use core::ops::Neg;

use crate::error::GfError;
use crate::fp::{add_mod, is_prime, order_checked, sub_mod};
use crate::poly::{fmt_coeffs, mul_reduce, reduce_in_place, unpack_poly, ACC_LEN};

/// Описание поля GF(p^m): p, m и упакованный мономиальный модуль.
///
/// Инвариант (проверяется [`new`](Self::new) и [`find`](Self::find)):
/// p простое, `1 <= m <= 64`, `p^m <= 2^64`, `0 < poly < p^m`.
/// Неприводимость модуля — контракт вызывающего (см. модульную
/// документацию).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FieldParams {
    p: u32,
    m: u32,
    poly: u128,
}

impl FieldParams {
    /// Максимальная поддерживаемая степень расширения — единый
    /// предел с [`poly::MAX_DEG`](crate::poly::MAX_DEG).
    pub const MAX_M: u32 = crate::poly::MAX_DEG as u32;

    /// Создаёт описание поля с проверкой структурных ограничений.
    ///
    /// `poly = 0` отклоняется сразу (без теста Рабина) как самая
    /// частая ошибка: при `m >= 2` это модуль `x^m` — приводимый,
    /// арифметика даёт делители нуля; при `m = 1` — модуль `x`,
    /// в котором `alpha()` нулевой.
    ///
    /// # Ошибки
    /// [`GfError::NotPrime`] — если `p` не простое;
    /// [`GfError::UnsupportedDegree`] — если `m` вне 1..=64;
    /// [`GfError::OrderTooLarge`] — если `p^m > 2^64`;
    /// [`GfError::InvalidPolynomial`] — если `poly >= p^m` или `poly = 0`.
    pub fn new(p: u32, m: u32, poly: u128) -> Result<Self, GfError> {
        if !is_prime(p) {
            return Err(GfError::NotPrime(p));
        }
        if m == 0 || m > Self::MAX_M {
            return Err(GfError::UnsupportedDegree(m));
        }
        let order = order_checked(p, m).ok_or(GfError::OrderTooLarge { p, m })?;
        if poly == 0 || poly >= order {
            return Err(GfError::InvalidPolynomial { p, m, poly });
        }
        Ok(Self { p, m, poly })
    }

    /// Описание с первым неприводимым многочленом степени `m`
    /// над GF(p): поиск тестом Рабина; кандидаты перебираются
    /// по возрастанию упакованного значения, с нулевым свободным
    /// членом — пропускаются (поэтому для `m = 1` результат —
    /// всегда модуль `x + 1`).
    ///
    /// # Ошибки
    /// Как у [`new`](Self::new), кроме `InvalidPolynomial`
    /// (найденный модуль корректен по построению).
    pub fn find(p: u32, m: u32) -> Result<Self, GfError> {
        if !is_prime(p) {
            return Err(GfError::NotPrime(p));
        }
        if m == 0 || m > Self::MAX_M {
            return Err(GfError::UnsupportedDegree(m));
        }
        let poly = crate::poly::find_irreducible(p, m).ok_or(GfError::OrderTooLarge { p, m })?;
        Ok(Self { p, m, poly })
    }

    /// Характеристика p.
    #[must_use]
    pub const fn p(self) -> u32 {
        self.p
    }

    /// Степень расширения m.
    #[must_use]
    pub const fn m(self) -> u32 {
        self.m
    }

    /// Упакованный модуль (без старшей единицы).
    #[must_use]
    pub const fn poly(self) -> u128 {
        self.poly
    }

    /// Порядок поля `p^m` (не больше 2^64 по инварианту).
    #[must_use]
    pub const fn order(self) -> u128 {
        // p^m <= 2^64 по инварианту конструкторов — переполнения нет
        let mut r: u128 = 1;
        let mut i = 0;
        while i < self.m {
            r *= self.p as u128;
            i += 1;
        }
        r
    }

    /// Коэффициенты модуля (без старшей единицы), `m` штук;
    /// хвост до 64 позиций — нули.
    fn q_coeffs(self) -> [u32; 64] {
        let mut q = [0u32; 64];
        unpack_poly(self.poly, self.p, self.m, &mut q);
        q
    }
}

/// Элемент поля GF(p^m) с рантайм-параметрами.
///
/// Элемент несёт копию описания поля ([`FieldParams`]) и канонические
/// коэффициенты (`[u32; 64]`, значимые — первые `m`). Операции между
/// элементами разных полей — контрактное нарушение (debug-проверка):
/// ⚠️ в release-сборке оно не диагностируется — результат неверен
/// либо происходит паника; сравнение элементов разных полей
/// побитово и потому тоже бессмысленно.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GfRuntime {
    params: FieldParams,
    value: [u32; 64],
}

impl GfRuntime {
    /// Создаёт элемент из упакованного целого (базис-p цифры —
    /// коэффициенты многочлена).
    ///
    /// # Ошибки
    /// [`GfError::ElementOutOfRange`] — если `value >= порядок`.
    pub fn new(params: FieldParams, value: u64) -> Result<Self, GfError> {
        let order = params.order();
        if value as u128 >= order {
            return Err(GfError::ElementOutOfRange { value, order });
        }
        let mut v = [0u32; 64];
        unpack_poly(value as u128, params.p, params.m, &mut v);
        Ok(Self { params, value: v })
    }

    /// Элемент из произвольного целого: цифры интерпретируются
    /// как многочлен и приводятся по модулю поля (total, без ошибок).
    #[must_use]
    pub fn reduce(params: FieldParams, value: u64) -> Self {
        let mut v = [0u32; 64];
        unpack_poly(value as u128, params.p, 64, &mut v);
        let q = params.q_coeffs();
        reduce_in_place(&mut v, &q[..params.m as usize], params.p);
        Self { params, value: v }
    }

    /// Нулевой элемент поля.
    #[must_use]
    pub fn zero(params: FieldParams) -> Self {
        Self {
            params,
            value: [0; 64],
        }
    }

    /// Единица поля.
    #[must_use]
    pub fn one(params: FieldParams) -> Self {
        let mut v = [0u32; 64];
        v[0] = 1;
        Self { params, value: v }
    }

    /// Класс многочлена `x`: примитивен, если модуль примитивный.
    /// Для `m = 1` — вычет `−q_0` (ср. [`Gf::alpha`](crate::Gf::alpha)).
    #[must_use]
    pub fn alpha(params: FieldParams) -> Self {
        let mut v = [0u32; 64];
        if params.m == 1 {
            let q0 = (params.poly % params.p as u128) as u32;
            v[0] = if q0 == 0 { 0 } else { params.p - q0 };
        } else {
            v[1] = 1;
        }
        Self { params, value: v }
    }

    /// Элемент из коэффициентов (по возрастанию).
    ///
    /// # Ошибки
    /// [`GfError::InvalidSliceLen`] — если длина не равна `m`;
    /// [`GfError::CoefficientOutOfRange`] — если коэффициент `>= p`.
    pub fn from_coeffs(params: FieldParams, coeffs: &[u32]) -> Result<Self, GfError> {
        let m = params.m as usize;
        if coeffs.len() != m {
            return Err(GfError::InvalidSliceLen {
                expected: m,
                got: coeffs.len(),
            });
        }
        for (index, &c) in coeffs.iter().enumerate() {
            if c >= params.p {
                return Err(GfError::CoefficientOutOfRange {
                    index,
                    coeff: c,
                    p: params.p,
                });
            }
        }
        let mut v = [0u32; 64];
        v[..m].copy_from_slice(coeffs);
        Ok(Self { params, value: v })
    }

    /// Коэффициенты по возрастанию (срез длины `m`).
    #[must_use]
    pub fn coeffs(&self) -> &[u32] {
        &self.value[..self.params.m as usize]
    }

    /// `i`-й коэффициент (`0` при `i >= m`).
    #[must_use]
    pub fn coeff(&self, i: usize) -> u32 {
        if i < self.params.m as usize {
            self.value[i]
        } else {
            0
        }
    }

    /// Упакованное целое представление: `Σ c_i·p^i`.
    #[must_use]
    pub fn value(&self) -> u64 {
        let mut r: u128 = 0;
        let m = self.params.m as usize;
        let mut i = m;
        while i > 0 {
            i -= 1;
            r = r * self.params.p as u128 + self.value[i] as u128;
        }
        r as u64 // < порядок поля <= 2^64
    }

    /// Описание поля элемента.
    #[must_use]
    pub const fn params(&self) -> FieldParams {
        self.params
    }

    /// Характеристика p.
    #[must_use]
    pub const fn p(&self) -> u32 {
        self.params.p
    }

    /// Степень расширения m.
    #[must_use]
    pub const fn m(&self) -> u32 {
        self.params.m
    }

    /// Упакованный модуль поля.
    #[must_use]
    pub const fn poly(&self) -> u128 {
        self.params.poly
    }

    /// Порядок поля `p^m`.
    #[must_use]
    pub const fn order(&self) -> u128 {
        self.params.order()
    }

    /// Проверка на нуль.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.coeffs().iter().all(|&c| c == 0)
    }

    /// Сложение: покомпонентно по модулю p.
    // Инherent-метод — первичный API; трейт Add делегирует ему.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn add(self, rhs: Self) -> Self {
        debug_assert_eq!(self.params, rhs.params, "сложение элементов разных полей");
        let m = self.params.m as usize;
        let mut v = [0u32; 64];
        for (dst, (&x, &y)) in v
            .iter_mut()
            .zip(self.value.iter().zip(rhs.value.iter()))
            .take(m)
        {
            *dst = add_mod(x, y, self.params.p);
        }
        Self {
            params: self.params,
            value: v,
        }
    }

    /// Вычитание: покомпонентно по модулю p.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn sub(self, rhs: Self) -> Self {
        debug_assert_eq!(self.params, rhs.params, "вычитание элементов разных полей");
        let m = self.params.m as usize;
        let mut v = [0u32; 64];
        for (dst, (&x, &y)) in v
            .iter_mut()
            .zip(self.value.iter().zip(rhs.value.iter()))
            .take(m)
        {
            *dst = sub_mod(x, y, self.params.p);
        }
        Self {
            params: self.params,
            value: v,
        }
    }

    /// Противоположный элемент: `p − c` (нуль остаётся нулём).
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn neg(self) -> Self {
        let p = self.params.p;
        let mut v = [0u32; 64];
        for (dst, &c) in v.iter_mut().zip(self.coeffs().iter()) {
            *dst = if c == 0 { 0 } else { p - c };
        }
        Self {
            params: self.params,
            value: v,
        }
    }

    /// Умножение: школьное с пошаговой редукцией по модулю поля.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn mul(self, rhs: Self) -> Self {
        debug_assert_eq!(self.params, rhs.params, "умножение элементов разных полей");
        let m = self.params.m as usize;
        let q = self.params.q_coeffs();
        let mut acc = [0u32; ACC_LEN];
        mul_reduce(
            &self.value[..m],
            &rhs.value[..m],
            &q[..m],
            self.params.p,
            &mut acc,
        );
        let mut v = [0u32; 64];
        v[..m].copy_from_slice(&acc[..m]);
        Self {
            params: self.params,
            value: v,
        }
    }

    /// Возведение в квадрат.
    #[must_use]
    pub fn sqr(self) -> Self {
        self.mul(self)
    }

    /// Обратный по малой теореме Ферма. Соглашение: `inv(0) = 0`.
    #[must_use]
    pub fn inv(self) -> Self {
        if self.is_zero() {
            return Self::zero(self.params);
        }
        self.pow((self.params.order() - 2) as u64)
    }

    /// Деление: `self / rhs`. Соглашение: `x / 0 = 0`.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn div(self, rhs: Self) -> Self {
        self.mul(rhs.inv())
    }

    /// Возведение в степень `k`. `0^0 = 1`; показатель ненулевых
    /// элементов приводится по модулю `p^m − 1`.
    #[must_use]
    pub fn pow(self, k: u64) -> Self {
        if self.is_zero() {
            return if k == 0 {
                Self::one(self.params)
            } else {
                Self::zero(self.params)
            };
        }
        let mut e = k % ((self.params.order() - 1) as u64);
        let mut r = Self::one(self.params);
        let mut base = self;
        while e > 0 {
            if e & 1 == 1 {
                r = r.mul(base);
            }
            base = base.sqr();
            e >>= 1;
        }
        r
    }

    /// Фробениус: `x^p`.
    #[must_use]
    pub fn frobenius(self) -> Self {
        self.pow(self.params.p as u64)
    }

    /// След: `Tr(x) = x + x^p + … + x^(p^(m−1))`; результат —
    /// элемент простого подполя GF(p).
    #[must_use]
    pub fn trace(self) -> Self {
        let mut acc = self;
        let mut cur = self;
        for _ in 1..self.params.m {
            cur = cur.frobenius();
            acc = acc.add(cur);
        }
        acc
    }

    /// Норма: `N(x) = x^(1 + p + … + p^(m−1))`; результат —
    /// элемент GF(p).
    #[must_use]
    pub fn norm(self) -> Self {
        let mut e: u64 = 1;
        for _ in 1..self.params.m {
            e = e * self.params.p as u64 + 1;
        }
        self.pow(e)
    }

    /// Квадратность (нуль считается квадратом). В характеристике 2 —
    /// всегда `true` (фробениус биективен); иначе критерий Эйлера.
    #[must_use]
    pub fn is_square(&self) -> bool {
        if self.params.p == 2 {
            return true;
        }
        if self.is_zero() {
            return true;
        }
        self.pow(((self.params.order() - 1) / 2) as u64) == Self::one(self.params)
    }

    /// Квадратный корень: `None` для невычетов, иначе некоторый
    /// корень. Реализация — generic-функция [`sqrt`](crate::field::sqrt)
    /// (характеристика 2 — возведение в `2^(m−1)`, нечётная —
    /// Тонелли–Шенкс), общая для обоих слоёв крейта.
    #[must_use]
    pub fn sqrt(&self) -> Option<Self> {
        crate::field::sqrt(*self)
    }
}

impl fmt::Display for GfRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_coeffs(f, self.coeffs())
    }
}

impl fmt::Debug for GfRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GfRuntime")
            .field("params", &self.params)
            .field("coeffs", &self.coeffs())
            .finish()
    }
}

macro_rules! impl_binop_rt {
    ($trait:ident, $method:ident, $inner:ident, $assign_trait:ident, $assign_method:ident) => {
        impl core::ops::$trait for GfRuntime {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Self) -> Self {
                GfRuntime::$inner(self, rhs)
            }
        }
        impl core::ops::$assign_trait for GfRuntime {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                *self = GfRuntime::$inner(*self, rhs);
            }
        }
    };
}

impl_binop_rt!(Add, add, add, AddAssign, add_assign);
impl_binop_rt!(Sub, sub, sub, SubAssign, sub_assign);
impl_binop_rt!(Mul, mul, mul, MulAssign, mul_assign);
impl_binop_rt!(Div, div, div, DivAssign, div_assign);

impl Neg for GfRuntime {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        GfRuntime::neg(self)
    }
}
