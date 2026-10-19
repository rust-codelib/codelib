//! Векторы из `ReedMullerExamples.pdf` и базовые свойства кодов.

use rm_codes::{Bit, Encoder, RmConfig, RmConfigurator, RmDecodeStatus, RmEncoder, RmError};

fn bits(text: &str) -> Vec<Bit> {
    text.bytes()
        .map(|byte| Bit::try_from(byte - b'0').unwrap())
        .collect()
}

fn render(bits: &[Bit]) -> String {
    bits.iter()
        .map(|bit| if bit.is_one() { '1' } else { '0' })
        .collect()
}

#[test]
fn rm_1_4_encodes_pdf_example() {
    let code = RmConfigurator::build(RmConfig::new(1, 4).unwrap());
    let message = bits("10011");

    let word = code.encode(&message).unwrap();

    assert_eq!(render(&word), "1001100110011001");
}

#[test]
fn rm_1_4_decodes_pdf_example() {
    let code = RmConfigurator::build(RmConfig::new(1, 4).unwrap());
    let received = bits("1001100110011110");

    let result = code.decode(&received, None).unwrap();

    assert_eq!(render(result.word()), "1001100110011001");
    assert_eq!(render(result.message()), "10011");
    assert_eq!(result.status(), RmDecodeStatus::Corrected);
    assert_eq!(result.changed_bits(), 3);
    assert_eq!(result.corrected_positions(), &[13, 14, 15]);
}

#[test]
fn rm_2_4_encodes_pdf_example() {
    let code = RmConfigurator::build(RmConfig::new(2, 4).unwrap());
    let message = bits("11100011001");

    let word = code.encode(&message).unwrap();

    assert_eq!(render(&word), "1110000101111000");
}

#[test]
fn rm_2_4_decodes_pdf_example() {
    let code = RmConfigurator::build(RmConfig::new(2, 4).unwrap());
    let received = bits("1100000101111000");

    let result = code.decode(&received, None).unwrap();

    assert_eq!(render(result.word()), "1110000101111000");
    assert_eq!(render(result.message()), "11100011001");
    assert_eq!(result.status(), RmDecodeStatus::Corrected);
    assert_eq!(result.changed_bits(), 1);
    assert_eq!(result.corrected_positions(), &[2]);
}

#[test]
fn rm_0_4_is_a_repetition_code() {
    let code = RmConfigurator::build(RmConfig::new(0, 4).unwrap());
    let message = vec![Bit::One];
    let mut received = code.encode(&message).unwrap();

    for slot in received.iter_mut().take(7) {
        *slot = slot.flip();
    }

    let result = code.decode(&received, None).unwrap();

    assert_eq!(result.message(), message.as_slice());
    assert!(result.word().iter().all(|bit| bit.is_one()));
    assert_eq!(result.status(), RmDecodeStatus::Corrected);
}

#[test]
fn rm_0_4_reports_tie_at_half_errors() {
    let code = RmConfigurator::build(RmConfig::new(0, 4).unwrap());
    let mut received = vec![Bit::One; 16];

    for slot in received.iter_mut().take(8) {
        *slot = Bit::Zero;
    }

    let result = code.decode(&received, None).unwrap();

    assert_eq!(result.status(), RmDecodeStatus::Ambiguous);
    assert_eq!(result.ambiguous_coefficients(), &[0]);
}

#[test]
fn full_code_round_trips_arbitrary_words() {
    let config = RmConfig::new(4, 4).unwrap();
    let code = RmConfigurator::build(config);
    let word = bits("1010010111000010");

    let result = code.decode(&word, None).unwrap();

    assert_eq!(result.status(), RmDecodeStatus::Valid);
    assert_eq!(result.word(), word.as_slice());
    assert_eq!(code.encode(result.message()).unwrap(), word);
    assert_eq!(code.extract_message(&word).unwrap(), result.message());
}

#[test]
fn generator_matrix_for_rm_2_4_has_expected_shape() {
    let encoder = RmEncoder::new(RmConfig::new(2, 4).unwrap());
    let matrix = encoder.generator_matrix();

    assert_eq!(matrix.len(), 11);
    assert!(matrix.iter().all(|row| row.len() == 16));
    assert_eq!(render(&matrix[0]), "1111111111111111");
    assert_eq!(render(&matrix[1]), "0000000011111111");
    assert_eq!(render(&matrix[5]), "0000000000001111");
}

#[test]
fn invalid_word_is_rejected_by_extract_message() {
    let encoder = RmEncoder::new(RmConfig::new(1, 4).unwrap());
    let mut word = vec![Bit::Zero; 16];
    word[0] = Bit::One;

    assert_eq!(
        encoder.extract_message(&word),
        Err(RmError::InvalidCodeword)
    );
}
