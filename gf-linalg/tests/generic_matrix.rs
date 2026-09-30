use gf_linalg::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};
use gfpm::Gf9;

fn rectangular_values() -> Vec<Gf9> {
    [1, 2, 3, 4, 5, 0].map(Gf9::new).to_vec()
}

#[test]
fn gf9_determinant_uses_subtraction_and_field_negation_for_row_swaps() {
    let subtraction = Matrix::<Gf9>::try_new(2, 2, [1, 2, 1, 1].map(Gf9::new).to_vec()).unwrap();
    let one_swap = Matrix::<Gf9>::try_new(2, 2, [0, 1, 1, 0].map(Gf9::new).to_vec()).unwrap();
    let two_swaps =
        Matrix::<Gf9>::try_new(3, 3, [0, 1, 0, 0, 0, 1, 1, 0, 0].map(Gf9::new).to_vec()).unwrap();
    let identity = Matrix::<Gf9>::try_new(2, 2, [1, 0, 0, 1].map(Gf9::new).to_vec()).unwrap();
    let triangular = Matrix::<Gf9>::try_new(2, 2, [3, 1, 0, 4].map(Gf9::new).to_vec()).unwrap();
    let singular = Matrix::<Gf9>::try_new(2, 2, [1, 2, 2, 1].map(Gf9::new).to_vec()).unwrap();
    let extension = Matrix::<Gf9>::try_new(1, 1, vec![Gf9::new(3)]).unwrap();

    for (matrix, expected) in [
        (&subtraction, Gf9::new(2)),
        (&one_swap, Gf9::new(2)),
        (&two_swaps, Gf9::one()),
        (&identity, Gf9::one()),
        (&triangular, Gf9::one()),
        (&singular, Gf9::zero()),
        (&extension, Gf9::new(3)),
    ] {
        let before = matrix.as_slice().to_vec();
        assert_eq!(matrix.try_determinant().unwrap(), expected);
        assert_eq!(matrix.as_slice(), before);
    }
}

#[test]
fn gf9_determinant_rejects_rectangles_with_the_original_shape() {
    let matrix = Matrix::<Gf9>::try_new(2, 3, rectangular_values()).unwrap();

    assert_eq!(
        matrix.try_determinant(),
        Err(LinalgError::NonSquareMatrix { rows: 2, cols: 3 })
    );
    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
    assert_eq!(matrix.as_slice(), rectangular_values());
}

