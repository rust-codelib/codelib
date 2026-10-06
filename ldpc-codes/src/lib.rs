#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bit;
mod error;

pub use bit::Bit;
pub use error::LdpcError;
