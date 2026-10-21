//! Binary Kerdock codes of length `n = 2^m` for even `m` from 4 to 10.
//!
//! Each block carries `2m` message bits; `false = 0`, `true = 1`.
//! The code is nonlinear, and encoding is not guaranteed to be systematic.
//! The minimum distance is `(n - 2^(m/2)) / 2`.
//!
//! # Example: one block with 13 errors
//!
//! ```
//! use kerdock_codes::{Kerdock, KerdockError, KerdockSize};
//!
//! # fn main() -> Result<(), KerdockError> {
//! let codec = Kerdock::new(KerdockSize::Bits64);
//! // a = 5 (5 bits), b = 19 (6 bits), c = 1; least-significant bit first.
//! let message = [
//!     true, false, true, false, false, true, true, false, false, true, false, true,
//! ];
//! let word = codec.encode(&message)?;
//! assert_eq!(word.len(), 64);
//! assert_eq!(codec.extract_message(&word)?, message);
//!
//! let positions = [0, 3, 7, 11, 16, 20, 25, 31, 38, 44, 50, 57, 63];
//! assert_eq!(positions.len(), codec.correction_radius());
//! let mut received = word.clone();
//! for position in positions {
//!     received[position] = !received[position];
//! }
//!
//! let decoded = codec.decode(&received)?;
//! assert_eq!(decoded.message(), message);
//! assert_eq!(decoded.word(), word);
//! assert_eq!(decoded.distance(), positions.len());
//! # Ok(())
//! # }
//! ```
//!
//! Input slices are unchanged, and results own their data. Each call handles
//! one block; byte packing, padding, and stream processing are not provided.
//! [`Kerdock::extract_message`] accepts only an exact codeword.
//! [`Kerdock::decode`] guarantees recovery of the original message after at most
//! `t = floor((d-1)/2)` bit flips. Beyond this radius, decoding may successfully
//! return a different message. [`DecodeResult::distance`] measures distance to
//! the returned word, not the error count relative to the unknown transmitted
//! word. On rejection, [`KerdockError::NoCodewordWithinRadius`] contains the
//! actual nearest distance; ties outside the radius are also rejected.
//!
//! Select a codeword bit length with [`KerdockSize`]. For a runtime bit length,
//! use `KerdockSize::try_from(bit_length)?` before calling [`Kerdock::new`];
//! the conversion takes neither an exponent nor a byte count.
//!
//! # Shared block interfaces
//!
//! [`coding_core::Encoder`] and [`coding_core::Decoder`] accept `&[bool]` and
//! `&mut self`, but retain no per-call state and emit no events (`Event = ()`).
//! Inputs and previous owned results remain unchanged on success, stop, or error.
//! Each input must be a complete block: short input is an error, never `NeedMore`.
//! Valid messages carry 8/12/16/20 bits (1/2/2/3 transmitted bytes), always below
//! [`coding_core::MAX_MESSAGE_BYTES`], regardless of Rust object storage.
//!
//! The inherent `encode`/`decode` methods keep their convenient one-argument API.
//! Importing a trait does not disambiguate these colliding method names: use a
//! generic trait bound or fully-qualified calls as below.
//!
//! ```
//! use coding_core::{Codeword, DecodeOutcome, Decoder, Encoder, NoopObserver};
//! use core::convert::Infallible;
//! use kerdock_codes::{
//!     DecodeResult, Kerdock, KerdockChecks, KerdockError, KerdockSize, KerdockStopReason,
//! };
//!
//! # fn main() -> Result<(), KerdockError> {
//! let mut codec = Kerdock::new(KerdockSize::Bits16);
//! let message = [true, false, false, false, false, false, false, false];
//! let word: Codeword<Vec<bool>> =
//!     <Kerdock as Encoder<&[bool]>>::encode(&mut codec, &message, &mut NoopObserver)?;
//! let outcome: DecodeOutcome<DecodeResult, KerdockChecks, Infallible, KerdockStopReason> =
//!     <Kerdock as Decoder<&[bool]>>::decode(&mut codec, &word.0, &mut NoopObserver)?;
//! match outcome {
//!     DecodeOutcome::Candidate(candidate) => {
//!         assert_eq!(candidate.value.message(), message);
//!         assert_eq!(candidate.checks.distance, 0);
//!         assert_eq!(candidate.checks.radius, 2);
//!     }
//!     DecodeOutcome::Stopped(reason) => panic!("no candidate: {reason:?}"),
//!     DecodeOutcome::NeedMore(never) => match never {},
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Shared decoding maps only [`KerdockError::NoCodewordWithinRadius`] to
//! [`coding_core::DecodeOutcome::Stopped`] with [`KerdockStopReason`]; malformed
//! input stays `Err`. The inherent decoder still returns its original error.
//! [`KerdockChecks`] describes distance to the returned word and the configured
//! radius, not proof of identity with the transmitted original.
//!
//! # Resources
//!
//! Requires `std` and Rust 1.81+. Quadratic rows are packed into `u64` words:
//! at `m=10`, the table contains 64 KiB of payload, excluding metadata and buffers.
//! The full codebook is not constructed. Encoding takes `O(n)` work; integer
//! FWHT search takes `O(n² log n)` operations and `O(n)` additional memory.
//! The absolute values of `i32` transform entries are bounded by `n <= 1024`.
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
//! The zero message produces the zero word; changing `c` complements the word.
//! Coordinate and message ordering need not match Magma or GUAVA.
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

mod contracts;
mod decode;
mod encode;
mod error;
mod quadratic;
mod size;
mod walsh;

pub use contracts::{KerdockChecks, KerdockStopReason};
pub use decode::DecodeResult;
pub use error::KerdockError;
use quadratic::QuadraticTable;
pub use size::KerdockSize;

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
    /// Prepares a code with the selected codeword bit length.
    #[must_use]
    pub fn new(size: KerdockSize) -> Self {
        let m = size.exponent();
        Self {
            m,
            quadratics: QuadraticTable::new(m),
        }
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