#[test]
fn gf9_inverse_uses_subtraction_in_both_buffers_and_has_two_sided_identity_products() {
    let matrix = Matrix::<Gf9>::try_new(2, 2, [1, 1, 1, 0].map(Gf9::new).to_vec()).unwrap();
    let before = matrix.as_slice().to_vec();
    let expected = Matrix::<Gf9>::try_new(2, 2, [0, 1, 1, 2].map(Gf9::new).to_vec()).unwrap();
    let identity = Matrix::<Gf9>::try_new(2, 2, [1, 0, 0, 1].map(Gf9::new).to_vec()).unwrap();

    let inverse = matrix.try_inverse().unwrap();

    assert_eq!(inverse, expected);
    assert_eq!(matrix.try_mul(&inverse).unwrap(), identity);
    assert_eq!(inverse.try_mul(&matrix).unwrap(), identity);
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn gf9_inverse_handles_row_swaps_and_extension_field_scalars() {
    let swap = Matrix::<Gf9>::try_new(2, 2, [0, 1, 1, 0].map(Gf9::new).to_vec()).unwrap();
    let swap_inverse = swap.try_inverse().unwrap();
    let swap_identity = Matrix::<Gf9>::try_new(2, 2, [1, 0, 0, 1].map(Gf9::new).to_vec()).unwrap();
    assert_eq!(swap_inverse, swap);
    assert_eq!(swap.try_mul(&swap_inverse).unwrap(), swap_identity);
    assert_eq!(swap_inverse.try_mul(&swap).unwrap(), swap_identity);

    let scalar = Matrix::<Gf9>::try_new(1, 1, vec![Gf9::new(3)]).unwrap();
    let scalar_inverse = scalar.try_inverse().unwrap();
    assert_eq!(scalar_inverse.as_slice(), &[Gf9::new(4)]);
    assert_eq!(
        scalar.try_mul(&scalar_inverse).unwrap().as_slice(),
        &[Gf9::one()]
    );
    assert_eq!(scalar.as_slice(), &[Gf9::new(3)]);
}

#[test]
fn gf9_inverse_reports_singular_and_nonsquare_inputs_without_mutating_them() {
    let singular = [
        Matrix::<Gf9>::try_new(1, 1, vec![Gf9::zero()]).unwrap(),
        Matrix::<Gf9>::try_new(2, 2, [1, 2, 2, 1].map(Gf9::new).to_vec()).unwrap(),
    ];
    for matrix in singular {
        let before = matrix.as_slice().to_vec();
        assert_eq!(matrix.try_inverse(), Err(LinalgError::SingularMatrix));
        assert_eq!(matrix.as_slice(), before);
    }

    let rectangular = Matrix::<Gf9>::try_new(2, 3, rectangular_values()).unwrap();
    let before = rectangular.as_slice().to_vec();
    assert_eq!(
        rectangular.try_inverse(),
        Err(LinalgError::NonSquareMatrix { rows: 2, cols: 3 })
    );
    assert_eq!(rectangular.as_slice(), before);
}

#[test]
fn gf9_matrix_preserves_form_row_order_and_checked_access() {
    let values = rectangular_values();
    let mut matrix = Matrix::<Gf9>::try_new(2, 3, values.clone()).unwrap();

    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
    assert_eq!(matrix.as_slice(), values);
    assert_eq!(matrix.row(0), Some(&values[..3]));
    assert_eq!(matrix.row(1), Some(&values[3..]));
    assert_eq!(matrix.get(0, 0), Some(Gf9::new(1)));
    assert_eq!(matrix.get(1, 0), Some(Gf9::new(4)));
    assert_eq!(matrix.get(0, 3), None);
    assert_eq!(matrix.get(usize::MAX, usize::MAX), None);
    assert_eq!(matrix.row(2), None);
    assert_eq!(matrix.row(usize::MAX), None);

    *matrix.get_mut(1, 1).unwrap() = Gf9::new(8);
    assert_eq!(matrix.get(1, 1), Some(Gf9::new(8)));
    assert_eq!(matrix.as_slice()[..5], [1, 2, 3, 4, 8].map(Gf9::new));
    assert!(matrix.get_mut(2, 0).is_none());
    assert!(matrix.get_mut(0, 3).is_none());
    assert!(matrix.get_mut(usize::MAX, usize::MAX).is_none());
    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
    assert_eq!(matrix.as_slice().len(), matrix.rows() * matrix.cols());
}

#[test]
fn gf9_matrix_validates_each_axis_length_and_dimension_error_priority() {
    let maximum_rows =
        Matrix::<Gf9>::try_new(MAX_MATRIX_DIM, 1, vec![Gf9::one(); MAX_MATRIX_DIM]).unwrap();
    assert_eq!(
        (maximum_rows.rows(), maximum_rows.cols()),
        (MAX_MATRIX_DIM, 1)
    );
    let maximum_cols =
        Matrix::<Gf9>::try_new(1, MAX_MATRIX_DIM, vec![Gf9::one(); MAX_MATRIX_DIM]).unwrap();
    assert_eq!(
        (maximum_cols.rows(), maximum_cols.cols()),
        (1, MAX_MATRIX_DIM)
    );

    assert_eq!(
        Matrix::<Gf9>::try_new(2, 3, vec![Gf9::zero(); 5]).unwrap_err(),
        LinalgError::ElementCountMismatch {
            expected: 6,
            actual: 5,
        }
    );

    for (rows, cols) in [
        (0, 3),
        (3, 0),
        (MAX_MATRIX_DIM + 1, 1),
        (1, MAX_MATRIX_DIM + 1),
        (usize::MAX, usize::MAX),
    ] {
        assert_eq!(
            Matrix::<Gf9>::try_new(rows, cols, vec![Gf9::zero()]).unwrap_err(),
            LinalgError::InvalidDimensions { rows, cols }
        );
    }
}

#[test]
fn gf9_matrix_equality_and_double_transpose_keep_shape_and_values() {
    let matrix = Matrix::<Gf9>::try_new(2, 3, rectangular_values()).unwrap();
    let same = Matrix::<Gf9>::try_new(2, 3, rectangular_values()).unwrap();
    let same_values_with_other_shape = Matrix::<Gf9>::try_new(3, 2, rectangular_values()).unwrap();

    assert_eq!(matrix, same);
    assert_ne!(matrix, same_values_with_other_shape);
    assert_eq!(matrix.transpose().transpose(), matrix);
    assert_eq!(matrix.as_slice(), rectangular_values());
}

#[test]
fn gf9_matrix_addition_keeps_shape_and_sources_for_both_operators() {
    let left = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(1), Gf9::new(2), Gf9::new(3), Gf9::new(4)],
    )
    .unwrap();
    let right = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(2), Gf9::new(1), Gf9::new(1), Gf9::new(2)],
    )
    .unwrap();
    let expected = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::zero(), Gf9::zero(), Gf9::new(4), Gf9::new(3)],
    )
    .unwrap();

    assert_eq!(left.try_add(&right).unwrap(), expected);
    assert_eq!((&left + &right).unwrap(), expected);
    assert_eq!(
        (Matrix::<Gf9>::try_new(2, 2, left.as_slice().to_vec()).unwrap()
            + Matrix::<Gf9>::try_new(2, 2, right.as_slice().to_vec()).unwrap())
        .unwrap(),
        expected
    );
    assert_eq!(left.as_slice(), &[1, 2, 3, 4].map(Gf9::new));
    assert_eq!(right.as_slice(), &[2, 1, 1, 2].map(Gf9::new));
}

