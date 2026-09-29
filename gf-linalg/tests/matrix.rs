use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};

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

#[test]
fn matrices_are_equal_only_when_shape_and_all_elements_match() {
    let matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let same_matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let same_values_with_different_shape = Matrix::try_new(3, 2, rectangular_values()).unwrap();
    let different_element = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1),
            Gf256::new(2),
            Gf256::new(3),
            Gf256::new(4),
            Gf256::new(5),
            Gf256::new(6),
        ],
    )
    .unwrap();

    assert_eq!(matrix, same_matrix);
    assert!(!(matrix != same_matrix));
    assert_ne!(matrix, same_values_with_different_shape);
    assert_ne!(matrix, different_element);
    assert!(matrix != same_values_with_different_shape);
    assert!(matrix != different_element);
}

#[test]
fn changing_an_element_through_get_mut_changes_matrix_equality() {
    let matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let mut changed_matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();

    assert_eq!(matrix, changed_matrix);
    *changed_matrix.get_mut(1, 2).unwrap() = Gf256::new(6);

    assert_ne!(matrix, changed_matrix);
    assert!(matrix != changed_matrix);
    assert!(!(matrix == changed_matrix));
}

#[test]
fn matrix_implements_debug_and_eq() {
    fn assert_debug_and_eq<T: std::fmt::Debug + Eq>() {}

    assert_debug_and_eq::<Matrix>();
}

#[test]
fn matrix_addition_preserves_rectangular_shape_row_order_and_gf256_arithmetic() {
    let left_values = [0x80, 0x00, 0x57, 0x01, 0xff, 0x42].map(Gf256::new);
    let right_values = [0x1d, 0x00, 0x83, 0x01, 0xff, 0x24].map(Gf256::new);
    let left = Matrix::try_new(2, 3, left_values.to_vec()).unwrap();
    let right = Matrix::try_new(2, 3, right_values.to_vec()).unwrap();

    let sum = left.try_add(&right).unwrap();

    assert_eq!(sum.rows(), 2);
    assert_eq!(sum.cols(), 3);
    assert_eq!(
        sum.as_slice(),
        [0x9d, 0x00, 0xd4, 0x00, 0x00, 0x66].map(Gf256::new)
    );
    assert_eq!(left.as_slice(), left_values);
    assert_eq!(right.as_slice(), right_values);
}

#[test]
fn borrowed_matrix_addition_matches_method_and_keeps_operands_available() {
    let left = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let right = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::zero(),
            Gf256::new(2),
            Gf256::new(1),
            Gf256::new(4),
            Gf256::new(5),
            Gf256::zero(),
        ],
    )
    .unwrap();
    let expected = left.try_add(&right).unwrap();

    let sum = (&left + &right).unwrap();

    assert_eq!(sum, expected);
    assert_eq!(left.as_slice(), rectangular_values());
    assert_eq!(right.get(0, 1), Some(Gf256::new(2)));
    assert_eq!(left.rows(), 2);
    assert_eq!(right.cols(), 3);
}

#[test]
fn owned_matrix_addition_returns_the_sum() {
    let left = Matrix::try_new(1, 2, vec![Gf256::new(1), Gf256::new(2)]).unwrap();
    let right = Matrix::try_new(1, 2, vec![Gf256::new(3), Gf256::new(4)]).unwrap();

    let sum = (left + right).unwrap();

    assert_eq!(sum.rows(), 1);
    assert_eq!(sum.cols(), 2);
    assert_eq!(sum.as_slice(), &[Gf256::new(2), Gf256::new(6)]);
}

#[test]
fn matrix_multiplication_handles_rectangular_matrices_and_preserves_row_order() {
    let left = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1),
            Gf256::zero(),
            Gf256::new(1),
            Gf256::zero(),
            Gf256::new(1),
            Gf256::new(1),
        ],
    )
    .unwrap();
    let right = Matrix::try_new(
        3,
        2,
        vec![
            Gf256::new(1),
            Gf256::new(2),
            Gf256::new(3),
            Gf256::new(4),
            Gf256::new(5),
            Gf256::new(6),
        ],
    )
    .unwrap();

    let product = left.try_mul(&right).unwrap();

    assert_eq!((product.rows(), product.cols()), (2, 2));
    assert_eq!(
        product.as_slice(),
        &[Gf256::new(4), Gf256::new(4), Gf256::new(6), Gf256::new(2),]
    );
}

#[test]
fn matrix_multiplication_handles_one_row_times_one_column() {
    let row = Matrix::try_new(1, 3, vec![Gf256::new(1), Gf256::new(1), Gf256::new(1)]).unwrap();
    let column = Matrix::try_new(3, 1, vec![Gf256::new(2), Gf256::new(3), Gf256::new(4)]).unwrap();

    let product = row.try_mul(&column).unwrap();

    assert_eq!((product.rows(), product.cols()), (1, 1));
    assert_eq!(product.as_slice(), &[Gf256::new(5)]);
}

