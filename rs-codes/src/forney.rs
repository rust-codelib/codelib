//! Алгоритм Форни: величины ошибок.

use gf2m::Gf;

use super::RsError;

/// Вычисляет величины ошибок по формуле Форни.
///
/// Для позиции `i` с локатором `X = α^i`:
/// `e_i = X^(1−first_root) · Ω(X^(−1)) / Λ'(X^(−1))`.
///
/// `evaluator` — Ω(x), `locator` — Λ(x) по возрастанию степеней,
/// `locator_deg` — степень Λ, `positions` — результат
/// [`error_positions`](super::error_positions). Результат:
/// `magnitudes_out[i]` — величина ошибки в `positions[i]`;
/// `magnitudes_out.len() == positions.len()`.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — несовместные длины срезов или
/// `locator_deg >= locator.len()`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn error_magnitudes<const N: usize, const POLY: u32>(
    evaluator: &[Gf<N, POLY>],
    locator: &[Gf<N, POLY>],
    locator_deg: usize,
    positions: &[Gf<N, POLY>],
    first_root: usize,
    magnitudes_out: &mut [Gf<N, POLY>],
) -> Result<(), RsError> {
    todo!()
}
