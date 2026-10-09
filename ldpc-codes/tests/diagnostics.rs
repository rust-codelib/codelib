use std::error::Error;

use ldpc_codes::{
    count_bit_errors, Bit, DecodeInput, DecodeStatus, Decoder, DecoderConfig, LdpcError,
    ParityCheckMatrix, SpaDecoder,
};

#[test]
fn counts_equal_multiple_all_and_empty_bit_sequences_without_changing_inputs() {
    let word = [Bit::Zero, Bit::One, Bit::Zero, Bit::One];
    let equal_reference = word;
    let multiple_errors_reference = [Bit::One, Bit::One, Bit::One, Bit::One];
    let all_errors_reference = [Bit::One, Bit::Zero, Bit::One, Bit::Zero];

    assert_eq!(count_bit_errors(&word, &equal_reference), Ok(0));
    assert_eq!(count_bit_errors(&word, &multiple_errors_reference), Ok(2));
    assert_eq!(count_bit_errors(&word, &all_errors_reference), Ok(4));
    assert_eq!(count_bit_errors(&[], &[]), Ok(0));

    assert_eq!(word, [Bit::Zero, Bit::One, Bit::Zero, Bit::One]);
    assert_eq!(equal_reference, word);
    assert_eq!(multiple_errors_reference, [Bit::One; 4]);
    assert_eq!(
        all_errors_reference,
        [Bit::One, Bit::Zero, Bit::One, Bit::Zero]
    );
}

#[test]
fn different_lengths_report_expected_and_actual_lengths_without_a_source() {
    let error = count_bit_errors(&[Bit::Zero, Bit::One, Bit::Zero], &[Bit::One])
        .expect_err("reference must have the same length as the word");

    assert_eq!(
        error,
        LdpcError::ReferenceLengthMismatch {
            expected: 3,
            actual: 1,
        }
    );
    assert_eq!(
        error.to_string(),
        "длина эталона 1 не совпадает с ожидаемой длиной слова 3"
    );
    assert!(error.source().is_none());
}

#[test]
fn parity_satisfied_word_can_have_three_errors_against_the_reference() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("repetition-code matrix is valid");
    let mut decoder = SpaDecoder::try_new(checks, DecoderConfig::default())
        .expect("valid repetition-code matrix");
    let transmitted = [Bit::One; 3];
    let result = decoder
        .decode(
            DecodeInput {
                llrs: &[1.0, 1.0, 1.0],
                erasures: &[],
            },
            None,
        )
        .expect("positive LLRs produce a valid zero word");

    assert_eq!(result.word(), [Bit::Zero; 3]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.changed_bits(), 0);
    assert_eq!(count_bit_errors(result.word(), &transmitted), Ok(3));
}