#[test]
fn gf9_matrix_multiplication_handles_rectangles_extension_products_and_accumulation() {
    let left = Matrix::<Gf9>::try_new(2, 3, [3, 4, 1, 1, 3, 4].map(Gf9::new).to_vec()).unwrap();
    let right = Matrix::<Gf9>::try_new(3, 2, [4, 1, 1, 3, 2, 4].map(Gf9::new).to_vec()).unwrap();
    let expected = Matrix::<Gf9>::try_new(2, 2, [4, 8, 3, 1].map(Gf9::new).to_vec()).unwrap();

    assert_eq!(left.try_mul(&right).unwrap(), expected);
    assert_eq!((&left * &right).unwrap(), expected);
    assert_eq!(
        (Matrix::<Gf9>::try_new(2, 3, left.as_slice().to_vec()).unwrap()
            * Matrix::<Gf9>::try_new(3, 2, right.as_slice().to_vec()).unwrap())
        .unwrap(),
        expected
    );
    assert_eq!((left.rows(), left.cols()), (2, 3));
    assert_eq!((right.rows(), right.cols()), (3, 2));
    assert_eq!(left.as_slice(), &[3, 4, 1, 1, 3, 4].map(Gf9::new));
    assert_eq!(right.as_slice(), &[4, 1, 1, 3, 2, 4].map(Gf9::new));

    let extension_product = Matrix::<Gf9>::try_new(1, 1, vec![Gf9::new(3)]).unwrap()
        * Matrix::<Gf9>::try_new(1, 1, vec![Gf9::new(4)]).unwrap();
    assert_eq!(extension_product.unwrap().as_slice(), &[Gf9::one()]);

    let row = Matrix::<Gf9>::try_new(1, 2, vec![Gf9::one(), Gf9::one()]).unwrap();
    let column = Matrix::<Gf9>::try_new(2, 1, vec![Gf9::one(), Gf9::one()]).unwrap();
    assert_eq!(row.try_mul(&column).unwrap().as_slice(), &[Gf9::new(2)]);
}

