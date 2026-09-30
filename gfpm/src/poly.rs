//! Многочлены над GF(p): срезы коэффициентов по возрастанию.
//!
//! Слой обслуживает три задачи: конструирование полей (тест Рабина
//! [`is_irreducible`], поиск [`find_irreducible`]), арифметику
//! рантайм-слоя ([`mul_reduce`], [`reduce_in_place`]) и общие
//! полиномиальные операции ([`poly_mul`], [`poly_divmod`], …)
//! для внешних алгоритмов.
//!
//! Соглашения слоя:
//! - коэффициенты — срезы `&[u32]` по возрастанию степеней;
//!   хвостовые нули допустимы, степень — [`poly_degree`]
//!   (нулевой многочлен — `usize::MAX`, соглашение «−1», как в `gf2m`);
//! - `p` — простое ([`fp::is_prime`](crate::fp::is_prime)): деление и НОД
//!   обращают старший коэффициент по Ферма;
//! - степени многочленов в операциях с фиксированными буферами
//!   (НОД, Рабин) — не больше [`MAX_DEG`], как и весь крейт;
//! - функции не аллоцируют: результаты пишутся в срезы вызывающего.
//!
//! ⚠️ Контракты размеров буферов и диапазонов проверяются только
//! `debug_assert!`: **в release-сборке их нарушение не
//! диагностируется** — результат молча неверен либо происходит
//! паника при выходе за границы среза (точный сценарий указан
//! в документации каждой функции). Если длины приходят из внешних
//! данных, валидируйте их заранее; для конструирования элементов
//! поля с проверками есть рантайм-слой
//! ([`GfRuntime`](crate::GfRuntime)), чьи конструкторы возвращают
//! `Result`.
//!
//! # Пример
//! ```rust
//! use gfpm::poly::{find_irreducible, is_irreducible};
//!
//! // x^2 + 1 над GF(3) неприводим (−1 — не квадрат), над GF(5) — нет
//! assert!(is_irreducible(&[1, 0, 1], 3));
//! assert!(!is_irreducible(&[1, 0, 1], 5));
//!
//! // Первый неприводимый степени 2 над GF(3) — это x^2 + 1
//! assert_eq!(find_irreducible(3, 2), Some(1));
//! ```

use core::fmt;

use crate::fp::{add_mod, inv_mod, is_prime, mul_mod, order_checked, sub_mod};

/// Максимальная поддерживаемая степень многочлена (степень
/// расширения всего крейта и размер буферов этого слоя).
/// Единый предел для всех слоёв: [`FieldParams::MAX_M`](crate::FieldParams::MAX_M)
/// определён через эту константу.
pub const MAX_DEG: usize = 64;

/// Размер аккумулятора умножения [`mul_reduce`] (и const-ядра
/// [`Gf`](crate::Gf)): произведение многочленов степени `< m` имеет
/// степень не выше `2m − 2`, то есть `2m − 1` коэффициентов при
/// `m <= MAX_DEG`. Общая константа всех слоёв крейта — чтобы
/// размер буферов не задавался «магическими» числами.
pub const ACC_LEN: usize = 2 * MAX_DEG - 1;

/// Степень многочлена: индекс старшего ненулевого коэффициента.
/// Для нулевого многочлена — `usize::MAX` (соглашение «−1»).
///
/// # Пример
/// ```rust
/// use gfpm::poly::poly_degree;
///
/// assert_eq!(poly_degree(&[1, 0, 2]), 2); // 2x^2 + 1
/// assert_eq!(poly_degree(&[5]), 0);
/// assert_eq!(poly_degree(&[0, 0]), usize::MAX);
/// ```
#[must_use]
pub fn poly_degree(a: &[u32]) -> usize {
    let mut d = a.len();
    while d > 0 && a[d - 1] == 0 {
        d -= 1;
    }
    if d == 0 {
        usize::MAX
    } else {
        d - 1
    }
}

