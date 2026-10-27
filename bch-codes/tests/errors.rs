//! Ошибки размеров, представления и рабочей памяти.

#[path = "support/bch7_generator.rs"]
mod bch7;

use bch7::bch7_generator;
use bch_codes::{build_generator, correct, encode_systematic, BchConfig, BchDecodeWorkspace};
use gf2m::Gf8;

#[test]
fn rejects_wrong_buffer_lengths_and_nonzero_padding() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    // GF(8): x^3 + x + 1.
    assert!(encode_systematic::<8, 0b1_011>(&config, &generator, &[], &mut [0u8; 1]).is_err());
    assert!(encode_systematic::<8, 0b1_011>(&config, &generator, &[0u8; 1], &mut []).is_err());
    assert!(encode_systematic::<8, 0b1_011>(&config, &generator, &[0x10], &mut [0u8; 1]).is_err());

    let mut padded_word = [0x80];
    let before = padded_word;
    let mut field = [Gf8::zero(); 32];
    let mut positions = [0usize; 4];
    let mut workspace = BchDecodeWorkspace {
        field: &mut field,
        positions: &mut positions,
    };
    // GF(8): x^3 + x + 1.
    assert!(correct::<8, 0b1_011>(&config, &generator, &mut padded_word, &mut workspace).is_err());
    assert_eq!(padded_word, before);

    let mut short_generator = [];
    let mut construction_workspace = [Gf8::zero(); 32];
    assert!(
        // GF(8): x^3 + x + 1.
        build_generator::<8, 0b1_011>(&config, &mut short_generator, &mut construction_workspace)
            .is_err()
    );
}

#[test]
fn rejects_insufficient_decode_workspace_without_modifying_input() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    let mut received = [0x01];
    let before = received;
    let mut no_field = [];
    let mut no_positions = [];
    let mut workspace = BchDecodeWorkspace {
        field: &mut no_field,
        positions: &mut no_positions,
    };
    // GF(8): x^3 + x + 1.
    assert!(correct::<8, 0b1_011>(&config, &generator, &mut received, &mut workspace).is_err());
    assert_eq!(received, before);
}
