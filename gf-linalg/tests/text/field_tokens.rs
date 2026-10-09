use gf2m::{Gf16, Gf65536};
use gf_linalg::{LinalgError, Matrix, Vector};
use gfpm::{Gf125, Gf25, Gf9};

#[test]
fn gf16_text_uses_two_hex_digits_and_round_trips_vectors_and_matrices() {
    let vector = Vector::<Gf16>::new(vec![Gf16::zero(), Gf16::new(0x0f), Gf16::new(0x03)]);
    assert_eq!(vector.to_string(), "vector 3 0x00 0x0f 0x03");
    assert_eq!(
        vector
            .to_string()
            .parse::<Vector<Gf16>>()
            .unwrap()
            .as_slice(),
        vector.as_slice()
    );

    let matrix = Matrix::<Gf16>::try_new(1, 3, vector.as_slice().to_vec()).unwrap();
    let text = "matrix 1 3\n0x00 0x0f 0x03";
    assert_eq!(matrix.to_string(), text);
    assert_eq!(text.parse::<Matrix<Gf16>>(), Ok(matrix));
}

#[test]
fn gf65536_text_uses_four_hex_digits_and_round_trips_vectors_and_matrices() {
    let vector = Vector::<Gf65536>::new(vec![
        Gf65536::zero(),
        Gf65536::new(0x00af),
        Gf65536::new(0xffff),
    ]);
    assert_eq!(vector.to_string(), "vector 3 0x0000 0x00af 0xffff");
    assert_eq!(
        vector
            .to_string()
            .parse::<Vector<Gf65536>>()
            .unwrap()
            .as_slice(),
        vector.as_slice()
    );

    let matrix = Matrix::<Gf65536>::try_new(1, 3, vector.as_slice().to_vec()).unwrap();
    let text = "matrix 1 3\n0x0000 0x00af 0xffff";
    assert_eq!(matrix.to_string(), text);
    assert_eq!(text.parse::<Matrix<Gf65536>>(), Ok(matrix));
}

#[test]
fn gf2m_parser_accepts_uppercase_digits_and_rejects_wrong_width_prefix_and_range() {
    assert_eq!(
        "vector 1 0x0F".parse::<Vector<Gf16>>().unwrap().as_slice(),
        &[Gf16::new(0x0f)]
    );

    for token in ["0x0", "0x00f", "0X0f", "0xgg", "0xＡ１", "0x10"] {
        assert_eq!(
            format!("vector 1 {token}").parse::<Vector<Gf16>>(),
            Err(LinalgError::InvalidTextElement { index: 0 }),
            "accepted invalid GF(16) token {token:?}"
        );
    }

    assert_eq!(
        "vector 1 0x10000".parse::<Vector<Gf65536>>(),
        Err(LinalgError::InvalidTextElement { index: 0 })
    );
}

#[test]
fn gf9_decimal_tokens_round_trip_every_element_in_both_containers() {
    let values = (0..9).map(Gf9::new).collect::<Vec<_>>();
    let vector = Vector::<Gf9>::new(values.clone());

    assert_eq!(vector.to_string(), "vector 9 0 1 2 3 4 5 6 7 8");
    assert_eq!(vector.to_string().parse::<Vector<Gf9>>().unwrap(), vector);

    let matrix = Matrix::<Gf9>::try_new(3, 3, values).unwrap();
    let text = "matrix 3 3\n0 1 2\n3 4 5\n6 7 8";
    assert_eq!(matrix.to_string(), text);
    assert_eq!(text.parse::<Matrix<Gf9>>(), Ok(matrix));
}

#[test]
fn gfpm_decimal_tokens_accept_leading_zeroes_and_ascii_whitespace() {
    let input = "\x0bvector\x0c0003\t0000\r\n0003 0008\x0c";
    let parsed = input.parse::<Vector<Gf9>>().unwrap();

    assert_eq!(parsed.as_slice(), &[Gf9::zero(), Gf9::new(3), Gf9::new(8)]);
    assert_eq!(parsed.to_string(), "vector 3 0 3 8");

    let gf25 = Vector::<Gf25>::new(vec![Gf25::zero(), Gf25::new(24)]);
    let gf125 = Vector::<Gf125>::new(vec![Gf125::new(64), Gf125::new(124)]);
    assert_eq!(gf25.to_string(), "vector 2 0 24");
    assert_eq!(gf125.to_string(), "vector 2 64 124");
    assert_eq!(gf25.to_string().parse::<Vector<Gf25>>().unwrap(), gf25);
    assert_eq!(gf125.to_string().parse::<Vector<Gf125>>().unwrap(), gf125);
}

#[test]
fn gfpm_parser_rejects_non_decimal_or_out_of_range_elements_at_flat_index() {
    for (token, index) in [
        ("9", 0),
        ("25", 1),
        ("-1", 0),
        ("+1", 1),
        ("0x01", 2),
        ("١", 3),
        ("18446744073709551616", 4),
        ("340282366920938463463374607431768211456", 5),
    ] {
        let mut elements = ["0"; 6];
        elements[index] = token;
        let input = format!("vector 6 {}", elements.join(" "));
        assert_eq!(
            input.parse::<Vector<Gf9>>(),
            Err(LinalgError::InvalidTextElement { index }),
            "accepted invalid decimal token {token:?}"
        );
    }

    assert_eq!(
        "vector 2 0 25".parse::<Vector<Gf25>>(),
        Err(LinalgError::InvalidTextElement { index: 1 })
    );
    assert_eq!(
        "matrix 2 2 0 1 2 25".parse::<Matrix<Gf9>>(),
        Err(LinalgError::InvalidTextElement { index: 3 })
    );
}

#[test]
fn gfpm_order_two_to_64_accepts_u64_max_but_rejects_order() {
    type WideField = gfpm::Gf<2, 64, 0x1B>;

    let vector = Vector::<WideField>::new(vec![WideField::new(u64::MAX), WideField::zero()]);
    assert_eq!(vector.to_string(), "vector 2 18446744073709551615 0");
    assert_eq!(
        vector.to_string().parse::<Vector<WideField>>().unwrap(),
        vector
    );

    let matrix =
        Matrix::<WideField>::try_new(1, 2, vec![WideField::new(u64::MAX), WideField::zero()])
            .unwrap();
    let matrix_text = "matrix 1 2\n18446744073709551615 0";
    assert_eq!(matrix.to_string(), matrix_text);
    assert_eq!(matrix_text.parse::<Matrix<WideField>>(), Ok(matrix));

    assert_eq!(
        "vector 1 18446744073709551616".parse::<Vector<WideField>>(),
        Err(LinalgError::InvalidTextElement { index: 0 })
    );
}
