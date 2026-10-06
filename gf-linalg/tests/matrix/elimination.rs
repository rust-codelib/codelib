use crate::common::rectangular_values;
use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix};

#[test]
fn determinant_handles_identity_triangular_and_non_triangular_matrices() {
    let identity = Matrix::try_new(
        3,
        3,
        vec![
            Gf256::one(),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::one(),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::one(),
        ],
    )
    .unwrap();
    let triangular = Matrix::try_new(
        3,
        3,
        vec![
            Gf256::new(2),
            Gf256::new(0x80),
            Gf256::new(0x57),
            Gf256::zero(),
            Gf256::new(3),
            Gf256::new(0xff),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::new(4),
        ],
    )
    .unwrap();
    let non_triangular = Matrix::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)],
    )
    .unwrap();

    assert_eq!(identity.try_determinant().unwrap(), Gf256::one());
    assert_eq!(triangular.try_determinant().unwrap(), Gf256::new(24));
    assert_eq!(non_triangular.try_determinant().unwrap(), Gf256::new(2));
}

#[test]
fn determinant_multiplies_pivots_using_gf256_reduction() {
    let matrix = Matrix::try_new(
        2,
        2,
        vec![
            Gf256::new(0x80),
            Gf256::zero(),
            Gf256::zero(),
            Gf256::new(0x02),
        ],
    )
    .unwrap();

    assert_eq!(matrix.try_determinant().unwrap(), Gf256::new(0x1d));
}

#[test]
fn determinant_of_one_by_one_matrix_is_its_only_element() {
    for (value, expected) in [
        (Gf256::zero(), Gf256::zero()),
        (Gf256::new(0x53), Gf256::new(0x53)),
    ] {
        let matrix = Matrix::try_new(1, 1, vec![value]).unwrap();

        assert_eq!(matrix.try_determinant().unwrap(), expected);
        assert_eq!(matrix.as_slice(), &[value]);
    }
}

#[test]
fn determinant_rejects_rectangular_matrices_with_their_exact_shape() {
    for (rows, cols, data) in [(2, 3, rectangular_values()), (3, 2, rectangular_values())] {
        let matrix = Matrix::try_new(rows, cols, data).unwrap();
        let before = matrix.as_slice().to_vec();

        assert_eq!(
            matrix.try_determinant().unwrap_err(),
            LinalgError::NonSquareMatrix { rows, cols }
        );
        assert_eq!((matrix.rows(), matrix.cols()), (rows, cols));
        assert_eq!(matrix.as_slice(), before);
    }
}

#[test]
fn determinant_returns_zero_when_a_later_pivot_is_missing() {
    let matrix = Matrix::try_new(
        3,
        3,
        vec![
            Gf256::one(),
            Gf256::zero(),
            Gf256::one(),
            Gf256::zero(),
            Gf256::one(),
            Gf256::one(),
            Gf256::one(),
            Gf256::one(),
            Gf256::zero(),
        ],
    )
    .unwrap();
    let before = matrix.as_slice().to_vec();

    assert_eq!(matrix.try_determinant().unwrap(), Gf256::zero());
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn determinant_swaps_rows_and_does_not_change_the_source_matrix() {
    let matrix = Matrix::try_new(
        2,
        2,
        vec![Gf256::zero(), Gf256::one(), Gf256::one(), Gf256::zero()],
    )
    .unwrap();
    let before = matrix.as_slice().to_vec();

    assert_eq!(matrix.try_determinant().unwrap(), Gf256::one());
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn inverse_of_identity_is_identity() {
    let identity = identity_matrix(3);
    let before = identity.as_slice().to_vec();

    let inverse = identity.try_inverse().unwrap();

    assert_eq!(inverse, identity);
    assert_eq!(identity.as_slice(), before);
}

#[test]
fn inverse_of_nonsingular_two_by_two_has_expected_elements_and_both_products_are_identity() {
    let matrix = Matrix::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)],
    )
    .unwrap();
    let expected_inverse = Matrix::try_new(
        2,
        2,
        vec![
            Gf256::new(2),
            Gf256::new(1),
            Gf256::new(0x8f),
            Gf256::new(0x8e),
        ],
    )
    .unwrap();
    let before = matrix.as_slice().to_vec();

    let inverse = matrix.try_inverse().unwrap();

    assert_eq!(inverse, expected_inverse);
    assert_eq!(matrix.try_mul(&inverse).unwrap(), identity_matrix(2));
    assert_eq!(inverse.try_mul(&matrix).unwrap(), identity_matrix(2));
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn inverse_of_nonzero_one_by_one_matrix_uses_the_field_inverse() {
    let matrix = Matrix::try_new(1, 1, vec![Gf256::new(2)]).unwrap();
    let before = matrix.as_slice().to_vec();

    let inverse = matrix.try_inverse().unwrap();

    assert_eq!((inverse.rows(), inverse.cols()), (1, 1));
    assert_eq!(inverse.as_slice(), &[Gf256::new(0x8e)]);
    assert_eq!(matrix.try_mul(&inverse).unwrap(), identity_matrix(1));
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn inverse_rejects_rectangular_matrices_with_exact_dimensions_before_calculation() {
    for (rows, cols, data) in [(2, 3, rectangular_values()), (3, 2, rectangular_values())] {
        let matrix = Matrix::try_new(rows, cols, data).unwrap();
        let before = matrix.as_slice().to_vec();

        assert_eq!(
            matrix.try_inverse().unwrap_err(),
            LinalgError::NonSquareMatrix { rows, cols }
        );
        assert_eq!((matrix.rows(), matrix.cols()), (rows, cols));
        assert_eq!(matrix.as_slice(), before);
    }
}

#[test]
fn inverse_rejects_zero_and_dependent_matrices_without_changing_the_source() {
    for matrix in [
        Matrix::try_new(1, 1, vec![Gf256::zero()]).unwrap(),
        Matrix::try_new(2, 2, vec![Gf256::zero(); 4]).unwrap(),
        Matrix::try_new(
            2,
            2,
            vec![Gf256::one(), Gf256::new(2), Gf256::new(2), Gf256::new(4)],
        )
        .unwrap(),
    ] {
        let before = matrix.as_slice().to_vec();

        assert_eq!(
            matrix.try_inverse().unwrap_err(),
            LinalgError::SingularMatrix
        );
        assert_eq!(matrix.as_slice(), before);
    }
}

#[test]
fn inverse_swaps_rows_and_keeps_the_source_unchanged() {
    let matrix = Matrix::try_new(
        2,
        2,
        vec![Gf256::zero(), Gf256::one(), Gf256::one(), Gf256::zero()],
    )
    .unwrap();
    let before = matrix.as_slice().to_vec();

    let inverse = matrix.try_inverse().unwrap();

    assert_eq!(inverse, matrix);
    assert_eq!(matrix.try_mul(&inverse).unwrap(), identity_matrix(2));
    assert_eq!(matrix.as_slice(), before);
}

fn identity_matrix(size: usize) -> Matrix {
    let mut data = vec![Gf256::zero(); size * size];
    for index in 0..size {
        data[index * size + index] = Gf256::one();
    }

    Matrix::try_new(size, size, data).unwrap()
}
