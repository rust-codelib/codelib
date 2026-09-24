//! Полный конвейер декодирования.

use gf2m::Gf;

use super::{RsConfig, RsError};

/// Исправляет ошибки в принятом слове `received` на месте.
///
/// Конвейер: `syndromes` → (`berlekamp_massey` | `euclid` — по
/// `config.solver`) → `error_positions` → `error_magnitudes` →
/// исправление. Возвращает число исправленных символов.
///
/// `workspace` — рабочая область без аллокаций: требуется
/// `workspace.len() >= 3 * config.nsym + 2` (Λ, Ω и промежуточные
/// данные; позиции ошибок хранятся как элементы поля).
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — неверные длины срезов или
/// параметры конфигурации; [`RsError::TooManyErrors`] — ошибок
/// больше, чем `t = nsym / 2`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn correct<const N: usize, const POLY: u32>(
    received: &mut [Gf<N, POLY>],
    config: &RsConfig,
    workspace: &mut [Gf<N, POLY>],
) -> Result<usize, RsError> {
    todo!()
}