#[test]
fn gf9_matrix_vector_multiplication_and_dimension_errors_are_generic() {
    let matrix = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(3), Gf9::new(4), Gf9::one(), Gf9::new(2)],
    )
    .unwrap();
    let vector = Vector::<Gf9>::new(vec![Gf9::new(4), Gf9::new(3)]);
    let expected = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::one()]);

    assert_eq!(matrix.try_mul_vector(&vector).unwrap(), expected);
    assert_eq!((&matrix * &vector).unwrap(), expected);
    assert_eq!(
        (Matrix::<Gf9>::try_new(2, 2, matrix.as_slice().to_vec()).unwrap()
            * Vector::<Gf9>::new(vector.as_slice().to_vec()))
        .unwrap(),
        expected
    );
    assert_eq!(matrix.as_slice(), &[3, 4, 1, 2].map(Gf9::new));
    assert_eq!(vector.as_slice(), &[Gf9::new(4), Gf9::new(3)]);

    let differently_shaped = Matrix::<Gf9>::try_new(3, 2, vec![Gf9::zero(); 6]).unwrap();
    assert_eq!(
        matrix.try_add(&differently_shaped),
        Err(LinalgError::MatrixShapeMismatch {
            left_rows: 2,
            left_cols: 2,
            right_rows: 3,
            right_cols: 2,
        })
    );
    let incompatible_product = Matrix::<Gf9>::try_new(3, 1, vec![Gf9::zero(); 3]).unwrap();
    assert_eq!(
        matrix.try_mul(&incompatible_product),
        Err(LinalgError::MatrixProductMismatch {
            left_cols: 2,
            right_rows: 3,
        })
    );
    assert_eq!(
        matrix.try_mul_vector(&Vector::<Gf9>::new(vec![Gf9::one()])),
        Err(LinalgError::MatrixVectorLengthMismatch {
            matrix_cols: 2,
            vector_len: 1,
        })
    );
    assert_eq!(matrix.as_slice(), &[3, 4, 1, 2].map(Gf9::new));
    assert_eq!(vector.as_slice(), &[Gf9::new(4), Gf9::new(3)]);
}

#[test]
fn gf9_matrix_subtraction_uses_field_subtraction_and_preserves_shape() {
    let left = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(1), Gf9::new(2), Gf9::zero(), Gf9::one()],
    )
    .unwrap();
    let right =
        Matrix::<Gf9>::try_new(2, 2, vec![Gf9::new(2), Gf9::one(), Gf9::one(), Gf9::new(2)])
            .unwrap();
    let expected = Matrix::<Gf9>::try_new(
        2,
        2,
        vec![Gf9::new(2), Gf9::one(), Gf9::new(2), Gf9::new(2)],
    )
    .unwrap();

    assert_eq!(left.try_sub(&right).unwrap(), expected);
    assert_eq!((&left - &right).unwrap(), expected);
    assert_eq!(
        (Matrix::<Gf9>::try_new(2, 2, left.as_slice().to_vec()).unwrap()
            - Matrix::<Gf9>::try_new(2, 2, right.as_slice().to_vec()).unwrap())
        .unwrap(),
        expected
    );
    assert_eq!(left.try_sub(&left).unwrap().as_slice(), &[Gf9::zero(); 4]);
    assert_eq!((left.rows(), left.cols()), (2, 2));
    assert_eq!(
        left.as_slice(),
        &[Gf9::new(1), Gf9::new(2), Gf9::zero(), Gf9::one()]
    );
    assert_eq!(
        right.as_slice(),
        &[Gf9::new(2), Gf9::one(), Gf9::one(), Gf9::new(2)]
    );
}

