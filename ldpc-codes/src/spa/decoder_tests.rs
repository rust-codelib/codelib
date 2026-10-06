use super::*;
use crate::{DecodeInput, Decoder};

#[test]
fn invalid_decode_inputs_do_not_change_any_workspace_buffer() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("valid matrix");
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks, config).expect("valid decoder");
    let initial_llrs = [2.0, -3.0, 4.0];
    decoder
        .decode(
            DecodeInput {
                llrs: &initial_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("initial block is valid");
    let previous_state = decoder.state.snapshot();

    let errors = [
        (
            &[][..],
            &[usize::MAX][..],
            LdpcError::LlrLengthMismatch {
                expected: 3,
                actual: 0,
            },
        ),
        (
            &[1.0, f64::NAN, 3.0][..],
            &[usize::MAX][..],
            LdpcError::NonFiniteLlr { index: 1 },
        ),
        (
            &[1.0, 2.0, 3.0][..],
            &[2, 1, 2, 3, 1][..],
            LdpcError::ErasureIndexOutOfBounds { bit: 3, bits: 3 },
        ),
        (
            &[1.0, 2.0, 3.0][..],
            &[2, 1, 2, 1][..],
            LdpcError::DuplicateErasureIndex { bit: 2 },
        ),
    ];

    for (llrs, erasures, expected_error) in errors {
        assert_eq!(
            decoder.decode(DecodeInput { llrs, erasures }, None),
            Err(expected_error)
        );
        assert_eq!(decoder.state.snapshot(), previous_state);
    }
}
