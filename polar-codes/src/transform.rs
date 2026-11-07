//! Полярное преобразование `G_N = F^{\otimes m}` над GF(2).

use crate::length::check_length;
use crate::{Bit, PolarError};

/// Применяет полярное преобразование на месте.
///
/// Ядро `F = [[1, 0], [1, 1]]`. Для вектора-строки `u` результат равен `u F^{\otimes m}`
/// в естественном порядке, без матрицы разворота битов `B_N`. Преобразование
/// совпадает со своим обратным: повторный вызов возвращает исходный вектор.
///
/// # Errors
///
/// Возвращает [`PolarError::InvalidLength`], если длина не является степенью
/// двойки из диапазона `1..=4096`.
pub fn polar_transform(bits: &mut [Bit]) -> Result<(), PolarError> {
    check_length(bits.len())?;
    transform_in_place(bits);
    Ok(())
}

/// Разворачивает младшие `log2(length)` битов индекса.
///
/// Если лекция задаёт порождающую матрицу как `B_N F^{\otimes m}`, индексы
/// информационных битов отличаются от естественного порядка этим разворотом.
///
/// # Errors
///
/// Возвращает [`PolarError::InvalidLength`], если длина недопустима, или
/// [`PolarError::IndexOutOfBounds`], если индекс не меньше длины.
pub fn bit_reverse(index: usize, length: usize) -> Result<usize, PolarError> {
    check_length(length)?;
    if index >= length {
        return Err(PolarError::IndexOutOfBounds { index, length });
    }

    let width = length.trailing_zeros();
    if width == 0 {
        return Ok(0);
    }
    Ok(index.reverse_bits() >> (usize::BITS - width))
}

pub(crate) fn transform_in_place(bits: &mut [Bit]) {
    let length = bits.len();
    if length <= 1 {
        return;
    }

    let half = length / 2;
    for index in 0..half {
        bits[index] = bits[index].xor(bits[index + half]);
    }
    transform_in_place(&mut bits[..half]);
    transform_in_place(&mut bits[half..]);
}
