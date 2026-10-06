use ldpc_codes::{
    bits_to_vector, count_bit_errors, vector_to_bits, Bit, ConfiguredLdpc, DecodeInput,
    DecodeStatus, Decoder, DecoderConfig, Encoder, LdpcConfigurator, ParityCheckMatrix,
};
use proptest::prelude::*;

#[derive(Clone, Debug)]
struct CodecCase {
    rank: usize,
    dense_checks: Vec<Vec<u8>>,
    first_message: Vec<Bit>,
    second_message: Vec<Bit>,
}

fn dense_checks_strategy() -> impl Strategy<Value = (usize, Vec<Vec<u8>>)> {
    (
        1usize..=4,
        0usize..=3,
        prop::collection::vec(any::<u8>(), 0..=4),
    )
        .prop_flat_map(|(rank, active_information_columns, dependencies)| {
            (
                Just(rank),
                Just(active_information_columns),
                Just(dependencies),
                prop::collection::vec(any::<bool>(), rank * active_information_columns),
            )
                .prop_map(
                    |(rank, active_information_columns, dependencies, coefficients)| {
                        // The leading square block is identity, so these rows are independent.
                        // The final non-pivot column stays isolated for every generated case.
                        let columns = rank + active_information_columns + 1;
                        let mut basis = vec![vec![0; columns]; rank];
                        for (pivot, row) in basis.iter_mut().enumerate() {
                            row[pivot] = 1;
                            for (information_index, coefficient) in
                                row[rank..columns - 1].iter_mut().enumerate()
                            {
                                let coefficient_index =
                                    pivot * active_information_columns + information_index;
                                *coefficient = u8::from(coefficients[coefficient_index]);
                            }
                        }

                        let mut checks = basis.clone();
                        checks.push(basis[0].clone()); // A dependent duplicate row.
                        checks.push(vec![0; columns]); // An empty row.
                        for dependency in dependencies {
                            let mut row = vec![0; columns];
                            for (pivot, basis_row) in basis.iter().enumerate() {
                                if dependency & (1 << pivot) != 0 {
                                    for (coefficient, &basis_coefficient) in
                                        row.iter_mut().zip(basis_row)
                                    {
                                        *coefficient ^= basis_coefficient;
                                    }
                                }
                            }
                            checks.push(row);
                        }

                        (rank, checks)
                    },
                )
        })
}

fn codec_case_strategy() -> impl Strategy<Value = CodecCase> {
    dense_checks_strategy().prop_flat_map(|(rank, dense_checks)| {
        let message_len = dense_checks[0].len() - rank;
        (
            Just((rank, dense_checks)),
            prop::collection::vec(any::<bool>(), message_len),
            prop::collection::vec(any::<bool>(), message_len),
        )
            .prop_map(|((rank, dense_checks), first, second)| CodecCase {
                rank,
                dense_checks,
                first_message: first.into_iter().map(bit_from_bool).collect(),
                second_message: second.into_iter().map(bit_from_bool).collect(),
            })
    })
}

fn bit_from_bool(value: bool) -> Bit {
    if value {
        Bit::One
    } else {
        Bit::Zero
    }
}

fn xor_bits(left: &[Bit], right: &[Bit]) -> Vec<Bit> {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| bit_from_bool(u8::from(left) != u8::from(right)))
        .collect()
}

fn complement_bits(bits: &[Bit]) -> Vec<Bit> {
    bits.iter()
        .map(|&bit| match bit {
            Bit::Zero => Bit::One,
            Bit::One => Bit::Zero,
        })
        .collect()
}

fn checks_from_dense(dense: &[Vec<u8>]) -> ParityCheckMatrix {
    let columns = dense[0].len();
    let rows = dense
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter_map(|(column, &value)| (value == 1).then_some(column))
                .collect()
        })
        .collect();

    ParityCheckMatrix::try_from_rows(columns, rows).expect("generated checks are valid")
}

// This deliberately uses a dense row-by-column calculation rather than the
// sparse ParityCheckMatrix::syndrome implementation under test.
fn dense_xor_syndrome(dense: &[Vec<u8>], word: &[Bit]) -> Vec<u8> {
    dense
        .iter()
        .map(|row| {
            row.iter()
                .zip(word)
                .fold(0, |parity, (&coefficient, &value)| {
                    parity ^ (coefficient & u8::from(value))
                })
        })
        .collect()
}

fn strong_llrs(word: &[Bit]) -> Vec<f64> {
    word.iter()
        .map(|bit| match bit {
            Bit::Zero => 8.0,
            Bit::One => -8.0,
        })
        .collect()
}

