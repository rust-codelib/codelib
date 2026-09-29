#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod coefficients;
mod error;
mod matrix;
mod text;
mod vector;

pub use coefficients::MAX_POLYNOMIAL_BYTES;
pub use error::LinalgError;
pub use matrix::Matrix;
pub use vector::Vector;

/// Максимальный размер каждой оси матрицы.
///
/// Матрица может иметь не более [`MAX_MATRIX_DIM`] строк и не более
/// [`MAX_MATRIX_DIM`] столбцов. Ограничение применяется отдельно к осям,
/// поэтому матрица `65 × 65` допустима, хотя содержит больше 4096 элементов.
pub const MAX_MATRIX_DIM: usize = 4096;
