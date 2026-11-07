#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Двоичные полярные коды: преобразование Арикана и выбор информационных позиций.

mod bit;
mod code;
mod construct;
mod decoder;
mod error;
mod length;
mod transform;

pub use bit::Bit;
pub use code::PolarCode;
pub use construct::bhattacharyya_bec;
pub use decoder::{Combine, ScDecision};
pub use error::PolarError;
pub use length::MAX_LENGTH;
pub use transform::{bit_reverse, polar_transform};
