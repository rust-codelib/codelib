#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bit;
mod conversion;
mod decode_input;
mod decoder_config;
mod encoder;
mod error;
mod parity_check;
mod spa;

pub use bit::Bit;
pub use conversion::{bits_to_vector, vector_to_bits, Gf2};
pub use decode_input::DecodeInput;
pub use decoder_config::DecoderConfig;
pub use encoder::{Encoder, SystematicEncoder};
pub use error::LdpcError;
pub use parity_check::ParityCheckMatrix;
pub use spa::{spa_step, SpaStepResult};
