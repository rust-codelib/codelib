use crate::common::rectangular_values;
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};

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
fn matrix_multiplication_handles_one_row_times_one_column() {
    let row = Matrix::try_new(1, 3, vec![Gf256::new(1), Gf256::new(1), Gf256::new(1)]).unwrap();
    let column = Matrix::try_new(3, 1, vec![Gf256::new(2), Gf256::new(3), Gf256::new(4)]).unwrap();

    let product = row.try_mul(&column).unwrap();

    assert_eq!((product.rows(), product.cols()), (1, 1));
    assert_eq!(product.as_slice(), &[Gf256::new(5)]);
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
fn matrix_multiplication_uses_gf256_reduction_for_products() {
    let left = Matrix::try_new(1, 1, vec![Gf256::new(0x80)]).unwrap();
    let right = Matrix::try_new(1, 1, vec![Gf256::new(0x02)]).unwrap();

    let product = left.try_mul(&right).unwrap();

    assert_eq!(product.as_slice(), &[Gf256::new(0x1d)]);
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
fn transposing_twice_restores_matrix_without_changing_the_source() {
    let matrix = Matrix::try_new(2, 3, rectangular_values()).unwrap();
    let original_values = matrix.as_slice().to_vec();

    let transposed = matrix.transpose();
    let restored = transposed.transpose();

    assert_eq!(restored, matrix);
    assert_eq!(matrix.as_slice(), original_values);
    assert_eq!((matrix.rows(), matrix.cols()), (2, 3));
}
