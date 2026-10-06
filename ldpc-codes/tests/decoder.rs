use ldpc_codes::{
    Bit, DecodeEvent, DecodeInput, DecodeObserver, DecodeStatus, Decoder, DecoderConfig, LdpcError,
    ParityCheckMatrix, SpaDecoder,
};

#[derive(Default)]
struct Events(Vec<DecodeEvent>);

impl DecodeObserver for Events {
    fn on_event(&mut self, event: &DecodeEvent) {
        self.0.push(*event);
    }
}

fn checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the hand-calculated matrix is valid")
}

#[test]
fn hand_calculated_two_step_decode_obeys_budgets_and_reports_events() {
    let expected = [
        (
            0,
            [2.0, -3.0, 4.0],
            [Bit::Zero, Bit::One, Bit::Zero],
            [Bit::One, Bit::One],
            DecodeStatus::IterationLimit,
            0,
            0,
            2,
            vec![],
        ),
        (
            1,
            [-1.0, 3.0, 1.0],
            [Bit::One, Bit::Zero, Bit::Zero],
            [Bit::One, Bit::Zero],
            DecodeStatus::IterationLimit,
            1,
            2,
            1,
            vec![DecodeEvent::IterationFinished {
                iteration: 1,
                unsatisfied_checks: 1,
                changed_bits: 2,
            }],
        ),
        (
            2,
            [3.0, 3.0, 3.0],
            [Bit::Zero, Bit::Zero, Bit::Zero],
            [Bit::Zero, Bit::Zero],
            DecodeStatus::ParitySatisfied,
            2,
            1,
            0,
            vec![
                DecodeEvent::IterationFinished {
                    iteration: 1,
                    unsatisfied_checks: 1,
                    changed_bits: 2,
                },
                DecodeEvent::IterationFinished {
                    iteration: 2,
                    unsatisfied_checks: 0,
                    changed_bits: 1,
                },
            ],
        ),
    ];

    for (
        budget,
        expected_posterior,
        expected_word,
        expected_syndrome,
        expected_status,
        expected_iterations,
        expected_changed_bits,
        expected_final_unsatisfied,
        iteration_events,
    ) in expected
    {
        let config = DecoderConfig::try_new(budget, 20.0).expect("the configuration is valid");
        let mut decoder = SpaDecoder::try_new(checks(), config).expect("the matrix is valid");
        assert_eq!(decoder.codeword_len(), 3);
        let llrs = [2.0, -3.0, 4.0];
        let mut events = Events::default();

        let result = decoder
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                Some(&mut events),
            )
            .expect("the LLR block is valid");

        assert_eq!(result.posterior_llrs(), expected_posterior);
        assert_eq!(result.word(), expected_word);
        assert_eq!(result.syndrome(), expected_syndrome);
        assert_eq!(result.status(), expected_status);
        assert_eq!(result.iterations(), expected_iterations);
        assert_eq!(result.changed_bits(), expected_changed_bits);
        assert_eq!(result.initial_unsatisfied_checks(), 2);
        assert_eq!(
            result.final_unsatisfied_checks(),
            expected_final_unsatisfied
        );

        let mut expected_events = vec![DecodeEvent::Started {
            unsatisfied_checks: 2,
        }];
        expected_events.extend(iteration_events);
        expected_events.push(DecodeEvent::Finished {
            iterations: expected_iterations,
            status: expected_status,
            unsatisfied_checks: expected_final_unsatisfied,
            changed_bits: expected_changed_bits,
        });
        assert_eq!(events.0, expected_events);
        assert_eq!(llrs, [2.0, -3.0, 4.0]);

        let mut unobserved_decoder =
            SpaDecoder::try_new(checks(), config).expect("the matrix is valid");
        let unobserved_result = unobserved_decoder
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                None,
            )
            .expect("the LLR block is valid");
        assert_eq!(result, unobserved_result);

        if budget == 2 {
            let saved_result = result.clone();
            let next_llrs = [-2.0, -3.0, -4.0];
            let next_result = decoder
                .decode(
                    DecodeInput {
                        llrs: &next_llrs,
                        erasures: &[],
                    },
                    None,
                )
                .expect("the next block is valid");

            assert_eq!(next_result.iterations(), 0);
            assert_eq!(next_result.status(), DecodeStatus::ParitySatisfied);
            assert_eq!(next_result.posterior_llrs(), next_llrs);
            assert_eq!(result, saved_result, "a result owns its arrays");
        }
    }
}

