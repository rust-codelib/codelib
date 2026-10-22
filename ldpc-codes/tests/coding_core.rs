use core::convert::Infallible;

use coding_core::{
    Candidate, Codeword, DecodeOutcome, Decoder, Encoder, MessageBytes, NoopObserver, Observer,
};
use ldpc_codes::{
    Bit, ConfiguredLdpc, DecodeChecks, DecodeEvent, DecodeInput, DecodeResult, DecodeStatus,
    DecoderConfig, LdpcConfigurator, LdpcDecodeOutcome, LdpcError, ParityCheckMatrix, SpaDecoder,
    SystematicEncoder,
};

fn two_check_three_bit_matrix() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the hand-calculated matrix is valid")
}

fn main_example_matrix() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )
    .expect("the general-plan matrix is valid")
}

fn repeated_and_empty_rows() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2], vec![0, 1], vec![]])
        .expect("repeated and empty rows are valid")
}

fn syndrome_unsatisfied_checks(result: &DecodeResult) -> usize {
    result
        .syndrome()
        .iter()
        .filter(|&&bit| bit == Bit::One)
        .count()
}

fn assert_summary_matches(result: &DecodeResult, expected_unsatisfied: usize) {
    let summary = result.checks();

    assert_eq!(summary.unsatisfied_checks(), expected_unsatisfied);
    assert_eq!(
        summary.unsatisfied_checks(),
        result.final_unsatisfied_checks()
    );
    assert_eq!(
        summary.unsatisfied_checks(),
        syndrome_unsatisfied_checks(result)
    );
    assert_eq!(summary.parity_satisfied(), expected_unsatisfied == 0);
    assert_eq!(
        summary.parity_satisfied(),
        result.final_unsatisfied_checks() == 0
    );

    match result.status() {
        DecodeStatus::ParitySatisfied => assert!(summary.parity_satisfied()),
        DecodeStatus::IterationLimit => assert!(!summary.parity_satisfied()),
    }
}

fn assert_encoder_contract<E, I>()
where
    E: Encoder<I, Output = Codeword<Vec<Bit>>, Error = LdpcError, Event = ()>,
{
}

fn assert_decoder_contract<D, I>()
where
    D: Decoder<
        I,
        Value = DecodeResult,
        Checks = DecodeChecks,
        NeedMore = Infallible,
        StopReason = Infallible,
        Error = LdpcError,
        Event = DecodeEvent,
    >,
{
}

fn encode_generic<'a, E>(encoder: &mut E, message: &'a [Bit]) -> Codeword<Vec<Bit>>
where
    E: Encoder<&'a [Bit], Output = Codeword<Vec<Bit>>, Error = LdpcError, Event = ()>,
{
    encoder
        .encode(message, &mut NoopObserver)
        .expect("the message length is valid")
}

fn decode_generic<'a, D>(decoder: &mut D, input: DecodeInput<'a>) -> LdpcDecodeOutcome
where
    D: Decoder<
        DecodeInput<'a>,
        Value = DecodeResult,
        Checks = DecodeChecks,
        NeedMore = Infallible,
        StopReason = Infallible,
        Error = LdpcError,
        Event = DecodeEvent,
    >,
{
    decoder
        .decode(input, &mut NoopObserver)
        .expect("the input length and values are valid")
}

fn expect_candidate(outcome: LdpcDecodeOutcome) -> Candidate<DecodeResult, DecodeChecks> {
    match outcome {
        DecodeOutcome::Candidate(candidate) => candidate,
        DecodeOutcome::NeedMore(reason) => match reason {},
        DecodeOutcome::Stopped(reason) => match reason {},
    }
}

