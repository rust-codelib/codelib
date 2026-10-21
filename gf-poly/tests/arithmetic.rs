use core::mem::size_of;

use gf2m::Gf256;
use gf_poly::{FieldElement, Polynomial, PolynomialError, MAX_POLYNOMIAL_BYTES};
use gfpm::{Gf, Gf9};

type F5 = Gf<5, 1, 1>;

fn polynomial<F: FieldElement>(coefficients: &[F]) -> Polynomial<F> {
    Polynomial::try_from_slice(coefficients).unwrap()
}

fn p5(coefficients: &[u64]) -> Polynomial<F5> {
    polynomial(
        &coefficients
            .iter()
            .copied()
            .map(F5::new)
            .collect::<Vec<_>>(),
    )
}

#[test]
fn known_results_over_odd_characteristic_preserve_signs() {
    let left = p5(&[1, 2, 3, 0]);
    let right = p5(&[4, 1, 0]);
    assert_eq!(left.try_add(&right).unwrap(), p5(&[0, 3, 3]));
    assert_eq!(left.try_sub(&right).unwrap(), p5(&[2, 1, 3]));
    assert_eq!(left.try_mul(&right).unwrap(), p5(&[4, 4, 4, 3]));
    assert_eq!(left.coefficients().len(), 4);
    assert_eq!(right.coefficients().len(), 3);
}

fn check_ring_laws<F: FieldElement>(a: F, b: F) {
    let x = polynomial(&[a, b, F::one(), F::zero()]);
    let y = polynomial(&[b, -a]);
    let z = polynomial(&[F::one(), a, b]);
    let zero = polynomial(&[F::zero(); 3]);
    let one = polynomial(&[F::one(), F::zero()]);
    assert_eq!(x.try_add(&zero).unwrap(), x);
    assert_eq!(x.try_mul(&one).unwrap(), x);
    assert!(x.try_mul(&zero).unwrap().coefficients().is_empty());
    assert!(x.try_sub(&x).unwrap().coefficients().is_empty());
    assert_eq!(x.try_add(&y).unwrap().try_sub(&y).unwrap(), x);
    assert_eq!(x.try_mul(&y).unwrap(), y.try_mul(&x).unwrap());
    assert_eq!(
        x.try_mul(&y.try_add(&z).unwrap()).unwrap(),
        x.try_mul(&y)
            .unwrap()
            .try_add(&x.try_mul(&z).unwrap())
            .unwrap()
    );
    assert_eq!(
        x.try_mul(&y).unwrap().try_mul(&z).unwrap(),
        x.try_mul(&y.try_mul(&z).unwrap()).unwrap()
    );
    assert_eq!((&x + &y).unwrap(), (x.clone() + y.clone()).unwrap());
    assert_eq!((&x - &y).unwrap(), (x.clone() - y.clone()).unwrap());
    assert_eq!((&x * &y).unwrap(), (x.clone() * y.clone()).unwrap());
    for result in [x.try_add(&y), x.try_sub(&y), x.try_mul(&y)] {
        let result = result.unwrap();
        assert!(result.coefficients().last().map_or(true, |v| !v.is_zero()));
    }
    assert_eq!(x.coefficients(), &[a, b, F::one(), F::zero()]);
}

#[test]
fn ring_laws_and_operators_for_prime_and_extension_fields() {
    for a in 0..9 {
        for b in 0..9 {
            check_ring_laws(Gf9::new(a), Gf9::new(b));
            check_ring_laws(Gf256::new(a as u16), Gf256::new(b as u16));
        }
    }
    check_ring_laws(F5::new(3), F5::new(4));
}