#[test]
fn initially_satisfied_word_stops_before_the_first_iteration() {
    let mut decoder =
        SpaDecoder::try_new(checks(), DecoderConfig::default()).expect("the matrix is valid");
    let llrs = [2.0, 3.0, 4.0];
    let mut events = Events::default();

    let result = decoder
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            Some(&mut events),
        )
        .expect("the LLR block is valid");

    assert_eq!(result.posterior_llrs(), llrs);
    assert_eq!(result.word(), [Bit::Zero; 3]);
    assert_eq!(result.syndrome(), [Bit::Zero; 2]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.changed_bits(), 0);
    assert_eq!(result.initial_unsatisfied_checks(), 0);
    assert_eq!(result.final_unsatisfied_checks(), 0);
    assert_eq!(
        events.0,
        [
            DecodeEvent::Started {
                unsatisfied_checks: 0
            },
            DecodeEvent::Finished {
                iterations: 0,
                status: DecodeStatus::ParitySatisfied,
                unsatisfied_checks: 0,
                changed_bits: 0,
            }
        ]
    );
}

#[test]
fn decoder_construction_accepts_any_valid_parity_check_matrix() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![]]).expect("matrix is valid");
    let config = DecoderConfig::try_new(0, 1.0).expect("limit is valid");

    let mut decoder = SpaDecoder::try_new(checks, config).expect("zero-rank H is decodable");
    let result = decoder
        .decode(
            DecodeInput {
                llrs: &[2.0, -3.0, 0.0],
                erasures: &[],
            },
            None,
        )
        .expect("a zero-rank matrix still has a valid syndrome");

    assert_eq!(result.word(), [Bit::Zero, Bit::One, Bit::Zero]);
    assert_eq!(result.syndrome(), [Bit::Zero]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 0);
}

#[test]
fn input_errors_keep_their_priority_emit_nothing_and_do_not_poison_the_next_block() {
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");

    let first_llrs = [2.0, -3.0, 4.0];
    let mut first_events = Events::default();
    let first_result = decoder
        .decode(
            DecodeInput {
                llrs: &first_llrs,
                erasures: &[],
            },
            Some(&mut first_events),
        )
        .expect("the first block is valid");
    let saved_first_result = first_result.clone();

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
            &[1.0, f64::INFINITY, f64::NAN][..],
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
        let result = decoder.decode(DecodeInput { llrs, erasures }, Some(&mut events));
        assert_eq!(result, Err(expected_error));
        assert!(events.0.is_empty(), "an invalid block emits no events");
    }

    let next_llrs = [-4.0, 0.0, 5.0];
    let next_erasures = [1];
    let reused_result = decoder
        .decode(
            DecodeInput {
                llrs: &next_llrs,
                erasures: &next_erasures,
            },
            None,
        )
        .expect("a valid block after errors is independent");
    let mut fresh_decoder = SpaDecoder::try_new(checks(), config).expect("fresh decoder is valid");
    let fresh_result = fresh_decoder
        .decode(
            DecodeInput {
                llrs: &next_llrs,
                erasures: &next_erasures,
            },
            None,
        )
        .expect("the same block is valid in a fresh decoder");

    assert_eq!(reused_result, fresh_result);
    assert_eq!(
        first_result, saved_first_result,
        "prior results own their data"
    );
    assert_eq!(first_llrs, [2.0, -3.0, 4.0]);
    assert_eq!(next_llrs, [-4.0, 0.0, 5.0]);
    assert_eq!(next_erasures, [1]);
}

