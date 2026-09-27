//! Универсальный контракт конечного поля для generic-алгоритмов.
//!
//! [`FiniteField`] — минимальный интерфейс, общий для обоих слоёв
//! крейта: константно-генерикового [`Gf`] и
//! [`GfRuntime`]. Конструкторы берут элемент-
//! приёмник (`self`), а не глобальные параметры: рантайм-слой хранит
//! описание поля внутри элемента, константный слой приёмник
//! игнорирует. Это позволяет писать алгоритмы (схема Горнера,
//! Тонелли–Шенкс, гауссовы исключения, коды Рида–Соломона)
//! единообразно над любым полем крейта.
//!
//! # Пример
//! ```rust
//! use gfpm::{poly_eval, FiniteField, Gf9, GfRuntime, FieldParams};
//!
//! // 1 + 2x в точке x (модуль GF(9)): 1 + 2x
//! let x = Gf9::alpha();
//! assert_eq!(poly_eval(&[Gf9::new(1), Gf9::new(2)], x), Gf9::new(7));
//!
//! // Тот же generic-код над рантайм-слоем GF(9)
//! let params = FieldParams::new(3, 2, 5).unwrap();
//! let x = GfRuntime::alpha(params);
//! let coeffs = [GfRuntime::new(params, 1).unwrap(), GfRuntime::new(params, 2).unwrap()];
//! assert_eq!(poly_eval(&coeffs, x), GfRuntime::new(params, 7).unwrap());
//! ```

use core::fmt::Debug;

use crate::gf::Gf;
use crate::runtime::GfRuntime;

/// Контракт элемента конечного поля.
///
/// Реализации: `Gf<P, M, POLY>` и `GfRuntime`. Соглашения нуля
/// едины для всего крейта: `inv(0) = 0`, `x / 0 = 0`, `0^0 = 1`.
pub trait FiniteField: Copy + Eq + Debug {
    /// Характеристика p (поле GF(p^m)).
    fn characteristic(self) -> u32;
    /// Степень расширения m.
    fn degree(self) -> u32;
    /// Порядок поля `p^m`.
    fn field_order(self) -> u128;
    /// Нуль поля.
    fn zero(self) -> Self;
    /// Единица поля.
    fn one(self) -> Self;
    /// Элемент из целого: значение приводится в диапазон поля.
    ///
    /// Метод-приёмник (`self`) — сознательное решение трейта:
    /// рантайм-слой хранит параметры поля внутри элемента.
    #[allow(clippy::wrong_self_convention)]
    fn from_int(self, value: u64) -> Self;
    /// Целое (упакованное) представление.
    fn to_int(self) -> u64;
    /// Проверка на нуль.
    fn is_zero(self) -> bool;
    /// Сложение.
    fn add(self, rhs: Self) -> Self;
    /// Вычитание.
    fn sub(self, rhs: Self) -> Self;
    /// Противоположный элемент.
    fn neg(self) -> Self;
    /// Умножение.
    fn mul(self, rhs: Self) -> Self;
    /// Деление (соглашение `x / 0 = 0`).
    fn div(self, rhs: Self) -> Self;
    /// Обратный (соглашение `inv(0) = 0`).
    fn inv(self) -> Self;
    /// Возведение в степень.
    fn pow(self, k: u64) -> Self;
}

impl<const P: u32, const M: usize, const POLY: u128> FiniteField for Gf<P, M, POLY> {
    fn characteristic(self) -> u32 {
        P
    }
    fn degree(self) -> u32 {
        M as u32
    }
    fn field_order(self) -> u128 {
        Self::ORDER
    }
    fn zero(self) -> Self {
        Gf::<P, M, POLY>::zero()
    }
    fn one(self) -> Self {
        Gf::<P, M, POLY>::one()
    }
    fn from_int(self, value: u64) -> Self {
        Gf::<P, M, POLY>::reduce(value)
    }
    fn to_int(self) -> u64 {
        self.value()
    }
    fn is_zero(self) -> bool {
        Gf::<P, M, POLY>::is_zero(self)
    }
    fn add(self, rhs: Self) -> Self {
        Gf::<P, M, POLY>::add(self, rhs)
    }
    fn sub(self, rhs: Self) -> Self {
        Gf::<P, M, POLY>::sub(self, rhs)
    }
    fn neg(self) -> Self {
        Gf::<P, M, POLY>::neg(self)
    }
    fn mul(self, rhs: Self) -> Self {
        Gf::<P, M, POLY>::mul(self, rhs)
    }
    fn div(self, rhs: Self) -> Self {
        Gf::<P, M, POLY>::div(self, rhs)
    }
    fn inv(self) -> Self {
        Gf::<P, M, POLY>::inv(self)
    }
    fn pow(self, k: u64) -> Self {
        Gf::<P, M, POLY>::pow(self, k)
    }
}

impl FiniteField for GfRuntime {
    fn characteristic(self) -> u32 {
        self.p()
    }
    fn degree(self) -> u32 {
        self.m()
    }
    fn field_order(self) -> u128 {
        self.order()
    }
    fn zero(self) -> Self {
        GfRuntime::zero(self.params())
    }
    fn one(self) -> Self {
        GfRuntime::one(self.params())
    }
    fn from_int(self, value: u64) -> Self {
        GfRuntime::reduce(self.params(), value)
    }
    fn to_int(self) -> u64 {
        self.value()
    }
    fn is_zero(self) -> bool {
        GfRuntime::is_zero(&self)
    }
    fn add(self, rhs: Self) -> Self {
        GfRuntime::add(self, rhs)
    }
    fn sub(self, rhs: Self) -> Self {
        GfRuntime::sub(self, rhs)
    }
    fn neg(self) -> Self {
        GfRuntime::neg(self)
    }
    fn mul(self, rhs: Self) -> Self {
        GfRuntime::mul(self, rhs)
    }
    fn div(self, rhs: Self) -> Self {
        GfRuntime::div(self, rhs)
    }
    fn inv(self) -> Self {
        GfRuntime::inv(self)
    }
    fn pow(self, k: u64) -> Self {
        GfRuntime::pow(self, k)
    }
}

