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
fn event_order_and_contents_match_zero_one_and_two_iteration_results() {
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
        let config = DecoderConfig::try_new(budget, 20.0).expect("valid configuration");
        let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");
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
            .expect("valid input");

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

        let mut decoder_without_observer =
            SpaDecoder::try_new(checks(), config).expect("valid matrix");
        let result_without_observer = decoder_without_observer
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                None,
            )
            .expect("valid input");
        assert_eq!(result, result_without_observer);
    }
}

#[test]
fn an_initially_satisfied_word_emits_started_then_finished_without_iterations() {
    let config = DecoderConfig::try_new(0, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");
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
        .expect("valid input");

    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.changed_bits(), 0);
    assert_eq!(result.initial_unsatisfied_checks(), 0);
    assert_eq!(result.final_unsatisfied_checks(), 0);
    assert_eq!(
        events.0,
        [
            DecodeEvent::Started {
                unsatisfied_checks: 0,
            },
            DecodeEvent::Finished {
                iterations: 0,
                status: DecodeStatus::ParitySatisfied,
                unsatisfied_checks: 0,
                changed_bits: 0,
            },
        ]
    );
}

#[test]
fn invalid_input_emits_no_events() {
    let config = DecoderConfig::try_new(2, 20.0).expect("valid configuration");
    let mut decoder = SpaDecoder::try_new(checks(), config).expect("valid matrix");
    let mut events = Events::default();

    let short_llrs = [1.0, 2.0];
    assert_eq!(
        decoder.decode(
            DecodeInput {
                llrs: &short_llrs,
                erasures: &[],
            },
            Some(&mut events),
        ),
        Err(LdpcError::LlrLengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert!(events.0.is_empty());

    let non_finite_llrs = [1.0, f64::NAN, 2.0];
    assert_eq!(
        decoder.decode(
            DecodeInput {
                llrs: &non_finite_llrs,
                erasures: &[],
            },
            Some(&mut events),
        ),
        Err(LdpcError::NonFiniteLlr { index: 1 })
    );
    assert!(events.0.is_empty());

    let llrs = [1.0, 2.0, 3.0];
    assert_eq!(
        decoder.decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[3],
            },
            Some(&mut events),
        ),
        Err(LdpcError::ErasureIndexOutOfBounds { bit: 3, bits: 3 })
    );
    assert!(events.0.is_empty());

    assert_eq!(
        decoder.decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[1, 1],
            },
            Some(&mut events),
        ),
        Err(LdpcError::DuplicateErasureIndex { bit: 1 })
    );
    assert!(events.0.is_empty());
}