fn assert_candidate_consistent(candidate: &Candidate<DecodeResult, DecodeChecks>) {
    assert_eq!(candidate.checks, candidate.value.checks());
    assert_eq!(
        candidate.checks.unsatisfied_checks(),
        candidate.value.final_unsatisfied_checks()
    );
    assert_eq!(
        candidate.checks.unsatisfied_checks(),
        syndrome_unsatisfied_checks(&candidate.value)
    );
    assert_eq!(
        candidate.checks.parity_satisfied(),
        candidate
            .value
            .syndrome()
            .iter()
            .all(|&bit| bit == Bit::Zero)
    );
    match candidate.value.status() {
        DecodeStatus::ParitySatisfied => assert!(candidate.checks.parity_satisfied()),
        DecodeStatus::IterationLimit => assert!(!candidate.checks.parity_satisfied()),
    }
}

fn strong_llrs(word: &[Bit]) -> Vec<f64> {
    word.iter()
        .map(|bit| match bit {
            Bit::Zero => 8.0,
            Bit::One => -8.0,
        })
        .collect()
}

#[derive(Default)]
struct Events(Vec<DecodeEvent>);

impl Observer<DecodeEvent> for Events {
    fn on_event(&mut self, event: &DecodeEvent) {
        self.0.push(*event);
    }
}

#[derive(Default)]
struct UnitEvents(usize);

impl Observer<()> for UnitEvents {
    fn on_event(&mut self, _: &()) {
        self.0 += 1;
    }
}

#[test]
fn successful_result_has_a_satisfied_checks_summary() {
    let checks = two_check_three_bit_matrix();
    let initial_word = [Bit::Zero; 3];
    let result = DecodeResult::try_new(
        &checks,
        &initial_word,
        vec![1.0, 2.0, 3.0],
        DecodeStatus::ParitySatisfied,
        0,
        0,
    )
    .expect("the all-zero word satisfies the matrix");

    assert_eq!(result.word(), initial_word);
    assert_eq!(result.syndrome(), [Bit::Zero, Bit::Zero]);
    assert_summary_matches(&result, 0);
}

#[test]
fn converged_result_summary_uses_the_final_syndrome() {
    let checks = two_check_three_bit_matrix();
    let initial_word = [Bit::One, Bit::Zero, Bit::Zero];
    let result = DecodeResult::try_new(
        &checks,
        &initial_word,
        vec![2.0, 2.0, 2.0],
        DecodeStatus::ParitySatisfied,
        2,
        5,
    )
    .expect("the positive posterior values decide the all-zero word");

    assert_eq!(result.word(), [Bit::Zero; 3]);
    assert_eq!(result.initial_unsatisfied_checks(), 1);
    assert_summary_matches(&result, 0);
}

#[test]
fn unsatisfied_summary_counts_repeated_rows_and_ignores_an_empty_row() {
    let checks = repeated_and_empty_rows();
    let initial_word = [Bit::Zero, Bit::One, Bit::Zero];
    let result = DecodeResult::try_new(
        &checks,
        &initial_word,
        vec![1.0, -1.0, 1.0],
        DecodeStatus::IterationLimit,
        0,
        0,
    )
    .expect("the initial word may exhaust a zero-iteration budget");

    assert_eq!(result.word(), initial_word);
    assert_eq!(result.syndrome(), [Bit::One, Bit::One, Bit::One, Bit::Zero]);
    assert_summary_matches(&result, 3);
}

