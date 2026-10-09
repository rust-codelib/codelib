//! Диагностика различий между двоичными последовательностями.

use crate::bit::bit_distance;
use crate::{Bit, LdpcError};

/// Считает число различающихся позиций в слове и эталоне.
///
/// Функция не зависит от проверочной матрицы и подходит для сравнения как
/// кодовых слов, так и информационных сообщений. Пустые последовательности
/// дают `0`; входные данные остаются неизменными.
///
/// Это сравнение с известным эталоном, которого декодер обычно не знает.
/// [`crate::DecodeStatus::ParitySatisfied`] означает только, что итоговое слово
/// удовлетворяет проверкам `H`, а [`crate::DecodeResult::changed_bits`] считает
/// изменения относительно начального решения декодера. Ни один из этих
/// показателей не сообщает, сколько битов совпадает с переданным словом.
///
/// # Ошибки
///
/// Возвращает [`LdpcError::ReferenceLengthMismatch`], если длины различаются.
/// В таком случае `expected` равен `word.len()`, а `actual` равен
/// `reference.len()`.
pub fn count_bit_errors(word: &[Bit], reference: &[Bit]) -> Result<usize, LdpcError> {
    if word.len() != reference.len() {
        return Err(LdpcError::ReferenceLengthMismatch {
            expected: word.len(),
            actual: reference.len(),
        });
    }

    Ok(bit_distance(word, reference))
}
