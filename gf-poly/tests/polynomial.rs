use core::mem::size_of;

use gf2m::{Gf16, Gf256};
use gf_poly::{FieldElement, Polynomial, PolynomialError, MAX_POLYNOMIAL_BYTES};
use gfpm::{Gf, Gf9};

fn check_representation<F: FieldElement>() {
    let values = [F::one(), -F::one(), F::zero(), F::zero()];
    let polynomial = Polynomial::try_new(values.to_vec()).unwrap();
    assert_eq!(polynomial.coefficients(), &values);
    assert_eq!(polynomial.degree(), Some(1));
    assert!(!polynomial.is_zero());
    assert_eq!(polynomial.clone().into_coefficients().as_ref(), &values);

    let mut source = values;
    let copied = Polynomial::try_from_slice(&source).unwrap();
    source[0] = F::zero();
    assert_eq!(source[0], F::zero());
    assert_eq!(copied.coefficients(), &values);

    let boxed = values.to_vec().into_boxed_slice();
    let pointer = boxed.as_ptr();
    let owned = Polynomial::try_from(boxed).unwrap();
    assert_eq!(owned.coefficients().as_ptr(), pointer);
    let returned = owned.into_coefficients();
    assert_eq!(returned.as_ptr(), pointer);
    assert_eq!(Polynomial::try_from(values.to_vec()).unwrap(), copied);
}

#[test]
fn constructors_preserve_coefficients_in_both_field_families() {
    check_representation::<Gf256>();
    check_representation::<Gf9>();
    check_representation::<Gf16>();
    check_representation::<gf2m::Gf<16, 0x19>>();
    check_representation::<Gf<2, 2, 3>>();
}

fn check_equality<F: FieldElement>() {
    let representations = [
        vec![F::one()],
        vec![F::one(), F::zero()],
        vec![F::one(), F::zero(), F::zero()],
    ];
    let polynomials: Vec<_> = representations
        .iter()
        .map(|values| Polynomial::try_from_slice(values).unwrap())
        .collect();
    for (left, source) in polynomials.iter().zip(&representations) {
        assert_eq!(left.coefficients(), source);
        for right in &polynomials {
            assert_eq!(left, right);
            assert_eq!(right, left);
        }
    }
    let x = Polynomial::try_new(vec![F::zero(), F::one()]).unwrap();
    let x_squared = Polynomial::try_new(vec![F::zero(), F::zero(), F::one()]).unwrap();
    assert_eq!(x.degree(), Some(1));
    assert_eq!(x_squared.degree(), Some(2));
    assert_ne!(polynomials[0], x);
    assert_ne!(x, x_squared);
    let x_plus_one = Polynomial::try_new(vec![F::one(), F::one()]).unwrap();
    assert_eq!(x_plus_one.degree(), x.degree());
    assert_ne!(x, x_plus_one);

    let zeros: Vec<_> = [vec![], vec![F::zero()], vec![F::zero(); 4]]
        .into_iter()
        .map(|values| Polynomial::try_new(values).unwrap())
        .collect();
    for left in &zeros {
        assert_eq!(left.degree(), None);
        assert!(left.is_zero());
        assert_ne!(left, &x);
        for right in &zeros {
            assert_eq!(left, right);
        }
    }
}

#[test]
fn equality_ignores_only_high_zero_coefficients_and_zero_has_no_degree() {
    check_equality::<Gf256>();
    check_equality::<Gf9>();
}

#[test]
fn equality_compares_formal_polynomials_even_if_their_functions_agree() {
    type Gf2 = Gf<2, 1, 1>;
    // x² + x и 0 дают нуль в обеих точках GF(2), но это разные многочлены.
    let coefficients = [Gf2::zero(), Gf2::one(), Gf2::one()];
    for x in [Gf2::zero(), Gf2::one()] {
        assert_eq!(
            coefficients[0] + coefficients[1] * x + coefficients[2] * x * x,
            Gf2::zero()
        );
    }
    let polynomial = Polynomial::try_from_slice(&coefficients).unwrap();
    assert_ne!(polynomial, Polynomial::try_new(Vec::new()).unwrap());
}

fn check_limits<F: FieldElement>() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<F>();
    let values = vec![F::zero(); max_coefficients];
    assert_eq!(
        Polynomial::try_new(values.clone())
            .unwrap()
            .coefficients()
            .len(),
        max_coefficients
    );
    assert_eq!(
        Polynomial::try_from_slice(&values).unwrap().coefficients(),
        &values
    );
    assert_eq!(
        Polynomial::try_from(values.into_boxed_slice())
            .unwrap()
            .coefficients()
            .len(),
        max_coefficients
    );

    let oversized = vec![F::zero(); max_coefficients + 1];
    let expected = Err(PolynomialError::CoefficientLimitExceeded {
        coefficients: max_coefficients + 1,
        max_coefficients,
    });
    assert_eq!(Polynomial::try_from_slice(&oversized), expected);
    assert_eq!(Polynomial::try_new(oversized.clone()), expected);
    assert_eq!(Polynomial::try_from(oversized.clone()), expected);
    assert_eq!(Polynomial::try_from(oversized.into_boxed_slice()), expected);

    let mut spare_capacity = Vec::with_capacity(max_coefficients + 1);
    spare_capacity.push(F::one());
    assert_eq!(
        Polynomial::try_new(spare_capacity).unwrap().coefficients(),
        &[F::one()]
    );
}

#[test]
fn all_constructors_check_actual_bytes_including_trailing_zeros() {
    assert_eq!(MAX_POLYNOMIAL_BYTES, 131_072);
    check_limits::<Gf256>();
    check_limits::<Gf9>();
    check_limits::<Gf<2, 64, 0x1B>>();
}
