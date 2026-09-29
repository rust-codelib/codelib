use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, MAX_MATRIX_DIM};

#[test]
fn rectangular_matrix_preserves_row_major_order_and_exposes_rows_and_coordinates() {
    let data = rectangular_values();
    let matrix = Matrix::try_new(2, 3, data.clone()).unwrap();

    assert_eq!(matrix.rows(), 2);
    assert_eq!(matrix.cols(), 3);
    assert_eq!(matrix.as_slice(), data.as_slice());
    assert_eq!(matrix.row(0), Some(&data[..3]));
    assert_eq!(matrix.row(1), Some(&data[3..]));

    for (row, col, expected) in [
        (0, 0, 1),
        (0, 1, 2),
        (0, 2, 3),
        (1, 0, 4),
        (1, 1, 5),
        (1, 2, 0),
    ] {
        assert_eq!(matrix.get(row, col), Some(Gf256::new(expected)));
    }
}

#[test]
fn accepts_single_cell_each_maximum_axis_and_more_than_maximum_elements() {
    assert!(Matrix::try_new(1, 1, vec![Gf256::new(1)]).is_ok());

    let maximum_rows =
        Matrix::try_new(MAX_MATRIX_DIM, 1, vec![Gf256::new(2); MAX_MATRIX_DIM]).unwrap();
    assert_eq!(maximum_rows.rows(), MAX_MATRIX_DIM);
    assert_eq!(maximum_rows.cols(), 1);

    let maximum_cols =
        Matrix::try_new(1, MAX_MATRIX_DIM, vec![Gf256::new(3); MAX_MATRIX_DIM]).unwrap();
    assert_eq!(maximum_cols.rows(), 1);
    assert_eq!(maximum_cols.cols(), MAX_MATRIX_DIM);

    let side = 65;
    let data = vec![Gf256::new(4); side * side];
    assert!(data.len() > MAX_MATRIX_DIM);
    let matrix = Matrix::try_new(side, side, data).unwrap();
    assert_eq!(matrix.rows() * matrix.cols(), matrix.as_slice().len());
}

#[test]
fn rejects_zero_oversized_and_usize_maximum_dimensions_without_panicking() {
    for (rows, cols) in [
        (0, 3),
        (3, 0),
        (0, 0),
        (MAX_MATRIX_DIM + 1, 1),
        (1, MAX_MATRIX_DIM + 1),
        (usize::MAX, 1),
        (1, usize::MAX),
        (usize::MAX, usize::MAX),
    ] {
        let result = Matrix::try_new(rows, cols, Vec::new());
        assert!(matches!(
            result,
            Err(LinalgError::InvalidDimensions {
                rows: actual_rows,
                cols: actual_cols,
            }) if actual_rows == rows && actual_cols == cols
        ));
    }
}

#[test]
fn rejects_data_lengths_that_do_not_match_the_dimensions() {
    for data in [Vec::new(), vec![Gf256::zero(); 5], vec![Gf256::zero(); 7]] {
        let actual = data.len();
        assert!(matches!(
            Matrix::try_new(2, 3, data),
            Err(LinalgError::ElementCountMismatch {
                expected: 6,
                actual: actual_len,
            }) if actual_len == actual
        ));
    }
}

#[test]
fn invalid_dimensions_take_precedence_over_data_length_mismatch() {
    assert!(matches!(
        Matrix::try_new(0, 3, vec![Gf256::zero()]),
        Err(LinalgError::InvalidDimensions { rows: 0, cols: 3 })
    ));
}

#[test]
fn row_and_element_access_return_none_at_boundaries_and_for_usize_maximum() {
    let mut matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();

    assert_eq!(matrix.row(2), None);
    assert_eq!(matrix.row(usize::MAX), None);
    assert_eq!(matrix.get(2, 0), None);
    assert_eq!(matrix.get(usize::MAX, 0), None);
    assert_eq!(matrix.get(0, 3), None);
    assert_eq!(matrix.get(0, usize::MAX), None);
    assert_eq!(matrix.get(usize::MAX, usize::MAX), None);

    // Столбец за пределами строки 0 не должен попасть в начало строки 1.
    assert_eq!(matrix.get(0, 3), None);

    assert!(matrix.get_mut(2, 0).is_none());
    assert!(matrix.get_mut(usize::MAX, 0).is_none());
    assert!(matrix.get_mut(0, 3).is_none());
    assert!(matrix.get_mut(0, usize::MAX).is_none());
    assert!(matrix.get_mut(usize::MAX, usize::MAX).is_none());
    assert_eq!(matrix.as_slice(), rectangular_values().as_slice());
}

#[test]
fn valid_mutable_access_changes_only_one_element_and_preserves_dimensions() {
    let mut matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();

    *matrix.get_mut(1, 1).unwrap() = Gf256::new(9);

    assert_eq!(
        matrix.as_slice(),
        &[
            Gf256::new(1),
            Gf256::new(2),
            Gf256::new(3),
            Gf256::new(4),
            Gf256::new(9),
            Gf256::zero(),
        ]
    );
    assert_eq!(matrix.rows() * matrix.cols(), matrix.as_slice().len());
    assert_eq!(matrix.get(1, 1), Some(Gf256::new(9)));
}

fn rectangular_values() -> Vec<Gf256> {
    vec![
        Gf256::new(1),
        Gf256::new(2),
        Gf256::new(3),
        Gf256::new(4),
        Gf256::new(5),
        Gf256::zero(),
    ]
}
