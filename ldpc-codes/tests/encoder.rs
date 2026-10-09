use ldpc_codes::{Bit, Encoder, LdpcError, ParityCheckMatrix, SystematicEncoder};

const MAIN_DENSE_H: [[u8; 6]; 4] = [
    [1, 1, 0, 1, 0, 0],
    [0, 1, 1, 0, 1, 0],
    [1, 0, 0, 0, 1, 1],
    [0, 0, 1, 1, 0, 1],
];

const CYCLE_DENSE_H: [[u8; 5]; 2] = [[1, 1, 0, 0, 1], [0, 0, 1, 1, 1]];

fn encoder(bits: usize, rows: Vec<Vec<usize>>) -> SystematicEncoder {
    SystematicEncoder::try_new(ParityCheckMatrix::try_from_rows(bits, rows).unwrap()).unwrap()
}

fn bits(values: &[u8]) -> Vec<Bit> {
    values
        .iter()
        .copied()
        .map(|value| Bit::try_from(value).unwrap())
        .collect()
}

fn dense_syndrome<const N: usize>(checks: &[[u8; N]], word: &[Bit]) -> Vec<u8> {
    checks
        .iter()
        .map(|row| {
            row.iter()
                .zip(word)
                .fold(0, |parity, (&coefficient, &bit)| {
                    parity ^ (coefficient & u8::from(bit))
                })
        })
        .collect()
}

fn assert_all_messages_match_dense_checks<const N: usize>(
    encoder: &SystematicEncoder,
    checks: &[[u8; N]],
) {
    for value in 0..(1 << encoder.message_len()) {
        let message: Vec<_> = (0..encoder.message_len())
            .map(|index| Bit::try_from(((value >> index) & 1) as u8).unwrap())
            .collect();
        let original_message = message.clone();
        let word = encoder.encode(&message).unwrap();

        assert_eq!(word.len(), encoder.codeword_len());
        assert_eq!(dense_syndrome(checks, &word), vec![0; checks.len()]);
        for (message_index, &position) in encoder.information_positions().iter().enumerate() {
            assert_eq!(word[position], message[message_index]);
        }
        assert_eq!(encoder.extract_message(&word).unwrap(), message);
        assert_eq!(message, original_message);
    }
}

#[test]
fn main_example_reports_rank_and_information_positions_using_dependent_rows() {
    let encoder = encoder(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    );

    assert_eq!(encoder.rank(), 3);
    assert_eq!(encoder.information_positions(), &[3, 4, 5]);
    assert_eq!(encoder.parity_positions(), &[0, 1, 2]);
}

#[test]
fn second_example_reports_cycle_three_permutations_positions_and_rank() {
    let encoder = encoder(5, vec![vec![0, 1, 4], vec![2, 3, 4]]);

    assert_eq!(encoder.rank(), 2);
    assert_eq!(encoder.information_positions(), &[1, 3, 4]);
    assert_eq!(encoder.parity_positions(), &[0, 2]);
}

#[test]
fn swap_and_identity_examples_have_the_expected_information_positions() {
    let swap = encoder(3, vec![vec![1, 2]]);
    assert_eq!(swap.information_positions(), &[0, 2]);

    let identity = encoder(3, vec![vec![2]]);
    assert_eq!(identity.information_positions(), &[0, 1]);
}

#[test]
fn rejects_zero_rank() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 0, cols: 3 }
    );
}

#[test]
fn rejects_full_rank() {
    let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![0], vec![1]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 2, cols: 2 }
    );
}

#[test]
fn rejects_one_column_encoder() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![0]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 1, cols: 1 }
    );
}

#[test]
fn rejects_zero_rank_for_one_column_encoder() {
    let checks = ParityCheckMatrix::try_from_rows(1, vec![vec![]]).unwrap();

    assert_eq!(
        SystematicEncoder::try_new(checks).unwrap_err(),
        LdpcError::InvalidEncoderRank { rank: 0, cols: 1 }
    );
}

#[test]
fn allows_dependent_rows_when_the_number_of_checks_is_at_least_the_column_count() {
    let encoder = encoder(2, vec![vec![0], vec![0], vec![0]]);

    assert_eq!(encoder.rank(), 1);
    assert_eq!(encoder.information_positions(), &[1]);
}

#[test]
fn adding_an_empty_and_repeated_check_preserves_rank_and_positions() {
    let base = encoder(3, vec![vec![0, 2], vec![1, 2]]);
    let redundant = encoder(3, vec![vec![0, 2], vec![1, 2], vec![], vec![0, 2]]);

    assert_eq!(base.rank(), 2);
    assert_eq!(redundant.rank(), base.rank());
    assert_eq!(
        redundant.information_positions(),
        base.information_positions()
    );
    assert_eq!(redundant.parity_positions(), base.parity_positions());
}

