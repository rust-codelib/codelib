#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod bytes;
mod code;
mod configurator;
mod decode;
mod error;
mod field;
mod observer;
#[cfg(test)]
mod paper_tests;
mod params;
mod repair;
mod structure;
mod subsets;

pub use bytes::{
    bits_per_symbol, pack_bytes, unpack_bytes, EncodedFile, FileDecodeResult, MAX_MESSAGE_BYTES,
};
pub use code::{CascadeCode, Encoder, NodeShare};
pub use configurator::CascadeConfigurator;
pub use decode::{DataDecoder, DecodeResult, DecodeStatus, DecoderConfig};
pub use error::CascadeError;
pub use field::CodeField;
pub use observer::{CascadeEvent, CascadeObserver, EventLog};
pub use params::{CascadeParams, MAX_ALPHA, MAX_NODES};
pub use repair::{NodeRepairer, RepairPayload};
