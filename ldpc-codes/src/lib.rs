#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bit;
mod configurator;
mod conversion;
mod decode_input;
mod decoder;
mod decoder_config;
mod diagnostics;
mod encoder;
mod error;
mod parity_check;
mod size;
mod spa;

pub use bit::Bit;
pub use configurator::{ConfiguredLdpc, LdpcConfigurator};
pub use conversion::{bits_to_vector, vector_to_bits, Gf2};
pub use decode_input::DecodeInput;
pub use decoder::{DecodeEvent, DecodeObserver, DecodeResult, DecodeStatus, Decoder};
pub use decoder_config::DecoderConfig;
pub use diagnostics::count_bit_errors;
pub use encoder::{Encoder, SystematicEncoder};
pub use error::LdpcError;
pub use parity_check::ParityCheckMatrix;
pub use spa::{spa_step, SpaDecoder, SpaStepResult};
