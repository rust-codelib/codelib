//! Константно-генериковое ядро: [`Gf<P, M, POLY>`](Gf) — элемент
//! поля GF(p^m) с параметрами-константами.
//!
//! `P` — простое, `M` — степень расширения (1..=64), `POLY` —
//! упакованный мономиальный модуль степени `M`. Единственное
//! содержательное ограничение — порядок поля: `P^M <= 2^64`
//! («p^m влезает в 64-битное слово»). Нарушения (непростое `P`,
//! `M` вне диапазона, `POLY >= P^M`, `POLY = 0`) ловятся
//! **компилятором**: ассоциированная константа `CHECK` с assert'ами
//! вычисляется при первом же конструировании элемента.
//!
//! Все операции — `const fn`, пригодны для вычислений на этапе
//! компиляции. Соглашения нуля (как во всём крейте): `inv(0) = 0`,
//! `x / 0 = 0`, `0^0 = 1`.
//!
//! ⚠️ Контракты конструкторов [`new`](Gf::new) и
//! [`from_coeffs`](Gf::from_coeffs) проверяются только
//! `debug_assert!`: в release-сборке значение вне поля или
//! коэффициент `>= P` не диагностируются — элемент получается
//! неканоническим, арифметика молча возвращает мусор. Тотальный
//! конструктор — [`reduce`](Gf::reduce); проверяющая альтернатива
//! с `Result` — [`GfRuntime::new`](crate::GfRuntime::new) /
//! [`GfRuntime::from_coeffs`](crate::GfRuntime::from_coeffs).
//!
//! # Пример
//! ```rust
//! use gfpm::Gf9; // GF(3^2), модуль x^2 + x + 2
//!
//! let a = Gf9::new(5); // 5 = x + 2 (базис-3 цифры)
//! let b = Gf9::new(3); // 3 = x
//! assert_eq!(a * b, Gf9::new(4));    // (x+2)·x = x^2 + 2x = x + 1
//! assert_eq!(b.inv(), Gf9::new(4));  // x·(x+1) = x^2 + x = 1
//! assert_eq!(b.trace(), Gf9::new(2)); // Tr(x) = x + x^3 = 2
//! assert_eq!(b.norm(), Gf9::new(2));  // N(x) = x^4 = 2
//! ```

use core::fmt;
use core::ops::Neg;

use crate::fp::{add_mod, is_prime, mul_mod, order_checked, sub_mod};
use crate::poly::ACC_LEN;

