#![no_std]
#![forbid(unsafe_code)]
//! Библиотечный пример кодека с переданными буферами для сборки на embedded-цели.
//! cargo build -p coding-core --example buffers --target thumbv7em-none-eabihf
#[path = "../tests/support/buffers.rs"]
mod buffers;
pub use buffers::*;
