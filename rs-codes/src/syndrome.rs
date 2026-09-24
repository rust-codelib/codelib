//! Синдромы принятого слова.

use gf2m::Gf;

use super::RsError;

/// Вычисляет синдром `S_j = R(α^(first_root + j))`, `j = 0..nsym−1`.
///
/// `received` — принятое слово длины `n <= N − 1` (символ с индексом
/// `i` — коэффициент при `x^i`); `out.len() == nsym`. Нулевой синдром
/// целиком означает отсутствие ошибок.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `nsym == 0`, `received`
/// пуст, `received.len() > N − 1` или `out.len() != nsym`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn syndromes<const N: usize, const POLY: u32>(
    received: &[Gf<N, POLY>],
    nsym: usize,
    first_root: usize,
    out: &mut [Gf<N, POLY>],
) -> Result<(), RsError> {
    todo!()
}
