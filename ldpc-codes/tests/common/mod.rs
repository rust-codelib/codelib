use coding_core::{Candidate, DecodeOutcome};
use ldpc_codes::{DecodeChecks, DecodeResult, LdpcDecodeOutcome};

pub fn decoded(outcome: LdpcDecodeOutcome) -> DecodeResult {
    let candidate: Candidate<DecodeResult, DecodeChecks> = match outcome {
        DecodeOutcome::Candidate(candidate) => candidate,
        DecodeOutcome::NeedMore(reason) => match reason {},
        DecodeOutcome::Stopped(reason) => match reason {},
    };
    assert_eq!(candidate.checks, candidate.value.checks());
    candidate.value
}
