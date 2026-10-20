#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod arithmetic;
mod coefficients;
mod decoder;
mod encoder;
mod error;
mod generator;
mod repair;
mod resources;
mod types;
mod wire;

pub use coefficients::*;
pub use decoder::*;
pub use encoder::*;
pub use error::*;
pub use generator::{canonical_support, logarithmic_draws};
pub use repair::*;
pub use resources::*;
pub use types::*;
pub use wire::{parse_descriptor, parse_symbol};
