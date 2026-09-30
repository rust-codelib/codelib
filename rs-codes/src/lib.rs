#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Коды Рида–Соломона над конечными полями GF(2^m).
//!
//! Статус: скелет, v0.1.0. Алгоритмические функции помечены `todo!()`:
//! контракты (сигнатуры, ошибки, соглашения о срезах) заморожены,
//! тела будут реализованы в следующих релизах. Приёмочные тесты
//! конвейера живут в `tests/reed_solomon.rs` под `#[ignore]`.
//!
//! Поле — из крейта `gf2m`: элемент [`Gf`](gf2m::Gf) с параметрами
//! `N = 2^m` и образующим многочленом `POLY`.
//!
//! Конвейер кодирования и декодирования:
//!
//! ```text
//! generator_poly → encode_parity
//! syndromes → (berlekamp_massey | euclid) → error_positions →
//! error_magnitudes → correct
//! ```
//!
//! Инварианты слоя: `no_std`, без аллокаций — все результаты пишутся
//! в срезы вызывающего; ошибки — через [`RsError`]; арифметика не
//! паникует (кроме `todo!()`-заглушек); MSRV 1.81.
//!
//! # Пример (заработает после реализации)
//! ```text
//! use gf2m::Gf256;
//! use rs_codes::{correct, encode_parity, generator_poly, RsConfig};
//!
//! let cfg = RsConfig::new(4, 0).unwrap(); // t = 2 ошибки
//! let mut gen = [Gf256::new(0); 5];
//! generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();
//! let msg = [Gf256::new(1); 251];
//! let mut parity = [Gf256::new(0); 4];
//! encode_parity(&msg, &gen, &mut parity).unwrap();
//! // … внести ошибки и вызвать correct …
//! ```

use core::fmt;

mod berlekamp_massey;
mod chien;
mod decode;
mod encode;
mod euclid;
mod forney;
mod generator;
mod syndrome;

pub use berlekamp_massey::berlekamp_massey;
pub use chien::error_positions;
pub use decode::correct;
pub use encode::encode_parity;
pub use euclid::euclid;
pub use forney::error_magnitudes;
pub use generator::generator_poly;
pub use syndrome::syndromes;

/// Конфигурация кода Рида–Соломона.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RsConfig {
    /// Число проверочных символов (`n − k`). Корректирующая
    /// способность: `t = nsym / 2` ошибок.
    pub nsym: usize,
    /// Показатель первой корневой степени: корни порождающего
    /// многочлена — `α^(first_root + i)`, `i = 0..nsym`.
    pub first_root: usize,
    /// Алгоритм решения ключевого уравнения.
    pub solver: KeyEquationSolver,
}

impl RsConfig {
    /// Создаёт конфигурацию с решателем по умолчанию
    /// (Берлекамп–Мэсси).
    ///
    /// # Ошибки
    /// [`RsError::InvalidParameters`] — если `nsym == 0`.
    pub fn new(nsym: usize, first_root: usize) -> Result<Self, RsError> {
        if nsym == 0 {
            return Err(RsError::InvalidParameters);
        }
        Ok(Self {
            nsym,
            first_root,
            solver: KeyEquationSolver::BerlekampMassey,
        })
    }

    /// Задаёт решатель ключевого уравнения.
    #[must_use]
    pub fn with_solver(mut self, solver: KeyEquationSolver) -> Self {
        self.solver = solver;
        self
    }

    /// Корректирующая способность: `nsym / 2` ошибок.
    #[must_use]
    pub const fn t(&self) -> usize {
        self.nsym / 2
    }
}

/// Алгоритм решения ключевого уравнения `Λ·Ω ≡ S (mod x^nsym)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyEquationSolver {
    /// Итеративный алгоритм Берлекампа–Мэсси (по умолчанию).
    #[default]
    BerlekampMassey,
    /// Расширенный алгоритм Евклида.
    Euclid,
}

/// Ошибка кода Рида–Соломона.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RsError {
    /// Несовместные параметры: длины срезов, `nsym`, `first_root`
    /// или `n`.
    InvalidParameters,
    /// Число ошибок превышает корректирующую способность
    /// `t = nsym / 2`.
    TooManyErrors,
}

impl fmt::Display for RsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameters => {
                write!(f, "недопустимые параметры кода Рида–Соломона")
            }
            Self::TooManyErrors => {
                write!(f, "число ошибок превышает корректирующую способность кода")
            }
        }
    }
}

impl core::error::Error for RsError {}