#[test]
fn non_finite_llrs_are_rejected_even_when_erased_without_events() {
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");
    let initial_llrs = [2.0, -3.0, 4.0];
    let previous = decoder
        .decode(
            DecodeInput {
                llrs: &initial_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("initial block is valid");
    let saved_previous = previous.clone();

    for bad_llr in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let llrs = [1.0, bad_llr, 3.0];
        let erasures = [1];
        let mut events = Events::default();
        assert_eq!(
            decoder.decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &erasures
                },
                Some(&mut events)
            ),
            Err(LdpcError::NonFiniteLlr { index: 1 })
        );
        assert!(events.0.is_empty());
    }

    let next_llrs = [-2.0, -3.0, -4.0];
    let result_after_errors = decoder
        .decode(
            DecodeInput {
                llrs: &next_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("the next valid block remains independent");
    let mut fresh_decoder = SpaDecoder::try_new(checks(), config).expect("fresh decoder is valid");
    let fresh_result = fresh_decoder
        .decode(
            DecodeInput {
                llrs: &next_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("the block is valid in a fresh decoder");

    assert_eq!(result_after_errors, fresh_result);
    assert_eq!(previous, saved_previous);
}

#[test]
fn zero_iteration_budget_checks_the_initial_syndrome_first() {
    let config = DecoderConfig::try_new(0, 20.0).expect("zero budget is valid");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");

    let satisfied_llrs = [2.0, 3.0, 4.0];
    let satisfied = decoder
        .decode(
            DecodeInput {
                llrs: &satisfied_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("an initially satisfied word is a successful decode");
    assert_eq!(satisfied.posterior_llrs(), satisfied_llrs);
    assert_eq!(satisfied.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(satisfied.iterations(), 0);

    let unsatisfied_llrs = [2.0, -3.0, 4.0];
    let unsatisfied = decoder
        .decode(
            DecodeInput {
                llrs: &unsatisfied_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("an unsatisfied word with no budget returns a normal result");
    assert_eq!(unsatisfied.posterior_llrs(), unsatisfied_llrs);
    assert_eq!(unsatisfied.status(), DecodeStatus::IterationLimit);
    assert_eq!(unsatisfied.iterations(), 0);
    assert_eq!(unsatisfied.initial_unsatisfied_checks(), 2);
    assert_eq!(unsatisfied.final_unsatisfied_checks(), 2);
}

#[test]
fn erasures_define_the_initial_word_and_changed_bits_baseline() {
    let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![0, 1]])
        .expect("the two-bit repetition check is valid");
    let config = DecoderConfig::try_new(1, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks, config).expect("valid matrix");
    let llrs = [-2.0, -3.0];
    let erasures = [0];
    let mut events = Events::default();

    let result = decoder
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &erasures,
            },
            Some(&mut events),
        )
        .expect("valid erasure input");

    assert_eq!(result.initial_unsatisfied_checks(), 1);
    assert_eq!(result.word(), [Bit::One, Bit::One]);
    assert_eq!(result.syndrome(), [Bit::Zero]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 1);
    assert_eq!(result.changed_bits(), 1);
    assert_eq!(
        events.0,
        [
            DecodeEvent::Started {
                unsatisfied_checks: 1
            },
            DecodeEvent::IterationFinished {
                iteration: 1,
                unsatisfied_checks: 0,
                changed_bits: 1,
            },
            DecodeEvent::Finished {
                iterations: 1,
                status: DecodeStatus::ParitySatisfied,
                unsatisfied_checks: 0,
                changed_bits: 1,
            }
        ]
    );
    assert_eq!(llrs, [-2.0, -3.0]);
    assert_eq!(erasures, [0]);
}

#[test]
fn zero_llrs_and_all_erasures_start_at_the_zero_codeword() {
    let config = DecoderConfig::try_new(5, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");

    for (llrs, erasures) in [
        ([0.0, -0.0, 0.0], &[][..]),
        ([-3.0, 4.0, -5.0], &[0, 1, 2][..]),
    ] {
        let mut events = Events::default();
        let result = decoder
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures,
                },
                Some(&mut events),
            )
            .expect("zero initial word satisfies every homogeneous check");

        assert_eq!(result.posterior_llrs(), [0.0; 3]);
        assert_eq!(result.word(), [Bit::Zero; 3]);
        assert_eq!(result.syndrome(), [Bit::Zero; 2]);
        assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
        assert_eq!(result.iterations(), 0);
        assert_eq!(result.changed_bits(), 0);
        assert_eq!(events.0.len(), 2);
        assert!(matches!(
            events.0[0],
            DecodeEvent::Started {
                unsatisfied_checks: 0
            }
        ));
        assert!(matches!(
            events.0[1],
            DecodeEvent::Finished { iterations: 0, .. }
        ));
    }
}

#[test]
fn zero_matrix_preserves_isolated_bits_and_full_rank_h_is_accepted() {
    let zero = ParityCheckMatrix::try_from_rows(4, vec![vec![], vec![]])
        .expect("zero matrix with isolated bits is valid");
    let mut zero_decoder =
        SpaDecoder::try_new(zero, DecoderConfig::default()).expect("zero-rank H is decodable");
    let extreme_llrs = [-0.0, f64::MAX, -f64::MAX, 0.0];
    let zero_result = zero_decoder
        .decode(
            DecodeInput {
                llrs: &extreme_llrs,
                erasures: &[],
            },
            None,
        )
        .expect("isolated bits accept finite extreme LLRs");

    assert_eq!(zero_result.posterior_llrs(), [-0.0, 20.0, -20.0, 0.0]);
    assert_eq!(
        zero_result.word(),
        [Bit::Zero, Bit::Zero, Bit::One, Bit::Zero]
    );
    assert_eq!(zero_result.syndrome(), [Bit::Zero; 2]);
    assert_eq!(zero_result.iterations(), 0);
    assert_eq!(extreme_llrs[0].to_bits(), (-0.0_f64).to_bits());
    assert_eq!(extreme_llrs[1], f64::MAX);
    assert_eq!(extreme_llrs[2], -f64::MAX);

    let full_rank = ParityCheckMatrix::try_from_rows(3, vec![vec![0], vec![1], vec![2]])
        .expect("identity H has full rank");
    let config = DecoderConfig::try_new(1, 20.0).expect("valid configuration");
    let mut full_rank_decoder =
        SpaDecoder::try_new(full_rank, config).expect("full-rank H is valid for SPA");
    let result = full_rank_decoder
        .decode(
            DecodeInput {
                llrs: &[-2.0, 3.0, -4.0],
                erasures: &[],
            },
            None,
        )
        .expect("full-rank matrix accepts a block");

    assert_eq!(result.word(), [Bit::Zero; 3]);
    assert_eq!(result.syndrome(), [Bit::Zero; 3]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 1);
}

#[test]
fn isolated_bit_keeps_its_channel_decision_while_another_component_runs_spa() {
    let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![0, 1], vec![1, 2]])
        .expect("the fourth bit is isolated from both checks");
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks, config).expect("valid matrix");
    let llrs = [2.0, -3.0, 4.0, -f64::MAX];
    let result = decoder
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            None,
        )
        .expect("finite LLRs include an isolated bit");

    assert_eq!(result.posterior_llrs(), [3.0, 3.0, 3.0, -20.0]);
    assert_eq!(result.word(), [Bit::Zero, Bit::Zero, Bit::Zero, Bit::One]);
    assert_eq!(result.syndrome(), [Bit::Zero; 2]);
    assert_eq!(result.iterations(), 2);
}

