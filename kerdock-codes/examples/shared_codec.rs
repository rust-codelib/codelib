use coding_core::{DecodeOutcome, Decoder, Encoder, NoopObserver};
use kerdock_codes::{Kerdock, KerdockError, KerdockSize, KerdockStopReason};

fn main() -> Result<(), KerdockError> {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    let message = [true, false, false, false, false, false, false, false];
    // Fully-qualified calls avoid the colliding inherent encode/decode methods.
    let word = <Kerdock as Encoder<&[bool]>>::encode(&mut codec, &message, &mut NoopObserver)?;
    let midpoint: Vec<_> = (0..16).map(|j| 0x02c0u16 & (1 << j) != 0).collect();
    for input in [word.0.as_slice(), midpoint.as_slice(), &[]] {
        match <Kerdock as Decoder<&[bool]>>::decode(&mut codec, input, &mut NoopObserver) {
            Ok(DecodeOutcome::Candidate(candidate)) => {
                assert_eq!(candidate.value.message(), message);
                assert_eq!(candidate.checks.distance, 0);
                assert_eq!(candidate.checks.radius, 2);
                println!("Candidate: {:?}", candidate.checks);
            }
            Ok(DecodeOutcome::Stopped(reason)) => {
                assert_eq!(
                    reason,
                    KerdockStopReason::OutsideCorrectionRadius {
                        nearest_distance: 3,
                        radius: 2,
                    }
                );
                println!("Stopped without a candidate: {reason:?}");
            }
            Ok(DecodeOutcome::NeedMore(never)) => match never {},
            Err(error) => {
                assert_eq!(
                    error,
                    KerdockError::WordLengthMismatch {
                        expected: 16,
                        actual: 0
                    }
                );
                println!("Malformed input: {error:?}");
            }
        }
    }
    Ok(())
}
