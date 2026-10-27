//! Интеграция с унифицированными ролями `coding-core`.

#[path = "support/bch7_generator.rs"]
mod bch7;

use bch7::bch7_generator;
use bch_codes::{BchCodec, BchConfig, BchDecodeInput, BchDecodeWorkspace, BchEncodeInput};
use coding_core::{Codeword, DecodeOutcome, Decoder, Encoder, NoopObserver};
use gf2m::Gf8;

#[test]
fn coding_core_encoder_and_decoder_correct_in_place() {
    let config = BchConfig::new(1, 0).unwrap();
    let generator = bch7_generator(&config);
    // GF(8): x^3 + x + 1.
    let mut codec = BchCodec::<8, 0b1_011>::new(config, &generator).unwrap();
    let mut encoded = [0u8; 1];
    let Codeword(word) = codec
        .encode(
            BchEncodeInput {
                message: &[0b1010],
                output: &mut encoded,
            },
            &mut NoopObserver,
        )
        .unwrap();
    let expected = [word[0]];
    word[0] ^= 1 << 5;
    let mut field = [Gf8::zero(); 32];
    let mut positions = [0usize; 4];
    let outcome = codec
        .decode(
            BchDecodeInput {
                received: word,
                workspace: BchDecodeWorkspace {
                    field: &mut field,
                    positions: &mut positions,
                },
            },
            &mut NoopObserver,
        )
        .unwrap();
    match outcome {
        DecodeOutcome::Candidate(candidate) => assert_eq!(candidate.value.0, &expected),
        DecodeOutcome::Stopped(reason) => panic!("unexpected stop: {reason:?}"),
        DecodeOutcome::NeedMore(never) => match never {},
    }
}