#[test]
fn alias_names_the_decode_result_and_checks_contract() {
    let checks = two_check_three_bit_matrix();
    let result = DecodeResult::try_new(
        &checks,
        &[Bit::Zero; 3],
        vec![1.0, 1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        0,
        0,
    )
    .expect("the all-zero word satisfies the matrix");

    let expected: DecodeOutcome<
        DecodeResult,
        DecodeChecks,
        core::convert::Infallible,
        core::convert::Infallible,
    > = DecodeOutcome::Candidate(Candidate {
        value: result.clone(),
        checks: result.checks(),
    });
    let outcome: LdpcDecodeOutcome = expected;

    match outcome {
        DecodeOutcome::Candidate(candidate) => {
            assert_eq!(candidate.value.word(), result.word());
            assert_eq!(candidate.checks, result.checks());
            assert!(candidate.checks.parity_satisfied());
        }
        DecodeOutcome::NeedMore(reason) => match reason {},
        DecodeOutcome::Stopped(reason) => match reason {},
    }
}

#[test]
fn all_implementations_expose_the_documented_associated_types() {
    assert_encoder_contract::<SystematicEncoder, &'static [Bit]>();
    assert_encoder_contract::<ConfiguredLdpc, &'static [Bit]>();
    assert_decoder_contract::<SpaDecoder, DecodeInput<'static>>();
    assert_decoder_contract::<ConfiguredLdpc, DecodeInput<'static>>();
}

#[test]
fn generic_calls_accept_inputs_from_short_local_scopes() {
    let mut encoder = SystematicEncoder::try_new(two_check_three_bit_matrix())
        .expect("rank one gives a one-bit message");
    let word = {
        let message = [Bit::One];
        encode_generic(&mut encoder, &message)
    };
    assert_eq!(word.0, [Bit::One, Bit::One, Bit::One]);
    assert_eq!(encoder.extract_message(&word.0), Ok(vec![Bit::One]));

    let mut decoder = SpaDecoder::try_new(two_check_three_bit_matrix(), DecoderConfig::default())
        .expect("the matrix is valid");
    let candidate = {
        let llrs = [2.0, 3.0, 4.0];
        let input = DecodeInput {
            llrs: &llrs,
            erasures: &[],
        };
        expect_candidate(decode_generic(&mut decoder, input))
    };
    assert_candidate_consistent(&candidate);
    assert_eq!(candidate.value.word(), [Bit::Zero; 3]);
}

#[test]
fn successful_and_limit_outcomes_are_candidates_with_consistent_checks() {
    let mut decoder = SpaDecoder::try_new(
        two_check_three_bit_matrix(),
        DecoderConfig::try_new(1, 20.0).expect("the configuration is valid"),
    )
    .expect("the matrix is valid");
    let llrs = [-2.0, 1.0, -3.0];
    let converged = expect_candidate(
        decoder
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                &mut NoopObserver,
            )
            .expect("the block is valid"),
    );
    assert_candidate_consistent(&converged);
    assert_eq!(converged.value.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(converged.value.iterations(), 1);
    assert!(converged.checks.parity_satisfied());

    let mut zero_budget = SpaDecoder::try_new(
        two_check_three_bit_matrix(),
        DecoderConfig::try_new(0, 20.0).expect("zero iterations is a valid budget"),
    )
    .expect("the matrix is valid");
    let limited = expect_candidate(
        zero_budget
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                &mut NoopObserver,
            )
            .expect("an exhausted budget is a normal outcome"),
    );
    assert_candidate_consistent(&limited);
    assert_eq!(limited.value.status(), DecodeStatus::IterationLimit);
    assert_eq!(limited.value.iterations(), 0);
    assert_eq!(limited.value.word(), [Bit::One, Bit::Zero, Bit::One]);
    assert_eq!(limited.checks.unsatisfied_checks(), 2);
    assert!(!limited.checks.parity_satisfied());
}

#[test]
fn the_encoder_is_silent_for_user_and_closure_observers() {
    let mut encoder = SystematicEncoder::try_new(main_example_matrix())
        .expect("rank three gives a three-bit message");
    let message = [Bit::Zero, Bit::One, Bit::One];
    let mut unit_events = UnitEvents::default();
    let word = encoder
        .encode(&message, &mut unit_events)
        .expect("the message length is valid");
    assert_eq!(word.0.len(), 6);
    assert_eq!(unit_events.0, 0);

    let mut closure_calls = 0;
    {
        let mut closure = |_: &()| closure_calls += 1;
        let again = encoder
            .encode(&message, &mut closure)
            .expect("the message length is valid");
        assert_eq!(again, word);
    }
    assert_eq!(closure_calls, 0);
}

