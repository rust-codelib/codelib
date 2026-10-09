use std::collections::BTreeSet;

use ldpc_codes::{Bit, Encoder, LdpcError, ParityCheckMatrix, SystematicEncoder};

const MAIN_H: [[u8; 6]; 4] = [
    [1, 1, 0, 1, 0, 0],
    [0, 1, 1, 0, 1, 0],
    [1, 0, 0, 0, 1, 1],
    [0, 0, 1, 1, 0, 1],
];

const CYCLE_H: [[u8; 5]; 2] = [[1, 1, 0, 0, 1], [0, 0, 1, 1, 1]];

fn bit(value: u8) -> Bit {
    match value {
        0 => Bit::Zero,
        1 => Bit::One,
        _ => unreachable!("test data contains only binary values"),
    }
}

fn bit_value(value: Bit) -> u8 {
    match value {
        Bit::Zero => 0,
        Bit::One => 1,
    }
}

fn bits_from_mask(width: usize, mask: usize) -> Vec<Bit> {
    (0..width)
        .map(|index| bit(((mask >> index) & 1) as u8))
        .collect()
}

fn mask_from_bits(bits: &[Bit]) -> usize {
    bits.iter().enumerate().fold(0, |mask, (index, &value)| {
        mask | (usize::from(bit_value(value)) << index)
    })
}

fn checks_from_dense<const N: usize>(dense: &[[u8; N]]) -> ParityCheckMatrix {
    let rows = dense
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter_map(|(column, &value)| (value == 1).then_some(column))
                .collect()
        })
        .collect();
    ParityCheckMatrix::try_from_rows(N, rows).unwrap()
}

fn dense_syndrome<const N: usize>(checks: &[[u8; N]], word: &[Bit]) -> Vec<u8> {
    assert_eq!(word.len(), N);
    checks
        .iter()
        .map(|row| {
            row.iter()
                .zip(word)
                .fold(0, |parity, (&coefficient, &value)| {
                    parity ^ (coefficient & bit_value(value))
                })
        })
        .collect()
}

fn assert_full_code_space<const ROWS: usize, const N: usize>(
    encoder: &SystematicEncoder,
    checks: &[[u8; N]; ROWS],
) {
    let mut encoded_words = BTreeSet::new();
    for message_mask in 0..(1usize << encoder.message_len()) {
        let message = bits_from_mask(encoder.message_len(), message_mask);
        let before = message.clone();
        let word = encoder.encode(&message).unwrap();

        assert_eq!(word.len(), N);
        assert_eq!(message, before);
        for (message_index, &position) in encoder.information_positions().iter().enumerate() {
            assert_eq!(word[position], message[message_index]);
        }
        assert_eq!(dense_syndrome(checks, &word), vec![0; ROWS]);
        assert_eq!(encoder.extract_message(&word).unwrap(), message);
        encoded_words.insert(mask_from_bits(&word));
    }

    let mut independent_kernel = BTreeSet::new();
    for word_mask in 0..(1usize << N) {
        let word = bits_from_mask(N, word_mask);
        let syndrome = dense_syndrome(checks, &word);
        let unsatisfied_checks = syndrome.iter().filter(|&&value| value != 0).count();

        if unsatisfied_checks == 0 {
            independent_kernel.insert(word_mask);
            let message = encoder.extract_message(&word).unwrap();
            assert_eq!(encoder.encode(&message).unwrap(), word);
        } else {
            assert_eq!(
                encoder.extract_message(&word),
                Err(LdpcError::InvalidCodeword { unsatisfied_checks }),
                "word mask {word_mask:#b} has syndrome {syndrome:?}"
            );
        }
    }

    assert_eq!(encoded_words, independent_kernel);
    assert_eq!(encoded_words.len(), 1usize << encoder.message_len());
}

