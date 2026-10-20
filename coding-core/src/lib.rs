#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod observation;
mod outcome;
mod repair;
mod roles;
mod size;

pub use observation::{LlrConvention, NoopObserver, Observer};
pub use outcome::{Candidate, Codeword, DecodeOutcome, Message, TransformInput};
pub use repair::{RepairExecutor, RepairHelper, RepairPlan, RepairPlanner};
pub use roles::{Decoder, Encoder, IncrementalDecoder, SymbolEncoder};
pub use size::{MessageBytes, Size, SizeError, SizeUnit, MAX_MESSAGE_BYTES};