fn check_arithmetic_limits<F: FieldElement>() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<F>();
    let mut values = vec![F::zero(); max_coefficients];
    values[max_coefficients - 1] = F::one();
    let large = Polynomial::try_new(values).unwrap();
    let one = polynomial(&[F::one()]);
    let x = polynomial(&[F::zero(), F::one()]);
    assert_eq!(large.try_mul(&one).unwrap(), large);
    assert_eq!(large.try_add(&one).unwrap().degree(), large.degree());
    assert_eq!(large.try_sub(&one).unwrap().degree(), large.degree());
    let expected = Err(PolynomialError::CoefficientLimitExceeded {
        coefficients: max_coefficients + 1,
        max_coefficients,
    });
    assert_eq!(large.try_mul(&x), expected);
    assert_eq!(&large * &x, expected);
    assert!(large.try_sub(&large).unwrap().coefficients().is_empty());
    let padded_zero = Polynomial::try_new(vec![F::zero(); max_coefficients]).unwrap();
    assert!(large
        .try_mul(&padded_zero)
        .unwrap()
        .coefficients()
        .is_empty());
    let mut padded_one = vec![F::zero(); max_coefficients];
    padded_one[0] = F::one();
    assert_eq!(
        large
            .try_mul(&Polynomial::try_new(padded_one).unwrap())
            .unwrap(),
        large
    );
}

#[test]
fn output_limits_use_field_size_and_effective_degree() {
    check_arithmetic_limits::<Gf256>();
    check_arithmetic_limits::<Gf9>();
}

#[test]
fn division_and_gcd_have_known_monic_results() {
    let dividend = p5(&[1, 2, 3, 0]);
    let divisor = p5(&[4, 1, 0]);
    let (quotient, remainder) = dividend.try_div_rem(&divisor).unwrap();
    assert_eq!(quotient.coefficients(), p5(&[0, 3]).coefficients());
    assert_eq!(remainder.coefficients(), p5(&[1]).coefficients());
    let a = p5(&[2, 3, 1]);
    let b = p5(&[1, 3, 2]);
    assert_eq!(
        a.try_gcd(&b).unwrap().coefficients(),
        p5(&[1, 1]).coefficients()
    );
    assert_eq!(a.try_div(&p5(&[2, 1])).unwrap(), p5(&[1, 1]));
    assert!(a.try_rem(&p5(&[2, 1])).unwrap().coefficients().is_empty());
}

fn check_division<F: FieldElement>(a: F, b: F) {
    let dividend = polynomial(&[a, b, F::one(), a, F::zero()]);
    let divisor = polynomial(&[b, a, F::zero()]);
    if divisor.is_zero() {
        assert_eq!(
            dividend.try_div_rem(&divisor),
            Err(PolynomialError::DivisionByZero)
        );
        return;
    }
    let (quotient, remainder) = dividend.try_div_rem(&divisor).unwrap();
    assert_eq!(
        divisor
            .try_mul(&quotient)
            .unwrap()
            .try_add(&remainder)
            .unwrap(),
        dividend
    );
    assert!(remainder.is_zero() || remainder.degree() < divisor.degree());
    assert_eq!((&dividend / &divisor).unwrap(), quotient);
    assert_eq!((dividend.clone() / divisor.clone()).unwrap(), quotient);
    assert_eq!((&dividend % &divisor).unwrap(), remainder);
    assert_eq!((dividend.clone() % divisor.clone()).unwrap(), remainder);
    for result in [&quotient, &remainder] {
        assert!(result.coefficients().last().map_or(true, |v| !v.is_zero()));
    }
    let gcd = dividend.try_gcd(&divisor).unwrap();
    assert_eq!(gcd.coefficients().last(), Some(&F::one()));
    assert!(dividend.try_rem(&gcd).unwrap().is_zero());
    assert!(divisor.try_rem(&gcd).unwrap().is_zero());
    assert_eq!(gcd, divisor.try_gcd(&dividend).unwrap());
    let zero = polynomial(&[F::zero(); 2]);
    assert_eq!(gcd.try_gcd(&zero).unwrap(), gcd);
    assert_eq!(zero.try_gcd(&gcd).unwrap(), gcd);
    assert!(zero.try_gcd(&zero).unwrap().coefficients().is_empty());
    assert_eq!(dividend.coefficients(), &[a, b, F::one(), a, F::zero()]);
    assert_eq!(divisor.coefficients(), &[b, a, F::zero()]);
}