fn assert_generator_matrix<const ROWS: usize, const N: usize>(
    encoder: &SystematicEncoder,
    checks: &[[u8; N]; ROWS],
) {
    let k = encoder.message_len();
    let generator_rows: Vec<Vec<u8>> = (0..k)
        .map(|message_index| {
            let mut basis = vec![Bit::Zero; k];
            basis[message_index] = Bit::One;
            encoder
                .encode(&basis)
                .unwrap()
                .iter()
                .copied()
                .map(bit_value)
                .collect()
        })
        .collect();
    assert!(generator_rows.iter().all(|row| row.len() == N));

    for (message_index, generator_row) in generator_rows.iter().enumerate() {
        for (information_index, &position) in encoder.information_positions().iter().enumerate() {
            assert_eq!(
                generator_row[position],
                u8::from(message_index == information_index),
                "G[{message_index}, {}] must form the identity on information positions",
                position
            );
        }
    }

    for (check_index, check_row) in checks.iter().enumerate() {
        for (generator_index, generator_row) in generator_rows.iter().enumerate() {
            let product = check_row
                .iter()
                .zip(generator_row)
                .fold(0, |parity, (&check, &generator)| {
                    parity ^ (check & generator)
                });
            assert_eq!(
                product, 0,
                "(H G^T)[{check_index}, {generator_index}] must be zero"
            );
        }
    }
}

#[test]
fn exhaustive_main_example_matches_the_independent_dense_kernel_and_generator() {
    let encoder = SystematicEncoder::try_new(checks_from_dense(&MAIN_H)).unwrap();

    assert_eq!(encoder.rank(), 3);
    assert_eq!(encoder.message_len(), 3);
    assert_full_code_space(&encoder, &MAIN_H);
    assert_generator_matrix(&encoder, &MAIN_H);
}

#[test]
fn exhaustive_cycle_example_matches_the_independent_dense_kernel_and_generator() {
    let encoder = SystematicEncoder::try_new(checks_from_dense(&CYCLE_H)).unwrap();

    assert_eq!(encoder.rank(), 2);
    assert_eq!(encoder.message_len(), 3);
    assert_full_code_space(&encoder, &CYCLE_H);
    assert_generator_matrix(&encoder, &CYCLE_H);
}

#[test]
fn cycle_example_is_linear_for_every_pair_of_messages() {
    let encoder = SystematicEncoder::try_new(checks_from_dense(&CYCLE_H)).unwrap();
    let message_count = 1usize << encoder.message_len();

    for left_mask in 0..message_count {
        for right_mask in 0..message_count {
            let left = bits_from_mask(encoder.message_len(), left_mask);
            let right = bits_from_mask(encoder.message_len(), right_mask);
            let xor_message: Vec<_> = left
                .iter()
                .zip(&right)
                .map(|(&a, &b)| bit(bit_value(a) ^ bit_value(b)))
                .collect();
            let left_word = encoder.encode(&left).unwrap();
            let right_word = encoder.encode(&right).unwrap();
            let expected_xor: Vec<_> = left_word
                .iter()
                .zip(&right_word)
                .map(|(&a, &b)| bit(bit_value(a) ^ bit_value(b)))
                .collect();

            assert_eq!(encoder.encode(&xor_message).unwrap(), expected_xor);
        }
    }
}

#[test]
fn empty_repeated_and_dependent_checks_preserve_rank_and_the_full_code_space() {
    const BASE_H: [[u8; 3]; 2] = [[1, 1, 0], [0, 1, 1]];
    const REDUNDANT_H: [[u8; 3]; 5] = [
        [1, 1, 0],
        [0, 1, 1],
        [1, 1, 0], // repeated row
        [1, 0, 1], // XOR of the first two rows
        [0, 0, 0], // empty row
    ];
    let base = SystematicEncoder::try_new(checks_from_dense(&BASE_H)).unwrap();
    let redundant = SystematicEncoder::try_new(checks_from_dense(&REDUNDANT_H)).unwrap();

    assert_eq!(redundant.rank(), base.rank());
    assert_eq!(redundant.message_len(), base.message_len());
    assert_eq!(
        redundant.information_positions(),
        base.information_positions()
    );
    assert_eq!(redundant.parity_positions(), base.parity_positions());
    assert_full_code_space(&base, &BASE_H);
    assert_full_code_space(&redundant, &REDUNDANT_H);

    for message_mask in 0..(1usize << base.message_len()) {
        let message = bits_from_mask(base.message_len(), message_mask);
        assert_eq!(base.encode(&message), redundant.encode(&message));
    }
}

fn sparse_syndrome(rows: &[Vec<usize>], word: &[Bit]) -> Vec<u8> {
    rows.iter()
        .map(|row| {
            row.iter()
                .fold(0, |parity, &column| parity ^ bit_value(word[column]))
        })
        .collect()
}

