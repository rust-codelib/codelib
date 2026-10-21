use core::mem::size_of;

use gf2m::Gf256;
use gf_poly::{FieldElement, Polynomial, PolynomialError, MAX_MESSAGE_BYTES, MAX_POLYNOMIAL_BYTES};
use gfpm::{Gf, Gf25, Gf9};

fn check_round_trip<F: FieldElement>() {
    for message in [vec![], vec![0], vec![0, 0], vec![0, 0x83, 0xff, 0x80, 0]] {
        let original = message.clone();
        let polynomial = Polynomial::<F>::try_from_message(&message).unwrap();
        assert_eq!(polynomial.coefficients().len(), message.len() * 8);
        assert!(polynomial
            .coefficients()
            .iter()
            .all(|&c| c == F::zero() || c == F::one()));
        assert_eq!(polynomial.try_to_message().unwrap(), message);
        assert_eq!(message, original);
    }

    let message: Vec<_> = (0..=u8::MAX).collect();
    let polynomial = Polynomial::<F>::try_from_message(&message).unwrap();
    assert_eq!(polynomial.try_to_message().unwrap(), message);
}

#[test]
fn arbitrary_bytes_round_trip_over_binary_prime_and_extension_fields() {
    check_round_trip::<Gf256>();
    check_round_trip::<Gf<2, 1, 1>>();
    check_round_trip::<gfpm::Gf5>();
    check_round_trip::<Gf9>();
    check_round_trip::<Gf25>();
}

#[test]
fn byte_order_and_bit_order_match_an_independently_written_vector() {
    let polynomial = Polynomial::<Gf9>::try_from_message(&[0x83, 0]).unwrap();
    let expected = [1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0].map(Gf9::new);
    assert_eq!(polynomial.coefficients(), &expected);
    assert_eq!(polynomial.degree(), Some(7));

    let external = Polynomial::try_new([0, 0, 1, 0, 0, 0, 0, 1].map(Gf256::new).to_vec()).unwrap();
    assert_eq!(external.try_to_message().unwrap(), [0x84]);
}

#[test]
fn message_bound_is_enforced_before_coefficient_memory_bound() {
    assert_eq!(MAX_MESSAGE_BYTES, 4096);
    for length in [MAX_MESSAGE_BYTES - 1, MAX_MESSAGE_BYTES] {
        let message: Vec<_> = (0..length).map(|i| (i % 256) as u8).collect();
        let polynomial = Polynomial::<Gf256>::try_from_message(&message).unwrap();
        assert_eq!(polynomial.try_to_message().unwrap(), message);
        let binary = Polynomial::<Gf<2, 1, 1>>::try_from_message(&message).unwrap();
        assert_eq!(binary.try_to_message().unwrap(), message);
    }

    let oversized = vec![0; MAX_MESSAGE_BYTES + 1];
    let expected = Err(PolynomialError::MessageTooLong {
        bytes: oversized.len(),
    });
    assert_eq!(Polynomial::<Gf256>::try_from_message(&oversized), expected);
    assert_eq!(
        Polynomial::<Gf9>::try_from_message(&oversized),
        Err(PolynomialError::MessageTooLong {
            bytes: oversized.len()
        })
    );
}

#[test]
fn reverse_conversion_enforces_message_limit_on_valid_polynomials() {
    let length = (MAX_MESSAGE_BYTES + 1) * 8;
    let polynomial = Polynomial::try_new(vec![Gf256::zero(); length]).unwrap();
    assert!(length * size_of::<Gf256>() <= MAX_POLYNOMIAL_BYTES);
    assert_eq!(
        polynomial.try_to_message(),
        Err(PolynomialError::MessageTooLong {
            bytes: MAX_MESSAGE_BYTES + 1
        })
    );
}

#[test]
fn large_field_elements_can_make_the_memory_bound_stricter_than_message_bound() {
    type WideField = Gf<2, 64, 0x1B>;
    for length in [63, 64] {
        let message = vec![0x83; length];
        let polynomial = Polynomial::<WideField>::try_from_message(&message).unwrap();
        assert_eq!(polynomial.try_to_message().unwrap(), message);
    }
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<WideField>();
    let length = max_coefficients / 8 + 1;
    assert!(length < MAX_MESSAGE_BYTES);
    assert_eq!(
        Polynomial::<WideField>::try_from_message(&vec![0; length]),
        Err(PolynomialError::CoefficientLimitExceeded {
            coefficients: length * 8,
            max_coefficients
        })
    );
}

#[test]
fn reverse_conversion_rejects_incomplete_bytes_and_non_binary_coefficients() {
    for coefficients in [1, 7, 9] {
        let polynomial = Polynomial::try_new(vec![Gf256::zero(); coefficients]).unwrap();
        assert_eq!(
            polynomial.try_to_message(),
            Err(PolynomialError::InvalidMessageCoefficientCount { coefficients })
        );
    }
    let mut coefficients = vec![Gf9::zero(); 16];
    coefficients[6] = Gf9::new(2);
    coefficients[9] = Gf9::new(3);
    let polynomial = Polynomial::try_new(coefficients).unwrap();
    assert_eq!(
        polynomial.try_to_message(),
        Err(PolynomialError::InvalidMessageCoefficient { index: 6 })
    );

    let non_bit = Polynomial::try_new(vec![Gf256::new(2); 8]).unwrap();
    assert_eq!(
        non_bit.try_to_message(),
        Err(PolynomialError::InvalidMessageCoefficient { index: 0 })
    );
}

#[test]
fn text_round_trip_preserves_message_length_even_for_equal_zero_polynomials() {
    let one_byte = Polynomial::<Gf9>::try_from_message(&[0]).unwrap();
    let two_bytes = Polynomial::<Gf9>::try_from_message(&[0, 0]).unwrap();
    assert_eq!(one_byte, two_bytes);
    for original in [one_byte, two_bytes] {
        let restored: Polynomial<Gf9> = original.to_string().parse().unwrap();
        assert_eq!(restored.coefficients(), original.coefficients());
        assert_eq!(
            restored.try_to_message().unwrap(),
            original.try_to_message().unwrap()
        );
    }
}
