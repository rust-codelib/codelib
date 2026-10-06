use super::*;
use crate::{Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

/// Медленный тестовый эталон: для каждого сообщения перебирает всех соседей,
/// кроме получателя, и использует `atanh` вместо промышленного преобразования.
struct DirectReference {
    rows: Vec<Vec<usize>>,
    bit_edges: Vec<Vec<usize>>,
    edge_bits: Vec<usize>,
    channel: Vec<f64>,
    q: Vec<f64>,
    r: Vec<f64>,
    posterior: Vec<f64>,
    word: Vec<Bit>,
    limit: f64,
}

impl DirectReference {
    fn new(checks: &ParityCheckMatrix, input: DecodeInput<'_>, config: DecoderConfig) -> Self {
        let limit = config.llr_limit();
        let mut channel: Vec<_> = input
            .llrs
            .iter()
            .map(|&llr| llr.clamp(-limit, limit))
            .collect();
        for &bit in input.erasures {
            channel[bit] = 0.0;
        }

        let mut bit_edges = vec![Vec::new(); checks.cols()];
        let mut edge_bits = Vec::with_capacity(checks.edge_count());
        let mut next_edge = 0;
        let rows = (0..checks.rows())
            .map(|check| {
                checks
                    .check_bits(check)
                    .expect("check index is valid")
                    .iter()
                    .map(|&bit| {
                        let edge = next_edge;
                        next_edge += 1;
                        edge_bits.push(bit);
                        bit_edges[bit].push(edge);
                        edge
                    })
                    .collect()
            })
            .collect();
        let q = edge_bits.iter().map(|&bit| channel[bit]).collect();
        let posterior = channel.clone();
        let word = channel
            .iter()
            .map(|&llr| if llr >= 0.0 { Bit::Zero } else { Bit::One })
            .collect();

        Self {
            rows,
            bit_edges,
            edge_bits,
            channel,
            q,
            r: vec![0.0; checks.edge_count()],
            posterior,
            word,
            limit,
        }
    }

    fn step(&mut self) {
        let mut next_r = vec![0.0; self.edge_bits.len()];
        for edges in &self.rows {
            match edges.as_slice() {
                [] => {}
                [edge] => next_r[*edge] = self.limit,
                [first, second] => {
                    next_r[*first] = self.q[*second];
                    next_r[*second] = self.q[*first];
                }
                _ => {
                    for &receiver in edges {
                        let product = edges
                            .iter()
                            .filter(|&&source| source != receiver)
                            .fold(1.0, |product, &source| {
                                product * (self.q[source] / 2.0).tanh()
                            });
                        let product_limit = 1.0 - f64::EPSILON;
                        next_r[receiver] = (2.0
                            * product.clamp(-product_limit, product_limit).atanh())
                        .clamp(-self.limit, self.limit);
                    }
                }
            }
        }

        let mut next_q = self.q.clone();
        for (bit, edges) in self.bit_edges.iter().enumerate() {
            if edges.is_empty() {
                self.posterior[bit] = self.channel[bit];
                self.word[bit] = if self.channel[bit] >= 0.0 {
                    Bit::Zero
                } else {
                    Bit::One
                };
                continue;
            }

            let total = self.channel[bit] + edges.iter().map(|&edge| next_r[edge]).sum::<f64>();
            self.posterior[bit] = total.clamp(-self.limit, self.limit);
            self.word[bit] = if self.posterior[bit] >= 0.0 {
                Bit::Zero
            } else {
                Bit::One
            };
            for &edge in edges {
                let other_messages = edges
                    .iter()
                    .filter(|&&other| other != edge)
                    .map(|&other| next_r[other])
                    .sum::<f64>();
                next_q[edge] = (self.channel[bit] + other_messages).clamp(-self.limit, self.limit);
            }
        }

        self.r = next_r;
        self.q = next_q;
    }
}

#[test]
fn direct_neighbor_reference_matches_two_steps_on_a_mixed_small_graph() {
    let checks = ParityCheckMatrix::try_from_rows(
        7,
        vec![
            vec![0, 1, 2, 3, 4],
            vec![],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![1, 3],
            vec![5],
        ],
    )
    .expect("small graph with duplicate and empty checks is valid");
    let input = DecodeInput {
        llrs: &[1.3, -0.7, 0.0, -1.1, -0.0, 0.2, -0.3],
        erasures: &[],
    };
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state.prepare_block(input, config).expect("input is valid");
    let mut reference = DirectReference::new(&checks, input, config);

    assert_state_matches_reference(&state, &reference, "before step 1");
    state.step(config);
    reference.step();
    assert_state_matches_reference(&state, &reference, "after step 1");
    // У первой проверки два нулевых множителя: каждое исключённое
    // произведение всё ещё содержит ноль.
    assert_eq!(
        state.graph.check_edges[0]
            .iter()
            .map(|&e| state.r[e])
            .collect::<Vec<_>>(),
        [0.0; 5]
    );
    // В проверках степени 3 один ноль оставляет ненулевым сообщение к нему.
    let single_zero_check = &state.graph.check_edges[2];
    assert!(state.r[single_zero_check[2]] != 0.0);
    assert_eq!(state.r[single_zero_check[0]], 0.0);
    assert_eq!(state.r[single_zero_check[1]], 0.0);
    // Повторяющиеся проверки сохраняются как отдельные одинаковые сообщения.
    assert_eq!(
        state.graph.check_edges[2]
            .iter()
            .map(|&edge| state.r[edge])
            .collect::<Vec<_>>(),
        state.graph.check_edges[3]
            .iter()
            .map(|&edge| state.r[edge])
            .collect::<Vec<_>>()
    );
    assert_eq!(state.graph.check_edges[1], []);
    assert!(state.graph.bit_edges[6].is_empty());

    state.step(config);
    reference.step();
    assert_state_matches_reference(&state, &reference, "after step 2");
}

#[test]
fn direct_reference_matches_q_r_and_posterior_for_two_llr2_steps() {
    let checks = ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )
    .expect("the general-plan parity-check matrix is valid");
    let input = DecodeInput {
        llrs: &[-0.5, 2.5, -4.0, 5.0, -3.5, 2.5],
        erasures: &[],
    };
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state.prepare_block(input, config).expect("LLR_2 is valid");
    let mut reference = DirectReference::new(&checks, input, config);

    assert_state_matches_reference(&state, &reference, "LLR_2 before step 1");
    for step in 1..=2 {
        state.step(config);
        reference.step();
        assert_state_matches_reference(&state, &reference, &format!("LLR_2 after step {step}"));
    }
}

