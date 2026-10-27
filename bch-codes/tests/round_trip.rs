//! Кодирование и исправление полных BCH-кодов.

#[path = "support/bch15_generator.rs"]
mod bch15;
#[path = "support/bch7_generator.rs"]
mod bch7;
#[path = "support/correct_bch7.rs"]
mod correction;

use bch15::bch15_generator;
use bch7::bch7_generator;
use bch_codes::{
    correct, encode_systematic, extract_message, BchConfig, BchCorrection, BchDecodeWorkspace,
};
use coding_core::{Codeword, Message};
use correction::correct_bch7;
use gf2m::Gf16;

#[test]
fn known_systematic_word_bch_7_4_3() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    let mut encoded = [0u8; 1];
    // m(x) = 1, x^3 m(x) mod g(x) = x + 1.
    // GF(8): x^3 + x + 1.
    encode_systematic::<8, 0b1_011>(&config, &generator, &[0x01], &mut encoded).unwrap();
    assert_eq!(encoded, [0b0000_1011]);
}

#[test]
fn valid_word_is_returned_without_correction() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    let mut word = [0u8; 1];
    // GF(8): x^3 + x + 1.
    encode_systematic::<8, 0b1_011>(&config, &generator, &[0b0110], &mut word).unwrap();
    let before = word;
    let BchCorrection::Candidate(checks) = correct_bch7(&config, &generator, &mut word) else {
        panic!("valid codeword must be a candidate");
    };
    assert!(checks.initial_syndromes_zero);
    assert!(checks.final_syndromes_zero);
    assert_eq!(checks.corrected_bits, 0);
    assert_eq!(word, before);
}

#[test]
fn bch_7_4_3_exhaustive_single_error_round_trip() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    for message in 0u8..16 {
        let mut encoded = [0u8; 1];
        // GF(8): x^3 + x + 1.
        encode_systematic::<8, 0b1_011>(&config, &generator, &[message], &mut encoded).unwrap();
        assert_eq!(encoded[0] >> 3, message, "message = {message:#x}");
        assert_eq!(encoded[0] & 0x80, 0, "message = {message:#x}");
        for error_position in 0..7 {
            let mut received = encoded;
            received[0] ^= 1 << error_position;
            let correction = correct_bch7(&config, &generator, &mut received);
            assert!(matches!(correction, BchCorrection::Candidate(_)));
            assert_eq!(
                received, encoded,
                "message={message:#x}, position={error_position}"
            );
            let mut decoded = [0u8; 1];
            let Message(bits) =
                extract_message::<8>(&config, &generator, Codeword(&received), &mut decoded)
                    .unwrap();
            assert_eq!(bits, &[message]);
        }
    }
}

#[test]
fn chien_search_uses_inverse_root_to_position_mapping() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    let mut encoded = [0u8; 1];
    // GF(8): x^3 + x + 1.
    encode_systematic::<8, 0b1_011>(&config, &generator, &[0b1010], &mut encoded).unwrap();
    for position in 0..7 {
        let mut received = encoded;
        received[0] ^= 1 << position;
        assert!(matches!(
            correct_bch7(&config, &generator, &mut received),
            BchCorrection::Candidate(_)
        ));
        assert_eq!(
            received, encoded,
            "incorrect mapping for position {position}"
        );
    }
}

#[test]
fn bch_15_7_5_corrects_representative_double_errors() {
    let config = BchConfig::new(2, 0).unwrap();
    let generator = bch15_generator(&config);
    for message in [0x00, 0x01, 0x35, 0x55, 0x7f] {
        let mut encoded = [0u8; 2];
        // GF(16): x^4 + x + 1.
        encode_systematic::<16, 0b1_0011>(&config, &generator, &[message], &mut encoded).unwrap();
        for (left, right) in [(0, 1), (0, 14), (3, 11), (7, 8), (12, 14)] {
            let mut received = encoded;
            received[left / 8] ^= 1 << (left % 8);
            received[right / 8] ^= 1 << (right % 8);
            let mut field = [Gf16::zero(); 64];
            let mut positions = [0usize; 4];
            let mut workspace = BchDecodeWorkspace {
                field: &mut field,
                positions: &mut positions,
            };
            assert!(matches!(
                // GF(16): x^4 + x + 1.
                correct::<16, 0b1_0011>(&config, &generator, &mut received, &mut workspace)
                    .unwrap(),
                BchCorrection::Candidate(_)
            ));
            assert_eq!(
                received, encoded,
                "message={message:#x}, errors=({left},{right})"
            );
        }
    }
}
