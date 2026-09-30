//! Расширенный алгоритм Евклида — альтернативный решатель
//! ключевого уравнения.

use gf2m::Gf;

use super::RsError;

/// Решает ключевое уравнение расширенным алгоритмом Евклида над
/// парой `(S(x), x^nsym)`: возвращает локатор `Λ(x)` и вычислитель
/// ошибок `Ω(x)`.
///
/// `locator_out` и `evaluator_out` — буферы коэффициентов по
/// возрастанию степеней; требуется `locator_out.len() ==
/// syndromes.len()` и `evaluator_out.len() == syndromes.len()`.
/// Возвращает степень Λ.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `syndromes` пуст или длины
/// буферов не совпадают с `syndromes.len()`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn euclid<const N: usize, const POLY: u32>(
    syndromes: &[Gf<N, POLY>],
    locator_out: &mut [Gf<N, POLY>],
    evaluator_out: &mut [Gf<N, POLY>],
) -> Result<usize, RsError> {
    todo!()
}