/// Покомпонентное сравнение коэффициентов: `const fn` не может
/// звать `PartialEq`, сравниваем циклом.
const fn eq_raw<const M: usize>(a: [u32; M], b: [u32; M]) -> bool {
    let mut i = 0;
    while i < M {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Коэффициенты модуля (без старшей единицы): базис-P цифры POLY.
const fn poly_q<const P: u32, const M: usize, const POLY: u128>() -> [u32; M] {
    let mut q = [0u32; M];
    let mut v = POLY;
    let mut i = 0;
    while i < M {
        q[i] = (v % (P as u128)) as u32;
        v /= P as u128;
        i += 1;
    }
    q
}

/// Распаковка элемента (u64) в M коэффициентов.
const fn unpack_u64<const P: u32, const M: usize>(value: u64) -> [u32; M] {
    let mut c = [0u32; M];
    let mut v = value;
    let mut i = 0;
    while i < M {
        c[i] = (v % (P as u64)) as u32;
        v /= P as u64;
        i += 1;
    }
    c
}

/// Редукция буфера 64 коэффициентов по модулю POLY. Массив
/// передаётся по значению: `const fn` не принимает `&mut`.
const fn reduce_buf<const P: u32, const M: usize, const POLY: u128>(
    mut a: [u32; 64],
    q: [u32; M],
) -> [u32; 64] {
    let mut d = 64;
    while d > M {
        d -= 1;
        let t = a[d];
        if t == 0 {
            continue;
        }
        let mut j = 0;
        while j < M {
            if q[j] != 0 {
                a[d - M + j] = sub_mod(a[d - M + j], mul_mod(t, q[j], P), P);
            }
            j += 1;
        }
    }
    a
}

/// Школьное умножение с пошаговой редукцией — const-двойник
/// [`poly::mul_reduce`](crate::poly::mul_reduce) (срезы с `&mut`
/// недоступны в `const fn`). Аккумулятор фиксированного размера
/// [`ACC_LEN`](crate::poly::ACC_LEN): арифметика `2·M − 1` в длине
/// массива требует нестабильных generic-константных выражений.
const fn mul_raw<const P: u32, const M: usize>(a: [u32; M], b: [u32; M], q: [u32; M]) -> [u32; M] {
    let mut acc = [0u32; ACC_LEN];
    let mut i = 0;
    while i < M {
        if a[i] != 0 {
            let mut j = 0;
            while j < M {
                if b[j] != 0 {
                    acc[i + j] = add_mod(acc[i + j], mul_mod(a[i], b[j], P), P);
                }
                j += 1;
            }
        }
        i += 1;
    }
    let mut d = 2 * M - 1; // обрабатываем степени 2M−2 … M
    while d > M {
        d -= 1;
        let t = acc[d];
        if t == 0 {
            continue;
        }
        let mut j = 0;
        while j < M {
            if q[j] != 0 {
                acc[d - M + j] = sub_mod(acc[d - M + j], mul_mod(t, q[j], P), P);
            }
            j += 1;
        }
    }
    let mut out = [0u32; M];
    let mut k = 0;
    while k < M {
        out[k] = acc[k];
        k += 1;
    }
    out
}

/// Элемент поля GF(p^m) с константными параметрами.
///
/// Значение — коэффициенты многочлена над GF(p) степени `< M`
/// (по возрастанию). Инвариант: каждый коэффициент `< P`;
/// нарушение невозможно через [`new`](Self::new) и
/// [`from_coeffs`](Self::from_coeffs) в debug-сборках (в release
/// контракт не диагностируется — см. документацию конструкторов).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gf<const P: u32, const M: usize, const POLY: u128>([u32; M]);

impl<const P: u32, const M: usize, const POLY: u128> Gf<P, M, POLY> {
    /// Проверка параметров типа: простота `P`, диапазон `M`,
    /// порядок `P^M <= 2^64`, структура `POLY < P^M` и запрет
    /// `POLY = 0`. Замыкается (`let () = Self::CHECK;`) во всех
    /// конструкторах — нарушения превращаются в ошибки компиляции
    /// E0080 при первом использовании типа.
    const CHECK: () = {
        assert!(is_prime(P), "gfpm: P должно быть простым");
        assert!(M >= 1, "gfpm: M должно быть не меньше 1");
        assert!(M <= 64, "gfpm: M должно быть не больше 64");
        let order = match order_checked(P, M as u32) {
            Some(o) => o,
            None => panic!("gfpm: порядок поля P^M должен быть не больше 2^64"),
        };
        assert!(
            POLY < order,
            "gfpm: POLY должно быть меньше P^M (мономиальный многочлен степени M)"
        );
        assert!(
            POLY != 0,
            "gfpm: POLY = 0 запрещено: при M >= 2 модуль x^M приводим, \
             при M = 1 alpha() даёт нуль (берите модуль x − g, g — первообразный корень)"
        );
    };

    /// Порядок поля `P^M` (не больше 2^64 по построению типа).
    pub const ORDER: u128 = match order_checked(P, M as u32) {
        Some(o) => o,
        None => 0, // недостижимо: параметры проверены константой CHECK
    };

    /// Элемент из упакованного целого: базис-P цифры числа —
    /// коэффициенты многочлена. Контракт: `value < ORDER`
    /// (проверяется в debug-сборках).
    ///
    /// ⚠️ В release-сборке значение `>= ORDER` не диагностируется:
    /// старшие цифры молча отбрасываются. Если значение может
    /// выходить за диапазон, используйте тотальный
    /// [`reduce`](Self::reduce) или проверяющий
    /// [`GfRuntime::new`](crate::GfRuntime::new).
    #[must_use]
    pub const fn new(value: u64) -> Self {
        let () = Self::CHECK;
        debug_assert!(
            (value as u128) < Self::ORDER,
            "значение вне поля: используйте Gf::reduce"
        );
        Self(unpack_u64::<P, M>(value))
    }

    /// Элемент из произвольного целого: базис-P цифры
    /// интерпретируются как многочлен и приводятся по модулю POLY
    /// (аналог `reduce` в `gf2m`, где цифры — биты).
    ///
    /// # Пример
    /// ```rust
    /// use gfpm::Gf9;
    ///
    /// // 100 = (1, 0, 2, 0, 1)_3 = x^4 + 2x^2 + 1 ≡ x + 2 (mod x^2 + x + 2)
    /// assert_eq!(Gf9::reduce(100), Gf9::new(5));
    /// // значение в диапазоне не меняется
    /// assert_eq!(Gf9::reduce(7), Gf9::new(7));
    /// ```
    #[must_use]
    pub const fn reduce(value: u64) -> Self {
        let () = Self::CHECK;
        let mut a = [0u32; 64];
        let mut v = value;
        let mut i = 0;
        while i < 64 {
            a[i] = (v % (P as u64)) as u32;
            v /= P as u64;
            i += 1;
        }
        let a = reduce_buf::<P, M, POLY>(a, poly_q::<P, M, POLY>());
        let mut c = [0u32; M];
        let mut k = 0;
        while k < M {
            c[k] = a[k];
            k += 1;
        }
        Self(c)
    }

    /// Элемент из коэффициентов (по возрастанию). Контракт:
    /// каждый коэффициент `< P` (проверяется в debug-сборках).
    ///
    /// ⚠️ В release-сборке коэффициент `>= P` не диагностируется:
    /// инвариант элемента нарушается, арифметика и сравнения
    /// молча дают мусор. Проверяющая альтернатива —
    /// [`GfRuntime::from_coeffs`](crate::GfRuntime::from_coeffs).
    #[must_use]
    pub const fn from_coeffs(coeffs: [u32; M]) -> Self {
        let () = Self::CHECK;
        let mut i = 0;
        while i < M {
            debug_assert!(coeffs[i] < P, "коэффициент вне GF(P)");
            i += 1;
        }
        Self(coeffs)
    }

    /// Коэффициенты по возрастанию (каждый в `0..P`).
    #[must_use]
    pub const fn coeffs(self) -> [u32; M] {
        self.0
    }

    /// `i`-й коэффициент (`0` при `i >= M`).
    #[must_use]
    pub const fn coeff(self, i: usize) -> u32 {
        if i < M {
            self.0[i]
        } else {
            0
        }
    }

    /// Упакованное целое представление: `Σ c_i·P^i`
    /// (меньше `ORDER`, значит помещается в `u64`).
    #[must_use]
    pub const fn value(self) -> u64 {
        let mut r: u128 = 0;
        let mut i = M;
        while i > 0 {
            i -= 1;
            r = r * (P as u128) + self.0[i] as u128;
        }
        r as u64
    }

    /// Нулевой элемент.
    #[must_use]
    pub const fn zero() -> Self {
        let () = Self::CHECK;
        Self([0; M])
    }

    /// Единица.
    #[must_use]
    pub const fn one() -> Self {
        let () = Self::CHECK;
        let mut c = [0u32; M];
        c[0] = 1;
        Self(c)
    }

    /// Класс многочлена `x`. Примитивен тогда и только тогда, когда
    /// POLY — примитивный многочлен (все псевдонимы ниже используют
    /// именно такие). Для `M = 1` это вычет `−q_0`: модуль `x − g`
    /// с первообразным корнем `g` даёт `alpha() = g`, как в
    /// псевдонимах простых полей (`POLY = 0` — модуль `x` — запрещён
    /// проверкой параметров типа: alpha был бы нулём).
    #[must_use]
    pub const fn alpha() -> Self {
        let () = Self::CHECK;
        let mut c = [0u32; M];
        if M == 1 {
            let q0 = (POLY % (P as u128)) as u32;
            c[0] = if q0 == 0 { 0 } else { P - q0 };
        } else {
            c[1] = 1;
        }
        Self(c)
    }

    /// Проверка на нуль.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        eq_raw::<M>(self.0, [0; M])
    }

    /// Сложение: покомпонентно по модулю P.
    #[must_use]
    pub const fn add(self, rhs: Self) -> Self {
        let mut c = [0u32; M];
        let mut i = 0;
        while i < M {
            c[i] = add_mod(self.0[i], rhs.0[i], P);
            i += 1;
        }
        Self(c)
    }

    /// Вычитание: покомпонентно по модулю P.
    #[must_use]
    pub const fn sub(self, rhs: Self) -> Self {
        let mut c = [0u32; M];
        let mut i = 0;
        while i < M {
            c[i] = sub_mod(self.0[i], rhs.0[i], P);
            i += 1;
        }
        Self(c)
    }

    /// Противоположный элемент: `P − c` (нуль остаётся нулём).
    #[must_use]
    pub const fn neg(self) -> Self {
        let mut c = [0u32; M];
        let mut i = 0;
        while i < M {
            c[i] = if self.0[i] == 0 { 0 } else { P - self.0[i] };
            i += 1;
        }
        Self(c)
    }

    /// Умножение: школьное с пошаговой редукцией по POLY.
    #[must_use]
    pub const fn mul(self, rhs: Self) -> Self {
        Self(mul_raw::<P, M>(self.0, rhs.0, poly_q::<P, M, POLY>()))
    }

    /// Возведение в квадрат.
    #[must_use]
    pub const fn sqr(self) -> Self {
        self.mul(self)
    }

    /// Обратный по малой теореме Ферма: `a^(P^M − 2)`.
    /// Соглашение: `inv(0) = 0`.
    #[must_use]
    pub const fn inv(self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        Self::pow(self, (Self::ORDER - 2) as u64)
    }

    /// Деление: `self / rhs`. Соглашение: `x / 0 = 0`.
    #[must_use]
    pub const fn div(self, rhs: Self) -> Self {
        self.mul(rhs.inv())
    }

    /// Возведение в степень `k`. `0^0 = 1`; показатель ненулевых
    /// элементов приводится по модулю `P^M − 1`.
    #[must_use]
    pub const fn pow(self, k: u64) -> Self {
        if self.is_zero() {
            return if k == 0 { Self::one() } else { Self::zero() };
        }
        let mut e = k % ((Self::ORDER - 1) as u64);
        let mut r = Self::one();
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

    /// Фробениус: `x^P` — автоморфизм поля, фиксирующий GF(P).
    #[must_use]
    pub const fn frobenius(self) -> Self {
        Self::pow(self, P as u64)
    }

    /// След: `Tr(x) = x + x^P + … + x^(P^(M−1))`; результат лежит
    /// в простом подполе GF(P). Для `M = 1` — тождество.
    /// Сложность — O(M) возведений в степень P.
    #[must_use]
    pub const fn trace(self) -> Self {
        let mut acc = self;
        let mut cur = self;
        let mut i = 1;
        while i < M {
            cur = cur.frobenius();
            acc = acc.add(cur);
            i += 1;
        }
        acc
    }

    /// Норма: `N(x) = x^(1 + P + … + P^(M−1))`; результат лежит
    /// в GF(P), `N(x·y) = N(x)·N(y)`. Для `M = 1` — тождество.
    #[must_use]
    pub const fn norm(self) -> Self {
        let mut e: u64 = 1;
        let mut i = 1;
        while i < M {
            e = e * (P as u64) + 1; // e <= (P^M−1)/(P−1) < 2^64 — переполнения нет
            i += 1;
        }
        Self::pow(self, e)
    }

    /// Квадратность: лежит ли `x` в образе отображения
    /// `y ↦ y^2`. В характеристике 2 ответ всегда `true`
    /// (фробениус биективен); для нечётной — критерий Эйлера
    /// `x^((P^M − 1)/2) = 1` (нуль считается квадратом).
    #[must_use]
    pub const fn is_square(self) -> bool {
        if P == 2 {
            return true;
        }
        if self.is_zero() {
            return true;
        }
        eq_raw::<M>(
            Self::pow(self, ((Self::ORDER - 1) / 2) as u64).0,
            Self::one().0,
        )
    }

    /// Квадратный корень: `None` для невычетов, иначе некоторый
    /// корень (какой именно — не специфицировано; `sqrt(x)^2 = x`).
    ///
    /// Характеристика 2 — возведение в степень `2^(M−1)`;
    /// нечётная — алгоритм Тонелли–Шенкса над мультипликативной
    /// группой порядка `P^M − 1`.
    #[must_use]
    pub const fn sqrt(self) -> Option<Self> {
        if P == 2 {
            let mut e: u64 = 1;
            let mut i = 1;
            while i < M {
                e <<= 1;
                i += 1;
            }
            return Some(Self::pow(self, e));
        }
        if self.is_zero() {
            return Some(Self::zero());
        }
        if !self.is_square() {
            return None;
        }
        // Тонелли–Шенкс: ORDER − 1 = 2^s · t, t нечётное
        let q1 = (Self::ORDER - 1) as u64;
        let s = q1.trailing_zeros();
        let t = q1 >> s;
        let half = ((Self::ORDER - 1) / 2) as u64;
        let one = Self::one();
        let neg_one = Self::zero().sub(one);
        // невычет z: z^((ORDER−1)/2) = −1. При M >= 2 поиск начинается
        // с x (упакованное значение P): элементы простого подполя при
        // чётном M — все квадраты (a^((q−1)/2) = (a^(p−1))^((1+p+…+p^(M−1))/2),
        // а сумма 1+p+…+p^(M−1) из M нечётных слагаемых чётна), перебор
        // подполя занял бы ~P шагов. Невычеты вне подполя существуют
        // всегда, так что цикл завершается.
        let mut z_int: u64 = if M >= 2 { P as u64 } else { 2 };
        let z = loop {
            debug_assert!(
                (z_int as u128) < Self::ORDER,
                "невычет не найден — недостижимо для простого P"
            );
            let cand = Self::new(z_int);
            if eq_raw::<M>(Self::pow(cand, half).0, neg_one.0) {
                break cand;
            }
            z_int += 1;
        };
        let mut c = Self::pow(z, t);
        let mut r = Self::pow(self, t.div_ceil(2));
        let mut w = Self::pow(self, t);
        let mut m_cur = s;
        while !eq_raw::<M>(w.0, one.0) {
            // наименьшее i >= 1: w^(2^i) = 1 (инвариант гарантирует i < m_cur)
            let mut i: u32 = 0;
            let mut cur = w;
            while !eq_raw::<M>(cur.0, one.0) {
                cur = cur.sqr();
                i += 1;
            }
            debug_assert!(i < m_cur, "инвариант Тонелли–Шенкса нарушен");
            // b = c^(2^(m_cur−i−1))
            let mut b = c;
            let mut k = m_cur - i - 1;
            while k > 0 {
                b = b.sqr();
                k -= 1;
            }
            c = b.sqr();
            r = r.mul(b);
            w = w.mul(c);
            m_cur = i;
        }
        Some(r)
    }
}

impl<const P: u32, const M: usize, const POLY: u128> Default for Gf<P, M, POLY> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const P: u32, const M: usize, const POLY: u128> fmt::Display for Gf<P, M, POLY> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::poly::fmt_coeffs(f, &self.0)
    }
}

