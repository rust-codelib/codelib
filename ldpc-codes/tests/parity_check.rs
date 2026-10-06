use gf_linalg::MAX_MATRIX_DIM;
use ldpc_codes::{LdpcError, ParityCheckMatrix};

#[test]
fn accepts_minimum_and_maximum_axis_sizes() {
    let minimum = ParityCheckMatrix::try_from_rows(1, vec![vec![]]).unwrap();
    assert_eq!(minimum.rows(), 1);
    assert_eq!(minimum.cols(), 1);

    let max_rows =
        ParityCheckMatrix::try_from_rows(1, vec![Vec::<usize>::new(); MAX_MATRIX_DIM]).unwrap();
    assert_eq!(max_rows.rows(), MAX_MATRIX_DIM);
    assert_eq!(max_rows.cols(), 1);

    let max_cols =
        ParityCheckMatrix::try_from_rows(MAX_MATRIX_DIM, vec![vec![MAX_MATRIX_DIM - 1]]).unwrap();
    assert_eq!(max_cols.rows(), 1);
    assert_eq!(max_cols.cols(), MAX_MATRIX_DIM);
    assert_eq!(max_cols.check_bits(0), Some(&[MAX_MATRIX_DIM - 1][..]));

    let max_sparse_matrix =
        ParityCheckMatrix::try_from_rows(MAX_MATRIX_DIM, vec![Vec::<usize>::new(); MAX_MATRIX_DIM])
            .unwrap();
    assert_eq!(max_sparse_matrix.rows(), MAX_MATRIX_DIM);
    assert_eq!(max_sparse_matrix.cols(), MAX_MATRIX_DIM);
}

#[test]
fn rejects_zero_and_over_limit_axes_before_indices() {
    assert_eq!(
        ParityCheckMatrix::try_from_rows(1, vec![]).unwrap_err(),
        LdpcError::InvalidDimensions { rows: 0, cols: 1 }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(0, vec![vec![usize::MAX]]).unwrap_err(),
        LdpcError::InvalidDimensions { rows: 1, cols: 0 }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(usize::MAX, vec![vec![usize::MAX]]).unwrap_err(),
        LdpcError::InvalidDimensions {
            rows: 1,
            cols: usize::MAX,
        }
    );

    let too_many_rows = vec![vec![]; MAX_MATRIX_DIM + 1];
    assert_eq!(
        ParityCheckMatrix::try_from_rows(1, too_many_rows).unwrap_err(),
        LdpcError::InvalidDimensions {
            rows: MAX_MATRIX_DIM + 1,
            cols: 1,
        }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(MAX_MATRIX_DIM + 1, vec![vec![]]).unwrap_err(),
        LdpcError::InvalidDimensions {
            rows: 1,
            cols: MAX_MATRIX_DIM + 1,
        }
    );
}

#[test]
fn sorts_each_row_and_keeps_empty_and_repeated_rows() {
    let matrix =
        ParityCheckMatrix::try_from_rows(4, vec![vec![3, 0, 2], vec![], vec![0, 2, 3], vec![]])
            .unwrap();

    assert_eq!(matrix.rows(), 4);
    assert_eq!(matrix.cols(), 4);
    assert_eq!(matrix.check_bits(0), Some(&[0, 2, 3][..]));
    assert_eq!(matrix.check_bits(1), Some(&[][..]));
    assert_eq!(matrix.check_bits(2), Some(&[0, 2, 3][..]));
    assert_eq!(matrix.check_bits(3), Some(&[][..]));
    assert_eq!(matrix.check_bits(4), None);
}

#[test]
fn distinguishes_a_missing_check_from_an_existing_empty_check() {
    let matrix = ParityCheckMatrix::try_from_rows(2, vec![vec![]]).unwrap();

    assert_eq!(matrix.check_bits(0), Some(&[][..]));
    assert_eq!(matrix.check_bits(1), None);
    assert_eq!(matrix.check_bits(usize::MAX), None);
}

#[test]
fn rejects_out_of_bounds_indices_with_their_input_position() {
    assert_eq!(
        ParityCheckMatrix::try_from_rows(3, vec![vec![2, 3]]).unwrap_err(),
        LdpcError::BitIndexOutOfBounds {
            check: 0,
            bit: 3,
            bits: 3,
        }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(3, vec![vec![5, 3]]).unwrap_err(),
        LdpcError::BitIndexOutOfBounds {
            check: 0,
            bit: 5,
            bits: 3,
        }
    );
}

#[test]
fn rejects_duplicate_indices_after_validating_the_whole_row() {
    assert_eq!(
        ParityCheckMatrix::try_from_rows(2, vec![vec![0, 0, 2]]).unwrap_err(),
        LdpcError::BitIndexOutOfBounds {
            check: 0,
            bit: 2,
            bits: 2,
        }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(2, vec![vec![1, 0, 1]]).unwrap_err(),
        LdpcError::DuplicateBitIndex { check: 0, bit: 1 }
    );
}

#[test]
fn earlier_rows_take_priority_over_later_rows() {
    assert_eq!(
        ParityCheckMatrix::try_from_rows(2, vec![vec![0, 0], vec![2]]).unwrap_err(),
        LdpcError::DuplicateBitIndex { check: 0, bit: 0 }
    );
    assert_eq!(
        ParityCheckMatrix::try_from_rows(2, vec![vec![2], vec![0, 0]]).unwrap_err(),
        LdpcError::BitIndexOutOfBounds {
            check: 0,
            bit: 2,
            bits: 2,
        }
    );
}
