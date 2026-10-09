use ldpc_codes::{
    Bit, DecodeEvent, DecodeInput, DecodeObserver, DecodeResult, DecodeStatus, Decoder, LdpcError,
    ParityCheckMatrix,
};

fn checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the hand-calculated matrix is valid")
}

#[derive(Default)]
struct Events(Vec<DecodeEvent>);

impl DecodeObserver for Events {
    fn on_event(&mut self, event: &DecodeEvent) {
        self.0.push(*event);
    }
}

struct ExternalDecoder {
    checks: ParityCheckMatrix,
}

impl Decoder for ExternalDecoder {
    fn codeword_len(&self) -> usize {
        self.checks.cols()
    }

    fn decode(
        &mut self,
        input: DecodeInput<'_>,
        observer: Option<&mut dyn DecodeObserver>,
    ) -> Result<DecodeResult, LdpcError> {
        let bits = self.codeword_len();
        if input.llrs.len() != bits {
            return Err(LdpcError::LlrLengthMismatch {
                expected: bits,
                actual: input.llrs.len(),
            });
        }
        if let Some(index) = input.llrs.iter().position(|llr| !llr.is_finite()) {
            return Err(LdpcError::NonFiniteLlr { index });
        }
        if let Some(&bit) = input.erasures.iter().find(|&&bit| bit >= bits) {
            return Err(LdpcError::ErasureIndexOutOfBounds { bit, bits });
        }
        if let Some(pair) = input
            .erasures
            .iter()
            .enumerate()
            .find_map(|(index, &bit)| input.erasures[..index].contains(&bit).then_some(bit))
        {
            return Err(LdpcError::DuplicateErasureIndex { bit: pair });
        }

        let mut initial_word: Vec<_> = input
            .llrs
            .iter()
            .map(|&llr| if llr >= 0.0 { Bit::Zero } else { Bit::One })
            .collect();
        for &bit in input.erasures {
            initial_word[bit] = Bit::Zero;
        }

        // This test decoder performs one scripted update to the all-zero codeword.
        let posterior_llrs = vec![2.0, 0.0, -0.0];
        let result = DecodeResult::try_new(
            &self.checks,
            &initial_word,
            posterior_llrs,
            DecodeStatus::ParitySatisfied,
            1,
            5,
        )?;

        if let Some(observer) = observer {
            observer.on_event(&DecodeEvent::Started {
                unsatisfied_checks: result.initial_unsatisfied_checks(),
            });
            observer.on_event(&DecodeEvent::IterationFinished {
                iteration: 1,
                unsatisfied_checks: result.final_unsatisfied_checks(),
                changed_bits: result.changed_bits(),
            });
            observer.on_event(&DecodeEvent::Finished {
                iterations: result.iterations(),
                status: result.status(),
                unsatisfied_checks: result.final_unsatisfied_checks(),
                changed_bits: result.changed_bits(),
            });
        }

        Ok(result)
    }
}

#[test]
fn external_decoder_can_return_its_own_successful_result_and_events() {
    let mut decoder = ExternalDecoder { checks: checks() };
    let llrs = [-1.0, 1.0, 3.0];
    let mut events = Events::default();
    let result = decoder
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            Some(&mut events),
        )
        .expect("the external decoder builds a valid result from the input");

    assert_eq!(result.word(), [Bit::Zero, Bit::Zero, Bit::Zero]);
    assert_eq!(result.posterior_llrs(), [2.0, 0.0, -0.0]);
    assert_eq!(result.syndrome(), [Bit::Zero, Bit::Zero]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 1);
    assert_eq!(result.changed_bits(), 1);
    assert_eq!(result.initial_unsatisfied_checks(), 1);
    assert_eq!(result.final_unsatisfied_checks(), 0);
    assert_eq!(
        events.0,
        [
            DecodeEvent::Started {
                unsatisfied_checks: 1,
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
            },
        ]
    );
}

#[test]
fn external_decoder_validates_input_length_before_sending_events() {
    let mut decoder = ExternalDecoder { checks: checks() };
    let mut events = Events::default();
    let result = decoder.decode(
        DecodeInput {
            llrs: &[1.0],
            erasures: &[],
        },
        Some(&mut events),
    );

    assert_eq!(
        result,
        Err(LdpcError::LlrLengthMismatch {
            expected: 3,
            actual: 1,
        })
    );
    assert!(events.0.is_empty());
}

