mod common;

use common::{message, reference_word};
use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

// Catch lost high message bits, reversed coefficient signs, incorrect distance,
// and rejecting the inclusive correction-radius boundary at any supported size.
#[test]
fn decoding_recovers_message_and_word_through_radius() {
    for (m, radius) in [(4, 2), (6, 13), (8, 59), (10, 247)] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        for constant in [false, true] {
            let input = message(m, (1 << (m - 2)) | 1, (1 << (m - 1)) | 3, constant);
            let expected_word = reference_word(m, &input);
            let encoded = code.encode(&input).unwrap();
            assert_eq!(encoded, expected_word);
            for errors in [0, 1, radius] {
                let mut received = encoded.clone();
                for i in 0..errors {
                    // Odd stride permutes a power-of-two block; includes its last bit.
                    let position = (received.len() - 1 + 37 * i) % received.len();
                    received[position] ^= true;
                }
                let original_received = received.clone();
                let result = code.decode(&received).unwrap();
                assert_eq!(result.message(), input, "m={m}, errors={errors}");
                assert_eq!(result.word(), expected_word, "m={m}, errors={errors}");
                assert_eq!(result.distance(), errors);
                assert_eq!(received, original_received);
            }
        }
    }
}

// Catch extraction silently correcting a word instead of requiring distance zero.
#[test]
fn extraction_accepts_only_exact_codewords() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        let input = message(m, 3, 5, true);
        let mut word = reference_word(m, &input);
        assert_eq!(code.extract_message(&word), Ok(input));
        let clean = word.clone();
        word[0] ^= true;
        assert_eq!(
            code.extract_message(&word),
            Err(KerdockError::InvalidCodeword)
        );
        word[0] ^= true;
        assert_eq!(word, clean);
    }
}

// Catch omitted length validation and incorrect reported lengths in either API.
#[test]
fn decoding_and_extraction_reject_empty_short_and_long_words() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        let expected = 1 << m;
        for actual in [0, expected - 1, expected + 1] {
            let word = vec![false; actual];
            let error = KerdockError::WordLengthMismatch { expected, actual };
            assert_eq!(code.decode(&word), Err(error.clone()));
            assert_eq!(code.extract_message(&word), Err(error));
        }
    }
}

// Catch arbitrary tie acceptance, a fabricated rejection distance, or stale state.
#[test]
fn midpoint_rejection_preserves_previous_results_and_later_decoding() {
    let code = Kerdock::new(KerdockSize::Bits16);
    let fixture: Vec<_> = (0..16).map(|j| 0x6ac0u16 & (1 << j) != 0).collect();
    let midpoint: Vec<_> = (0..16).map(|j| 0x02c0u16 & (1 << j) != 0).collect();
    let original_fixture = fixture.clone();
    let original_midpoint = midpoint.clone();
    let before = code.decode(&fixture).unwrap();
    let saved = before.clone();
    assert_eq!(before.message(), message(4, 1, 0, false));
    assert_eq!(before.word(), fixture);
    assert_eq!(before.distance(), 0);
    // Midpoint is distance 3 from zero and Q_1 (distance 6 apart).
    // Any word closer than 3 would violate minimum distance 6 from zero.
    assert_eq!(
        code.decode(&midpoint),
        Err(KerdockError::NoCodewordWithinRadius {
            nearest_distance: 3,
            radius: 2
        })
    );
    assert_eq!(
        code.extract_message(&midpoint),
        Err(KerdockError::InvalidCodeword)
    );
    let after = code.decode(&[false; 16]).unwrap();
    assert_eq!(after.message(), [false; 8]);
    assert_eq!(after.word(), [false; 16]);
    assert_eq!(after.distance(), 0);
    assert_eq!(before, saved);
    assert_eq!(fixture, original_fixture);
    assert_eq!(midpoint, original_midpoint);
}

// Catch an API that mistakes distance to the returned word for channel errors.
#[test]
fn another_exact_codeword_succeeds_beyond_transmitted_words_radius() {
    let code = Kerdock::new(KerdockSize::Bits16);
    // Transmitted zero, but six flips produce Q_1: a different exact codeword.
    let received: Vec<_> = (0..16).map(|j| 0x6ac0u16 & (1 << j) != 0).collect();
    assert_eq!(received.iter().filter(|&&bit| bit).count(), 6);
    let result = code.decode(&received).unwrap();
    assert_eq!(result.message(), message(4, 1, 0, false));
    assert_eq!(result.word(), received);
    assert_eq!(result.distance(), 0);
}
