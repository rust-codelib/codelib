use super::*;
use crate::{DecodeEvent, DecodeInput, DecodeObserver, Decoder};

#[derive(Default)]
struct Events(Vec<DecodeEvent>);

impl DecodeObserver for Events {
    fn on_event(&mut self, event: &DecodeEvent) {
        self.0.push(*event);
    }
}

#[test]
fn invalid_decode_inputs_preserve_semantic_state_and_emit_no_events() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("valid matrix");
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks, config).expect("valid decoder");
    let initial_llrs = [2.0, -3.0, 4.0];
    let initial_erasures = [2];
    decoder
        .decode(
            DecodeInput {
                llrs: &initial_llrs,
                erasures: &initial_erasures,
            },
            None,
        )
        .expect("initial block is valid");
    let previous_state = decoder.state.snapshot();
    let previous_mask = decoder.state.erased_positions.clone();
    assert_eq!(previous_mask, [false, false, true]);

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
        let mut events = Events::default();
        assert_eq!(
            decoder.decode(DecodeInput { llrs, erasures }, Some(&mut events),),
            Err(expected_error)
        );
        assert!(events.0.is_empty(), "an invalid block emits no events");
        assert_eq!(decoder.state.snapshot(), previous_state);
    }

    assert_ne!(
        decoder.state.erased_positions, previous_mask,
        "duplicate validation may leave its reusable scratch mask partially filled"
    );
    assert_eq!(decoder.state.erased_positions, [false, true, true]);
}

#[test]
fn channel_and_erasure_mask_allocations_are_reused_between_blocks() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("valid matrix");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    let channel_ptr = state.channel.as_ptr();
    let channel_capacity = state.channel.capacity();
    let mask_ptr = state.erased_positions.as_ptr();
    let mask_capacity = state.erased_positions.capacity();

    state
        .prepare_block(
            DecodeInput {
                llrs: &[1.0, 2.0, 3.0],
                erasures: &[1],
            },
            config,
        )
        .expect("first block is valid");
    state.step(config);
    state
        .prepare_block(
            DecodeInput {
                llrs: &[-4.0, 5.0, 6.0],
                erasures: &[2],
            },
            config,
        )
        .expect("second block is valid");

    assert_eq!(state.channel, [-4.0, 5.0, 0.0]);
    assert_eq!(state.erased_positions, [false, false, true]);
    assert_eq!(state.channel.as_ptr(), channel_ptr);
    assert_eq!(state.channel.capacity(), channel_capacity);
    assert_eq!(state.erased_positions.as_ptr(), mask_ptr);
    assert_eq!(state.erased_positions.capacity(), mask_capacity);
}
