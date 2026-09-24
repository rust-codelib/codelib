//! Алгоритм Берлекампа–Мэсси.

use gf2m::Gf;

use super::RsError;

/// Решает ключевое уравнение по синдромам: строит локатор ошибок
/// `Λ(x)`.
///
/// `locator_out` — буфер коэффициентов Λ по возрастанию степеней
/// (`locator_out[0] = 1`); требуется
/// `locator_out.len() == syndromes.len()`. Возвращает фактическую
/// степень Λ (число ошибок).
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `syndromes` пуст или
/// `locator_out.len() != syndromes.len()`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn berlekamp_massey<const N: usize, const POLY: u32>(
    syndromes: &[Gf<N, POLY>],
    locator_out: &mut [Gf<N, POLY>],
) -> Result<usize, RsError> {
    todo!()
}
