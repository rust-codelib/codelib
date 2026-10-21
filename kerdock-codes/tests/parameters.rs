use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

#[test]
fn supported_parameters_match_classical_kerdock_codes() {
    for (size, m, k, n, d, t) in [
        (KerdockSize::Bits16, 4, 8, 16, 6, 2),
        (KerdockSize::Bits64, 6, 12, 64, 28, 13),
        (KerdockSize::Bits256, 8, 16, 256, 120, 59),
        (KerdockSize::Bits1024, 10, 20, 1024, 496, 247),
    ] {
        let code = Kerdock::new(size);
        assert_eq!(code.exponent(), m);
        assert_eq!(code.message_len(), k);
        assert_eq!(code.codeword_len(), n);
        assert_eq!(code.minimum_distance(), d);
        assert_eq!(code.correction_radius(), t);
    }
}

#[test]
fn supported_bit_lengths_convert_to_explicit_sizes() {
    for (bits, size) in [
        (16, KerdockSize::Bits16),
        (64, KerdockSize::Bits64),
        (256, KerdockSize::Bits256),
        (1024, KerdockSize::Bits1024),
    ] {
        assert_eq!(KerdockSize::try_from(bits), Ok(size));
    }
}

#[test]
fn unsupported_bit_lengths_preserve_the_rejected_value() {
    for bits in [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        15,
        17,
        31,
        32,
        33,
        63,
        65,
        127,
        128,
        129,
        255,
        257,
        511,
        512,
        513,
        1023,
        1025,
        2048,
        usize::MAX,
    ] {
        assert_eq!(
            KerdockSize::try_from(bits),
            Err(KerdockError::UnsupportedBlockLength { bits })
        );
    }
}

#[test]
fn unsupported_bit_length_diagnostic_reports_the_rejected_length() {
    let error = KerdockSize::try_from(1023).unwrap_err();
    let standard_error: &dyn std::error::Error = &error;
    assert!(standard_error.to_string().contains("1023"));
    assert!(standard_error.source().is_none());
}
