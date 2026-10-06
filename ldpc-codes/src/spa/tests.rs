use super::*;
use crate::{Bit, DecodeInput, DecoderConfig, ParityCheckMatrix};

#[test]
fn manual_degree_two_example_updates_r_posterior_word_and_next_q() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[2.0, -1.0, 3.0],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");

    assert_eq!(state.q, [2.0, -1.0, -1.0, 3.0]);
    state.step(config);

    assert_eq!(state.r, [-1.0, 2.0, 3.0, -1.0]);
    assert_eq!(state.posterior, [1.0, 4.0, 2.0]);
    assert_eq!(state.word, [Bit::Zero, Bit::Zero, Bit::Zero]);
    assert_eq!(state.q, [2.0, 2.0, 1.0, 3.0]);
}

#[test]
fn a_second_internal_step_uses_the_previous_step_q_messages() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[2.0, -1.0, 3.0],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");

    state.step(config);
    state.step(config);

    assert_eq!(state.r, [2.0, 2.0, 3.0, 1.0]);
    assert_eq!(state.posterior, [4.0, 4.0, 4.0]);
}

#[test]
fn degree_three_ln_three_input_produces_ln_five_posterior() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let llr = 3.0_f64.ln();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[llr, llr, llr],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");

    state.step(config);

    for &message in &state.r {
        assert_close((5.0_f64 / 3.0).ln(), message);
    }
    for &posterior in &state.posterior {
        assert_close(5.0_f64.ln(), posterior);
    }
    assert_eq!(state.q, [llr; 3]);
}

#[test]
fn degree_one_bit_keeps_a_tiny_channel_value_in_next_q() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![0]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[1e-300],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");

    state.step(config);

    assert_eq!(state.r, [20.0]);
    assert_eq!(state.posterior, [20.0]);
    assert_eq!(state.q, [1e-300]);
}

#[test]
fn posterior_is_clipped_after_the_full_sum_and_q_uses_the_unclipped_total() {
    let checks =
        ParityCheckMatrix::try_from_rows(1, vec![vec![0], vec![0]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[15.0],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");
    state.r.copy_from_slice(&[15.0, 15.0]);

    state.update_bit_messages(config);

    assert_eq!(state.posterior, [20.0]);
    assert_eq!(state.q, [20.0, 20.0]);
}

#[test]
fn bit_sum_is_not_clipped_between_positive_and_negative_messages() {
    let checks =
        ParityCheckMatrix::try_from_rows(1, vec![vec![0], vec![0]]).expect("matrix is valid");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[15.0],
                erasures: &[],
            },
            config,
        )
        .expect("input is valid");
    state.r.copy_from_slice(&[15.0, -15.0]);

    state.update_bit_messages(config);

    assert_eq!(state.posterior, [15.0]);
    assert_eq!(state.q, [0.0, 20.0]);
}

fn assert_close(expected: f64, actual: f64) {
    assert!(actual.is_finite(), "actual value is not finite: {actual}");
    let tolerance = 1e-10 + 1e-10 * expected.abs();
    assert!(
        (expected - actual).abs() <= tolerance,
        "expected {expected}, got {actual}, tolerance {tolerance}"
    );
}