#[test]
fn llr_lengths_cover_empty_short_and_long_inputs() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1]]).expect("valid matrix");
    for actual in [0, 2, 4] {
        let llrs = vec![0.0; actual];
        assert_eq!(
            prepare_channel(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                checks.cols(),
                DecoderConfig::default(),
            ),
            Err(LdpcError::LlrLengthMismatch {
                expected: 3,
                actual,
            })
        );
    }
}

#[test]
fn every_non_finite_value_is_rejected_even_at_an_erased_position() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1]]).expect("valid matrix");
    for bad_llr in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let llrs = [1.0, bad_llr, 2.0];
        let error = prepare_channel(
            DecodeInput {
                llrs: &llrs,
                erasures: &[1],
            },
            checks.cols(),
            DecoderConfig::default(),
        )
        .expect_err("non-finite erased LLR must be rejected");
        assert_eq!(error, LdpcError::NonFiniteLlr { index: 1 });
    }

    let llrs = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    assert_eq!(
        prepare_channel(
            DecodeInput {
                llrs: &llrs,
                erasures: &[0, 1, 2],
            },
            checks.cols(),
            DecoderConfig::default(),
        ),
        Err(LdpcError::NonFiniteLlr { index: 0 })
    );
}

#[test]
fn erasure_bounds_and_first_error_follow_input_order() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1]]).expect("valid matrix");
    let llrs = [0.0; 3];
    for bit in [3, usize::MAX] {
        assert_eq!(
            prepare_channel(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[1, bit],
                },
                checks.cols(),
                DecoderConfig::default(),
            ),
            Err(LdpcError::ErasureIndexOutOfBounds { bit, bits: 3 })
        );
    }
    assert_eq!(
        prepare_channel(
            DecodeInput {
                llrs: &llrs,
                erasures: &[2, 1, 2, 1],
            },
            checks.cols(),
            DecoderConfig::default(),
        ),
        Err(LdpcError::DuplicateErasureIndex { bit: 2 })
    );
}

