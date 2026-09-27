//! Арифметика простого поля GF(p): константные примитивы.
//!
//! Это нижний слой крейта: модулярные операции, из которых
//! собирается арифметика расширений GF(p^m), а также проверка
//! простоты характеристики. Все функции — `const fn`, работают
//! в `no_std` и пригодны для вычислений на этапе компиляции.
//!
//! Реализации тотальны: любые `u32` на входе дают корректный
//! результат по модулю `p` (промежуточные вычисления — в `u64`).
//! Единственное соглашение — [`inv_mod`]: она обращает по малой
//! теореме Ферма и потому требует простого `p`; нуль переводится
//! в нуль (соглашение всего крейта).
//!
//! # Пример
//! ```rust
//! use gfpm::fp::{add_mod, inv_mod, mul_mod, pow_mod, sub_mod};
//!
//! assert_eq!(add_mod(3, 4, 5), 2);
//! assert_eq!(sub_mod(3, 4, 5), 4);        // −1 ≡ 4
//! assert_eq!(mul_mod(3, 4, 5), 2);
//! assert_eq!(pow_mod(2, 10, 1000), 24);   // 1024 mod 1000
//! assert_eq!(inv_mod(3, 5), 2);           // 3·2 = 6 ≡ 1
//! ```

/// Проверка простоты перебором нечётных делителей: O(√n).
///
/// Подходит и для этапа компиляции (в проверках параметров типа
/// [`Gf`](crate::Gf)), и для рантайма ([`FieldParams`](crate::FieldParams)).
/// Для `p < 2^32` перебор ограничен ~65536 итерациями.
///
/// # Пример
/// ```rust
/// use gfpm::fp::is_prime;
///
/// assert!(is_prime(2));
/// assert!(is_prime(65537));
/// assert!(is_prime(2147483647));
/// assert!(!is_prime(1));
/// assert!(!is_prime(561)); // число Кармайкла
/// ```
#[must_use]
pub const fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    if n % 3 == 0 {
        return n == 3;
    }
    let mut d: u64 = 5;
    while d * d <= n as u64 {
        if n as u64 % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

/// `(a + b) mod p`.
///
/// Тотальна: корректна для любых `u32` на входе. Приводить
/// операнды к `p` заранее не нужно — сумма двух `u32` помещается
/// в `u64`, а итоговый `% p` даёт тот же вычет. Деление на ноль
/// в `p = 0` паникует, как любое целочисленное деление; контракт
/// слоя — `p >= 1`.
#[must_use]
pub const fn add_mod(a: u32, b: u32, p: u32) -> u32 {
    (((a as u64) + (b as u64)) % (p as u64)) as u32
}

/// `(a − b) mod p` — тотальна, как [`add_mod`].
///
/// Вычитаемое приводится к `p` явно: у `u64` нет отрицательных
/// значений, и без приведения выражение `a + p − b` могло бы
/// уйти в ноль при `b > a + p`. Уменьшаемое приводить не нужно —
/// `a + p − (b mod p)` всегда неотрицательно (вычитается меньше `p`).
#[must_use]
pub const fn sub_mod(a: u32, b: u32, p: u32) -> u32 {
    (((a as u64) + (p as u64) - ((b as u64) % (p as u64))) % (p as u64)) as u32
}

/// `(a · b) mod p` — тотальна; произведение вычисляется в `u64`,
/// поэтому любые `a, b < 2^32` дают точный результат.
#[must_use]
pub const fn mul_mod(a: u32, b: u32, p: u32) -> u32 {
    ((a as u64 * b as u64) % p as u64) as u32
}

/// `base^exp mod p` — двоичное возведение в степень.
///
/// `0^0 = 1` (соглашение крейта); `p = 1` корректно даёт нуль.
#[must_use]
pub const fn pow_mod(base: u32, mut exp: u64, p: u32) -> u32 {
    let mut r: u32 = 1 % p;
    let mut b: u32 = (base as u64 % p as u64) as u32;
    while exp > 0 {
        if exp & 1 == 1 {
            r = mul_mod(r, b, p);
        }
        b = mul_mod(b, b, p);
        exp >>= 1;
    }
    r
}

/// Обратный элемент по малой теореме Ферма: `a^(p−2) mod p`.
///
/// Контракт: `p` — простое (иначе результат бессмысленнен).
/// Соглашение нуля: `a ≡ 0` переводится в `0`; вырожденное
/// `p < 2` — тоже в `0`.
#[must_use]
pub const fn inv_mod(a: u32, p: u32) -> u32 {
    if p < 2 || a % p == 0 {
        return 0;
    }
    pow_mod(a, p as u64 - 2, p)
}

/// Порядок поля GF(p^m), если он не превосходит 2^64; иначе `None`.
///
/// Это единое ограничение крейта — «p^m влезает в 64-битное слово».
/// Простота `p` не проверяется (это [`is_prime`]); контракт — `p >= 2`.
///
/// # Пример
/// ```rust
/// use gfpm::fp::order_checked;
///
/// assert_eq!(order_checked(2, 64), Some(1u128 << 64)); // ровно 2^64 — ещё влезает
/// assert_eq!(order_checked(2, 65), None);
/// assert!(order_checked(3, 40).is_some());
/// assert_eq!(order_checked(3, 41), None);
/// ```
#[must_use]
pub const fn order_checked(p: u32, m: u32) -> Option<u128> {
    let limit: u128 = 1 << 64;
    let mut r: u128 = 1;
    let mut i: u32 = 0;
    while i < m {
        match r.checked_mul(p as u128) {
            Some(v) => r = v,
            None => return None,
        }
        if r > limit {
            return None;
        }
        i += 1;
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::{add_mod, inv_mod, is_prime, mul_mod, order_checked, pow_mod, sub_mod};

    #[test]
    fn primality_known_values() {
        let primes = [
            2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 97, 101, 251, 257, 65537, 2147483647,
            4294967291,
        ];
        for &p in &primes {
            assert!(is_prime(p), "{p} должно быть простым");
        }
        let composites = [
            0u32, 1, 4, 6, 8, 9, 15, 21, 25, 27, 49, 561, 1105, 65536, 2147483649, 4294967295,
        ];
        for &c in &composites {
            assert!(!is_prime(c), "{c} должно быть составным");
        }
    }

    #[test]
    fn mod_ops_match_naive() {
        const P: u32 = 97;
        for a in 0..P {
            for b in 0..P {
                assert_eq!(add_mod(a, b, P), (a + b) % P, "add({a}, {b})");
                assert_eq!(sub_mod(a, b, P), ((a + P - b) % P), "sub({a}, {b})");
                assert_eq!(mul_mod(a, b, P), (a * b) % P, "mul({a}, {b})");
            }
        }
    }

    #[test]
    fn mod_ops_total_for_arbitrary_inputs() {
        // операнды вне 0..p: результат всё равно корректный вычет
        const P: u32 = 97;
        let cases = [
            (0u32, 0u32),
            (0, 96),
            (96, 96),
            (100, 50),
            (123_456_789, 4_000_000_000),
            (u32::MAX, u32::MAX),
            (u32::MAX, 1),
            (1, u32::MAX),
        ];
        for (a, b) in cases {
            let (ra, rb) = (a % P, b % P);
            assert_eq!(add_mod(a, b, P), (ra + rb) % P, "add({a}, {b})");
            assert_eq!(sub_mod(a, b, P), (ra + P - rb) % P, "sub({a}, {b})");
            assert_eq!(mul_mod(a, b, P), (ra * rb) % P, "mul({a}, {b})");
        }
    }

    #[test]
    fn pow_mod_matches_loop() {
        const P: u32 = 251;
        for base in 0..P {
            let mut expected: u32 = 1;
            for e in 0..=300u64 {
                assert_eq!(pow_mod(base, e, P), expected, "{base}^{e}");
                expected = mul_mod(expected, base, P);
            }
        }
    }

    #[test]
    fn inv_mod_round_trip() {
        for p in [2u32, 3, 5, 7, 251, 65537] {
            assert_eq!(inv_mod(0, p), 0, "inv(0) = 0");
            for a in 1..p {
                let inv = inv_mod(a, p);
                assert_ne!(inv, 0, "inv({a}) не нуль в GF({p})");
                assert_eq!(mul_mod(a, inv, p), 1, "{a}·inv({a}) в GF({p})");
            }
        }
        assert_eq!(inv_mod(3, 1), 0); // вырожденный модуль
    }

    #[test]
    fn order_checked_boundaries() {
        assert_eq!(order_checked(2, 0), Some(1)); // тривиальный случай
        assert_eq!(order_checked(2, 1), Some(2));
        assert_eq!(order_checked(2, 64), Some(1u128 << 64));
        assert_eq!(order_checked(2, 65), None);
        assert!(order_checked(3, 40).is_some());
        assert_eq!(order_checked(3, 41), None);
        assert_eq!(order_checked(65537, 4), None); // (2^16+1)^4 > 2^64
        assert!(order_checked(65537, 3).is_some());
        assert!(order_checked(2147483647, 2).is_some()); // ~2^62
        assert_eq!(order_checked(2147483647, 3), None);
    }
}
