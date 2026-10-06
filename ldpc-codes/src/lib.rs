#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bit;
mod conversion;
mod error;
mod parity_check;

#[cfg(test)]
mod encoder;

pub use bit::Bit;
pub use conversion::{bits_to_vector, vector_to_bits, Gf2};
pub use error::LdpcError;
pub use parity_check::ParityCheckMatrix;
