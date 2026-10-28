#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Двоичные примитивные узкосмысловые BCH-коды.
//!
//! Поле задаётся типом [`Gf`] с `N = 2^m`. Полное кодовое слово имеет
//! длину `N - 1`; укорочение удаляет старшие информационные позиции.
//! Биты упакованы в байты младшим битом вперёд: позиция `j` соответствует
//! коэффициенту при `x^j`.
//!
//! Крейt не выделяет память. Порождающий многочлен, выходные и рабочие
//! буферы предоставляет вызывающий код. Успех ограниченного декодера
//! означает, что найдено кодовое слово на расстоянии не более `t` от
//! принятого. При фактическом числе ошибок больше `t` другое кодовое слово
//! может быть принято за исходное — это ограничение теории BCH.

#[cfg(feature = "alloc")]
extern crate alloc;

mod berlekamp_massey;
mod bits;
mod chien;
mod codec;
mod config;
mod decode;
mod encode;
mod error;
mod generator;
#[cfg(feature = "alloc")]
mod owned;
mod syndrome;

pub use codec::{BchCodec, BchDecodeInput, BchEncodeInput};
pub use config::{
    codeword_bits, construction_workspace_len, decode_workspace_len, generator_bytes,
    generator_degree, message_bits, BchConfig,
};
pub use decode::{correct, BchDecodeWorkspace};
pub use encode::{encode_systematic, extract_message};
pub use error::{BchChecks, BchCorrection, BchError, BchEvent, BchStopReason};
pub use generator::build_generator;
pub use gf2m::Gf;
#[cfg(feature = "alloc")]
pub use owned::{OwnedBchCodec, OwnedBchDecodeInput, OwnedBchDecodeOutcome};
