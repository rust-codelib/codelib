//! Автоматический выбор параметров владеющего кодека.

use bch_codes::{BchError, OwnedBchCodec};

#[test]
fn selects_smallest_field_and_shortens_it_to_the_message() {
    let one_bit = OwnedBchCodec::new(1, 1).unwrap();
    assert_eq!(one_bit.message_bits(), 1);
    assert_eq!(one_bit.codeword_bits(), 3);

    let four_bits = OwnedBchCodec::new(4, 1).unwrap();
    assert_eq!(four_bits.message_bits(), 4);
    assert_eq!(four_bits.codeword_bits(), 7);
    assert_eq!(four_bits.correctable_errors(), 1);
}

#[test]
fn skips_fields_that_are_too_small_for_correction_capability() {
    let codec = OwnedBchCodec::new(7, 2).unwrap();
    assert_eq!(codec.message_bits(), 7);
    assert_eq!(codec.codeword_bits(), 15);
}

#[test]
fn rejects_zero_and_unsupported_parameters() {
    assert!(matches!(
        OwnedBchCodec::new(0, 1),
        Err(BchError::InvalidParameters)
    ));
    assert!(matches!(
        OwnedBchCodec::new(1, 0),
        Err(BchError::InvalidParameters)
    ));
    assert!(matches!(
        OwnedBchCodec::new(65_536, 1),
        Err(BchError::InvalidParameters)
    ));
}
