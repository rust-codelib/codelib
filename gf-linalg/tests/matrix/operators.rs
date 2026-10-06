use crate::common::rectangular_values;
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector};

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
fn owned_matrix_addition_returns_the_sum() {
    let left = Matrix::try_new(1, 2, vec![Gf256::new(1), Gf256::new(2)]).unwrap();
    let right = Matrix::try_new(1, 2, vec![Gf256::new(3), Gf256::new(4)]).unwrap();

    let sum = (left + right).unwrap();

    assert_eq!(sum.rows(), 1);
    assert_eq!(sum.cols(), 2);
    assert_eq!(sum.as_slice(), &[Gf256::new(2), Gf256::new(6)]);
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
fn matrix_addition_rejects_each_shape_mismatch_with_ordered_dimensions() {
    for (left_rows, left_cols, right_rows, right_cols) in [
        (2, 3, 1, 3), // строки отличаются
        (2, 3, 2, 2), // столбцы отличаются
        (2, 3, 1, 2), // отличаются обе оси
        (2, 3, 3, 2), // число элементов одинаковое, форма различается
    ] {
        let expected = LinalgError::MatrixShapeMismatch {
            left_rows,
            left_cols,
            right_rows,
            right_cols,
        };

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!(left.try_add(&right).unwrap_err(), expected);

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!((&left + &right).unwrap_err(), expected);
        assert_eq!(left.rows(), left_rows);
        assert_eq!(left.cols(), left_cols);
        assert_eq!(right.rows(), right_rows);
        assert_eq!(right.cols(), right_cols);

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!((left + right).unwrap_err(), expected);
    }
}

fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    Matrix::try_new(rows, cols, vec![Gf256::zero(); rows * cols]).unwrap()
}