/// `c = a + b` покомпонентно; длины операндов выравниваются нулями.
///
/// Контракт: `out.len() >= max(a.len(), b.len())`; весь `out`
/// перезаписывается (сумма, затем нули).
///
/// В release-сборке слишком короткий `out` не диагностируется:
/// цикл идёт по `out.len()` и старшие коэффициенты суммы молча
/// теряются.
pub fn poly_add(a: &[u32], b: &[u32], p: u32, out: &mut [u32]) {
    let n = a.len().max(b.len());
    debug_assert!(out.len() >= n, "буфер суммы слишком мал");
    for i in 0..out.len() {
        let x = if i < a.len() { a[i] } else { 0 };
        let y = if i < b.len() { b[i] } else { 0 };
        out[i] = add_mod(x, y, p);
    }
}

/// `c = a − b` покомпонентно; контракт как у [`poly_add`] (в release
/// короткий `out` молча усекает результат).
pub fn poly_sub(a: &[u32], b: &[u32], p: u32, out: &mut [u32]) {
    let n = a.len().max(b.len());
    debug_assert!(out.len() >= n, "буфер разности слишком мал");
    for i in 0..out.len() {
        let x = if i < a.len() { a[i] } else { 0 };
        let y = if i < b.len() { b[i] } else { 0 };
        out[i] = sub_mod(x, y, p);
    }
}

/// `c = a · b` — полное произведение без редукции.
///
/// Контракт: `out.len() >= a.len() + b.len() − 1` для непустых
/// операндов; пустой операнд — нулевой многочлен. Весь `out`
/// перезаписывается (произведение, затем нули).
///
/// В release-сборке слишком короткий `out` даёт панику
/// (запись `out[i + j]` за границей среза).
///
/// # Пример
/// ```rust
/// use gfpm::poly::poly_mul;
///
/// // (x + 1)(x + 2) = x^2 + 3x + 2 = x^2 + 2 над GF(3)
/// let mut c = [0u32; 3];
/// poly_mul(&[1, 1], &[2, 1], 3, &mut c);
/// assert_eq!(c, [2, 0, 1]);
/// ```
pub fn poly_mul(a: &[u32], b: &[u32], p: u32, out: &mut [u32]) {
    out.fill(0);
    if a.is_empty() || b.is_empty() {
        return;
    }
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0 {
            continue;
        }
        for (j, &bj) in b.iter().enumerate() {
            if bj == 0 {
                continue;
            }
            out[i + j] = add_mod(out[i + j], mul_mod(ai, bj, p), p);
        }
    }
}

/// `a = q·b + r`, `deg r < deg b`; вычисляет `q` и `r`.
///
/// Контракт: `b` — ненулевой многочлен (при нулевом защитно
/// возвращается `q = 0`, `r = a`, как в `gf2m::poly_mod`);
/// `q_out.len() >= a.len()`, `r_out.len() >= a.len()`; оба
/// буфера перезаписываются целиком.
///
/// В release-сборке буферы короче `a` дают панику
/// (`copy_from_slice`/индекс за границей среза).
pub fn poly_divmod(a: &[u32], b: &[u32], p: u32, q_out: &mut [u32], r_out: &mut [u32]) {
    debug_assert!(q_out.len() >= a.len() && r_out.len() >= a.len());
    q_out.fill(0);
    r_out.fill(0);
    let la = a.len();
    r_out[..la].copy_from_slice(a);

    let db = poly_degree(b);
    if db == usize::MAX {
        return; // деление на нулевой многочлен — контракт нарушен, защитно q = 0, r = a
    }
    let inv_lead = inv_mod(b[db], p);
    let mut d = poly_degree(&r_out[..la]);
    while d != usize::MAX && d >= db {
        let coef = mul_mod(r_out[d], inv_lead, p);
        if coef != 0 {
            let shift = d - db;
            q_out[shift] = coef;
            for j in 0..=db {
                r_out[shift + j] = sub_mod(r_out[shift + j], mul_mod(coef, b[j], p), p);
            }
        }
        if d == 0 {
            break; // деление на константу: остаток обнулён, дальше некуда
        }
        d -= 1;
    }
}

/// Значение многочлена в точке `x ∈ GF(p)` по схеме Горнера.
///
/// # Пример
/// ```rust
/// use gfpm::poly::poly_eval;
///
/// // x^2 + 1 в точке 2 над GF(5): 4 + 1 = 0
/// assert_eq!(poly_eval(&[1, 0, 1], 2, 5), 0);
/// ```
#[must_use]
pub fn poly_eval(a: &[u32], x: u32, p: u32) -> u32 {
    let mut acc: u32 = 0;
    for &c in a.iter().rev() {
        acc = add_mod(mul_mod(acc, x, p), c, p);
    }
    acc
}

