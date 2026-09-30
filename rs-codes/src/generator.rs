//! Порождающий многочлен кода РС.

use gf2m::Gf;

use super::RsError;

/// Строит порождающий многочлен
/// `g(x) = Π (x − α^(first_root + i))`, `i = 0..nsym−1`.
///
/// Коэффициенты записываются в `out` по убыванию степеней:
/// `out[0] = 1` (старший, при `x^nsym`), `out[nsym]` — свободный
/// член. Требуется `out.len() == nsym + 1`.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `nsym == 0`, `nsym >= N − 1`,
/// `first_root >= N − 1` или `out.len() != nsym + 1`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn generator_poly<const N: usize, const POLY: u32>(
    nsym: usize,
    first_root: usize,
    out: &mut [Gf<N, POLY>],
) -> Result<(), RsError> {
    todo!()
}
