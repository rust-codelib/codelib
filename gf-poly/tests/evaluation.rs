use gf2m::Gf256;
use gf_poly::{FieldElement, Polynomial};
use gfpm::{Gf5, Gf9};

fn check_evaluation<F: FieldElement>(elements: &[F]) {
    for &a in elements {
        for &b in elements {
            let coefficients = [a, b, -F::one(), a, F::zero()];
            let polynomial = Polynomial::try_from_slice(&coefficients).unwrap();
            for &x in elements {
                let direct = a + b * x - x * x + a * x * x * x;
                assert_eq!(polynomial.evaluate(x), direct);
            }
            assert_eq!(polynomial.evaluate(F::zero()), a);
            assert_eq!(polynomial.coefficients(), &coefficients);
        }
    }
    let empty = Polynomial::<F>::try_new(vec![]).unwrap();
    let zero = Polynomial::try_new(vec![F::zero(); 3]).unwrap();
    let constant = Polynomial::try_new(vec![-F::one(), F::zero()]).unwrap();
    for &x in elements {
        assert_eq!(empty.evaluate(x), F::zero());
        assert_eq!(zero.evaluate(x), F::zero());
        assert_eq!(constant.evaluate(x), -F::one());
    }
}

#[test]
fn horner_matches_explicit_polynomial_values_across_fields() {
    check_evaluation(&(0..5).map(Gf5::new).collect::<Vec<_>>());
    check_evaluation(&(0..9).map(Gf9::new).collect::<Vec<_>>());
    check_evaluation(&(0..16).map(Gf256::new).collect::<Vec<_>>());
}

#[test]
fn evaluation_respects_sum_and_product() {
    let a = Polynomial::try_new(vec![Gf9::new(3), -Gf9::one(), Gf9::new(8)]).unwrap();
    let b = Polynomial::try_new(vec![Gf9::new(5), Gf9::new(6)]).unwrap();
    for x in (0..9).map(Gf9::new) {
        assert_eq!(
            a.try_add(&b).unwrap().evaluate(x),
            a.evaluate(x) + b.evaluate(x)
        );
        assert_eq!(
            a.try_sub(&b).unwrap().evaluate(x),
            a.evaluate(x) - b.evaluate(x)
        );
        assert_eq!(
            a.try_mul(&b).unwrap().evaluate(x),
            a.evaluate(x) * b.evaluate(x)
        );
    }
}
