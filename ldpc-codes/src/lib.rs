#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bit;
mod configurator;
mod decode;
mod encoder;
mod error;
mod parity_check;
mod size;
mod spa;

pub use bit::{bits_to_vector, count_bit_errors, vector_to_bits, Bit, Gf2};
pub use configurator::{ConfiguredLdpc, LdpcConfigurator};
pub use decode::{
    DecodeChecks, DecodeEvent, DecodeInput, DecodeResult, DecodeStatus, DecoderConfig,
    LdpcDecodeOutcome,
};
pub use encoder::SystematicEncoder;
pub use error::LdpcError;
pub use parity_check::ParityCheckMatrix;
pub use spa::{spa_step, SpaDecoder, SpaStepResult};

#[cfg(test)]
#[path = "../tests/unit/support.rs"]
mod test_support;
