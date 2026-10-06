use core::ops::{Add, Mul, Neg, Sub};

use gf2m::{Gf as Gf2m, Gf16, Gf256};
use gf_linalg::{FieldElement, LinalgError, Matrix, Vector};
use gfpm::{Gf as Gfpm, Gf25, Gf9};

fn common_field_regressions<F: FieldElement>() {
    let zero = F::zero();
    let one = F::one();

    let identity = Matrix::<F>::try_new(2, 2, vec![one, zero, zero, one]).unwrap();
    assert_eq!(identity.try_mul(&identity).unwrap(), identity);

    let rectangular = Matrix::<F>::try_new(2, 3, vec![one, one, zero, zero, one, one]).unwrap();
    assert_eq!(rectangular.transpose().transpose(), rectangular);

    let triangular =
        Matrix::<F>::try_new(3, 3, vec![one, one, one, zero, one, one, zero, zero, one]).unwrap();
    assert_eq!(triangular.try_determinant().unwrap(), one);

    let inverse = triangular.try_inverse().unwrap();
    let identity = Matrix::<F>::try_new(
        3,
        3,
        vec![one, zero, zero, zero, one, zero, zero, zero, one],
    )
    .unwrap();
    assert_eq!(triangular.try_mul(&inverse).unwrap(), identity);
    assert_eq!(inverse.try_mul(&triangular).unwrap(), identity);

    let singular = Matrix::<F>::try_new(2, 2, vec![one, one, one, one]).unwrap();
    assert_eq!(singular.try_determinant(), Ok(zero));
    assert_eq!(singular.try_inverse(), Err(LinalgError::SingularMatrix));
}

fn check_2x2_formula_and_inverses<F: FieldElement, const N: usize>(cases: [[F; 4]; N]) {
    let identity =
        Matrix::<F>::try_new(2, 2, vec![F::one(), F::zero(), F::zero(), F::one()]).unwrap();
    let mut invertible_cases = 0;

    for [a, b, c, d] in cases {
        let matrix = Matrix::<F>::try_new(2, 2, vec![a, b, c, d]).unwrap();
        let formula = a * d - b * c;
        assert_eq!(matrix.try_determinant(), Ok(formula));

        if !formula.is_zero() {
            let inverse = matrix.try_inverse().unwrap();
            assert_eq!(matrix.try_mul(&inverse).unwrap(), identity);
            assert_eq!(inverse.try_mul(&matrix).unwrap(), identity);
            invertible_cases += 1;
        }
    }

    assert!(invertible_cases >= 2);
}

#[test]
fn common_regressions_cover_named_and_direct_field_parameters() {
    common_field_regressions::<Gf256>();
    common_field_regressions::<Gf9>();
    common_field_regressions::<Gf16>();
    common_field_regressions::<Gf25>();

    // GF(16) with x^4 + x^3 + 1; this parameter pair has no named alias.
    common_field_regressions::<Gf2m<16, 0x19>>();
    // GF(4) over GF(2), with the irreducible modulus x^2 + x + 1.
    common_field_regressions::<Gfpm<2, 2, 3>>();
}

#[test]
fn two_by_two_determinants_match_the_independent_formula_in_both_characteristics() {
    check_2x2_formula_and_inverses::<Gf256, 4>([
        [Gf256::zero(), Gf256::one(), Gf256::one(), Gf256::zero()],
        [Gf256::one(), Gf256::one(), Gf256::one(), Gf256::zero()],
        [Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)],
        [Gf256::one(), Gf256::one(), Gf256::one(), Gf256::one()],
    ]);

    check_2x2_formula_and_inverses::<Gf9, 4>([
        [Gf9::zero(), Gf9::one(), Gf9::one(), Gf9::zero()],
        [Gf9::one(), Gf9::one(), Gf9::one(), Gf9::zero()],
        [Gf9::new(1), Gf9::new(2), Gf9::new(1), Gf9::new(1)],
        [Gf9::new(3), Gf9::new(1), Gf9::one(), Gf9::one()],
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArithmeticOnly(Gf9);

impl Add for ArithmeticOnly {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for ArithmeticOnly {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for ArithmeticOnly {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Neg for ArithmeticOnly {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl FieldElement for ArithmeticOnly {
    fn zero() -> Self {
        Self(Gf9::zero())
    }

    fn one() -> Self {
        Self(Gf9::one())
    }

    fn inv(self) -> Self {
        assert!(
            !self.0.is_zero(),
            "FieldElement::inv must receive nonzero values"
        );
        Self(self.0.inv())
    }
}

#[test]
fn arithmetic_only_field_supports_linear_algebra_and_coefficient_conversion() {
    let one = ArithmeticOnly::one();
    let two = ArithmeticOnly(Gf9::new(2));

    let left = Vector::<ArithmeticOnly>::new(vec![one, two, ArithmeticOnly::zero()]);
    let right = Vector::<ArithmeticOnly>::new(vec![two, one, ArithmeticOnly::zero()]);
    assert_eq!(left.try_add(&right).unwrap().as_slice(), &[one + two; 3]);
    assert_eq!(
        left.try_sub(&right).unwrap().as_slice(),
        &[two, one, ArithmeticOnly::zero()]
    );

    let swap = Matrix::<ArithmeticOnly>::try_new(
        2,
        2,
        vec![ArithmeticOnly::zero(), one, one, ArithmeticOnly::zero()],
    )
    .unwrap();
    assert_eq!(swap.try_determinant(), Ok(-one));
    let reduced = swap.rref();
    assert_eq!(reduced.pivot_columns(), &[0, 1]);
    assert_eq!(
        reduced.matrix(),
        &Matrix::try_new(
            2,
            2,
            vec![one, ArithmeticOnly::zero(), ArithmeticOnly::zero(), one]
        )
        .unwrap()
    );

    let inverse = swap.try_inverse().unwrap();
    assert_eq!(
        swap.try_mul(&inverse).unwrap().as_slice(),
        &[one, ArithmeticOnly::zero(), ArithmeticOnly::zero(), one]
    );
    assert_eq!(
        inverse.try_mul(&swap).unwrap().as_slice(),
        &[one, ArithmeticOnly::zero(), ArithmeticOnly::zero(), one]
    );

    let singular = Matrix::<ArithmeticOnly>::try_new(2, 2, vec![one, two, two, one]).unwrap();
    assert_eq!(singular.try_determinant(), Ok(ArithmeticOnly::zero()));
    assert_eq!(singular.try_inverse(), Err(LinalgError::SingularMatrix));

    let zero = Matrix::<ArithmeticOnly>::try_new(2, 2, vec![ArithmeticOnly::zero(); 4]).unwrap();
    assert_eq!(zero.try_inverse(), Err(LinalgError::SingularMatrix));
    let zero_result = zero.rref();
    assert_eq!(zero_result.matrix(), &zero);
    assert!(zero_result.pivot_columns().is_empty());
    assert_eq!(zero_result.rank(), 0);

    let coefficients = left.try_to_polynomial_coefficients().unwrap();
    assert_eq!(coefficients.as_ref(), left.as_slice());
    assert_eq!(Vector::from_polynomial_coefficients(coefficients), left);

    let rows = swap.try_to_polynomial_rows().unwrap();
    assert_eq!(Matrix::try_from_polynomial_rows(rows).unwrap(), swap);
}
