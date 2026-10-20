//! Binary Kerdock codes of length `n = 2^m` for even `m` from 4 to 10.
//!
//! Each block carries `2m` message bits; `false = 0`, `true = 1`.
//! The code is nonlinear, and encoding is not guaranteed to be systematic.
//! The minimum distance is `(n - 2^(m/2)) / 2`.
//!
//! # Bit ordering and construction
//!
//! A message contains `m-1` bits for field element `a`, then `m` bits for mask `b`
//! (both least-significant bit first), followed by constant `c`.
//! Coordinate `j` in `0..n` represents `(x,u)`: the low `m-1` bits encode `x`
//! in the polynomial basis of `GF(2^(m-1))`, and the high bit is `u`.
//! The field moduli from [`gf2m::GfRuntime::standard_poly`] are `0xB`, `0x25`,
//! `0x83`, and `0x211` for degrees 3, 5, 7, and 9, respectively.
//!
//! Using the absolute trace `Tr`, define
//! `B_a((x,u),(y,v)) = Tr(a²xy) + Tr(ax)Tr(ay) + uTr(ay) + vTr(ax)`.
//! Addition here is XOR. In the specified basis, choose
//! `Q_a(w) = Σ_{i<j} B_a(e_i,e_j) w_i w_j` with no linear part.
//! The codeword is `Q_a(j) XOR parity(b & j) XOR c`.
//!
//! The forms `B_a` are alternating. For `h = a+b != 0`, a vector `(x,u)`
//! in the radical of `B_a+B_b` satisfies `Tr(hx)=0`,
//! `Tr(ax)=Tr(bx)=r`, and `h²x+hr+uh=0`. Thus `hx=r+u` is a binary scalar.
//! The odd field degree gives `Tr(hx)=hx`, so `x=u=0`.
//! Differences of forms are nonsingular; the quadratic Walsh spectrum gives
//! distances `(n ± sqrt(n))/2` between distinct quadratic cosets.
//! Within a coset, nonzero distances are `n/2` and `n`.
//! See [Kerdock (1972)](https://doi.org/10.1016/S0019-9958(72)90376-2)
//! and [Error Correction Zoo](https://errorcorrectionzoo.org/c/kerdock).

#![forbid(unsafe_code)]

mod decode;
mod encode;
mod error;
mod quadratic;
mod walsh;

pub use decode::DecodeResult;
pub use error::KerdockError;
use quadratic::QuadraticTable;

/// An immutable, prepared binary Kerdock construction.
///
/// Supports block lengths 16, 64, 256, and 1024 with one general construction.
/// The upper bound is an implementation resource limit; at `m=10`, the packed
/// quadratic table contains 64 KiB of payload.
pub struct Kerdock {
    pub(crate) m: u32,
    pub(crate) quadratics: QuadraticTable,
}

impl Kerdock {
    /// Prepares a length-`2^m` code carrying `2m` message bits.
    ///
    /// # Errors
    /// Returns [`KerdockError::UnsupportedExponent`] unless `m` is even and
    /// within `4..=10`. Validation precedes all size calculations.
    pub fn try_new(m: u32) -> Result<Self, KerdockError> {
        if !(4..=10).contains(&m) || m % 2 != 0 {
            return Err(KerdockError::UnsupportedExponent { m });
        }
        Ok(Self {
            m,
            quadratics: QuadraticTable::new(m),
        })
    }

    /// The binary block-length exponent `m`.
    #[must_use]
    pub fn exponent(&self) -> u32 {
        self.m
    }

    /// The number of bits in one message (`2m`).
    #[must_use]
    pub fn message_len(&self) -> usize {
        2 * self.m as usize
    }

    /// The number of bits in one codeword (`2^m`).
    #[must_use]
    pub fn codeword_len(&self) -> usize {
        1 << self.m
    }

    /// The minimum Hamming distance between distinct codewords.
    #[must_use]
    pub fn minimum_distance(&self) -> usize {
        (self.codeword_len() - (1 << (self.m / 2))) / 2
    }

    /// The unique error-correction radius: `floor((d-1)/2)`.
    #[must_use]
    pub fn correction_radius(&self) -> usize {
        (self.minimum_distance() - 1) / 2
    }
}