#[test]
fn stretched_one_by_4096_matrix_keeps_isolated_columns_and_message_tail_zeros() {
    let rows = vec![vec![0]];
    let checks = ParityCheckMatrix::try_from_rows(4096, rows.clone()).unwrap();
    let encoder = SystematicEncoder::try_new(checks).unwrap();

    assert_eq!(encoder.rank(), 1);
    assert_eq!(encoder.message_len(), 4095);
    assert_eq!(encoder.codeword_len(), 4096);
    assert_eq!(
        encoder.information_positions(),
        (1..4096).collect::<Vec<_>>()
    );
    assert_eq!(encoder.parity_positions(), &[0]);

    let mut message = vec![Bit::Zero; 4095];
    message[0] = Bit::One;
    let original_message = message.clone();
    let word = encoder.encode(&message).unwrap();
    let original_word = word.clone();

    assert_eq!(word.len(), 4096);
    assert_eq!(word[0], Bit::Zero);
    assert_eq!(word[1], Bit::One);
    assert!(word[2..].iter().all(|&value| value == Bit::Zero));
    assert_eq!(sparse_syndrome(&rows, &word), vec![0]);
    assert_eq!(encoder.extract_message(&word).unwrap(), message);
    assert_eq!(encoder.encode(&message).unwrap(), original_word);
    assert_eq!(message, original_message);
    assert_eq!(word, original_word);
}

#[test]
fn stretched_4096_by_2_matrix_supports_many_dependent_checks() {
    let mut rows = vec![vec![0], vec![0]];
    rows.resize(4096, Vec::new());
    let checks = ParityCheckMatrix::try_from_rows(2, rows.clone()).unwrap();
    let encoder = SystematicEncoder::try_new(checks).unwrap();

    assert_eq!(encoder.rank(), 1);
    assert_eq!(encoder.message_len(), 1);
    assert_eq!(encoder.codeword_len(), 2);
    assert_eq!(encoder.information_positions(), &[1]);
    assert_eq!(encoder.parity_positions(), &[0]);

    let message = vec![Bit::One];
    let word = encoder.encode(&message).unwrap();
    let original_word = word.clone();
    assert_eq!(word, vec![Bit::Zero, Bit::One]);
    assert_eq!(sparse_syndrome(&rows, &word), vec![0; 4096]);
    assert_eq!(encoder.extract_message(&word).unwrap(), message);

    let invalid_word = vec![Bit::One, Bit::Zero];
    let original_invalid_word = invalid_word.clone();
    assert_eq!(
        sparse_syndrome(&rows, &invalid_word)
            .iter()
            .filter(|&&value| value != 0)
            .count(),
        2
    );
    assert_eq!(
        encoder.extract_message(&invalid_word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 2
        })
    );
    assert_eq!(
        encoder.extract_message(&invalid_word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 2
        })
    );
    assert_eq!(encoder.encode(&message).unwrap(), original_word);
    assert_eq!(message, vec![Bit::One]);
    assert_eq!(word, original_word);
    assert_eq!(invalid_word, original_invalid_word);
}

#[test]
fn repeated_successful_and_invalid_calls_preserve_inputs_and_prior_results() {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![1, 2]]).unwrap();
    let encoder = SystematicEncoder::try_new(checks).unwrap();

    let message = vec![Bit::One, Bit::Zero];
    let original_message = message.clone();
    let result = encoder.encode(&message).unwrap();
    let original_result = result.clone();
    let invalid_message = vec![Bit::One];
    let original_invalid_message = invalid_message.clone();
    let invalid_word = vec![Bit::One, Bit::One, Bit::Zero];
    let original_invalid_word = invalid_word.clone();

    assert_eq!(
        encoder.encode(&invalid_message),
        Err(LdpcError::MessageLengthMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        encoder.encode(&invalid_message),
        Err(LdpcError::MessageLengthMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        encoder.extract_message(&invalid_word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 1,
        })
    );
    assert_eq!(encoder.encode(&message).unwrap(), original_result);
    assert_eq!(
        encoder.extract_message(&invalid_word),
        Err(LdpcError::InvalidCodeword {
            unsatisfied_checks: 1,
        })
    );

    assert_eq!(message, original_message);
    assert_eq!(invalid_message, original_invalid_message);
    assert_eq!(invalid_word, original_invalid_word);
    assert_eq!(result, original_result);
}