#[test]
fn decoder_observes_through_a_user_type_a_closure_and_noop_with_the_same_result() {
    let checks = two_check_three_bit_matrix();
    let config = DecoderConfig::try_new(2, 20.0).expect("the configuration is valid");
    let llrs = [2.0, -3.0, 4.0];
    let input = DecodeInput {
        llrs: &llrs,
        erasures: &[],
    };

    let mut observed_decoder = SpaDecoder::try_new(checks.clone(), config).expect("valid matrix");
    let mut events = Events::default();
    let observed = expect_candidate(
        observed_decoder
            .decode(input, &mut events)
            .expect("the block is valid"),
    );

    let mut closure_decoder = SpaDecoder::try_new(checks.clone(), config).expect("valid matrix");
    let mut closure_events = Vec::new();
    let closed = {
        let mut closure = |event: &DecodeEvent| closure_events.push(*event);
        expect_candidate(
            closure_decoder
                .decode(input, &mut closure)
                .expect("the block is valid"),
        )
    };

    let mut noop_decoder = SpaDecoder::try_new(checks, config).expect("valid matrix");
    let unobserved = expect_candidate(
        noop_decoder
            .decode(input, &mut NoopObserver)
            .expect("the block is valid"),
    );

    assert_eq!(observed, closed);
    assert_eq!(observed, unobserved);
    assert_eq!(events.0, closure_events);
    assert_eq!(events.0.len(), 4);
    assert_candidate_consistent(&observed);
    assert!(matches!(events.0[0], DecodeEvent::Started { .. }));
    assert!(matches!(
        events.0[1],
        DecodeEvent::IterationFinished { iteration: 1, .. }
    ));
    assert!(matches!(
        events.0[2],
        DecodeEvent::IterationFinished { iteration: 2, .. }
    ));
    assert!(matches!(
        events.0[3],
        DecodeEvent::Finished {
            iterations: 2,
            status: DecodeStatus::ParitySatisfied,
            ..
        }
    ));
}

#[test]
fn input_errors_emit_no_events_do_not_poison_reuse_or_change_old_candidates() {
    let checks = two_check_three_bit_matrix();
    let config = DecoderConfig::try_new(2, 20.0).expect("the configuration is valid");
    let mut decoder = SpaDecoder::try_new(checks.clone(), config).expect("valid matrix");
    let first_llrs = [2.0, -3.0, 4.0];
    let first = expect_candidate(
        decoder
            .decode(
                DecodeInput {
                    llrs: &first_llrs,
                    erasures: &[],
                },
                &mut NoopObserver,
            )
            .expect("the first block is valid"),
    );
    let saved = first.clone();

    let mut events = Events::default();
    assert_eq!(
        decoder.decode(
            DecodeInput {
                llrs: &[1.0],
                erasures: &[],
            },
            &mut events,
        ),
        Err(LdpcError::LlrLengthMismatch {
            expected: 3,
            actual: 1,
        })
    );
    assert!(events.0.is_empty());
    assert_eq!(first, saved);

    let next_llrs = [-4.0, 0.0, 5.0];
    let next_erasures = [1];
    let reused = expect_candidate(
        decoder
            .decode(
                DecodeInput {
                    llrs: &next_llrs,
                    erasures: &next_erasures,
                },
                &mut NoopObserver,
            )
            .expect("the next valid block is independent"),
    );
    let mut fresh =
        SpaDecoder::try_new(checks, config).expect("a fresh decoder has the same matrix");
    let fresh_candidate = expect_candidate(
        fresh
            .decode(
                DecodeInput {
                    llrs: &next_llrs,
                    erasures: &next_erasures,
                },
                &mut NoopObserver,
            )
            .expect("the same block is valid in a fresh decoder"),
    );

    assert_eq!(reused, fresh_candidate);
    assert_eq!(first, saved, "a prior candidate owns its data");
}

