//! Поведение укороченных BCH-кодов.

#[path = "support/bch7_generator.rs"]
mod bch7;
#[path = "support/correct_bch7.rs"]
mod correction;

use bch7::bch7_generator;
use bch_codes::{encode_systematic, extract_message, BchConfig, BchCorrection, BchStopReason};
use coding_core::{Codeword, Message};
use correction::correct_bch7;

#[test]
fn shortened_bch_7_has_three_message_bits_and_corrects_transmitted_positions() {
    let config = BchConfig::new(1, 1).unwrap();
    let generator = bch7_generator(&config);
    for message in 0u8..8 {
        let mut encoded = [0u8; 1];
        // GF(8): x^3 + x + 1.
        encode_systematic::<8, 0b1_011>(&config, &generator, &[message], &mut encoded).unwrap();
        assert_eq!(encoded[0] & 0b1100_0000, 0);
        for position in 0..6 {
            let mut received = encoded;
            received[0] ^= 1 << position;
            assert!(matches!(
                correct_bch7(&config, &generator, &mut received),
                BchCorrection::Candidate(_)
            ));
            assert_eq!(received, encoded);
            let mut decoded = [0u8; 1];
            let Message(bits) =
                extract_message::<8>(&config, &generator, Codeword(&received), &mut decoded)
                    .unwrap();
            assert_eq!(bits, &[message]);
        }
    }
}

#[test]
fn shortened_decoder_stops_for_locator_in_removed_position() {
    let config = BchConfig::new(1, 1).unwrap();
    let generator = bch7_generator(&config);
    // Errors at 0 and 2 mimic one error at removed position 6.
    let mut received = [0b0000_0101];
    let before = received;
    assert_eq!(
        correct_bch7(&config, &generator, &mut received),
        BchCorrection::Stopped(BchStopReason::ShortenedPosition { position: 6 })
    );
    assert_eq!(received, before, "Stopped must preserve the received word");
}
