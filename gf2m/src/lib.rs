#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! gf2m — конечные поля GF(2^m), m = 2..=16.
//!
//! Четыре слоя:
//! - [`Gf<N, POLY>`](Gf) — константно-генериковое ядро: тип поля
//!   задаётся параметрами, операции — `const fn` (умножение —
//!   [`clmul`] с полиномиальной редукцией);
//! - [`tables`] — компилируемые log/exp-таблицы для GF(256) и
//!   GF(65536) (умножение за O(1));
//! - полиномиальный слой — [`clmul`], [`poly_mod`]: многочлены над GF(2)
//!   в машинном слове;
//! - [`GfRuntime`] — рантайм-слой: степень поля выбирается во время
//!   выполнения, обращение — алгоритм Ито–Цудзии.
//!
//! Коды Рида–Соломона поверх поля живут в отдельном крейте
//! `rs-codes` этого же воркспейса.
//!
//! Инварианты крейта: `no_std` (только `core`),
//! `#![forbid(unsafe_code)]`, без аллокаций, MSRV 1.81, арифметика
//! не паникует. Соглашения нуля: `inv(0) = 0`, `x / 0 = 0`, `0^0 = 1`.
//!
//! # Пример
//! ```rust
//! use gf2m::{Gf256, GfRuntime};
//!
//! // Константно-генериковый слой: поле Рида–Соломона 0x11D (QR)
//! let a = Gf256::new(0x80);
//! assert_eq!(a * Gf256::new(0x02), Gf256::new(0x1D)); // редукция t^8
//! assert_eq!(a.inv() * a, Gf256::new(1));
//!
//! // Рантайм-слой: степень поля — параметр выполнения
//! let b = GfRuntime::new(8, 0x57).unwrap();
//! let c = GfRuntime::new(8, 0x02).unwrap();
//! assert_eq!(b * c, GfRuntime::new(8, 0xAE).unwrap());
//! ```

mod error;
mod gf;
mod poly;
mod runtime;
pub mod tables;

pub use error::GfError;
#[cfg(not(target_pointer_width = "16"))]
pub use gf::Gf65536;
pub use gf::{
    Gf, Gf1024, Gf128, Gf16, Gf16384, Gf2048, Gf256, Gf32, Gf32768, Gf4, Gf4096, Gf512, Gf64, Gf8,
    Gf8192,
};
pub use poly::{clmul, poly_add, poly_degree, poly_mod};
pub use runtime::{GfRuntime, POLYS};
