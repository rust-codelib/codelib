use fountain_codes::*;
fn desc() -> BlockDescriptor {
    BlockDescriptor::try_new(Profile::Gf65536, [3; 16], 11, 4, 5, 7).unwrap()
}
#[test]
fn descriptor_strict_lengths_and_error_precedence() {
    let good = desc().to_wire().unwrap();
    for len in 0..48 {
        assert!(matches!(
            parse_descriptor(&good[..len]),
            Err(FountainError::WireLength { .. })
        ));
    }
    let mut b = good.clone();
    b.push(0);
    assert!(parse_descriptor(&b).is_err());
    let cases: [(usize, u8, FountainError); 9] = [
        (0, 0, FountainError::WireMagic),
        (4, 2, FountainError::WireVersion { actual: 2 }),
        (6, 3, FountainError::WireProfile { actual: 3 }),
        (44, 1, FountainError::WireReserved),
        (
            40,
            0,
            FountainError::WirePolynomial {
                expected: 0x1002d,
                actual: 0x10000,
            },
        ),
        (
            26,
            0,
            FountainError::InvalidParameter {
                parameter: "symbol_bytes",
                actual: 0,
            },
        ),
        (
            26,
            3,
            FountainError::InvalidParameter {
                parameter: "symbol_bytes",
                actual: 3,
            },
        ),
        (
            28,
            4,
            FountainError::InvalidParameter {
                parameter: "source_symbols",
                actual: 4,
            },
        ),
        (
            30,
            0,
            FountainError::InvalidParameter {
                parameter: "parity_draws",
                actual: 0,
            },
        ),
    ];
    for (i, v, expected) in cases {
        let mut b = good.clone();
        b[i] = v;
        assert_eq!(parse_descriptor(&b), Err(expected));
    }
    let mut b = good.clone();
    b[24..26].copy_from_slice(&4097u16.to_le_bytes());
    b[26] = 0;
    assert!(matches!(
        parse_descriptor(&b),
        Err(FountainError::MessageSize(_))
    ));
    // Multiple errors: reserved beats polynomial; K beats d.
    let mut b = good.clone();
    b[44] = 1;
    b[40] = 0;
    assert_eq!(parse_descriptor(&b), Err(FountainError::WireReserved));
    let mut b = good.clone();
    b[28] = 4;
    b[30] = 0;
    assert_eq!(
        parse_descriptor(&b),
        Err(FountainError::InvalidParameter {
            parameter: "source_symbols",
            actual: 4
        })
    );
    // Wire accepts a representable d above the default local policy.
    let mut b = good;
    b[30..32].copy_from_slice(&65535u16.to_le_bytes());
    let d = parse_descriptor(&b).unwrap();
    assert!(RepairableDecoder::try_new(d.clone(), ResourceLimits::default()).is_err());
    let generous = ResourceLimits::try_new(65535, 16384, 16384, 1048576, 67108864).unwrap();
    RepairableDecoder::try_new(d, generous).unwrap();
}
#[test]
fn packet_checks_before_payload_copy_and_no_panics_on_garbage() {
    let symbol = EncodedSymbol::try_new(desc(), u64::MAX, vec![0; 4]).unwrap();
    let good = symbol.to_wire().unwrap();
    for len in 0..good.len() {
        assert!(parse_symbol(&good[..len]).is_err());
    }
    let mut b = good.clone();
    b.push(1);
    assert!(parse_symbol(&b).is_err());
    for i in [0, 4, 8, 10, 44, 48, 60, 62] {
        let mut b = good.clone();
        b[i] ^= 0xff;
        assert!(parse_symbol(&b).is_err());
    }
    let empty = BlockDescriptor::try_new(Profile::Gf256, [0; 16], 0, 1, 0, 0).unwrap();
    let mut b = good.clone();
    b[4..52].copy_from_slice(&empty.to_wire().unwrap());
    b[62] = 1;
    assert_eq!(parse_symbol(&b), Err(FountainError::WireReserved));
    b[62] = 0;
    assert_eq!(parse_symbol(&b), Err(FountainError::EmptyBlock));
    // Finite deterministic malformed input sweep.
    for len in 0..4200 {
        let garbage = vec![(len % 251) as u8; len];
        assert!(parse_symbol(&garbage).is_err());
    }
}
#[test]
fn coefficient_adapter_preserves_exact_length_and_zeros() {
    for bytes in [vec![], vec![0], vec![128, 255, 0, 0], vec![0; 4096]] {
        let coeff = message_to_coefficients(&bytes).unwrap();
        assert_eq!(coeff.len(), bytes.len());
        assert_eq!(coefficients_to_message(&coeff).unwrap(), bytes);
    }
    assert!(message_to_coefficients(&[0; 4097]).is_err());
    assert!(coefficients_to_message(&[gf2m::Gf256::zero(); 4097]).is_err());
}
