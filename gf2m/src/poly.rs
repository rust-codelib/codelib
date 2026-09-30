//! Многочлены над GF(2) в машинном слове: степень, сумма, бесструктурное
//! произведение и остаток от деления.
//!
//! Биты `u32` — коэффициенты: бит `i` — коэффициент при `x^i`.
//! Нулевой многочлен — это `0`; его степень — [`u32::MAX`].

/// Степень многочлена: номер старшего установленного бита.
/// Для нулевого многочлена возвращает `u32::MAX` (соглашение «−1»).
///
/// # Пример
/// ```rust
/// use gf2m::poly_degree;
///
/// assert_eq!(poly_degree(0b1011), 3); // x^3 + x + 1
/// assert_eq!(poly_degree(1), 0);      // константа 1
/// assert_eq!(poly_degree(0), u32::MAX); // нулевой многочлен
/// ```
#[must_use]
pub const fn poly_degree(p: u32) -> u32 {
    if p == 0 {
        u32::MAX
    } else {
        31 - p.leading_zeros()
    }
}

/// Сумма (и она же разность) многочленов над GF(2): побитовое XOR.
///
/// # Пример
/// ```rust
/// use gf2m::poly_add;
///
/// // (x^2 + 1) + (x^2 + x) = x + 1
/// assert_eq!(poly_add(0b101, 0b110), 0b011);
/// ```
#[must_use]
pub const fn poly_add(a: u32, b: u32) -> u32 {
    a ^ b
}

/// Бесструктурное умножение (carry-less multiply): свёртка коэффициентов
/// без переносов — произведение многочленов над GF(2).
///
/// Для операндов степени не выше 15 результат точен в `u32`
/// (степень произведения не выше 30); старшие биты свыше 31
/// обрезаются.
///
/// # Пример
/// ```rust
/// use gf2m::clmul;
///
/// // (x + 1)^2 = x^2 + 1 над GF(2)
/// assert_eq!(clmul(0b11, 0b11), 0b101);
/// ```
#[must_use]
pub const fn clmul(a: u32, b: u32) -> u32 {
    let mut r = 0;
    let mut a = a;
    let mut b = b;
    while b != 0 {
        if b & 1 != 0 {
            r ^= a;
        }
        a <<= 1;
        b >>= 1;
    }
    r
}

/// Остаток от деления `a` на `b` в кольце GF(2)\[x\].
///
/// Контракт: `b != 0` — деление на нулевой многочлен не определено
/// (при `b == 0` функция молча возвращает `a` без изменений;
/// это гарантированно не зацикливается).
///
/// # Пример
/// ```rust
/// use gf2m::poly_mod;
///
/// // (x^2 + x + 1)(x^3 + x^2 + 1) = x^5 + x + 1 — делится нацело
/// assert_eq!(poly_mod(0b100011, 0b111), 0);
/// // x^4 + 1 = (x^2 + x + 1)(x^2 + x) + (x + 1)
/// assert_eq!(poly_mod(0b10001, 0b111), 0b11);
/// ```
#[must_use]
pub const fn poly_mod(mut a: u32, b: u32) -> u32 {
    if b == 0 {
        return a;
    }
    let db = poly_degree(b);
    while a != 0 && poly_degree(a) >= db {
        let shift = poly_degree(a) - db;
        a ^= b << shift;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::{clmul, poly_add, poly_degree, poly_mod};

    #[test]
    fn degree_of_zero_is_max() {
        assert_eq!(poly_degree(0), u32::MAX);
        assert_eq!(poly_degree(1), 0);
        assert_eq!(poly_degree(1 << 31), 31);
    }

    #[test]
    fn mod_by_zero_returns_dividend() {
        // Контракт задокументирован: при b == 0 возвращается a.
        assert_eq!(poly_mod(0b1011, 0), 0b1011);
    }

    #[test]
    fn clmul_matches_shift_and_xor() {
        // Сверка с прямым вычислением для небольших операндов
        for a in 0..64u32 {
            for b in 0..64u32 {
                let mut expected = 0u32;
                for i in 0..6 {
                    if b & (1 << i) != 0 {
                        expected ^= a << i;
                    }
                }
                assert_eq!(clmul(a, b), expected, "clmul({a}, {b})");
            }
        }
    }

    #[test]
    fn mod_division_invariant() {
        for b in 1..32u32 {
            for a in 0..1024u32 {
                let r = poly_mod(a, b);
                // остаток меньше делителя по степени
                assert!(r == 0 || poly_degree(r) < poly_degree(b));
                // a + r делится на b
                let q = poly_add(a, r);
                assert_eq!(poly_mod(q, b), 0, "a = {a}, b = {b}");
            }
        }
    }
}