#[test]
fn gf9_matrix_subtraction_keeps_rectangular_single_axis_shapes() {
    let row_left =
        Matrix::<Gf9>::try_new(1, 3, vec![Gf9::new(3), Gf9::new(4), Gf9::zero()]).unwrap();
    let row_right =
        Matrix::<Gf9>::try_new(1, 3, vec![Gf9::new(4), Gf9::new(3), Gf9::zero()]).unwrap();
    let row_difference = row_left.try_sub(&row_right).unwrap();
    assert_eq!((row_difference.rows(), row_difference.cols()), (1, 3));
    assert_eq!(
        row_difference.as_slice(),
        &[Gf9::new(2), Gf9::one(), Gf9::zero()]
    );

    let column_left =
        Matrix::<Gf9>::try_new(3, 1, vec![Gf9::one(), Gf9::new(2), Gf9::new(3)]).unwrap();
    let column_right =
        Matrix::<Gf9>::try_new(3, 1, vec![Gf9::new(2), Gf9::one(), Gf9::new(4)]).unwrap();
    let column_difference = column_left.try_sub(&column_right).unwrap();
    assert_eq!((column_difference.rows(), column_difference.cols()), (3, 1));
    assert_eq!(
        column_difference.as_slice(),
        &[Gf9::new(2), Gf9::one(), Gf9::new(2)]
    );
}

#[test]
fn matrix_subtraction_errors_report_each_shape_mismatch_and_gf256_matches_addition() {
    use gf2m::Gf256;

    let left = Matrix::<Gf9>::try_new(2, 3, vec![Gf9::one(); 6]).unwrap();
    let row_mismatch = Matrix::<Gf9>::try_new(3, 3, vec![Gf9::one(); 9]).unwrap();
    let col_mismatch = Matrix::<Gf9>::try_new(2, 2, vec![Gf9::one(); 4]).unwrap();
    let equal_element_count_mismatch = Matrix::<Gf9>::try_new(3, 2, vec![Gf9::one(); 6]).unwrap();

    assert_eq!(
        left.try_sub(&row_mismatch),
        Err(LinalgError::MatrixSubtractionShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 3,
            right_cols: 3,
        })
    );
    assert_eq!(
        left.try_sub(&col_mismatch),
        Err(LinalgError::MatrixSubtractionShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 2,
            right_cols: 2,
        })
    );
    assert_eq!(
        left.try_sub(&equal_element_count_mismatch),
        Err(LinalgError::MatrixSubtractionShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 3,
            right_cols: 2,
        })
    );
    assert_eq!(
        (&left - &equal_element_count_mismatch),
        Err(LinalgError::MatrixSubtractionShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 3,
            right_cols: 2,
        })
    );
    assert_eq!(
        (Matrix::<Gf9>::try_new(2, 3, left.as_slice().to_vec()).unwrap()
            - Matrix::<Gf9>::try_new(3, 2, equal_element_count_mismatch.as_slice().to_vec())
                .unwrap()),
        Err(LinalgError::MatrixSubtractionShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 3,
            right_cols: 2,
        })
    );
    assert_eq!(left.as_slice(), &[Gf9::one(); 6]);
    assert_eq!(equal_element_count_mismatch.as_slice(), &[Gf9::one(); 6]);

    let left_256 = Matrix::<Gf256>::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::new(18), Gf256::new(42), Gf256::zero()],
    )
    .unwrap();
    let right_256 = Matrix::<Gf256>::try_new(
        2,
        2,
        vec![Gf256::new(2), Gf256::new(18), Gf256::zero(), Gf256::new(5)],
    )
    .unwrap();
    assert_eq!(
        left_256.try_sub(&right_256).unwrap(),
        left_256.try_add(&right_256).unwrap()
    );
}