#[test]
fn result_constructor_rejects_mismatched_word_length() {
    let error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero],
        vec![1.0, 1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        0,
        0,
    )
    .expect_err("the initial decision must match H's number of columns");

    assert_eq!(
        error,
        LdpcError::WordLengthMismatch {
            expected: 3,
            actual: 1,
        }
    );
}

#[test]
fn result_constructor_rejects_mismatched_posterior_length() {
    let error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        0,
        0,
    )
    .expect_err("posterior values must match H's number of columns");

    assert_eq!(
        error,
        LdpcError::LlrLengthMismatch {
            expected: 3,
            actual: 2,
        }
    );
}

#[test]
fn result_constructor_rejects_non_finite_posterior_values() {
    for non_finite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = DecodeResult::try_new(
            &checks(),
            &[Bit::Zero; 3],
            vec![1.0, non_finite, 1.0],
            DecodeStatus::ParitySatisfied,
            0,
            0,
        )
        .expect_err("posterior values must be finite");

        assert_eq!(error, LdpcError::NonFiniteLlr { index: 1 });
    }
}

#[test]
fn result_constructor_rejects_a_status_that_disagrees_with_the_syndrome() {
    let error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, -1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        1,
        3,
    )
    .expect_err("the posterior decisions violate both parity checks");

    assert_eq!(error, LdpcError::DecodeResultStatusMismatch);
}

#[test]
fn iteration_limit_status_requires_a_nonzero_syndrome_and_exhausted_budget() {
    let satisfied_error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, 1.0, 1.0],
        DecodeStatus::IterationLimit,
        3,
        3,
    )
    .expect_err("a satisfied result must use ParitySatisfied");
    assert_eq!(satisfied_error, LdpcError::DecodeResultStatusMismatch);

    let incomplete_error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, -1.0, 1.0],
        DecodeStatus::IterationLimit,
        2,
        3,
    )
    .expect_err("an iteration-limit result must exhaust its declared budget");
    assert_eq!(incomplete_error, LdpcError::DecodeResultStatusMismatch);
}

#[test]
fn iteration_limit_results_are_constructible_at_zero_and_positive_budgets() {
    let zero_budget = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero, Bit::One, Bit::Zero],
        vec![1.0, -1.0, 1.0],
        DecodeStatus::IterationLimit,
        0,
        0,
    )
    .expect("an unsatisfied initial word may exhaust a zero-iteration budget");

    assert_eq!(zero_budget.word(), [Bit::Zero, Bit::One, Bit::Zero]);
    assert_eq!(zero_budget.syndrome(), [Bit::One, Bit::One]);
    assert_eq!(zero_budget.status(), DecodeStatus::IterationLimit);
    assert_eq!(zero_budget.iterations(), 0);
    assert_eq!(zero_budget.changed_bits(), 0);
    assert_eq!(zero_budget.initial_unsatisfied_checks(), 2);
    assert_eq!(zero_budget.final_unsatisfied_checks(), 2);

    let positive_budget = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, -1.0, 1.0],
        DecodeStatus::IterationLimit,
        2,
        2,
    )
    .expect("a nonzero syndrome may remain when a positive budget is exhausted");

    assert_eq!(positive_budget.word(), [Bit::Zero, Bit::One, Bit::Zero]);
    assert_eq!(positive_budget.syndrome(), [Bit::One, Bit::One]);
    assert_eq!(positive_budget.status(), DecodeStatus::IterationLimit);
    assert_eq!(positive_budget.iterations(), 2);
    assert_eq!(positive_budget.changed_bits(), 1);
    assert_eq!(positive_budget.initial_unsatisfied_checks(), 0);
    assert_eq!(positive_budget.final_unsatisfied_checks(), 2);
}

#[test]
fn result_constructor_rejects_iterations_above_the_declared_budget() {
    let error = DecodeResult::try_new(
        &checks(),
        &[Bit::Zero; 3],
        vec![1.0, 1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        2,
        1,
    )
    .expect_err("completed iterations cannot exceed the declared budget");

    assert_eq!(
        error,
        LdpcError::DecodeIterationsExceedLimit {
            iterations: 2,
            limit: 1,
        }
    );
}

#[test]
fn zero_iterations_require_the_initial_and_final_hard_words_to_match() {
    let error = DecodeResult::try_new(
        &checks(),
        &[Bit::One, Bit::Zero, Bit::Zero],
        vec![1.0, 1.0, 1.0],
        DecodeStatus::ParitySatisfied,
        0,
        5,
    )
    .expect_err("an unchanged algorithm cannot report a changed hard word");

    assert_eq!(error, LdpcError::DecodeResultZeroIterationsMismatch);
}
