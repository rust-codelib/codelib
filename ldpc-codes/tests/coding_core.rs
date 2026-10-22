use coding_core::{Candidate, DecodeOutcome};
use ldpc_codes::{
    Bit, DecodeChecks, DecodeResult, DecodeStatus, LdpcDecodeOutcome, ParityCheckMatrix,
};

fn two_check_three_bit_matrix() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the hand-calculated matrix is valid")
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