/// Нормировка к мономиальному виду; `false` — если `a` нулевой
/// (тогда `out` обнуляется). Весь `out` перезаписывается.
pub fn poly_monic(a: &[u32], p: u32, out: &mut [u32]) -> bool {
    out.fill(0);
    let la = a.len().min(out.len());
    out[..la].copy_from_slice(&a[..la]);
    let d = poly_degree(out);
    if d == usize::MAX {
        return false;
    }
    let inv = inv_mod(out[d], p);
    for c in out[..=d].iter_mut() {
        *c = mul_mod(*c, inv, p);
    }
    true
}

/// НОД двух многочленов (алгоритм Евклида), результат — мономиальный.
///
/// Контракт: `a.len() <= MAX_DEG + 1 && b.len() <= MAX_DEG + 1`
/// (буферы слоя); `out.len() >= max(a.len(), b.len())`. Нулевой `out`
/// соответствует `НОД(0, 0) = 0`.
///
/// В release-сборке операнд длиннее `MAX_DEG + 1` не
/// диагностируется и молча усекается до `MAX_DEG + 1` коэффициентов.
pub fn poly_gcd(a: &[u32], b: &[u32], p: u32, out: &mut [u32]) {
    debug_assert!(a.len() <= MAX_DEG + 1 && b.len() <= MAX_DEG + 1);
    let mut x = [0u32; MAX_DEG + 1];
    let mut y = [0u32; MAX_DEG + 1];
    let mut r = [0u32; MAX_DEG + 1];
    let mut q = [0u32; MAX_DEG + 1];
    x[..a.len().min(MAX_DEG + 1)].copy_from_slice(&a[..a.len().min(MAX_DEG + 1)]);
    y[..b.len().min(MAX_DEG + 1)].copy_from_slice(&b[..b.len().min(MAX_DEG + 1)]);
    while poly_degree(&y) != usize::MAX {
        poly_divmod(&x, &y, p, &mut q, &mut r);
        x = y;
        y = r;
    }
    poly_monic(&x, p, out);
}

/// Умножение с пошаговой редукцией: `a · b mod (x^m + Σ q_j x^j)`.
///
/// Старшая единица модуля не передаётся — только коэффициенты
/// `q_0..q_{m−1}`; это основная операция умножения в поле
/// (используется и рантайм-слоем, и тестом Рабина).
///
/// Контракт: `a.len() == b.len() == q.len() == m <= MAX_DEG`,
/// `acc.len() >= 2m − 1` (естественный выбор буфера —
/// [`ACC_LEN`]); коэффициенты `a`, `b` уже меньше `p`.
/// Результат — `acc[..m]`; диапазон `acc[..2m−1]` перезаписывается
/// целиком, хвост не трогается.
///
/// В release-сборке нарушение контракта не диагностируется:
/// операнды короче `m` или аккумулятор короче `2m − 1` дают
/// панику (индекс за границей среза).
pub fn mul_reduce(a: &[u32], b: &[u32], q: &[u32], p: u32, acc: &mut [u32]) {
    let m = q.len();
    if m == 0 {
        return;
    }
    debug_assert_eq!(a.len(), m);
    debug_assert_eq!(b.len(), m);
    debug_assert!(m <= MAX_DEG);
    debug_assert!(acc.len() >= 2 * m - 1);
    for v in acc.iter_mut().take(2 * m - 1) {
        *v = 0;
    }
    for i in 0..m {
        if a[i] == 0 {
            continue;
        }
        for j in 0..m {
            if b[j] == 0 {
                continue;
            }
            acc[i + j] = add_mod(acc[i + j], mul_mod(a[i], b[j], p), p);
        }
    }
    let mut d = 2 * m - 1; // обрабатываем степени 2m−2 … m
    while d > m {
        d -= 1;
        let t = acc[d];
        if t == 0 {
            continue;
        }
        for j in 0..m {
            if q[j] == 0 {
                continue;
            }
            acc[d - m + j] = sub_mod(acc[d - m + j], mul_mod(t, q[j], p), p);
        }
    }
}