#[test]
fn matrix_multiplication_with_identity_preserves_the_other_matrix() {
    let identity = Matrix::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::zero(), Gf256::zero(), Gf256::new(1)],
    )
    .unwrap();
    let matrix = Matrix::try_new(
        2,
        2,
        vec![Gf256::new(2), Gf256::new(3), Gf256::new(4), Gf256::new(5)],
    )
    .unwrap();
    let original = matrix.as_slice().to_vec();

    assert_eq!(identity.try_mul(&matrix).unwrap(), matrix);
    assert_eq!(matrix.try_mul(&identity).unwrap(), matrix);
    assert_eq!(matrix.as_slice(), original);
}

#[test]
fn matrix_multiplication_uses_gf256_reduction_for_products() {
    let left = Matrix::try_new(1, 1, vec![Gf256::new(0x80)]).unwrap();
    let right = Matrix::try_new(1, 1, vec![Gf256::new(0x02)]).unwrap();

    let product = left.try_mul(&right).unwrap();

    assert_eq!(product.as_slice(), &[Gf256::new(0x1d)]);
}

#[test]
fn borrowed_matrix_multiplication_matches_method_and_keeps_operands_available() {
    let left = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1),
            Gf256::zero(),
            Gf256::new(1),
            Gf256::zero(),
            Gf256::new(1),
            Gf256::new(1),
        ],
    )
    .unwrap();
    let right = Matrix::try_new(
        3,
        2,
        vec![
            Gf256::new(1),
            Gf256::new(2),
            Gf256::new(3),
            Gf256::new(4),
            Gf256::new(5),
            Gf256::new(6),
        ],
    )
    .unwrap();
    let expected = left.try_mul(&right).unwrap();

    let product = (&left * &right).unwrap();

    assert_eq!(product, expected);
    assert_eq!(left.rows(), 2);
    assert_eq!(left.cols(), 3);
    assert_eq!(right.rows(), 3);
    assert_eq!(right.cols(), 2);
    assert_eq!(left.as_slice()[0], Gf256::new(1));
    assert_eq!(right.as_slice()[5], Gf256::new(6));
}

#[test]
fn owned_matrix_multiplication_returns_the_product() {
    let left = Matrix::try_new(1, 2, vec![Gf256::new(1), Gf256::new(1)]).unwrap();
    let right = Matrix::try_new(2, 1, vec![Gf256::new(7), Gf256::new(8)]).unwrap();

    let product = (left * right).unwrap();

    assert_eq!((product.rows(), product.cols()), (1, 1));
    assert_eq!(product.as_slice(), &[Gf256::new(15)]);
}

#[test]
fn matrix_multiplication_reports_incompatible_dimensions_in_operand_order() {
    let expected = LinalgError::MatrixProductMismatch {
        left_cols: 3,
        right_rows: 4,
    };

    let left = Matrix::try_new(2, 3, vec![Gf256::zero(); 6]).unwrap();
    let right = Matrix::try_new(4, 2, vec![Gf256::zero(); 8]).unwrap();
    assert_eq!(left.try_mul(&right).unwrap_err(), expected);

    let left = Matrix::try_new(2, 3, vec![Gf256::zero(); 6]).unwrap();
    let right = Matrix::try_new(4, 2, vec![Gf256::zero(); 8]).unwrap();
    assert_eq!((&left * &right).unwrap_err(), expected);

    let left = Matrix::try_new(2, 3, vec![Gf256::zero(); 6]).unwrap();
    let right = Matrix::try_new(4, 2, vec![Gf256::zero(); 8]).unwrap();
    assert_eq!((left * right).unwrap_err(), expected);
}

#[test]
fn transpose_swaps_rectangular_dimensions_and_preserves_row_major_order() {
    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(1),
            Gf256::new(2),
            Gf256::new(3),
            Gf256::new(4),
            Gf256::new(5),
            Gf256::new(6),
        ],
    )
    .unwrap();

    let transposed = matrix.transpose();

    assert_eq!(transposed.rows(), 3);
    assert_eq!(transposed.cols(), 2);
    assert_eq!(
        transposed.as_slice(),
        &[
            Gf256::new(1),
            Gf256::new(4),
            Gf256::new(2),
            Gf256::new(5),
            Gf256::new(3),
            Gf256::new(6),
        ]
    );
}

