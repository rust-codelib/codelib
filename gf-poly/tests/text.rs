use gf2m::{Gf16, Gf256, Gf65536};
use gf_poly::{FieldText, Polynomial, PolynomialError, MAX_POLYNOMIAL_BYTES};
use gfpm::{Gf, Gf25, Gf5, Gf9};

fn check_round_trip<F: FieldText>(values: Vec<F>) {
    let polynomial = Polynomial::try_new(values.clone()).unwrap();
    let text = polynomial.to_string();
    assert!(!text.ends_with('\n'));
    let restored: Polynomial<F> = text.parse().unwrap();
    // Eq игнорирует хвостовые нули, поэтому отдельно проверяем массив целиком.
    assert_eq!(restored.coefficients(), values);
    assert_eq!(restored, polynomial);
    assert_eq!(restored.to_string(), text);
}

#[test]
fn text_round_trip_preserves_empty_arrays_trailing_zeros_and_field_tokens() {
    check_round_trip::<Gf256>(vec![]);
    check_round_trip(vec![Gf256::zero(); 4]);
    check_round_trip(vec![Gf16::new(15), Gf16::zero()]);
    check_round_trip(vec![Gf256::new(255), Gf256::one(), Gf256::zero()]);
    check_round_trip(vec![Gf65536::new(0xffff), Gf65536::zero()]);
    check_round_trip((0..9).map(Gf9::new).collect());
    check_round_trip(vec![Gf25::new(24), Gf25::zero(), Gf25::zero()]);

    assert_eq!(
        Polynomial::<Gf256>::try_new(vec![]).unwrap().to_string(),
        "polynomial 0"
    );
    assert_eq!(
        Polynomial::try_new(vec![Gf16::new(15)])
            .unwrap()
            .to_string(),
        "polynomial 1 0x0f"
    );
    assert_eq!(
        Polynomial::try_new(vec![Gf65536::new(255)])
            .unwrap()
            .to_string(),
        "polynomial 1 0x00ff"
    );
    assert_eq!(
        Polynomial::try_new(vec![Gf9::new(8), Gf9::zero()])
            .unwrap()
            .to_string(),
        "polynomial 2 8 0"
    );
}

#[test]
fn signs_apply_field_negation_instead_of_integer_modulo_field_order() {
    let odd: Polynomial<Gf9> = "polynomial 4 -1 +3 -3 -0".parse().unwrap();
    assert_eq!(
        odd.coefficients(),
        &[Gf9::new(2), Gf9::new(3), Gf9::new(6), Gf9::zero()]
    );
    assert_eq!(odd.to_string(), "polynomial 4 2 3 6 0");

    let prime: Polynomial<Gf5> = "polynomial 3 -1 -2 +4".parse().unwrap();
    assert_eq!(
        prime.coefficients(),
        &[Gf5::new(4), Gf5::new(3), Gf5::new(4)]
    );

    let binary: Polynomial<Gf256> = "polynomial 3 -0xff +0x01 -0x00".parse().unwrap();
    assert_eq!(
        binary.coefficients(),
        &[Gf256::new(255), Gf256::one(), Gf256::zero()]
    );
}

#[test]
fn parser_accepts_ascii_whitespace_and_canonicalizes_allowed_variants() {
    let binary: Polynomial<Gf16> = "\x0bpolynomial\x0c0003\t+0x0F\r\n-0x01 0x00\x0c"
        .parse()
        .unwrap();
    assert_eq!(binary.to_string(), "polynomial 3 0x0f 0x01 0x00");

    let odd: Polynomial<Gf9> = "polynomial 0003 0001 -0003 +0000".parse().unwrap();
    assert_eq!(odd.to_string(), "polynomial 3 1 6 0");
}

