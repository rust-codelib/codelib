#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

//! Двоичные коды Рида–Маллера `RM(r, m)`.

mod bit;
mod config;
mod configurator;
mod decoder;
mod diagnostics;
mod encoder;
mod error;
mod layout;

pub use bit::Bit;
pub use config::{RmConfig, MAX_CODE_LENGTH, MAX_M};
pub use configurator::{ConfiguredRm, RmConfigurator};
pub use decoder::{
    Decoder, RmDecodeEvent, RmDecodeObserver, RmDecodeResult, RmDecodeStatus, RmDecoder,
};
pub use diagnostics::count_bit_errors;
pub use encoder::{Encoder, RmEncoder};
pub use error::RmError;
