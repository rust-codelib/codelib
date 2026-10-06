use ldpc_codes::{
    bits_to_vector, count_bit_errors, vector_to_bits, Bit, DecodeInput, DecodeStatus,
    DecoderConfig, Encoder, LdpcConfigurator, ParityCheckMatrix,
};

fn main_checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )
    .expect("the main example matrix is valid")
}

fn repeat_checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the repetition-code matrix is valid")
}

#[test]
fn noiseless_round_trip_preserves_zero_and_trailing_zero_messages() {
    let mut configured = LdpcConfigurator::build(main_checks(), DecoderConfig::default())
        .expect("rank three gives a valid three-bit message");
    assert_eq!(configured.encoder().message_len(), 3);
    assert_eq!(configured.encoder().information_positions(), &[3, 4, 5]);

    for message in [
        [Bit::One, Bit::Zero, Bit::Zero],
        [Bit::Zero, Bit::Zero, Bit::Zero],
    ] {
        let message_vector = bits_to_vector(&message);
        assert_eq!(message_vector.len(), 3);
        let message_bits = vector_to_bits(&message_vector).expect("GF(2) values are bits");
        assert_eq!(message_bits, message);

        let codeword = configured
            .encoder()
            .encode(&message_bits)
            .expect("message has length k");
        let llrs: Vec<_> = codeword
            .iter()
            .map(|bit| match bit {
                Bit::Zero => 3.0,
                Bit::One => -3.0,
            })
            .collect();
        let result = configured
            .decode(
                DecodeInput {
                    llrs: &llrs,
                    erasures: &[],
                },
                None,
            )
            .expect("the LLR block has the expected length");

        assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
        assert_eq!(result.iterations(), 0);
        assert_eq!(result.word(), codeword);
        assert_eq!(count_bit_errors(result.word(), &codeword), Ok(0));

        // Extraction goes through the encoder because information positions are [3, 4, 5].
        let recovered_bits = configured
            .encoder()
            .extract_message(result.word())
            .expect("the result satisfies the parity checks");
        assert_eq!(recovered_bits, message_bits);
        assert_eq!(count_bit_errors(&recovered_bits, &message_bits), Ok(0));

        let recovered_vector = bits_to_vector(&recovered_bits);
        assert_eq!(recovered_vector.len(), message_vector.len());
        assert_eq!(vector_to_bits(&recovered_vector), Ok(message.to_vec()));
    }
}

#[test]
fn spa_corrects_the_planned_channel_values_and_round_trips_the_message() {
    let message_vector = bits_to_vector(&[Bit::One]);
    let message = vector_to_bits(&message_vector).expect("GF(2) values are bits");
    let mut configured = LdpcConfigurator::build(
        repeat_checks(),
        DecoderConfig::try_new(1, 20.0).expect("the LLR limit is valid"),
    )
    .expect("the repetition code has a valid encoder rank");
    let codeword = configured
        .encoder()
        .encode(&message)
        .expect("message has length k");
    assert_eq!(codeword, [Bit::One, Bit::One, Bit::One]);

    let llrs = [-2.0, 1.0, -3.0];
    let result = configured
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            None,
        )
        .expect("the LLR block has the expected length");

    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 1);
    assert_eq!(result.word(), codeword);
    assert_eq!(count_bit_errors(result.word(), &codeword), Ok(0));

    let recovered_message = configured
        .encoder()
        .extract_message(result.word())
        .expect("the result satisfies the parity checks");
    assert_eq!(count_bit_errors(&recovered_message, &message), Ok(0));
    let recovered_vector = bits_to_vector(&recovered_message);
    assert_eq!(recovered_vector.len(), message_vector.len());
    assert_eq!(vector_to_bits(&recovered_vector), Ok(message));
}

#[test]
fn exhausted_iteration_budget_reports_status_without_extracting_message() {
    let mut configured = LdpcConfigurator::build(
        repeat_checks(),
        DecoderConfig::try_new(0, 20.0).expect("zero iterations is a valid budget"),
    )
    .expect("the repetition code has a valid encoder rank");
    let expected_codeword = configured
        .encoder()
        .encode(&[Bit::One])
        .expect("message has length k");
    let llrs = [-2.0, 1.0, -3.0];
    let result = configured
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            None,
        )
        .expect("the LLR block is valid even if it cannot be decoded yet");

    assert_eq!(result.status(), DecodeStatus::IterationLimit);
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.word(), [Bit::One, Bit::Zero, Bit::One]);
    assert_eq!(count_bit_errors(result.word(), &expected_codeword), Ok(1));
    // A result with a nonzero syndrome must not be passed to extract_message.
}
