//! Граничные случаи и защита от некорректного входа.

use rm_codes::{
    count_bit_errors, Bit, Encoder, RmConfig, RmConfigurator, RmDecodeStatus, RmEncoder, RmError,
    MAX_CODE_LENGTH, MAX_M,
};

fn bits_from_seed(seed: u64, len: usize) -> Vec<Bit> {
    (0..len)
        .map(|index| Bit::from_bool((seed >> (index % 64)) & 1 == 1))
        .collect()
}

#[test]
fn invalid_parameters_are_rejected() {
    for (r, m) in [(0, 0), (0, 1), (1, 0), (0, MAX_M + 1), (4, 3)] {
        if r <= m && (1..=MAX_M).contains(&m) {
            continue;
        }
        assert_eq!(
            RmConfig::new(r, m),
            Err(RmError::InvalidParameters { r, m })
        );
    }
}

#[test]
fn max_parameters_fit_project_limits() {
    let config = RmConfig::new(MAX_M, MAX_M).unwrap();
    assert_eq!(config.length(), MAX_CODE_LENGTH);
    assert_eq!(config.dimension(), MAX_CODE_LENGTH);
    assert_eq!(config.distance(), 1);
}

#[test]
fn max_length_encode_decode_smoke() {
    let config = RmConfig::new(1, MAX_M).unwrap();
    let code = RmConfigurator::build(config);
    let message = bits_from_seed(0xDEAD_BEEF, config.dimension());

    let word = code.encode(&message).unwrap();
    assert_eq!(word.len(), MAX_CODE_LENGTH);

    let result = code.decode(&word, None).unwrap();
    assert_eq!(result.status(), RmDecodeStatus::Valid);
    assert_eq!(result.message(), message.as_slice());
}

#[test]
fn invalid_bit_is_rejected() {
    assert_eq!(Bit::try_from(2), Err(RmError::InvalidBit { value: 2 }));
}

#[test]
fn wrong_lengths_are_rejected() {
    let config = RmConfig::new(1, 4).unwrap();
    let code = RmConfigurator::build(config);
    let encoder = RmEncoder::new(config);

    assert_eq!(
        code.encode(&[Bit::Zero; 4]),
        Err(RmError::MessageLengthMismatch {
            expected: 5,
            actual: 4
        })
    );
    assert_eq!(
        code.decode(&[Bit::Zero; 15], None),
        Err(RmError::WordLengthMismatch {
            expected: 16,
            actual: 15
        })
    );
    assert_eq!(
        encoder.extract_message(&[Bit::Zero; 15]),
        Err(RmError::WordLengthMismatch {
            expected: 16,
            actual: 15
        })
    );
}

#[test]
fn reference_length_mismatch_is_rejected() {
    assert_eq!(
        count_bit_errors(&[Bit::Zero; 3], &[Bit::Zero; 2]),
        Err(RmError::ReferenceLengthMismatch {
            expected: 3,
            actual: 2
        })
    );
}

#[test]
fn repetition_code_handles_zero_and_one_messages() {
    let code = RmConfigurator::build(RmConfig::new(0, 3).unwrap());

    for bit in [Bit::Zero, Bit::One] {
        let message = [bit];
        let word = code.encode(&message).unwrap();
        let result = code.decode(&word, None).unwrap();

        assert_eq!(result.message(), &message);
        assert_eq!(result.word(), word.as_slice());
        assert_eq!(result.status(), RmDecodeStatus::Valid);
    }
}
