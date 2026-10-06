use super::*;

mod step_tests {
    use super::*;
    use crate::{Bit, DecodeInput, DecoderConfig, ParityCheckMatrix};

    #[test]
    fn manual_degree_two_example_updates_r_posterior_word_and_next_q() {
        let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
            .expect("matrix is valid");
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
        let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
            .expect("matrix is valid");
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
        let checks =
            ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
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
}

mod state_tests {
    use super::*;
    use crate::{Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

    #[test]
    fn graph_numbers_edges_in_row_major_order_and_shares_indices() {
        let checks = ParityCheckMatrix::try_from_rows(
            6,
            vec![vec![4, 1], vec![], vec![3, 1], vec![4, 1], vec![]],
        )
        .expect("matrix is valid");

        let graph = graph::SparseGraph::try_new(&checks).expect("graph sizes are representable");

        assert_eq!(graph.edge_bits, [1, 4, 1, 3, 1, 4]);
        assert_eq!(
            graph.check_edges,
            [vec![0, 1], vec![], vec![2, 3], vec![4, 5], vec![]]
        );
        assert_eq!(
            graph.bit_edges,
            [vec![], vec![0, 2, 4], vec![], vec![3], vec![1, 5], vec![]]
        );
        assert_eq!(graph.check_edges.iter().map(Vec::len).max(), Some(2));
    }

    #[test]
    fn graph_and_state_allow_no_edges_and_isolated_bits() {
        let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![], vec![], vec![]])
            .expect("empty checks are valid");
        let mut state = SpaState::try_new(&checks).expect("empty graph is representable");

        assert_eq!(state.graph.edge_bits, []);
        assert_eq!(state.graph.check_edges, [vec![], vec![], vec![]]);
        assert_eq!(state.graph.bit_edges, [vec![], vec![], vec![], vec![]]);
        assert_eq!(state.tanh, [0.0]);
        assert_eq!(state.prefix, [1.0]);
        assert_eq!(state.suffix, [1.0]);

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, -2.0, 0.0, 3.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("isolated bits can be prepared");

        assert_eq!(state.channel, [1.0, -2.0, 0.0, 3.0]);
        assert_eq!(state.posterior, [1.0, -2.0, 0.0, 3.0]);
        assert!(state.q.is_empty());
        assert!(state.r.is_empty());
        assert_eq!(state.word, [Bit::Zero, Bit::One, Bit::Zero, Bit::Zero]);
    }