#[test]
fn erasure_order_is_irrelevant_and_all_erased_bits_start_at_zero() {
    let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1, 2], vec![1, 2, 3]])
        .expect("valid matrix");
    let llrs = [3.0, -4.0, 5.0, -6.0];
    let config = DecoderConfig::default();
    let mut forward = SpaState::try_new(&checks).expect("workspace is representable");
    let mut reverse = SpaState::try_new(&checks).expect("workspace is representable");
    forward
        .prepare_block(
            DecodeInput {
                llrs: &llrs,
                erasures: &[0, 2],
            },
            config,
        )
        .expect("valid erasures");
    reverse
        .prepare_block(
            DecodeInput {
                llrs: &llrs,
                erasures: &[2, 0],
            },
            config,
        )
        .expect("the same erasures in another order are valid");
    assert_eq!(forward.channel, reverse.channel);
    forward.step(config);
    reverse.step(config);
    assert_eq!(forward.snapshot(), reverse.snapshot());

    let all_erased = prepare_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &[3, 1, 0, 2],
        },
        checks.cols(),
        config,
    )
    .expect("all positions may be erased in any order");
    assert_eq!(all_erased, [0.0; 4]);
}

#[test]
fn zero_rank_and_full_rank_matrices_are_valid_for_spa() {
    let config = DecoderConfig::default();
    let zero =
        ParityCheckMatrix::try_from_rows(3, vec![vec![], vec![]]).expect("zero matrix is valid");
    let mut zero_state = SpaState::try_new(&zero).expect("zero matrix has a valid workspace");
    zero_state
        .prepare_block(
            DecodeInput {
                llrs: &[-0.0, 1.0, -2.0],
                erasures: &[],
            },
            config,
        )
        .expect("zero matrix accepts a block");
    zero_state.step(config);
    assert_eq!(zero_state.r, []);
    assert_eq!(zero_state.posterior, [-0.0, 1.0, -2.0]);
    assert_eq!(zero_state.word, [Bit::Zero, Bit::Zero, Bit::One]);

    let full_rank = ParityCheckMatrix::try_from_rows(3, vec![vec![0], vec![1], vec![2]])
        .expect("full-rank square matrix is valid");
    let mut full_rank_state =
        SpaState::try_new(&full_rank).expect("full-rank matrix has a valid workspace");
    full_rank_state
        .prepare_block(
            DecodeInput {
                llrs: &[1.0, -2.0, 3.0],
                erasures: &[],
            },
            config,
        )
        .expect("full-rank matrix accepts a block");
    full_rank_state.step(config);
    assert_eq!(full_rank_state.r, [20.0; 3]);
    assert_eq!(full_rank_state.posterior, [20.0, 18.0, 20.0]);
}

#[test]
fn extreme_and_tiny_values_leave_every_workspace_buffer_finite_and_bounded() {
    let checks = ParityCheckMatrix::try_from_rows(6, vec![vec![0], vec![1, 2], vec![2, 3, 4, 5]])
        .expect("valid graph");
    let config = DecoderConfig::default();
    let mut state = SpaState::try_new(&checks).expect("workspace sizes are representable");
    state
        .prepare_block(
            DecodeInput {
                llrs: &[-20.0, 20.0, 1e-300, -1e-300, f64::MAX, -f64::MAX],
                erasures: &[],
            },
            config,
        )
        .expect("finite values beyond the limit are clipped");
    assert_eq!(state.channel, [-20.0, 20.0, 1e-300, -1e-300, 20.0, -20.0]);
    state.step(config);

    assert_finite_in_range(&state.channel, -20.0, 20.0);
    assert_finite_in_range(&state.q, -20.0, 20.0);
    assert_finite_in_range(&state.r, -20.0, 20.0);
    assert_finite_in_range(&state.posterior, -20.0, 20.0);
    assert_finite_in_range(&state.tanh, -1.0, 1.0);
    assert_finite_in_range(&state.prefix, -1.0, 1.0);
    assert_finite_in_range(&state.suffix, -1.0, 1.0);

    let tiny = ParityCheckMatrix::try_from_rows(1, vec![vec![0]]).expect("valid degree-one graph");
    let mut tiny_state = SpaState::try_new(&tiny).expect("workspace is representable");
    tiny_state
        .prepare_block(
            DecodeInput {
                llrs: &[1e-300],
                erasures: &[],
            },
            config,
        )
        .expect("tiny finite input is valid");
    tiny_state.step(config);
    assert_eq!(tiny_state.q, [1e-300]);
}

