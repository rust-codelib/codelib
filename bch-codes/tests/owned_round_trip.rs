//! Кодирование и декодирование через владеющий API.

use bch_codes::OwnedBchCodec;
use coding_core::{DecodeOutcome, Message};

#[test]
fn round_trip_without_errors_returns_the_message() {
    let codec = OwnedBchCodec::new(4, 1).unwrap();
    let message = [0b1010];
    let encoded = codec.encode(&message).unwrap();

    let outcome = codec.decode(&encoded).unwrap();
    let DecodeOutcome::Candidate(candidate) = outcome else {
        panic!("clean codeword must produce a candidate");
    };
    let Message(decoded) = candidate.value;
    assert_eq!(decoded, message);
    assert_eq!(candidate.checks.corrected_bits, 0);
    assert!(candidate.checks.initial_syndromes_zero);
}

#[test]
fn corrects_the_configured_number_of_errors() {
    let codec = OwnedBchCodec::new(7, 2).unwrap();
    let message = [0b101_0110];
    let mut received = codec.encode(&message).unwrap();
    received[0] ^= 1 << 1;
    received[1] ^= 1 << 4;

    let outcome = codec.decode(&received).unwrap();
    let DecodeOutcome::Candidate(candidate) = outcome else {
        panic!("two errors must be correctable");
    };
    assert_eq!(candidate.value.0, message);
    assert_eq!(candidate.checks.corrected_bits, 2);
    assert!(candidate.checks.final_syndromes_zero);
}

#[test]
fn rejects_wrong_message_and_codeword_lengths() {
    let codec = OwnedBchCodec::new(9, 1).unwrap();
    assert!(codec.encode(&[0]).is_err());
    assert!(codec.decode(&[0]).is_err());
}
