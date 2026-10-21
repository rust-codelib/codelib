use gf2m::{Gf16, Gf256};
use gf_poly::{Polynomial, PolynomialError};
use gfpm::{Gf5, Gf9};

#[test]
fn interpolation_is_explicitly_unimplemented_for_all_inputs() {
    let error = Err(PolynomialError::NotImplemented {
        operation: "интерполяция многочлена",
    });
    for points in [
        vec![],
        vec![(Gf9::zero(), Gf9::one())],
        vec![(Gf9::one(), Gf9::one()), (Gf9::one(), Gf9::zero())],
    ] {
        assert_eq!(Polynomial::<Gf9>::try_interpolate(&points), error);
    }
    assert!(matches!(
        Polynomial::<Gf256>::try_interpolate(&[]),
        Err(PolynomialError::NotImplemented { .. })
    ));
}

#[test]
fn homomorphism_has_a_typed_target_and_does_not_call_the_mapping() {
    for source in [vec![], vec![Gf16::zero()], vec![Gf16::one()]] {
        let source = Polynomial::try_new(source).unwrap();
        let image = Polynomial::try_new(vec![Gf256::zero(), Gf256::one()]).unwrap();
        let result: Result<Polynomial<Gf256>, _> = source.try_homomorphism(
            |_| panic!("заглушка не должна вызывать отображление коэффициентов"),
            &image,
        );
        assert_eq!(
            result,
            Err(PolynomialError::NotImplemented {
                operation: "гомоморфизм многочленов"
            })
        );
        assert_eq!(image.coefficients(), &[Gf256::zero(), Gf256::one()]);
    }
    let identity = Polynomial::try_new(vec![Gf5::one()]).unwrap();
    assert!(matches!(
        identity.try_homomorphism(|coefficient| coefficient, &identity),
        Err(PolynomialError::NotImplemented { .. })
    ));
}
