#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod arithmetic;
mod error;
mod field;
mod field_text;
mod message;
mod polynomial;
mod stubs;
mod text;

pub use error::PolynomialError;
pub use field::FieldElement;
pub use field_text::FieldText;
pub use polynomial::Polynomial;

/// Предел массива коэффициентов: 128 КиБ по `len * size_of::<F>()`,
/// без метаданных `Box` и расходов аллокатора.
pub const MAX_POLYNOMIAL_BYTES: usize = 128 * 1024;

/// Максимальная длина исходного или восстановленного сообщения: 4096 байт.
pub const MAX_MESSAGE_BYTES: usize = 4096;
