//! Допустимая длина полярного блока.

use crate::PolarError;

/// Наибольшая длина блока `N = 2^m`.
///
/// Совпадает с пределом одной оси матрицы в остальной библиотеке.
pub const MAX_LENGTH: usize = 4096;

pub(crate) fn check_length(length: usize) -> Result<(), PolarError> {
    if length == 0 || length > MAX_LENGTH || !length.is_power_of_two() {
        Err(PolarError::InvalidLength { length })
    } else {
        Ok(())
    }
}
