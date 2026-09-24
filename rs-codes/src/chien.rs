//! Поиск позиций ошибок (алгоритм Ченя).

use gf2m::Gf;

use super::RsError;

/// Ищет корни локатора ошибок Λ — позиции ошибок кодового слова.
///
/// `locator` — коэффициенты Λ(x) по возрастанию степеней
/// (`locator[0] = 1`), `locator_deg` — степень Λ из решения ключевого
/// уравнения. Проверяются позиции `0..n` (позиция `i` — индекс в
/// срезе принятого слова, то есть коэффициент при `x^i`; локатор
/// имеет корень `α^(N−1−i)`). Найденные позиции записываются в
/// `positions_out` по возрастанию — как элементы поля со значением,
/// равным индексу позиции. Возвращает число найденных позиций.
///
/// # Ошибки
/// [`RsError::InvalidParameters`] — если `locator_deg >= locator.len()`,
/// `n == 0`, `n > N − 1` или `positions_out.len() < locator_deg`.
///
/// # Паника
/// Пока не реализовано: тело функции — `todo!()`.
#[allow(unused_variables)]
pub fn error_positions<const N: usize, const POLY: u32>(
    locator: &[Gf<N, POLY>],
    locator_deg: usize,
    n: usize,
    positions_out: &mut [Gf<N, POLY>],
) -> Result<usize, RsError> {
    todo!()
}
