//! Интеграция владеющего кодека с ролями `coding-core`.

use bch_codes::{BchEncodeInput, OwnedBchCodec, OwnedBchDecodeInput};
use coding_core::{Codeword, DecodeOutcome, Decoder, Encoder, NoopObserver};

#[test]
fn unified_decoder_allocates_its_workspace_internally() {
    let mut codec = OwnedBchCodec::new(4, 1).unwrap();
    let mut encoded = [0u8; 1];
    let Codeword(word) = Encoder::encode(
        &mut codec,
        BchEncodeInput {
            message: &[0b1010],
            output: &mut encoded,
        },
        &mut NoopObserver,
    )
    .unwrap();
    let expected = [word[0]];
    word[0] ^= 1 << 5;

    let outcome = Decoder::decode(
        &mut codec,
        OwnedBchDecodeInput { received: word },
        &mut NoopObserver,
    )
    .unwrap();
    let DecodeOutcome::Candidate(candidate) = outcome else {
        panic!("one error must be correctable");
    };
    assert_eq!(candidate.value.0, &expected);
    assert_eq!(candidate.checks.corrected_bits, 1);
}