#[test]
fn configured_pair_matches_its_standalone_encoder_and_decoder() {
    let checks = main_example_matrix();
    let config = DecoderConfig::default();
    let mut configured =
        LdpcConfigurator::build(checks.clone(), config).expect("the matrix has an encoder rank");
    let mut encoder = SystematicEncoder::try_new(checks.clone()).expect("the matrix is valid");
    let mut decoder = SpaDecoder::try_new(checks, config).expect("the matrix is valid");

    for message in [
        [Bit::Zero, Bit::One, Bit::One],
        [Bit::One, Bit::Zero, Bit::Zero],
    ] {
        let configured_word = configured
            .encode(&message, &mut NoopObserver)
            .expect("the message length is valid");
        let standalone_word = encoder
            .encode(&message, &mut NoopObserver)
            .expect("the message length is valid");
        assert_eq!(configured_word, standalone_word);

        let llrs = strong_llrs(&configured_word.0);
        let mut configured_events = Events::default();
        let configured_candidate = expect_candidate(
            configured
                .decode(
                    DecodeInput {
                        llrs: &llrs,
                        erasures: &[],
                    },
                    &mut configured_events,
                )
                .expect("the noiseless block is valid"),
        );
        let mut standalone_events = Vec::new();
        let standalone_candidate = {
            let mut observer = |event: &DecodeEvent| standalone_events.push(*event);
            expect_candidate(
                decoder
                    .decode(
                        DecodeInput {
                            llrs: &llrs,
                            erasures: &[],
                        },
                        &mut observer,
                    )
                    .expect("the noiseless block is valid"),
            )
        };

        assert_eq!(configured_candidate, standalone_candidate);
        assert_eq!(configured_events.0, standalone_events);
        assert!(!configured_events.0.is_empty());
        assert_candidate_consistent(&configured_candidate);
        assert_eq!(
            configured
                .encoder()
                .extract_message(configured_candidate.value.word()),
            Ok(message.to_vec())
        );
    }

    assert_eq!(configured.message_len(), encoder.message_len());
    assert_eq!(configured.codeword_len(), encoder.codeword_len());
}

#[test]
fn information_bits_live_at_information_positions_not_the_word_prefix() {
    let mut configured = LdpcConfigurator::build(main_example_matrix(), DecoderConfig::default())
        .expect("the matrix has an encoder rank");
    assert_eq!(configured.encoder().information_positions(), &[3, 4, 5]);

    let message = [Bit::One, Bit::Zero, Bit::Zero];
    let word = configured
        .encode(&message, &mut NoopObserver)
        .expect("the message length is valid");
    for (&bit, &position) in message
        .iter()
        .zip(configured.encoder().information_positions())
    {
        assert_eq!(word.0[position], bit);
    }
    assert_ne!(&word.0[..message.len()], &message[..]);
    assert_eq!(
        configured.encoder().extract_message(&word.0),
        Ok(message.to_vec())
    );
}

#[test]
fn sizes_follow_the_shared_bit_to_byte_limit() {
    let configured =
        LdpcConfigurator::build(two_check_three_bit_matrix(), DecoderConfig::default())
            .expect("the matrix has an encoder rank");
    assert_eq!(configured.message_len(), 1);
    assert_eq!(configured.codeword_len(), 3);
    assert!(MessageBytes::from_bits(configured.message_len()).is_ok());
    assert!(MessageBytes::from_bits(configured.codeword_len()).is_ok());
}

#[test]
fn maximum_matrix_dimension_uses_one_independent_row_without_a_dense_rref() {
    let checks = ParityCheckMatrix::try_from_rows(gf_linalg::MAX_MATRIX_DIM, vec![vec![0]])
        .expect("one sparse row over the maximum column count is valid");
    let mut encoder = SystematicEncoder::try_new(checks).expect("rank one is encodable");
    assert_eq!(encoder.codeword_len(), gf_linalg::MAX_MATRIX_DIM);
    assert_eq!(encoder.message_len(), gf_linalg::MAX_MATRIX_DIM - 1);
    assert!(MessageBytes::from_bits(encoder.message_len()).is_ok());
    assert!(MessageBytes::from_bits(encoder.codeword_len()).is_ok());

    let mut message = vec![Bit::Zero; encoder.message_len()];
    message[0] = Bit::One;
    let word = encoder
        .encode(&message, &mut NoopObserver)
        .expect("the boundary message length is valid");
    assert_eq!(word.0.len(), gf_linalg::MAX_MATRIX_DIM);
    assert_eq!(word.0[0], Bit::Zero);
    assert_eq!(word.0[1], Bit::One);
    assert_eq!(encoder.extract_message(&word.0), Ok(message));
}