#[test]
fn parser_checks_headers_lengths_limits_and_token_counts_before_values() {
    for input in ["", "Polynomial 0", "vector 0", "polynomial\u{a0}0"] {
        assert_eq!(
            input.parse::<Polynomial>(),
            Err(PolynomialError::InvalidTextHeader)
        );
    }
    for length in [
        "",
        "-1",
        "+1",
        "1.0",
        "١",
        "0x01",
        "340282366920938463463374607431768211456",
    ] {
        assert_eq!(
            format!("polynomial {length}").parse::<Polynomial>(),
            Err(PolynomialError::InvalidTextLength)
        );
    }
    assert_eq!(
        "polynomial 2 bad".parse::<Polynomial>(),
        Err(PolynomialError::TextCoefficientCountMismatch {
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        "polynomial 0 bad".parse::<Polynomial>(),
        Err(PolynomialError::TextCoefficientCountMismatch {
            expected: 0,
            actual: 1
        })
    );
    assert_eq!(
        "polynomial 1 0x00 0x01".parse::<Polynomial>(),
        Err(PolynomialError::TextCoefficientCountMismatch {
            expected: 1,
            actual: 2
        })
    );

    let max_coefficients = MAX_POLYNOMIAL_BYTES / core::mem::size_of::<Gf256>();
    for length in [max_coefficients + 1, usize::MAX] {
        assert_eq!(
            format!("polynomial {length} bad").parse::<Polynomial>(),
            Err(PolynomialError::CoefficientLimitExceeded {
                coefficients: length,
                max_coefficients
            })
        );
    }
}

#[test]
fn malformed_or_out_of_range_coefficients_report_their_index() {
    for token in [
        "-",
        "+",
        "--1",
        "+-1",
        "-+1",
        "++1",
        "−1",
        "9",
        "-9",
        "+9",
        "0x01",
        "١",
        "18446744073709551616",
    ] {
        assert_eq!(
            format!("polynomial 3 1 {token} 0").parse::<Polynomial<Gf9>>(),
            Err(PolynomialError::InvalidTextCoefficient { index: 1 }),
            "accepted token {token:?}"
        );
    }
    for token in [
        "0x0", "0x000", "0X0f", "0xgg", "0x10", "-0x10", "+0x10", "0xＡ１",
    ] {
        assert_eq!(
            format!("polynomial 2 0x00 {token}").parse::<Polynomial<Gf16>>(),
            Err(PolynomialError::InvalidTextCoefficient { index: 1 }),
            "accepted token {token:?}"
        );
    }
    assert_eq!(
        "polynomial 1 - 1".parse::<Polynomial<Gf9>>(),
        Err(PolynomialError::TextCoefficientCountMismatch {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        "polynomial 1 1\u{a0}".parse::<Polynomial<Gf9>>(),
        Err(PolynomialError::InvalidTextCoefficient { index: 0 })
    );
}

#[test]
fn maximum_packed_value_and_its_negation_do_not_require_signed_integers() {
    type WideField = Gf<2, 64, 0x1B>;
    let polynomial: Polynomial<WideField> =
        "polynomial 2 18446744073709551615 -18446744073709551615"
            .parse()
            .unwrap();
    assert_eq!(polynomial.coefficients(), &[WideField::new(u64::MAX); 2]);
    check_round_trip(polynomial.into_coefficients().into_vec());
    assert_eq!(
        "polynomial 1 -18446744073709551616".parse::<Polynomial<WideField>>(),
        Err(PolynomialError::InvalidTextCoefficient { index: 0 })
    );
}

#[test]
fn text_can_exceed_message_limit_and_reach_the_coefficient_memory_limit() {
    let length = MAX_POLYNOMIAL_BYTES / core::mem::size_of::<Gf256>();
    let text = format!("polynomial {length} {}", "0x00 ".repeat(length));
    assert!(text.len() > 4096);
    let polynomial: Polynomial = text.parse().unwrap();
    assert_eq!(polynomial.coefficients().len(), length);
    assert!(polynomial.is_zero());
    let restored: Polynomial = polynomial.to_string().parse().unwrap();
    assert_eq!(restored.coefficients(), polynomial.coefficients());

    type WideField = Gf<2, 64, 0x1B>;
    let max_coefficients = MAX_POLYNOMIAL_BYTES / core::mem::size_of::<WideField>();
    let oversized = max_coefficients + 1;
    assert_eq!(
        format!("polynomial {oversized}").parse::<Polynomial<WideField>>(),
        Err(PolynomialError::CoefficientLimitExceeded {
            coefficients: oversized,
            max_coefficients
        })
    );
}