/// Значение многочлена по схеме Горнера над произвольным полем.
///
/// `coeffs` — коэффициенты **по возрастанию** степеней
/// (`coeffs[i]` — при `x^i`), как во всём крейте.
///
/// # Пример
/// ```rust
/// use gfpm::{poly_eval, FiniteField, Gf25};
///
/// // x^2 + 1 в точке alpha: alpha^2 + 1 = (4x + 3) + 1 = 4x + 4
/// let x = Gf25::alpha();
/// assert_eq!(poly_eval(&[Gf25::new(1), Gf25::zero(), Gf25::new(1)], x), x.sqr() + x.one());
/// ```
#[must_use]
pub fn poly_eval<F: FiniteField>(coeffs: &[F], x: F) -> F {
    let mut acc = x.zero();
    for &c in coeffs.iter().rev() {
        acc = acc.mul(x).add(c);
    }
    acc
}

/// Квадратный корень в произвольном конечном поле: `None` для
/// невычетов, иначе некоторый корень (`sqrt(x)^2 = x`).
///
/// Характеристика 2 — возведение в степень `2^(m−1)`
/// (фробениус биективен); нечётная — алгоритм Тонелли–Шенкса
/// над мультипликативной группой. Это общая реализация для обоих
/// слоёв крейта; константно-генериковое ядро держит собственную
/// `const fn`-копию (тест сверяет их результаты).
#[must_use]
pub fn sqrt<F: FiniteField>(x: F) -> Option<F> {
    let one = x.one();
    if x.characteristic() == 2 {
        // x^(2^(m−1))
        let mut e: u64 = 1;
        for _ in 1..x.degree() {
            e <<= 1;
        }
        return Some(x.pow(e));
    }
    if x.is_zero() {
        return Some(x.zero());
    }
    if x.pow(((x.field_order() - 1) / 2) as u64) != one {
        return None; // невычет
    }
    // Тонелли–Шенкс: q − 1 = 2^s · t
    let q1 = (x.field_order() - 1) as u64;
    let s = q1.trailing_zeros();
    let t = q1 >> s;
    let half = ((x.field_order() - 1) / 2) as u64;
    let neg_one = x.zero().sub(one);
    // невычет z: z^((q−1)/2) = −1. При m >= 2 поиск начинается с x
    // (упакованное значение p): при чётном m все элементы простого
    // подполя — квадраты (см. Gf::sqrt), перебор подполя занял бы ~p
    // шагов; невычеты вне подполя существуют всегда.
    let mut z_int: u64 = if x.degree() >= 2 {
        x.characteristic() as u64
    } else {
        2
    };
    let z = loop {
        debug_assert!(
            (z_int as u128) < x.field_order(),
            "невычет не найден — недостижимо"
        );
        let cand = x.from_int(z_int);
        if cand.pow(half) == neg_one {
            break cand;
        }
        z_int += 1;
    };
    let mut c = z.pow(t);
    let mut r = x.pow(t.div_ceil(2));
    let mut w = x.pow(t);
    let mut m_cur = s;
    while w != one {
        let mut i: u32 = 0;
        let mut cur = w;
        while cur != one {
            cur = cur.mul(cur);
            i += 1;
        }
        debug_assert!(i < m_cur, "инвариант Тонелли–Шенкса нарушен");
        let mut b = c;
        for _ in 0..(m_cur - i - 1) {
            b = b.mul(b);
        }
        c = b.mul(b);
        r = r.mul(b);
        w = w.mul(c);
        m_cur = i;
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::{poly_eval, sqrt, FiniteField};
    use crate::runtime::FieldParams;
    use crate::{Gf25, Gf5, GfRuntime};

    #[test]
    fn horner_matches_direct() {
        // (x + 2)(x + 1) = x^2 + 3x + 2 над GF(5)
        let one = Gf5::one();
        let f = |v: u64| one.from_int(v);
        for xv in 0..5u64 {
            let x = f(xv);
            let direct = x.mul(x).add(f(3).mul(x)).add(f(2));
            let coeffs = [f(2), f(3), f(1)];
            assert_eq!(poly_eval(&coeffs, x), direct, "x = {xv}");
        }
    }

    #[test]
    fn generic_sqrt_matches_inherent() {
        // константный слой
        for v in 0..25u64 {
            let x = Gf25::one().from_int(v);
            assert_eq!(sqrt(x), x.sqrt(), "Gf25 x = {v}");
        }
        // рантайм-слой
        let params = FieldParams::new(3, 2, 5).unwrap();
        for v in 0..9u64 {
            let x = GfRuntime::new(params, v).unwrap();
            assert_eq!(sqrt(x), x.sqrt(), "runtime GF(9) x = {v}");
        }
    }

    #[test]
    fn trait_impls_agree_with_inherent() {
        use crate::Gf81;
        let one = Gf81::one();
        let a = one.from_int(17);
        let b = one.from_int(40);
        assert_eq!(a.add(b), a + b);
        assert_eq!(a.mul(b), a * b);
        assert_eq!(a.sub(b).add(b), a);
        assert_eq!(a.inv().mul(a), one);
        assert_eq!(a.pow(5), a.pow(2).mul(a.pow(3)));
        assert_eq!(a.to_int(), 17);
        assert_eq!(a.characteristic(), 3);
        assert_eq!(a.degree(), 4);
        assert_eq!(a.field_order(), 81);
    }
}