#[test]
fn preparation_is_deterministic() {
    let rows = vec![vec![0, 1, 4], vec![2, 3, 4]];
    let first = encoder(5, rows.clone());
    let second = encoder(5, rows);

    assert_eq!(first.rank(), second.rank());
    assert_eq!(
        first.information_positions(),
        second.information_positions()
    );
    assert_eq!(first.parity_positions(), second.parity_positions());
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

#[test]
fn encodes_the_main_example_exactly_and_all_messages_satisfy_independent_checks() {
    let encoder = encoder(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    );

    assert_eq!(encoder.message_len(), 3);
    assert_eq!(encoder.codeword_len(), 6);
    assert_eq!(encoder.information_positions(), &[3, 4, 5]);
    assert_eq!(
        encoder.encode(&bits(&[0, 1, 1])).unwrap(),
        bits(&[0, 0, 1, 0, 1, 1])
    );
    assert_all_messages_match_dense_checks(&encoder, &MAIN_DENSE_H);
}

#[test]
fn encodes_the_cycle_example_exactly_and_all_messages_satisfy_independent_checks() {
    let encoder = encoder(5, vec![vec![0, 1, 4], vec![2, 3, 4]]);

    assert_eq!(encoder.information_positions(), &[1, 3, 4]);
    assert_eq!(
        encoder.encode(&bits(&[1, 0, 1])).unwrap(),
        bits(&[0, 1, 1, 0, 1])
    );
    assert_all_messages_match_dense_checks(&encoder, &CYCLE_DENSE_H);
}

#[test]
fn preserves_trailing_zeros_and_encodes_the_zero_message() {
    let encoder = encoder(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    );
    let trailing_zeros = bits(&[1, 0, 0]);

    assert_eq!(
        encoder.encode(&trailing_zeros).unwrap(),
        bits(&[0, 1, 1, 1, 0, 0])
    );
    assert_eq!(trailing_zeros, bits(&[1, 0, 0]));
    assert_eq!(
        encoder.encode(&bits(&[0, 0, 0])).unwrap(),
        vec![Bit::Zero; 6]
    );
}

#[test]
fn reports_message_length_errors_for_empty_short_and_long_inputs() {
    let encoder = encoder(6, vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5]]);

    for message in [vec![], bits(&[1, 0]), bits(&[1, 0, 1, 1])] {
        let original = message.clone();
        assert_eq!(
            encoder.encode(&message),
            Err(LdpcError::MessageLengthMismatch {
                expected: 3,
                actual: message.len(),
            })
        );
        assert_eq!(message, original);
    }
}

#[test]
fn extracts_only_words_satisfying_the_original_checks_and_counts_repeated_rows() {
    let encoder = encoder(3, vec![vec![0, 2], vec![0, 2], vec![1, 2]]);
    let word = bits(&[1, 0, 0]);
    let original = word.clone();

    assert_eq!(
        encoder.extract_message(&word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 2,
        })
    );
    assert_eq!(word, original);
}

#[test]
fn extract_checks_word_length_before_computing_syndrome() {
    let encoder = encoder(3, vec![vec![0, 2], vec![1, 2]]);

    assert_eq!(
        encoder.extract_message(&bits(&[1, 0])),
        Err(LdpcError::WordLengthMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(
        encoder.extract_message(&[]),
        Err(LdpcError::WordLengthMismatch {
            expected: 3,
            actual: 0,
        })
    );
    assert_eq!(
        encoder.extract_message(&bits(&[1, 1, 1, 1])),
        Err(LdpcError::WordLengthMismatch {
            expected: 3,
            actual: 4,
        })
    );
}

#[test]
fn accepts_a_different_valid_codeword_and_rejects_a_foreign_codeword() {
    let encoder = encoder(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    );
    let other_valid_word = encoder.encode(&bits(&[1, 0, 0])).unwrap();
    assert_eq!(
        encoder.extract_message(&other_valid_word).unwrap(),
        bits(&[1, 0, 0])
    );

    let foreign_checks = ParityCheckMatrix::try_from_rows(6, vec![vec![1, 2]]).unwrap();
    let foreign_word = bits(&[1, 0, 0, 0, 0, 0]);
    assert!(foreign_checks.is_codeword(&foreign_word).unwrap());
    assert_eq!(
        encoder.extract_message(&foreign_word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 2,
        })
    );
}

#[test]
fn repeated_calls_are_independent_and_preserve_each_input() {
    let encoder = encoder(5, vec![vec![0, 1, 4], vec![2, 3, 4]]);
    let first = bits(&[1, 0, 1]);
    let second = bits(&[0, 1, 0]);
    let first_before = first.clone();
    let second_before = second.clone();
    let expected_first = bits(&[0, 1, 1, 0, 1]);
    let expected_second = bits(&[0, 0, 1, 1, 0]);
    let expected_first_before = expected_first.clone();
    let expected_second_before = expected_second.clone();

    assert_eq!(encoder.encode(&first).unwrap(), expected_first);
    assert_eq!(encoder.encode(&second).unwrap(), expected_second);
    assert_eq!(encoder.encode(&first).unwrap(), expected_first);
    assert_eq!(encoder.extract_message(&expected_first).unwrap(), first);
    assert_eq!(encoder.extract_message(&expected_second).unwrap(), second);
    assert_eq!(first, first_before);
    assert_eq!(second, second_before);
    assert_eq!(expected_first, expected_first_before);
    assert_eq!(expected_second, expected_second_before);
}
