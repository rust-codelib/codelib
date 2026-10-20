use kerdock_codes::{Kerdock, KerdockError};

#[test]
fn supported_parameters_match_classical_kerdock_codes() {
    for (m, k, n, d, t) in [
        (4, 8, 16, 6, 2),
        (6, 12, 64, 28, 13),
        (8, 16, 256, 120, 59),
        (10, 20, 1024, 496, 247),
    ] {
        let code = Kerdock::try_new(m).unwrap();
        assert_eq!(code.exponent(), m);
        assert_eq!(code.message_len(), k);
        assert_eq!(code.codeword_len(), n);
        assert_eq!(code.minimum_distance(), d);
        assert_eq!(code.correction_radius(), t);
    }
}

#[test]
fn unsupported_exponents_are_rejected_before_size_arithmetic() {
    for m in [0, 1, 2, 3, 5, 7, 9, 11, 12, 31, 32, u32::MAX] {
        assert_eq!(
            Kerdock::try_new(m).err(),
            Some(KerdockError::UnsupportedExponent { m })
        );
    }
}
