use ldpc_codes::{LdpcError, ParityCheckMatrix, SystematicEncoder};

fn encoder(bits: usize, rows: Vec<Vec<usize>>) -> SystematicEncoder {
    SystematicEncoder::try_new(ParityCheckMatrix::try_from_rows(bits, rows).unwrap()).unwrap()
}

#[test]
fn main_example_reports_rank_and_information_positions_using_dependent_rows() {
    let encoder = encoder(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    );

    assert_eq!(encoder.rank(), 3);
    assert_eq!(encoder.information_positions(), &[3, 4, 5]);
    assert_eq!(encoder.parity_positions(), &[0, 1, 2]);
}

#[test]
fn second_example_reports_cycle_three_permutations_positions_and_rank() {
    let encoder = encoder(5, vec![vec![0, 1, 4], vec![2, 3, 4]]);

    assert_eq!(encoder.rank(), 2);
    assert_eq!(encoder.information_positions(), &[1, 3, 4]);
    assert_eq!(encoder.parity_positions(), &[0, 2]);
}

#[test]
fn swap_and_identity_examples_have_the_expected_information_positions() {
    let swap = encoder(3, vec![vec![1, 2]]);
    assert_eq!(swap.information_positions(), &[0, 2]);

    let identity = encoder(3, vec![vec![2]]);
    assert_eq!(identity.information_positions(), &[0, 1]);
}

#[test]
fn rejects_zero_rank() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 0, cols: 3 }
    );
}

#[test]
fn rejects_full_rank() {
    let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![0], vec![1]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 2, cols: 2 }
    );
}

#[test]
fn rejects_one_column_encoder() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![0]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 1, cols: 1 }
    );
}

#[test]
fn rejects_zero_rank_for_one_column_encoder() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 0, cols: 1 }
    );
}

#[test]
fn allows_dependent_rows_when_the_number_of_checks_is_at_least_the_column_count() {
    let encoder = encoder(2, vec![vec![0], vec![0], vec![0]]);

    assert_eq!(encoder.rank(), 1);
    assert_eq!(encoder.information_positions(), &[1]);
}

#[test]
fn adding_an_empty_and_repeated_check_preserves_rank_and_positions() {
    let base = encoder(3, vec![vec![0, 2], vec![1, 2]]);
    let redundant = encoder(3, vec![vec![0, 2], vec![1, 2], vec![], vec![0, 2]]);

    assert_eq!(base.rank(), 2);
    assert_eq!(redundant.rank(), base.rank());
    assert_eq!(
        redundant.information_positions(),
        base.information_positions()
    );
    assert_eq!(redundant.parity_positions(), base.parity_positions());
}

#[test]
fn preparation_is_deterministic() {
    let rows = vec![vec![0, 1, 4], vec![2, 3, 4]];
    let first = encoder(5, rows.clone());
    let second = encoder(5, rows);

    assert_eq!(first.rank(), second.rank());
    assert_eq!(
        first.information_positions(),
        second.information_positions()
    );
    assert_eq!(first.parity_positions(), second.parity_positions());
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}