/// Редукция многочлена произвольной длины по модулю
/// `x^m + Σ q_j x^j` — на месте.
///
/// После возврата `a[..m]` содержит остаток, `a[m..]` — нули.
/// Используется [`GfRuntime::reduce`](crate::GfRuntime::reduce)
/// и константным ядром для приведения целых вне диапазона.
pub fn reduce_in_place(a: &mut [u32], q: &[u32], p: u32) {
    let m = q.len();
    if m == 0 || a.len() <= m {
        return;
    }
    let mut d = a.len(); // обрабатываем степени len−1 … m
    while d > m {
        d -= 1;
        let t = a[d];
        a[d] = 0;
        if t == 0 {
            continue;
        }
        for j in 0..m {
            if q[j] == 0 {
                continue;
            }
            a[d - m + j] = sub_mod(a[d - m + j], mul_mod(t, q[j], p), p);
        }
    }
}

/// Упаковка коэффициентов (по возрастанию) в базисе p: `Σ c_i·p^i`.
///
/// `None` — если значение не помещается в `u128` или какой-то
/// коэффициент не меньше `p` (упаковка обязана быть однозначной).
///
/// # Пример
/// ```rust
/// use gfpm::poly::{pack_poly, unpack_poly};
///
/// // x^2 + x + 2 над GF(3): 2 + 1·3 + 1·9 = 14
/// assert_eq!(pack_poly(&[2, 1, 1], 3), Some(14));
/// let mut q = [0u32; 3];
/// unpack_poly(14, 3, 3, &mut q);
/// assert_eq!(q, [2, 1, 1]);
/// ```
#[must_use]
pub fn pack_poly(coeffs: &[u32], p: u32) -> Option<u128> {
    if p < 2 {
        return None;
    }
    let mut r: u128 = 0;
    for &c in coeffs.iter().rev() {
        if c >= p {
            return None;
        }
        r = r.checked_mul(p as u128)?.checked_add(c as u128)?;
    }
    Some(r)
}

/// Распаковка `m` младших базис-p цифр числа `packed`.
///
/// Контракт: `out.len() >= m`, `p >= 2`. Цифры сверх `m`
/// отбрасываются (для упакованных модулей и элементов поля
/// их просто не бывает).
///
/// В release-сборке `out` короче `m` не диагностируется:
/// молча записываются только `out.len()` младших цифр.
pub fn unpack_poly(packed: u128, p: u32, m: u32, out: &mut [u32]) {
    debug_assert!(out.len() >= m as usize);
    let mut v = packed;
    for slot in out.iter_mut().take(m as usize) {
        *slot = (v % p as u128) as u32;
        v /= p as u128;
    }
}

/// Тест Рабина на неприводимость мономиального многочлена `f`
/// степени m над GF(p).
///
/// `f` — коэффициенты по возрастанию, `f[m] == 1`, `2 <= m <= 64`;
/// при нарушении структуры возвращается `false` (включая `p`,
/// не являющееся простым). Критерий: `x^(p^m) ≡ x (mod f)` и
/// `НОД(x^(p^(m/q)) − x, f) = 1` для каждого простого делителя `q`
/// числа `m`.
///
/// Сложность — O(m·log p) умножений многочленов.
#[must_use]
pub fn is_irreducible(f: &[u32], p: u32) -> bool {
    if f.len() < 2 || f.len() > MAX_DEG + 1 || f[f.len() - 1] != 1 {
        return false;
    }
    if !is_prime(p) {
        return false;
    }
    let m = f.len() - 1;
    if m == 1 {
        return true; // линейный мономиальный всегда неприводим
    }
    let mut x = [0u32; MAX_DEG + 1];
    x[1] = 1; // класс x (степень 1 < m)
    let mut a = [0u32; MAX_DEG + 1];
    let mut b = [0u32; MAX_DEG + 1];
    let mut acc = [0u32; ACC_LEN];

    // Условие 1: x^(p^m) ≡ x
    a[..m].copy_from_slice(&x[..m]);
    for _ in 0..m {
        pow_x_p(&a[..m], f, p, &mut b[..m], &mut acc);
        core::mem::swap(&mut a, &mut b);
    }
    if a[..m] != x[..m] {
        return false;
    }

    // Условие 2: НОД(x^(p^(m/q)) − x, f) = 1 для каждого простого q | m
    for q in prime_divisors(m as u64).into_iter().take_while(|&q| q != 0) {
        a[..m].copy_from_slice(&x[..m]);
        for _ in 0..(m as u64 / q) {
            pow_x_p(&a[..m], f, p, &mut b[..m], &mut acc);
            core::mem::swap(&mut a, &mut b);
        }
        let mut diff = [0u32; MAX_DEG + 2];
        poly_sub(&a[..m + 1], &x[..m + 1], p, &mut diff[..m + 1]);
        let mut g = [0u32; MAX_DEG + 1];
        poly_gcd(&diff[..m + 1], f, p, &mut g);
        let dg = poly_degree(&g);
        if dg != usize::MAX && dg >= 1 {
            return false; // нетривиальный делитель
        }
    }
    true
}