#[test]
fn euclidean_division_and_gcd_laws_across_fields() {
    for a in 0..9 {
        for b in 0..9 {
            check_division(Gf9::new(a), Gf9::new(b));
            check_division(Gf256::new(a as u16), Gf256::new(b as u16));
        }
    }
    for a in 0..5 {
        for b in 0..5 {
            check_division(F5::new(a), F5::new(b));
        }
    }
}

#[test]
fn zero_dividend_short_dividend_and_all_zero_divisors() {
    let zero = p5(&[]);
    let a = p5(&[2, 1, 0]);
    assert_eq!(zero.try_div_rem(&a).unwrap(), (zero.clone(), zero.clone()));
    assert_eq!(p5(&[2]).try_div_rem(&a).unwrap(), (zero.clone(), p5(&[2])));
    assert_eq!(a.try_div_rem(&a).unwrap(), (p5(&[1]), zero.clone()));
    assert_eq!(p5(&[3]).try_div_rem(&p5(&[2])).unwrap(), (p5(&[4]), zero));
    assert_eq!(a.try_gcd(&p5(&[3])).unwrap(), p5(&[1]));
    for divisor in [p5(&[]), p5(&[0]), p5(&[0, 0])] {
        for dividend in [p5(&[]), p5(&[0, 0]), a.clone()] {
            assert_eq!(
                dividend.try_div_rem(&divisor),
                Err(PolynomialError::DivisionByZero)
            );
            assert_eq!(&dividend / &divisor, Err(PolynomialError::DivisionByZero));
            assert_eq!(&dividend % &divisor, Err(PolynomialError::DivisionByZero));
        }
    }
}

#[test]
fn division_and_gcd_support_degrees_above_the_low_level_gfpm_limit() {
    let max = MAX_POLYNOMIAL_BYTES / size_of::<F5>();
    let mut coefficients = vec![F5::zero(); max];
    coefficients[max - 1] = F5::new(3);
    let large = Polynomial::try_new(coefficients).unwrap();
    let x = p5(&[0, 1]);
    let (quotient, remainder) = large.try_div_rem(&x).unwrap();
    assert_eq!(quotient.degree(), Some(max - 2));
    assert_eq!(quotient.coefficients().last(), Some(&F5::new(3)));
    assert!(remainder.coefficients().is_empty());
    assert_eq!(large.try_gcd(&x).unwrap(), x);
    assert_eq!(large.try_div_rem(&large).unwrap(), (p5(&[1]), p5(&[])));
    let monic = large.try_gcd(&p5(&[])).unwrap();
    assert_eq!(monic.degree(), large.degree());
    assert_eq!(monic.coefficients().last(), Some(&F5::one()));
}

#[test]
fn prime_field_results_agree_with_the_existing_low_level_implementation() {
    let samples = [
        vec![],
        vec![0, 0],
        vec![3],
        vec![4, 2, 0],
        vec![1, 3, 2],
        vec![2, 3, 1],
        vec![4, 0, 1, 2, 3],
    ];
    let convert = |values: &[u32]| p5(&values.iter().map(|&x| u64::from(x)).collect::<Vec<_>>());
    for a in &samples {
        for b in &samples {
            let left = convert(a);
            let right = convert(b);
            let mut expected = vec![0; 16];
            gfpm::poly::poly_add(a, b, 5, &mut expected);
            assert_eq!(left.try_add(&right).unwrap(), convert(&expected));
            gfpm::poly::poly_sub(a, b, 5, &mut expected);
            assert_eq!(left.try_sub(&right).unwrap(), convert(&expected));
            gfpm::poly::poly_mul(a, b, 5, &mut expected);
            assert_eq!(left.try_mul(&right).unwrap(), convert(&expected));
            gfpm::poly::poly_gcd(a, b, 5, &mut expected);
            assert_eq!(left.try_gcd(&right).unwrap(), convert(&expected));
            if !right.is_zero() {
                let mut remainder = vec![0; 16];
                gfpm::poly::poly_divmod(a, b, 5, &mut expected, &mut remainder);
                assert_eq!(
                    left.try_div_rem(&right).unwrap(),
                    (convert(&expected), convert(&remainder))
                );
            }
        }
    }
}
