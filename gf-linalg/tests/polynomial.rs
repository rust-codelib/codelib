use core::mem::size_of;

use gf2m::Gf256;
use gf_linalg::{FieldElement, Matrix, Vector, MAX_POLYNOMIAL_BYTES};
use gf_poly::{Polynomial, PolynomialError};
use gfpm::Gf9;

fn check_round_trip<F: FieldElement>() {
    for values in [
        vec![],
        vec![F::zero(); 3],
        vec![F::one(), F::zero(), F::zero()],
    ] {
        let vector = Vector::new(values);
        let polynomial =
            Polynomial::try_from(vector.try_to_polynomial_coefficients().unwrap()).unwrap();
        assert_eq!(polynomial.coefficients(), vector.as_slice());
        assert_eq!(
            Vector::from_polynomial_coefficients(polynomial.into_coefficients()),
            vector
        );
    }

    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            F::one(),
            F::zero(),
            F::zero(),
            F::zero(),
            F::zero(),
            F::zero(),
        ],
    )
    .unwrap();
    let polynomials: Vec<Polynomial<F>> = matrix
        .try_to_polynomial_rows()
        .unwrap()
        .into_iter()
        .map(|row| Polynomial::try_from(row).unwrap())
        .collect();
    assert_eq!(polynomials[0].degree(), Some(0));
    assert!(polynomials[1].is_zero());
    assert!(polynomials
        .iter()
        .all(|polynomial| polynomial.coefficients().len() == 3));
    let restored = Matrix::try_from_polynomial_rows(
        polynomials
            .into_iter()
            .map(Polynomial::into_coefficients)
            .collect(),
    )
    .unwrap();
    assert_eq!(restored, matrix);
}

#[test]
fn vectors_and_matrix_rows_round_trip_through_polynomials_without_losing_width() {
    assert_eq!(MAX_POLYNOMIAL_BYTES, gf_poly::MAX_POLYNOMIAL_BYTES);
    check_round_trip::<Gf256>();
    check_round_trip::<Gf9>();
}

#[test]
fn oversized_raw_coefficients_remain_valid_vectors_but_cannot_create_polynomials() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    let raw = vec![Gf256::zero(); max_coefficients + 1].into_boxed_slice();
    let vector = Vector::from_polynomial_coefficients(raw.clone());
    assert_eq!(vector.len(), max_coefficients + 1);
    assert_eq!(
        Polynomial::try_from(raw),
        Err(PolynomialError::CoefficientLimitExceeded {
            coefficients: max_coefficients + 1,
            max_coefficients
        })
    );
}