impl<const P: u32, const M: usize, const POLY: u128> fmt::Debug for Gf<P, M, POLY> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Gf").field(&self.0).finish()
    }
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inner:ident, $assign_trait:ident, $assign_method:ident) => {
        impl<const P: u32, const M: usize, const POLY: u128> core::ops::$trait for Gf<P, M, POLY> {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Self) -> Self {
                Gf::<P, M, POLY>::$inner(self, rhs)
            }
        }
        impl<const P: u32, const M: usize, const POLY: u128> core::ops::$assign_trait
            for Gf<P, M, POLY>
        {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                *self = Gf::<P, M, POLY>::$inner(*self, rhs);
            }
        }
    };
}

impl_binop!(Add, add, add, AddAssign, add_assign);
impl_binop!(Sub, sub, sub, SubAssign, sub_assign);
impl_binop!(Mul, mul, mul, MulAssign, mul_assign);
impl_binop!(Div, div, div, DivAssign, div_assign);

impl<const P: u32, const M: usize, const POLY: u128> Neg for Gf<P, M, POLY> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Gf::<P, M, POLY>::neg(self)
    }
}

// ---- псевдонимы ходовых полей ----
// Простые поля: POLY = p − g, где g — наименьший первообразный корень
// (модуль x − g), поэтому alpha() = g порождает GF(p)*.
// Расширения: первые примитивные многочлены — alpha() = x имеет
// порядок p^m − 1 (проверяется тестом primitive_aliases).