#[test]
fn transpose_handles_single_row_and_single_column() {
    let row = Matrix::try_new(1, 3, vec![Gf256::new(2), Gf256::new(0), Gf256::new(7)]).unwrap();
    let column = Matrix::try_new(3, 1, vec![Gf256::new(2), Gf256::new(0), Gf256::new(7)]).unwrap();

    let transposed_row = row.transpose();
    let transposed_column = column.transpose();

    assert_eq!((transposed_row.rows(), transposed_row.cols()), (3, 1));
    assert_eq!(transposed_row.as_slice(), row.as_slice());
    assert_eq!((transposed_column.rows(), transposed_column.cols()), (1, 3));
    assert_eq!(transposed_column.as_slice(), column.as_slice());
}

#[test]
fn transpose_handles_single_cell_and_square_matrices() {
    let single_cell = Matrix::try_new(1, 1, vec![Gf256::new(9)]).unwrap();
    let square = Matrix::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)],
    )
    .unwrap();

    assert_eq!(single_cell.transpose(), single_cell);
    assert_eq!(
        square.transpose().as_slice(),
        &[Gf256::new(1), Gf256::new(3), Gf256::new(2), Gf256::new(4),]
    );
}

#[test]
fn transposing_twice_restores_matrix_without_changing_the_source() {
    let matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let original_values = matrix.as_slice().to_vec();

    let transposed = matrix.transpose();
    let restored = transposed.transpose();

    assert_eq!(restored, matrix);
    assert_eq!(matrix.as_slice(), original_values);
    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
}

#[test]
fn transpose_handles_maximum_single_row_without_losing_elements() {
    let values = (0..MAX_MATRIX_DIM)
        .map(|value| Gf256::new((value % 256) as u16))
        .collect::<Vec<_>>();
    let matrix = Matrix::try_new(1, MAX_MATRIX_DIM, values.clone()).unwrap();

    let transposed = matrix.transpose();

    assert_eq!(transposed.rows(), MAX_MATRIX_DIM);
    assert_eq!(transposed.cols(), 1);
    assert_eq!(transposed.as_slice(), values);
    assert_eq!(matrix.as_slice(), values);
}

#[test]
fn matrix_vector_multiplication_preserves_row_order_and_uses_gf256_arithmetic() {
    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(0x80),
            Gf256::new(0x57),
            Gf256::new(0xff),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::zero(),
        ],
    )
    .unwrap();
    let vector = Vector::new(vec![Gf256::new(0x02), Gf256::new(0x02), Gf256::new(1)]);

    let product = matrix.try_mul_vector(&vector).unwrap();

    assert_eq!(product.len(), 2);
    assert_eq!(product.as_slice(), &[Gf256::new(0x4c), Gf256::zero()]);
}

#[test]
fn matrix_vector_multiplication_operators_match_method_and_preserve_borrowed_operands() {
    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(0x80),
            Gf256::new(0x57),
            Gf256::new(0xff),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::zero(),
        ],
    )
    .unwrap();
    let vector = Vector::new(vec![Gf256::new(0x02), Gf256::new(0x02), Gf256::new(1)]);
    let expected = matrix.try_mul_vector(&vector).unwrap();

    let borrowed_product = (&matrix * &vector).unwrap();
    assert_eq!(borrowed_product, expected);
    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
    assert_eq!(matrix.get(0, 0), Some(Gf256::new(0x80)));
    assert_eq!(
        vector.as_slice(),
        &[Gf256::new(0x02), Gf256::new(0x02), Gf256::new(1)]
    );

    let owned_product = (matrix * vector).unwrap();
    assert_eq!(owned_product, expected);
}

#[test]
fn matrix_vector_multiplication_reports_each_incompatible_length_exactly() {
    let matrix_cols = 3;
    for vector_len in [0, 2, 4, MAX_MATRIX_DIM + 1] {
        let expected = LinalgError::MatrixVectorLengthMismatch {
            matrix_cols,
            vector_len,
        };
        let matrix = Matrix::try_new(2, matrix_cols, vec![Gf256::zero(); 2 * matrix_cols]).unwrap();
        let vector = Vector::new(vec![Gf256::zero(); vector_len]);
        assert_eq!(matrix.try_mul_vector(&vector).unwrap_err(), expected);

        let matrix = Matrix::try_new(2, matrix_cols, vec![Gf256::zero(); 2 * matrix_cols]).unwrap();
        let vector = Vector::new(vec![Gf256::zero(); vector_len]);
        let original_matrix = matrix.as_slice().to_vec();
        let original_vector = vector.as_slice().to_vec();
        assert_eq!((&matrix * &vector).unwrap_err(), expected);
        assert_eq!((matrix.rows(), matrix.cols()), (2, matrix_cols));
        assert_eq!(matrix.as_slice(), original_matrix);
        assert_eq!(vector.as_slice(), original_vector);

        let matrix = Matrix::try_new(2, matrix_cols, vec![Gf256::zero(); 2 * matrix_cols]).unwrap();
        let vector = Vector::new(vec![Gf256::zero(); vector_len]);
        assert_eq!((matrix * vector).unwrap_err(), expected);
    }
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
