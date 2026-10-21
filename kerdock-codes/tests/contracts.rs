mod common;

use coding_core::{Codeword, DecodeOutcome, Decoder, Encoder, NoopObserver, Observer};
use core::convert::Infallible;
use kerdock_codes::{
    DecodeResult, Kerdock, KerdockChecks, KerdockError, KerdockSize, KerdockStopReason,
};

type Outcome = DecodeOutcome<DecodeResult, KerdockChecks, Infallible, KerdockStopReason>;

fn encode<'a, C>(
    codec: &mut C,
    input: &'a [bool],
    observer: &mut dyn Observer<()>,
) -> Result<Codeword<Vec<bool>>, KerdockError>
where
    C: Encoder<&'a [bool], Output = Codeword<Vec<bool>>, Error = KerdockError, Event = ()>,
{
    codec.encode(input, observer)
}

fn decode<'a, C>(
    codec: &mut C,
    input: &'a [bool],
    observer: &mut dyn Observer<()>,
) -> Result<Outcome, KerdockError>
where
    C: Decoder<
        &'a [bool],
        Value = DecodeResult,
        Checks = KerdockChecks,
        NeedMore = Infallible,
        StopReason = KerdockStopReason,
        Error = KerdockError,
        Event = (),
    >,
{
    codec.decode(input, observer)
}

fn bits(mask: u16) -> Vec<bool> {
    (0..16).map(|j| mask & (1 << j) != 0).collect()
}

#[derive(Default)]
struct CountingObserver(usize);

impl Observer<()> for CountingObserver {
    fn on_event(&mut self, _: &()) {
        self.0 += 1;
    }
}

// Catch forwarding to the wrong bit mapping or returning an unwrapped message.
#[test]
fn generic_encoder_matches_literal_quadratic_word() {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    let message = [true, false, false, false, false, false, false, false];
    let saved = message;
    assert_eq!(
        encode(&mut codec, &message, &mut NoopObserver),
        Ok(Codeword(bits(0x6ac0)))
    );
    assert_eq!(message, saved);
    assert_eq!(codec.encode(&message), Ok(bits(0x6ac0)));
}

// Catch altered encoding, a strict radius boundary, or fabricated check distances.
#[test]
fn generic_candidates_match_independent_words_through_radius_at_all_sizes() {
    for (m, radius) in [(4, 2), (6, 13), (8, 59), (10, 247)] {
        let mut codec = Kerdock::new(KerdockSize::try_from(1 << m).unwrap());
        let message = common::message(m, (1 << (m - 2)) | 1, (1 << (m - 1)) | 3, true);
        let expected_word = common::reference_word(m, &message);
        assert_eq!(
            encode(&mut codec, &message, &mut NoopObserver),
            Ok(Codeword(expected_word.clone()))
        );
        for errors in [0, 1, radius] {
            let mut received = expected_word.clone();
            for i in 0..errors {
                let position = (received.len() - 1 + 37 * i) % received.len();
                received[position] ^= true;
            }
            let saved = received.clone();
            let outcome = decode(&mut codec, &received, &mut NoopObserver).unwrap();
            let candidate = match outcome {
                DecodeOutcome::Candidate(candidate) => candidate,
                other => panic!("expected candidate, got {other:?}"),
            };
            assert_eq!(candidate.value.message(), message);
            assert_eq!(candidate.value.word(), expected_word);
            assert_eq!(candidate.value.distance(), errors);
            assert_eq!(
                candidate.checks,
                KerdockChecks {
                    distance: errors,
                    radius
                }
            );
            assert_eq!(candidate.checks.distance, candidate.value.distance());
            assert_eq!(received, saved);
        }
    }
}

// Catch mapping ordinary rejection to Err or accepting an out-of-radius tie.
#[test]
fn midpoint_stops_shared_decoder_but_remains_an_inherent_error() {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    let midpoint = bits(0x02c0);
    assert_eq!(
        decode(&mut codec, &midpoint, &mut NoopObserver),
        Ok(DecodeOutcome::Stopped(
            KerdockStopReason::OutsideCorrectionRadius {
                nearest_distance: 3,
                radius: 2,
            }
        ))
    );
    assert_eq!(
        codec.decode(&midpoint),
        Err(KerdockError::NoCodewordWithinRadius {
            nearest_distance: 3,
            radius: 2
        })
    );
    assert_eq!(midpoint, bits(0x02c0));
}