fn assert_noiseless_block(
    configured: &mut ConfiguredLdpc,
    dense_checks: &[Vec<u8>],
    message: &[Bit],
) {
    let message_vector = bits_to_vector(message);
    assert_eq!(message_vector.len(), message.len());
    let vector_message = vector_to_bits(&message_vector).expect("bits map to GF(2)");
    assert_eq!(vector_message, message);

    let codeword = configured
        .encoder()
        .encode(&vector_message)
        .expect("message length matches the configured code");
    assert_eq!(codeword.len(), dense_checks[0].len());
    assert_eq!(
        dense_xor_syndrome(dense_checks, &codeword),
        vec![0; dense_checks.len()]
    );
    for (&position, &bit) in configured
        .encoder()
        .information_positions()
        .iter()
        .zip(message)
    {
        assert_eq!(codeword[position], bit);
    }

    let llrs = strong_llrs(&codeword);
    let result = configured
        .decoder_mut()
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            None,
        )
        .expect("the noiseless LLR block has the expected length");

    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.changed_bits(), 0);
    assert_eq!(result.initial_unsatisfied_checks(), 0);
    assert_eq!(result.final_unsatisfied_checks(), 0);
    assert_eq!(result.word(), codeword);
    assert_eq!(result.syndrome(), vec![Bit::Zero; dense_checks.len()]);
    assert_eq!(
        dense_xor_syndrome(dense_checks, result.word()),
        vec![0; dense_checks.len()]
    );
    assert_eq!(count_bit_errors(result.word(), &codeword), Ok(0));

    let recovered = configured
        .encoder()
        .extract_message(result.word())
        .expect("the decoded word satisfies H");
    assert_eq!(recovered, message);
    assert_eq!(count_bit_errors(&recovered, message), Ok(0));

    let recovered_vector = bits_to_vector(&recovered);
    assert_eq!(recovered_vector.len(), message.len());
    assert_eq!(vector_to_bits(&recovered_vector), Ok(message.to_vec()));
}

#[test]
fn exhaustively_round_trips_every_message_through_the_configured_codec() {
    // Rank 2 of 5: information positions are [2, 3, 4], column 4 is isolated,
    // and the last two rows are respectively dependent and empty.
    let dense_checks = vec![
        vec![1, 0, 0, 1, 0],
        vec![0, 1, 0, 1, 0],
        vec![1, 0, 0, 1, 0],
        vec![0, 0, 0, 0, 0],
    ];
    let mut configured =
        LdpcConfigurator::build(checks_from_dense(&dense_checks), DecoderConfig::default())
            .expect("the constructed matrix has rank strictly between zero and n");
    assert_eq!(configured.encoder().rank(), 2);
    assert_eq!(configured.encoder().message_len(), 3);
    assert_eq!(configured.encoder().codeword_len(), 5);
    assert_eq!(configured.encoder().information_positions(), &[2, 3, 4]);

    for message_mask in 0..(1usize << configured.encoder().message_len()) {
        let message: Vec<_> = (0..configured.encoder().message_len())
            .map(|index| bit_from_bool(message_mask & (1 << index) != 0))
            .collect();
        assert_noiseless_block(&mut configured, &dense_checks, &message);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn generated_codes_are_linear_and_satisfy_an_independent_dense_syndrome(case in codec_case_strategy()) {
        let checks = checks_from_dense(&case.dense_checks);
        prop_assert_eq!(checks.cols(), case.dense_checks[0].len());
        prop_assert_eq!(checks.rows(), case.dense_checks.len());
        prop_assert_eq!(checks.check_bits(case.rank), checks.check_bits(0));
        prop_assert_eq!(checks.check_bits(case.rank + 1), Some(&[][..]));
        prop_assert_eq!(checks.bit_checks(checks.cols() - 1), Some(&[][..]));

        let configured = LdpcConfigurator::build(checks, DecoderConfig::default())
            .expect("the identity block guarantees rank in (0, n)");
        let encoder = configured.encoder();
        prop_assert_eq!(encoder.rank(), case.rank);
        prop_assert_eq!(encoder.message_len(), case.dense_checks[0].len() - case.rank);
        prop_assert!(encoder.information_positions().iter().all(|&position| position > 0));

        let combined_message = xor_bits(&case.first_message, &case.second_message);
        let first_word = encoder.encode(&case.first_message).expect("message length is valid");
        let second_word = encoder.encode(&case.second_message).expect("message length is valid");
        let combined_word = encoder.encode(&combined_message).expect("message length is valid");
        let expected_combined_word = xor_bits(&first_word, &second_word);

        prop_assert_eq!(&combined_word, &expected_combined_word);
        for word in [&first_word, &second_word, &combined_word] {
            prop_assert_eq!(
                dense_xor_syndrome(&case.dense_checks, word),
                vec![0; case.dense_checks.len()]
            );
        }
    }

    #[test]
    fn generated_codes_round_trip_distinct_blocks_through_one_decoder(case in codec_case_strategy()) {
        let checks = checks_from_dense(&case.dense_checks);
        let mut configured = LdpcConfigurator::build(checks, DecoderConfig::default())
            .expect("the identity block guarantees rank in (0, n)");
        prop_assert_eq!(configured.encoder().rank(), case.rank);

        let second_message = complement_bits(&case.first_message);
        prop_assert_ne!(&case.first_message, &second_message);
        assert_noiseless_block(&mut configured, &case.dense_checks, &case.first_message);
        assert_noiseless_block(&mut configured, &case.dense_checks, &second_message);

        // Also exercise the independently generated message in this same decoder instance.
        assert_noiseless_block(&mut configured, &case.dense_checks, &case.second_message);
    }
}