/// GF(3), alpha = 2.
pub type Gf3 = Gf<3, 1, 1>;
/// GF(5), alpha = 2.
pub type Gf5 = Gf<5, 1, 3>;
/// GF(7), alpha = 3.
pub type Gf7 = Gf<7, 1, 4>;
/// GF(11), alpha = 2.
pub type Gf11 = Gf<11, 1, 9>;
/// GF(13), alpha = 2.
pub type Gf13 = Gf<13, 1, 11>;
/// GF(251) — наибольшее простое меньше 2^8: байтовые коды РС; alpha = 6.
pub type Gf251 = Gf<251, 1, 245>;
/// GF(65537) — простое Ферма, 16-битные символы; alpha = 3.
pub type Gf65537 = Gf<65537, 1, 65534>;
/// GF(2147483647) — простое Мерсенна 2^31 − 1; alpha = 7.
pub type Gf2147483647 = Gf<2147483647, 1, 2147483640>;
/// GF(3^2) = GF(9), примитивный модуль x^2 + x + 2.
pub type Gf9 = Gf<3, 2, 5>;
/// GF(3^3) = GF(27), примитивный модуль x^3 + 2x + 1.
pub type Gf27 = Gf<3, 3, 7>;
/// GF(3^4) = GF(81), примитивный модуль x^4 + x + 2.
pub type Gf81 = Gf<3, 4, 5>;
/// GF(5^2) = GF(25), примитивный модуль x^2 + x + 2.
pub type Gf25 = Gf<5, 2, 7>;
/// GF(5^3) = GF(125), примитивный модуль x^3 + 3x + 2.
pub type Gf125 = Gf<5, 3, 17>;
/// GF(7^2) = GF(49), примитивный модуль x^2 + x + 3.
pub type Gf49 = Gf<7, 2, 10>;
/// GF(11^2) = GF(121), примитивный модуль x^2 + x + 7.
pub type Gf121 = Gf<11, 2, 18>;
/// GF(13^2) = GF(169), примитивный модуль x^2 + x + 2.
pub type Gf169 = Gf<13, 2, 15>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operators_match_methods() {
        let a = Gf9::new(5);
        let b = Gf9::new(7);
        assert_eq!(a + b, a.add(b));
        assert_eq!(a - b, a.sub(b));
        assert_eq!(a * b, a.mul(b));
        assert_eq!(a / b, a.div(b));
        assert_eq!(-a, a.neg());
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

    #[test]
    fn default_is_zero() {
        assert_eq!(Gf25::default(), Gf25::zero());
        assert!(Gf25::default().is_zero());
    }

    #[test]
    fn coeffs_accessors() {
        let a = Gf9::from_coeffs([2, 1]); // x + 2
        assert_eq!(a.coeffs(), [2, 1]);
        assert_eq!(a.coeff(0), 2);
        assert_eq!(a.coeff(1), 1);
        assert_eq!(a.coeff(2), 0); // за пределами M
        assert_eq!(a.value(), 5);
        assert_eq!(Gf9::new(5), a); // round trip
                                    // alpha = x для M >= 2
        assert_eq!(Gf9::alpha().coeffs(), [0, 1]);
        // GF(5): alpha = 2 (модуль x − 2)
        assert_eq!(Gf5::alpha(), Gf5::new(2));
    }

    #[test]
    fn const_context_computations() {
        // арифметика доступна на этапе компиляции
        const PRODUCT: Gf9 = Gf9::new(5).mul(Gf9::new(3));
        const INV: Gf9 = Gf9::new(3).inv();
        const TRACE: Gf9 = Gf9::new(3).trace();
        const NORM: Gf9 = Gf9::new(3).norm();
        assert_eq!(PRODUCT, Gf9::new(4));
        assert_eq!(INV, Gf9::new(4));
        assert_eq!(TRACE, Gf9::new(2)); // Tr(x) = 2
        assert_eq!(NORM, Gf9::new(2)); // N(x) = x^4 = 2
    }
}
