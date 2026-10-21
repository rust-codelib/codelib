use core::convert::Infallible;

use coding_core::{Candidate, Codeword, DecodeOutcome, Decoder, Encoder, Observer};

use crate::{DecodeResult, Kerdock, KerdockError};

/// The distance-bound check actually performed on a decoding candidate.
///
/// `distance <= radius` holds for every returned candidate. This does not prove
/// identity with the transmitted original or determine the channel error count.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KerdockChecks {
    /// Hamming distance from the received block to the returned codeword.
    pub distance: usize,
    /// The configured unique correction radius.
    pub radius: usize,
}

/// Why a complete-block decoding attempt finished without a candidate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KerdockStopReason {
    /// No codeword is within the correction radius; outside-radius ties also stop.
    OutsideCorrectionRadius {
        /// The actual distance to a globally nearest codeword.
        nearest_distance: usize,
        /// The configured unique correction radius.
        radius: usize,
    },
}

/// Encodes one complete message using the unchanged inherent bit mapping.
///
/// Valid messages have 8, 12, 16, or 20 transmitted bits (1, 2, 2, or 3 bytes),
/// so they always fit [`coding_core::MAX_MESSAGE_BYTES`], independently of Rust
/// `bool` storage. Other lengths return [`KerdockError::MessageLengthMismatch`]
/// before allocating output. No events are emitted or per-call state retained;
/// inputs and previous owned results remain unchanged, including on error.
/// Use a fully-qualified [`Encoder::encode`] call to avoid the inherent method
/// with the same name; see the crate-level example.
impl Encoder<&[bool]> for Kerdock {
    type Output = Codeword<Vec<bool>>;
    type Error = KerdockError;
    type Event = ();

    fn encode(
        &mut self,
        input: &[bool],
        _: &mut dyn Observer<Self::Event>,
    ) -> Result<Self::Output, Self::Error> {
        Kerdock::encode(self, input).map(Codeword)
    }
}

/// Independently decodes one complete hard-decision block.
///
/// A candidate owns the inherent [`DecodeResult`] plus its executed distance
/// check. Only [`KerdockError::NoCodewordWithinRadius`] becomes `Stopped`;
/// malformed input and all other errors stay `Err`. `NeedMore` is uninhabited:
/// short blocks are errors, not streaming input. Recovered messages have at most
/// 20 transmitted bits, satisfying [`coding_core::MAX_MESSAGE_BYTES`].
/// No events are emitted, inputs are unchanged, and no per-call state is retained
/// on success, stopping, or error. Previous owned results remain unchanged.
/// Use a fully-qualified [`Decoder::decode`] call to avoid the inherent method
/// with the same name; see the crate-level example.
impl Decoder<&[bool]> for Kerdock {
    type Value = DecodeResult;
    type Checks = KerdockChecks;
    type NeedMore = Infallible;
    type StopReason = KerdockStopReason;
    type Error = KerdockError;
    type Event = ();

    fn decode(
        &mut self,
        input: &[bool],
        _: &mut dyn Observer<Self::Event>,
    ) -> Result<
        DecodeOutcome<Self::Value, Self::Checks, Self::NeedMore, Self::StopReason>,
        Self::Error,
    > {
        match Kerdock::decode(self, input) {
            Ok(value) => {
                let checks = KerdockChecks {
                    distance: value.distance(),
                    radius: self.correction_radius(),
                };
                Ok(DecodeOutcome::Candidate(Candidate { value, checks }))
            }
            Err(KerdockError::NoCodewordWithinRadius {
                nearest_distance,
                radius,
            }) => Ok(DecodeOutcome::Stopped(
                KerdockStopReason::OutsideCorrectionRadius {
                    nearest_distance,
                    radius,
                },
            )),
            Err(error) => Err(error),
        }
    }
}
