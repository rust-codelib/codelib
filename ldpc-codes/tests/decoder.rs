use ldpc_codes::{
    Bit, DecodeEvent, DecodeInput, DecodeObserver, DecodeStatus, Decoder, DecoderConfig,
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