/// Первый неприводимый мономиальный многочлен степени `m` над GF(p)
/// в порядке возрастания упакованного значения — среди кандидатов
/// **с ненулевым свободным членом** (фильтр `n % p != 0`).
///
/// Кандидаты с нулевым свободным членом пропускаются: при `m >= 2`
/// они кратны `x`, то есть заведомо приводимы. Побочный эффект для
/// `m = 1`: кандидат `poly = 0` (модуль `x`) не рассматривается,
/// поэтому результат для `m = 1` — всегда `x + 1` (упакованное
/// значение 1), а не `x`.
///
/// Возвращается упакованное значение `Σ q_j·p^j` (без старшей
/// единицы). `None` — если `p` не простое, `m` вне 1..=64 или
/// `p^m > 2^64` (математически неприводимый существует всегда,
/// так что `None` при корректных параметрах недостижим).
///
/// # Пример
/// ```rust
/// use gfpm::poly::find_irreducible;
///
/// assert_eq!(find_irreducible(3, 2), Some(1));  // x^2 + 1
/// assert_eq!(find_irreducible(5, 3), Some(6));  // x^3 + x + 1
/// assert_eq!(find_irreducible(5, 1), Some(1));  // x + 1 — poly = 0 (модуль x) пропущен
/// assert_eq!(find_irreducible(4, 2), None);     // 4 — не простое
/// ```
#[must_use]
pub fn find_irreducible(p: u32, m: u32) -> Option<u128> {
    if !is_prime(p) || m == 0 || m > MAX_DEG as u32 {
        return None;
    }
    let pm = order_checked(p, m)?;
    let mut f = [0u32; MAX_DEG + 1];
    let mut n: u128 = 1;
    while n < pm {
        if n % p as u128 != 0 {
            // свободный член нулевой ⟹ x делит многочлен ⟹ приводим
            unpack_poly(n, p, m, &mut f[..m as usize]);
            f[m as usize] = 1;
            if is_irreducible(&f[..=m as usize], p) {
                return Some(n);
            }
        }
        n += 1;
    }
    None
}

/// Возведение класса `x^k` (передаётся как многочлен степени < m)
/// в степень `p` по модулю `f`: один шаг Фробениуса.
fn pow_x_p(base: &[u32], f: &[u32], p: u32, out: &mut [u32], acc: &mut [u32]) {
    let m = f.len() - 1;
    debug_assert!(base.len() == m && out.len() == m);
    let mut r = [0u32; MAX_DEG + 1];
    r[0] = 1;
    let mut b = [0u32; MAX_DEG + 1];
    b[..m].copy_from_slice(base);
    let mut e: u64 = p as u64;
    while e > 0 {
        if e & 1 == 1 {
            mul_reduce(&r[..m], &b[..m], &f[..m], p, acc);
            r[..m].copy_from_slice(&acc[..m]);
        }
        if e > 1 {
            mul_reduce(&b[..m], &b[..m], &f[..m], p, acc);
            b[..m].copy_from_slice(&acc[..m]);
        }
        e >>= 1;
    }
    out[..m].copy_from_slice(&r[..m]);
}

/// Простые делители числа `n <= 64` (возвращает до 3 значений).
fn prime_divisors(n: u64) -> [u64; 3] {
    let mut res = [0u64; 3];
    let mut count = 0;
    let mut v = n;
    let mut d: u64 = 2;
    while d * d <= v {
        if v % d == 0 {
            res[count] = d;
            count += 1;
            while v % d == 0 {
                v /= d;
            }
        }
        d += 1;
    }
    if v > 1 {
        res[count] = v;
    }
    res
}

