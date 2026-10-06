use ldpc_codes::{Bit, LdpcError, ParityCheckMatrix};

const DENSE_H: [[u8; 3]; 2] = [[1, 0, 1], [0, 1, 1]];

fn bits(values: &[u8]) -> Vec<Bit> {
    values
        .iter()
        .copied()
        .map(|value| Bit::try_from(value).unwrap())
        .collect()
}

fn dense_syndrome(word: &[u8]) -> Vec<Bit> {
    DENSE_H
        .iter()
        .map(|row| {
            let parity = row
                .iter()
                .zip(word)
                .fold(0, |parity, (&coefficient, &bit)| {
                    parity ^ (coefficient & bit)
                });
            Bit::try_from(parity).unwrap()
        })
        .collect()
}

#[test]
fn matches_independent_dense_calculation_for_all_eight_words() {
    let matrix = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 2], vec![1, 2]]).unwrap();

    for value in 0..8 {
        let word = [
            (value & 1) as u8,
            ((value >> 1) & 1) as u8,
            ((value >> 2) & 1) as u8,
        ];
        let input = bits(&word);
        let expected = dense_syndrome(&word);

        assert_eq!(matrix.syndrome(&input).unwrap(), expected, "word {word:?}");
        assert_eq!(
            matrix.is_codeword(&input).unwrap(),
            expected.iter().all(|&bit| bit == Bit::Zero),
            "word {word:?}"
        );
    }
}

#[test]
fn matches_examples_from_the_plan() {
    let matrix = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 2], vec![1, 2]]).unwrap();

    assert_eq!(matrix.syndrome(&bits(&[1, 1, 1])).unwrap(), bits(&[0, 0]));
    assert!(matrix.is_codeword(&bits(&[1, 1, 1])).unwrap());
    assert_eq!(matrix.syndrome(&bits(&[1, 0, 0])).unwrap(), bits(&[1, 0]));
    assert!(!matrix.is_codeword(&bits(&[1, 0, 0])).unwrap());
}

#[test]
fn zero_matrix_has_zero_syndrome_for_every_bit() {
    let matrix = ParityCheckMatrix::try_from_rows(3, vec![vec![], vec![]]).unwrap();

    assert_eq!(matrix.syndrome(&bits(&[1, 0, 1])).unwrap(), bits(&[0, 0]));
    assert!(matrix.is_codeword(&bits(&[1, 0, 1])).unwrap());
}

#[test]
fn empty_check_contributes_zero_in_its_original_row_position() {
    let matrix = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![], vec![1, 2]]).unwrap();

    assert_eq!(
        matrix.syndrome(&bits(&[1, 0, 1])).unwrap(),
        bits(&[1, 0, 1])
    );
}

#[test]
fn isolated_bit_does_not_change_the_syndrome() {
    let matrix = ParityCheckMatrix::try_from_rows(4, vec![vec![0, 2], vec![1, 2]]).unwrap();
    assert_eq!(matrix.bit_checks(3), Some(&[][..]));

    assert_eq!(
        matrix.syndrome(&bits(&[0, 0, 0, 0])).unwrap(),
        bits(&[0, 0])
    );
    assert_eq!(
        matrix.syndrome(&bits(&[0, 0, 0, 1])).unwrap(),
        bits(&[0, 0])
    );
}

#[test]
fn rejects_words_of_any_length_other_than_the_number_of_columns() {
    let matrix = ParityCheckMatrix::try_from_rows(3, vec![vec![]]).unwrap();

    for actual in [2, 4, 0] {
        let word = vec![Bit::Zero; actual];
        let expected_error = LdpcError::WordLengthMismatch {
            expected: 3,
            actual,
        };

        assert_eq!(matrix.syndrome(&word), Err(expected_error));
        assert_eq!(
            matrix.is_codeword(&word),
            Err(LdpcError::WordLengthMismatch {
                expected: 3,
                actual
            })
        );
    }
}

#[test]
fn preserves_the_word_matrix_and_both_adjacencies() {
    let matrix =
        ParityCheckMatrix::try_from_rows(4, vec![vec![3, 0], vec![], vec![2, 0], vec![3, 0]])
            .unwrap();
    let check_bits_before: Vec<_> = (0..matrix.rows())
        .map(|check| matrix.check_bits(check).unwrap().to_vec())
        .collect();
    let bit_checks_before: Vec<_> = (0..matrix.cols())
        .map(|bit| matrix.bit_checks(bit).unwrap().to_vec())
        .collect();
    let word = bits(&[1, 0, 1, 1]);
    let word_before = word.clone();

    let _ = matrix.syndrome(&word).unwrap();
    let _ = matrix.is_codeword(&word).unwrap();

    assert_eq!(word, word_before);
    assert_eq!(
        (0..matrix.rows())
            .map(|check| matrix.check_bits(check).unwrap().to_vec())
            .collect::<Vec<_>>(),
        check_bits_before
    );
    assert_eq!(
        (0..matrix.cols())
            .map(|bit| matrix.bit_checks(bit).unwrap().to_vec())
            .collect::<Vec<_>>(),
        bit_checks_before
    );
}

#[test]
fn repeated_rows_keep_repeated_syndrome_entries() {
    let matrix =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 2], vec![1], vec![0, 2]]).unwrap();

    assert_eq!(
        matrix.syndrome(&bits(&[1, 0, 1])).unwrap(),
        bits(&[0, 0, 0])
    );
    assert_eq!(
        matrix.syndrome(&bits(&[1, 0, 0])).unwrap(),
        bits(&[1, 0, 1])
    );
}
