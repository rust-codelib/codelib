use ldpc_codes::{Bit, LdpcError};

#[test]
fn try_from_accepts_only_zero_and_one_for_every_u8_value() {
    for value in u8::MIN..=u8::MAX {
        let result = Bit::try_from(value);

        match value {
            0 => assert_eq!(result, Ok(Bit::Zero)),
            1 => assert_eq!(result, Ok(Bit::One)),
            _ => assert_eq!(result, Err(LdpcError::InvalidBit { value })),
        }
    }
}

#[test]
fn converting_bits_to_u8_and_back_preserves_each_bit() {
    for bit in [Bit::Zero, Bit::One] {
        let value = u8::from(bit);

        assert_eq!(Bit::try_from(value), Ok(bit));
    }
}

#[test]
fn invalid_bit_error_display_includes_the_invalid_value() {
    let error = LdpcError::InvalidBit { value: u8::MAX };

    assert!(error.to_string().contains("255"));
}

#[test]
fn ldpc_error_implements_standard_error_and_equality_traits() {
    fn assert_error<T: std::error::Error>() {}
    fn assert_equality<T: PartialEq + Eq>() {}

    assert_error::<LdpcError>();
    assert_equality::<LdpcError>();
}