#[test]
fn a_bit_with_4096_neighbor_checks_keeps_all_messages_finite() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![0]; 4096])
        .expect("4096 checks on one bit are within the matrix limit");
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
    state.step(config);

    assert_eq!(state.graph.bit_edges[0].len(), 4096);
    assert_eq!(state.r.len(), 4096);
    assert_eq!(state.posterior, [20.0]);
    assert_finite_in_range(&state.channel, -20.0, 20.0);
    assert_finite_in_range(&state.q, -20.0, 20.0);
    assert_finite_in_range(&state.r, -20.0, 20.0);
    assert_finite_in_range(&state.posterior, -20.0, 20.0);
}

#[test]
fn reuse_after_two_steps_matches_a_fresh_state_and_invalid_prepare_is_atomic() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1, 2], vec![1, 2]]).expect("valid graph");
    let config = DecoderConfig::default();
    let mut reused = SpaState::try_new(&checks).expect("workspace is representable");
    reused
        .prepare_block(
            DecodeInput {
                llrs: &[1.0, 2.0, 3.0],
                erasures: &[],
            },
            config,
        )
        .expect("first block is valid");
    reused.step(config);
    reused.step(config);
    let previous = reused.snapshot();

    assert_eq!(
        reused.prepare_block(
            DecodeInput {
                llrs: &[f64::NAN, 8.0, 9.0],
                erasures: &[usize::MAX],
            },
            config,
        ),
        Err(LdpcError::NonFiniteLlr { index: 0 })
    );
    assert_eq!(reused.snapshot(), previous);

    let next = DecodeInput {
        llrs: &[-4.0, 0.0, 5.0],
        erasures: &[1],
    };
    reused
        .prepare_block(next, config)
        .expect("next block is valid");
    let mut fresh = SpaState::try_new(&checks).expect("fresh workspace is representable");
    fresh
        .prepare_block(next, config)
        .expect("same block is valid in a fresh state");
    assert_eq!(reused.snapshot(), fresh.snapshot());

    for _ in 0..2 {
        reused.step(config);
        fresh.step(config);
    }
    assert_eq!(reused.snapshot(), fresh.snapshot());
}

fn assert_state_matches_reference(state: &SpaState, reference: &DirectReference, phase: &str) {
    assert_float_slices_close(
        &reference.channel,
        &state.channel,
        &format!("{phase}: channel"),
    );
    assert_float_slices_close(&reference.q, &state.q, &format!("{phase}: q"));
    assert_float_slices_close(&reference.r, &state.r, &format!("{phase}: r"));
    assert_float_slices_close(
        &reference.posterior,
        &state.posterior,
        &format!("{phase}: posterior"),
    );
    assert_eq!(state.word, reference.word, "{phase}: hard decisions");
}

fn assert_float_slices_close(expected: &[f64], actual: &[f64], phase: &str) {
    assert_eq!(actual.len(), expected.len(), "{phase}: length");
    for (index, (&expected, &actual)) in expected.iter().zip(actual).enumerate() {
        assert!(
            actual.is_finite(),
            "{phase}[{index}] is not finite: {actual}"
        );
        let tolerance = 1e-10 + 1e-10 * expected.abs();
        assert!(
            (expected - actual).abs() <= tolerance,
            "{phase}[{index}]: expected {expected}, got {actual}, tolerance {tolerance}"
        );
    }
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
