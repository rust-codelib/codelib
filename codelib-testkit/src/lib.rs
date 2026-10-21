#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod benchmark;
mod block;
pub mod measure;
pub mod report;

pub use benchmark::BenchmarkReport;
pub use block::{candidate, BlockLayout, BlockScenario, Channel, CodecOps, Decoded, Trial};
pub use report::ChannelKind;