    #[test]
    fn initial_hard_decision_treats_both_signed_zeros_as_zero() {
        let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![]]).expect("valid matrix");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[-0.0, 0.0, -0.25, 0.25],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("finite LLRs can be prepared");

        state.step(DecoderConfig::default());

        assert_eq!(state.channel[0].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(state.channel[1].to_bits(), 0.0_f64.to_bits());
        assert_eq!(state.word, [Bit::Zero, Bit::Zero, Bit::One, Bit::Zero]);
    }

    #[test]
    fn preparing_a_new_block_resets_every_workspace_buffer() {
        let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2], vec![1, 2]])
            .expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, 2.0, 3.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("first block is valid");

        state.step(DecoderConfig::default());
        assert_ne!(state.r, [0.0; 5]);
        assert_ne!(state.tanh, [0.0; 4]);
        assert_ne!(state.prefix, [1.0; 4]);
        assert_ne!(state.suffix, [1.0; 4]);

        state
            .prepare_block(
                DecodeInput {
                    llrs: &[-4.0, 0.0, 5.0],
                    erasures: &[1],
                },
                DecoderConfig::default(),
            )
            .expect("second block is valid");

        assert_eq!(state.channel, [-4.0, 0.0, 5.0]);
        assert_eq!(state.posterior, [-4.0, 0.0, 5.0]);
        assert_eq!(state.word, [Bit::One, Bit::Zero, Bit::Zero]);
        assert_eq!(state.q, [-4.0, 0.0, 5.0, 0.0, 5.0]);
        assert_eq!(state.r, [0.0; 5]);
        assert_eq!(state.tanh, [0.0; 4]);
        assert_eq!(state.prefix, [1.0; 4]);
        assert_eq!(state.suffix, [1.0; 4]);
    }

    #[test]
    fn invalid_block_does_not_change_the_prepared_state() {
        let checks =
            ParityCheckMatrix::try_from_rows(2, vec![vec![0, 1]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[1.0, -2.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("initial block is valid");
        let previous = state.snapshot();

        let error = state
            .prepare_block(
                DecodeInput {
                    llrs: &[f64::NAN, 9.0],
                    erasures: &[usize::MAX],
                },
                DecoderConfig::default(),
            )
            .expect_err("non-finite input must fail before state replacement");

        assert_eq!(error, LdpcError::NonFiniteLlr { index: 0 });
        assert_eq!(state.snapshot(), previous);

        let error = state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0, 4.0],
                    erasures: &[1, 1],
                },
                DecoderConfig::default(),
            )
            .expect_err("duplicate erasures must fail before state replacement");

        assert_eq!(error, LdpcError::DuplicateErasureIndex { bit: 1 });
        assert_eq!(state.snapshot(), previous);
    }

    #[test]
    fn scratch_size_helper_checks_degree_increment() {
        assert_eq!(graph::scratch_len_for_degree(0), Ok(1));
        assert_eq!(graph::scratch_len_for_degree(4), Ok(5));
        assert_eq!(
            graph::scratch_len_for_degree(usize::MAX),
            Err(LdpcError::SizeOverflow)
        );
    }

    #[test]
    fn check_messages_handle_degrees_zero_one_and_two_exactly() {
        let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![], vec![0], vec![0, 1]])
            .expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let config = DecoderConfig::try_new(0, 7.0).expect("limit is valid");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0, -4.0],
                    erasures: &[],
                },
                config,
            )
            .expect("input is valid");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.r, [7.0, -4.0, 3.0]);
        assert_eq!(float_bits(&state.q), old_q);
    }

    #[test]
    fn degree_three_messages_match_the_hand_calculation() {
        let checks =
            ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llr = 3.0_f64.ln();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, llr, llr],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");
        let old_q = float_bits(&state.q);

        state.update_check_messages(DecoderConfig::default());

        for &message in &state.r {
            assert_close((5.0_f64 / 3.0).ln(), message);
        }
        assert_eq!(float_bits(&state.q), old_q);
    }

    #[test]
    fn check_messages_preserve_sign_and_handle_one_zero_multiplier() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llr = 3.0_f64.ln();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, -llr, llr, 0.0],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_close((3.0_f64 / 5.0).ln(), state.r[0]);
        assert_close((5.0_f64 / 3.0).ln(), state.r[1]);
        assert_close((3.0_f64 / 5.0).ln(), state.r[2]);

        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2, 3]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[llr, -llr, 0.0, llr],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_eq!(state.r[0], 0.0);
        assert_eq!(state.r[1], 0.0);
        assert_close((7.0_f64 / 9.0).ln(), state.r[2]);
        assert_eq!(state.r[3], 0.0);
    }

    #[test]
    fn two_zero_multipliers_make_every_excluded_product_zero() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2, 3]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[3.0_f64.ln(), 0.0, 0.0, -3.0_f64.ln()],
                    erasures: &[],
                },
                DecoderConfig::default(),
            )
            .expect("input is valid");

        state.update_check_messages(DecoderConfig::default());

        assert_eq!(state.r, [0.0; 4]);
    }

    #[test]
    fn product_conversion_clamps_unit_boundaries_and_keeps_zero_exact() {
        let config = DecoderConfig::try_new(0, 6.5).expect("limit is valid");

        assert_eq!(state::llr_from_product(1.0, config), 6.5);
        assert_eq!(state::llr_from_product(-1.0, config), -6.5);
        assert_eq!(
            state::llr_from_product(0.0, config).to_bits(),
            0.0_f64.to_bits()
        );
    }

    #[test]
    fn extreme_channel_values_produce_finite_bounded_messages() {
        let checks =
            ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2]]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let config = DecoderConfig::default();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &[f64::MAX, -f64::MAX, f64::MAX],
                    erasures: &[],
                },
                config,
            )
            .expect("finite extreme inputs are saturated");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.q, [20.0, -20.0, 20.0]);
        assert_eq!(float_bits(&state.q), old_q);
        assert_finite_in_range(&state.r, -20.0, 20.0);
        assert_finite_in_range(&state.tanh, -1.0, 1.0);
        assert_finite_in_range(&state.prefix, -1.0, 1.0);
        assert_finite_in_range(&state.suffix, -1.0, 1.0);
    }

    #[test]
    fn degree_4096_uses_finite_reusable_scratch_without_changing_q() {
        let degree = 4096;
        let bits: Vec<_> = (0..degree).collect();
        let checks = ParityCheckMatrix::try_from_rows(degree, vec![bits]).expect("matrix is valid");
        let mut state = SpaState::try_new(&checks).expect("sizes are representable");
        let llrs = vec![f64::MAX; degree];
        let config = DecoderConfig::default();
        state
            .prepare_block(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                config,
            )
            .expect("finite extreme inputs are saturated");
        let old_q = float_bits(&state.q);

        state.update_check_messages(config);

        assert_eq!(state.r.len(), degree);
        assert_eq!(state.tanh.len(), degree + 1);
        assert_eq!(state.prefix.len(), degree + 1);
        assert_eq!(state.suffix.len(), degree + 1);
        assert_eq!(float_bits(&state.q), old_q);
        assert_finite_in_range(&state.r, -20.0, 20.0);
        assert_finite_in_range(&state.tanh, -1.0, 1.0);
        assert_finite_in_range(&state.prefix, -1.0, 1.0);
        assert_finite_in_range(&state.suffix, -1.0, 1.0);
    }

    fn assert_close(expected: f64, actual: f64) {
        assert!(actual.is_finite(), "actual value is not finite: {actual}");
        let tolerance = 1e-10 + 1e-10 * expected.abs();
        assert!(
            (expected - actual).abs() <= tolerance,
            "expected {expected}, got {actual}, tolerance {tolerance}"
        );
    }

    fn assert_finite_in_range(values: &[f64], min: f64, max: f64) {
        for (index, &value) in values.iter().enumerate() {
            assert!(value.is_finite(), "value at index {index} is not finite");
            assert!(
                (min..=max).contains(&value),
                "value at index {index} is outside [{min}, {max}]: {value}"
            );
        }
    }
}