// Catch swallowed input errors, wrong lengths, or partial output on oversized input.
#[test]
fn malformed_blocks_preserve_exact_errors_and_inputs() {
    for (size, k, n) in [
        (KerdockSize::Bits16, 8, 16),
        (KerdockSize::Bits64, 12, 64),
        (KerdockSize::Bits256, 16, 256),
        (KerdockSize::Bits1024, 20, 1024),
    ] {
        let mut codec = Kerdock::new(size);
        for actual in [0, k - 1, k + 1, 32769] {
            let input = vec![true; actual];
            let saved = input.clone();
            assert_eq!(
                encode(&mut codec, &input, &mut NoopObserver),
                Err(KerdockError::MessageLengthMismatch {
                    expected: k,
                    actual
                })
            );
            assert_eq!(input, saved);
        }
        for actual in [0, n - 1, n + 1] {
            let input = vec![true; actual];
            let saved = input.clone();
            assert_eq!(
                decode(&mut codec, &input, &mut NoopObserver),
                Err(KerdockError::WordLengthMismatch {
                    expected: n,
                    actual
                })
            );
            assert_eq!(input, saved);
        }
    }
}

// Catch confusing bool storage bytes with transmitted bits or admitting other lengths.
#[test]
fn fixed_message_capacities_fit_transmitted_byte_limit() {
    for (size, k, bytes) in [
        (KerdockSize::Bits16, 8, 1),
        (KerdockSize::Bits64, 12, 2),
        (KerdockSize::Bits256, 16, 2),
        (KerdockSize::Bits1024, 20, 3),
    ] {
        let mut codec = Kerdock::new(size);
        assert_eq!(codec.message_len(), k);
        assert_eq!(
            coding_core::MessageBytes::from_bits(k).unwrap().get(),
            bytes
        );
        let word = encode(&mut codec, &vec![false; k], &mut NoopObserver).unwrap();
        match decode(&mut codec, &word.0, &mut NoopObserver).unwrap() {
            DecodeOutcome::Candidate(candidate) => {
                assert_eq!(candidate.value.message(), vec![false; k])
            }
            other => panic!("expected candidate, got {other:?}"),
        }
    }
}

// Catch emitted events, observer-dependent results, or retained state across calls.
#[test]
fn observers_are_silent_and_calls_preserve_owned_results_after_stop_and_error() {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    let mut observer = CountingObserver::default();
    let message = common::message(4, 1, 0, false);
    let word = bits(0x6ac0);
    let midpoint = bits(0x02c0);
    let malformed = vec![true; 7];
    let zero_word = vec![false; 16];
    let before = decode(&mut codec, &word, &mut observer).unwrap();
    let saved = before.clone();
    let encoded = encode(&mut codec, &message, &mut observer).unwrap();
    let saved_encoded = encoded.clone();
    for input in [&word, &midpoint, &malformed, &zero_word] {
        assert_eq!(
            decode(&mut codec, input, &mut observer),
            decode(&mut codec, input, &mut NoopObserver)
        );
    }
    for input in [&message, &malformed, &message] {
        assert_eq!(
            encode(&mut codec, input, &mut observer),
            encode(&mut codec, input, &mut NoopObserver)
        );
    }
    assert_eq!(decode(&mut codec, &word, &mut observer), Ok(saved.clone()));
    match decode(&mut codec, &zero_word, &mut observer).unwrap() {
        DecodeOutcome::Candidate(candidate) => {
            assert_eq!(candidate.value.message(), [false; 8]);
            assert_eq!(candidate.value.word(), zero_word);
            assert_eq!(candidate.value.distance(), 0);
            assert_eq!(
                candidate.checks,
                KerdockChecks {
                    distance: 0,
                    radius: 2
                }
            );
        }
        other => panic!("expected candidate, got {other:?}"),
    }
    assert_eq!(before, saved);
    assert_eq!(encoded, saved_encoded);
    assert_eq!(encoded, Codeword(word.clone()));
    assert_eq!(word, bits(0x6ac0));
    assert_eq!(midpoint, bits(0x02c0));
    assert_eq!(malformed, vec![true; 7]);
    assert_eq!(zero_word, vec![false; 16]);
    assert_eq!(message, common::message(4, 1, 0, false));
    assert_eq!(observer.0, 0);
}

// Catch checks that claim knowledge of the transmitted original rather than distance.
#[test]
fn another_exact_codeword_reports_zero_distance_not_channel_error_count() {
    let mut codec = Kerdock::new(KerdockSize::Bits16);
    // Six flips of transmitted zero give a different exact word, outside t = 2.
    let received = bits(0x6ac0);
    assert_eq!(received.iter().filter(|&&bit| bit).count(), 6);
    match decode(&mut codec, &received, &mut NoopObserver).unwrap() {
        DecodeOutcome::Candidate(candidate) => {
            assert_eq!(candidate.value.message(), common::message(4, 1, 0, false));
            assert_eq!(candidate.value.word(), received);
            assert_eq!(candidate.value.distance(), 0);
            assert_eq!(
                candidate.checks,
                KerdockChecks {
                    distance: 0,
                    radius: 2
                }
            );
        }
        other => panic!("expected candidate, got {other:?}"),
    }
}