/// Форматирование коэффициентов (по возрастанию) как многочлена:
/// `2x^2 + x + 1`. Нулевой многочлен печатается как `0`.
pub(crate) fn fmt_coeffs(f: &mut fmt::Formatter<'_>, coeffs: &[u32]) -> fmt::Result {
    let mut wrote = false;
    for (i, &c) in coeffs.iter().enumerate().rev() {
        if c == 0 {
            continue;
        }
        if wrote {
            f.write_str(" + ")?;
        }
        wrote = true;
        match i {
            0 => write!(f, "{c}")?,
            1 if c == 1 => f.write_str("x")?,
            1 => write!(f, "{c}x")?,
            _ if c == 1 => write!(f, "x^{i}")?,
            _ => write!(f, "{c}x^{i}")?,
        }
    }
    if !wrote {
        f.write_str("0")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        find_irreducible, is_irreducible, mul_reduce, pack_poly, poly_add, poly_degree,
        poly_divmod, poly_eval, poly_gcd, poly_mul, poly_sub, reduce_in_place, unpack_poly,
    };

    #[test]
    fn degree_and_zeros() {
        assert_eq!(poly_degree(&[]), usize::MAX);
        assert_eq!(poly_degree(&[0]), usize::MAX);
        assert_eq!(poly_degree(&[0, 0, 3]), 2);
    }

    #[test]
    fn add_sub_round_trip() {
        let mut s = [0u32; 4];
        let mut d = [0u32; 4];
        poly_add(&[1, 0, 2], &[2, 1], 5, &mut s);
        assert_eq!(s, [3, 1, 2, 0]);
        poly_sub(&s, &[2, 1, 0, 0], 5, &mut d);
        assert_eq!(d, [1, 0, 2, 0]);
    }

    #[test]
    fn mul_matches_schoolbook() {
        const P: u32 = 7;
        for a in 0..P {
            for b in 0..P {
                for c in 0..P {
                    for d in 0..P {
                        // (ax + b)(cx + d) над GF(7)
                        let mut out = [0u32; 3];
                        poly_mul(&[b, a], &[d, c], P, &mut out);
                        assert_eq!(out, [(b * d) % P, (a * d + b * c) % P, (a * c) % P]);
                    }
                }
            }
        }
    }

    #[test]
    fn divmod_invariant() {
        const P: u32 = 5;
        for blen in 1..4usize {
            for b2 in 0..P {
                for b1 in 0..P {
                    for b0 in 0..P {
                        let b = [b0, b1, b2];
                        if poly_degree(&b[..blen]) == usize::MAX {
                            continue;
                        }
                        for a in [
                            [0u32; 3],
                            [1, 0, 0],
                            [2, 3, 0],
                            [1, 1, 1],
                            [0, 0, 1],
                            [4, 2, 3],
                        ] {
                            let mut q = [0u32; 3];
                            let mut r = [0u32; 3];
                            poly_divmod(&a, &b[..blen], P, &mut q, &mut r);
                            // инвариант: r = a − q·b
                            let mut recon = [0u32; 3];
                            let mut qb = [0u32; 3];
                            poly_mul(&q, &b[..blen], P, &mut qb);
                            poly_sub(&a, &qb, P, &mut recon);
                            assert_eq!(recon, r, "a = {a:?}, b = {:?}", &b[..blen]);
                            assert!(
                                poly_degree(&r) == usize::MAX
                                    || poly_degree(&r) < poly_degree(&b[..blen])
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn divmod_by_zero_is_defensive() {
        let mut q = [0u32; 2];
        let mut r = [0u32; 2];
        poly_divmod(&[1, 2], &[0, 0], 5, &mut q, &mut r);
        assert_eq!(q, [0, 0]);
        assert_eq!(r, [1, 2]);
    }

    #[test]
    fn eval_horner() {
        // (x + 1)(x + 2) = x^2 + 2 над GF(3): корни 1 и 2
        assert_eq!(poly_eval(&[2, 0, 1], 1, 3), 0);
        assert_eq!(poly_eval(&[2, 0, 1], 2, 3), 0);
        assert_eq!(poly_eval(&[2, 0, 1], 0, 3), 2);
        assert_eq!(poly_eval(&[], 5, 7), 0);
    }

    #[test]
    fn gcd_euclid() {
        // НОД(x^2 + 1, x + 1) над GF(2): x^2 + 1 = (x + 1)^2
        let mut g = [0u32; 3];
        poly_gcd(&[1, 0, 1], &[1, 1], 2, &mut g);
        assert_eq!(g, [1, 1, 0]);
        // НОД(x^2 + 1, x^2 + x) над GF(3): оба делятся на x + 1?
        // x^2 + 1 неприводим над GF(3) ⟹ НОД = 1
        let mut g2 = [0u32; 3];
        poly_gcd(&[1, 0, 1], &[0, 1, 1], 3, &mut g2);
        assert_eq!(g2, [1, 0, 0]);
        // НОД(0, f) = f (мономиальный)
        let mut g3 = [0u32; 3];
        poly_gcd(&[0, 0, 0], &[2, 0, 1], 3, &mut g3);
        assert_eq!(g3, [2, 0, 1]);
    }

    #[test]
    fn mul_reduce_matches_divmod() {
        // произведение с редукцией == произведение, затем остаток от деления
        const P: u32 = 5;
        const M: usize = 2;
        let q = [2u32, 1]; // модуль x^2 + x + 2 над GF(5)
        for a0 in 0..P {
            for a1 in 0..P {
                for b0 in 0..P {
                    for b1 in 0..P {
                        let mut acc = [0u32; 2 * M - 1];
                        mul_reduce(&[a0, a1], &[b0, b1], &q, P, &mut acc);
                        let mut prod = [0u32; 2 * M - 1];
                        poly_mul(&[a0, a1], &[b0, b1], P, &mut prod);
                        let mut qq = [0u32; 3];
                        let mut r = [0u32; 3];
                        let f = [q[0], q[1], 1];
                        poly_divmod(&prod, &f, P, &mut qq, &mut r);
                        assert_eq!(acc[0], r[0], "({a0},{a1})·({b0},{b1})");
                        assert_eq!(acc[1], r[1]);
                    }
                }
            }
        }
    }

    #[test]
    fn reduce_in_place_high_degree() {
        // x^4 по модулю x^2 + 1 над GF(3): x^2 ≡ −1 ⟹ x^4 ≡ 1
        let mut a = [0u32, 0, 0, 0, 1];
        reduce_in_place(&mut a, &[1, 0], 3);
        assert_eq!(a, [1, 0, 0, 0, 0]);
        // буфер короче модуля — без изменений
        let mut short = [1u32, 2];
        reduce_in_place(&mut short, &[1, 0, 1], 3);
        assert_eq!(short, [1, 2]);
    }

    #[test]
    fn pack_unpack_round_trip() {
        for &p in &[2u32, 3, 5, 251] {
            let mut buf = [0u32; 5];
            for v in [0u128, 1, 2, p as u128, p as u128 + 1, 100] {
                unpack_poly(v, p, 5, &mut buf);
                if v < (p as u128).pow(5) {
                    assert_eq!(pack_poly(&buf, p), Some(v));
                }
            }
        }
        assert_eq!(pack_poly(&[3, 0, 0], 3), None); // коэффициент вне GF(3)
        assert_eq!(pack_poly(&[1, 2, 3, 4, 5, 6, 7], 2), None); // коэффициенты вне GF(2)
                                                                // 81 цифра «2» в базисе 3 не помещается в u128 (3^81 > 2^128)
        assert_eq!(pack_poly(&[2u32; 81], 3), None);
    }

    #[test]
    fn rabin_matches_brute_force() {
        // Независимая проверка: перебор делителей для малых полей
        // (без аллокаций — фиксированные буферы)
        fn brute(f: &[u32], p: u32) -> bool {
            let m = f.len() - 1;
            if m <= 1 {
                return m == 1;
            }
            let mut divisor = [0u32; 33];
            for d in 1..=m / 2 {
                let count = (p as u64).pow(d as u32);
                for n in 0..count {
                    for (i, slot) in divisor[..d].iter_mut().enumerate() {
                        *slot = ((n / (p as u64).pow(i as u32)) % p as u64) as u32;
                    }
                    divisor[d] = 1;
                    let mut q = [0u32; 10];
                    let mut r = [0u32; 10];
                    poly_divmod(f, &divisor[..d + 1], p, &mut q, &mut r);
                    if poly_degree(&r) == usize::MAX {
                        return false; // найден делитель — приводим
                    }
                }
            }
            true
        }

        for &(p, m) in &[
            (2u32, 4usize),
            (2, 5),
            (3, 2),
            (3, 3),
            (3, 4),
            (5, 2),
            (7, 2),
        ] {
            for n in 0..(p as u64).pow(m as u32) {
                let mut f = [0u32; 65];
                for (i, slot) in f[..m].iter_mut().enumerate() {
                    *slot = ((n / (p as u64).pow(i as u32)) % p as u64) as u32;
                }
                f[m] = 1;
                assert_eq!(
                    is_irreducible(&f[..=m], p),
                    brute(&f[..=m], p),
                    "расхождение Рабина и перебора: p = {p}, m = {m}, f = {:?}",
                    &f[..=m]
                );
            }
        }
    }

    #[test]
    fn rabin_structural_rejections() {
        assert!(!is_irreducible(&[], 3));
        assert!(!is_irreducible(&[1], 3)); // степень 0
        assert!(!is_irreducible(&[1, 2], 3)); // не мономиальный
        assert!(!is_irreducible(&[1, 0, 1], 4)); // p не простое
        assert!(is_irreducible(&[0, 1], 3)); // линейный x
    }

    #[test]
    fn find_matches_rabin() {
        for &(p, m) in &[
            (2u32, 1u32),
            (2, 4),
            (2, 8),
            (2, 16),
            (2, 64),
            (3, 2),
            (3, 3),
            (3, 4),
            (5, 2),
            (5, 3),
            (7, 2),
            (11, 2),
            (13, 2),
            (251, 2),
        ] {
            let found = find_irreducible(p, m).expect("неприводимый существует");
            let mut f = [0u32; 65];
            unpack_poly(found, p, m, &mut f[..m as usize]);
            f[m as usize] = 1;
            assert!(
                is_irreducible(&f[..=m as usize], p),
                "find({p}, {m}) вернул приводимый: {found}"
            );
        }
        assert_eq!(find_irreducible(1, 2), None);
        assert_eq!(find_irreducible(3, 0), None);
        assert_eq!(find_irreducible(3, 65), None);
        assert_eq!(find_irreducible(3, 41), None); // 3^41 > 2^64
        assert_eq!(find_irreducible(2, 65), None);
        // m = 1: poly = 0 (модуль x) пропускается фильтром —
        // первый кандидат всегда x + 1
        for p in [2u32, 3, 5, 251, 65537] {
            assert_eq!(find_irreducible(p, 1), Some(1), "find({p}, 1)");
        }
    }

    #[test]
    fn fmt_coeffs_polynomial_form() {
        // Мини-райтер без аллокаций: fmt_coeffs пишет через fmt::Write
        struct Buf([u8; 128], usize);
        impl core::fmt::Write for Buf {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                if self.1 + bytes.len() > self.0.len() {
                    return Err(core::fmt::Error);
                }
                self.0[self.1..self.1 + bytes.len()].copy_from_slice(bytes);
                self.1 += bytes.len();
                Ok(())
            }
        }
        impl Buf {
            fn as_str(&self) -> &str {
                core::str::from_utf8(&self.0[..self.1]).unwrap_or("")
            }
        }
        fn check(coeffs: &[u32], expected: &str) {
            let mut b = Buf([0u8; 128], 0);
            struct W<'a>(&'a [u32]);
            impl core::fmt::Display for W<'_> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    super::fmt_coeffs(f, self.0)
                }
            }
            use core::fmt::Write as _;
            let _ = write!(b, "{}", W(coeffs));
            assert_eq!(b.as_str(), expected, "форматирование {coeffs:?}");
        }
        check(&[0], "0");
        check(&[0, 0, 0], "0");
        check(&[5], "5");
        check(&[2, 1], "x + 2");
        check(&[1, 2], "2x + 1");
        check(&[3, 0, 1], "x^2 + 3");
        check(&[0, 1, 0, 1], "x^3 + x");
        check(&[1, 1, 1], "x^2 + x + 1");
        check(&[0, 2], "2x");
    }
}