#[test]
fn empty_and_repeated_checks_keep_syndrome_order_and_observer_is_observational() {
    let rows = vec![vec![0, 1], vec![], vec![0, 1], vec![2]];
    let checks = ParityCheckMatrix::try_from_rows(3, rows.clone())
        .expect("empty and repeated rows are valid");
    let config = DecoderConfig::try_new(0, 20.0).expect("valid zero-budget configuration");
    let llrs = [-1.0, 1.0, -1.0];
    let input = DecodeInput {
        llrs: &llrs,
        erasures: &[],
    };
    let mut observed_decoder =
        SpaDecoder::try_new(checks, config).expect("repeated and empty checks are decodable");
    let mut events = Events::default();
    let observed = observed_decoder
        .decode(input, Some(&mut events))
        .expect("the block is valid");
    let mut unobserved_decoder = SpaDecoder::try_new(
        ParityCheckMatrix::try_from_rows(3, rows).expect("same matrix is valid"),
        config,
    )
    .expect("same matrix is decodable");
    let unobserved = unobserved_decoder
        .decode(input, None)
        .expect("the same block is valid without an observer");

    assert_eq!(observed, unobserved);
    assert_eq!(observed.word(), [Bit::One, Bit::Zero, Bit::One]);
    assert_eq!(
        observed.syndrome(),
        [Bit::One, Bit::Zero, Bit::One, Bit::One]
    );
    assert_eq!(observed.initial_unsatisfied_checks(), 3);
    assert_eq!(observed.final_unsatisfied_checks(), 3);
    assert_eq!(observed.iterations(), 0);
    assert_eq!(
        events.0,
        [
            DecodeEvent::Started {
                unsatisfied_checks: 3
            },
            DecodeEvent::Finished {
                iterations: 0,
                status: DecodeStatus::IterationLimit,
                unsatisfied_checks: 3,
                changed_bits: 0,
            }
        ]
    );
}

#[test]
fn a_satisfied_word_can_still_differ_from_the_transmitted_word() {
    let mut decoder = SpaDecoder::try_new(checks(), DecoderConfig::default())
        .expect("valid repetition-code matrix");
    let transmitted = [Bit::One; 3];
    let result = decoder
        .decode(
            DecodeInput {
                llrs: &[1.0, 1.0, 1.0],
                erasures: &[],
            },
            None,
        )
        .expect("positive LLRs produce a valid zero word");
    let errors_against_transmitted = result
        .word()
        .iter()
        .zip(transmitted)
        .filter(|(actual, expected)| **actual != *expected)
        .count();

    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.syndrome(), [Bit::Zero; 2]);
    assert_eq!(result.changed_bits(), 0);
    assert_eq!(errors_against_transmitted, 3);
}
