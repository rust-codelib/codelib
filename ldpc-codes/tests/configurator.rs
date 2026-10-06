use ldpc_codes::{
    Bit, DecodeInput, DecodeStatus, Decoder, DecoderConfig, Encoder, LdpcConfigurator, LdpcError,
    ParityCheckMatrix,
};

fn checks_with_dependent_and_empty_rows() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(5, vec![vec![0, 1, 4], vec![2, 3, 4], vec![0, 1, 4], vec![]])
        .expect("the parity-check matrix is valid")
}

#[test]
fn build_derives_dimensions_and_nonleading_information_positions_from_rank() {
    let checks = checks_with_dependent_and_empty_rows();
    let expected_checks = checks.clone();
    let mut configured = LdpcConfigurator::build(checks, DecoderConfig::default())
        .expect("rank two gives a valid three-bit message");

    let encoder = configured.encoder();
    assert_eq!(encoder.rank(), 2);
    assert_eq!(encoder.message_len(), 3);
    assert_eq!(encoder.codeword_len(), 5);
    assert_eq!(encoder.information_positions(), &[1, 3, 4]);
    assert_eq!(encoder.parity_positions(), &[0, 2]);

    let message = [Bit::One, Bit::Zero, Bit::Zero];
    let codeword = encoder.encode(&message).expect("message has length k");
    assert_eq!(
        codeword,
        [Bit::One, Bit::One, Bit::Zero, Bit::Zero, Bit::Zero]
    );
    assert_eq!(expected_checks.syndrome(&codeword).unwrap(), [Bit::Zero; 4]);
    assert_eq!(encoder.extract_message(&codeword), Ok(message.to_vec()));

    let llrs: Vec<_> = codeword
        .iter()
        .map(|bit| match bit {
            Bit::Zero => 3.0,
            Bit::One => -3.0,
        })
        .collect();
    let result = configured
        .decoder_mut()
        .decode(
            DecodeInput {
                llrs: &llrs,
                erasures: &[],
            },
            None,
        )
        .expect("a valid noiseless block can be decoded");

    assert_eq!(result.word(), codeword);
    assert_eq!(result.posterior_llrs(), llrs);
    assert_eq!(result.syndrome(), [Bit::Zero; 4]);
    assert_eq!(result.status(), DecodeStatus::ParitySatisfied);
    assert_eq!(result.iterations(), 0);
    assert_eq!(
        configured.encoder().extract_message(result.word()),
        Ok(message.to_vec())
    );
}

#[test]
fn decoder_syndrome_keeps_duplicate_and_empty_source_rows_in_order() {
    let mut configured = LdpcConfigurator::build(
        checks_with_dependent_and_empty_rows(),
        DecoderConfig::try_new(0, 20.0).expect("the LLR limit is valid"),
    )
    .expect("dependent and empty checks are allowed");

    let result = configured
        .decoder_mut()
        .decode(
            DecodeInput {
                llrs: &[-1.0, 1.0, 1.0, 1.0, 1.0],
                erasures: &[],
            },
            None,
        )
        .expect("the input has the expected length");

    assert_eq!(
        result.word(),
        [Bit::One, Bit::Zero, Bit::Zero, Bit::Zero, Bit::Zero]
    );
    assert_eq!(
        result.syndrome(),
        [Bit::One, Bit::Zero, Bit::One, Bit::Zero]
    );
    assert_eq!(result.initial_unsatisfied_checks(), 2);
    assert_eq!(result.final_unsatisfied_checks(), 2);
    assert_eq!(result.status(), DecodeStatus::IterationLimit);
    assert_eq!(result.iterations(), 0);
}

#[test]
fn build_rejects_zero_and_full_rank_using_encoder_rank_errors() {
    let zero_rank = ParityCheckMatrix::try_from_rows(3, vec![vec![]]).unwrap();
    assert!(matches!(
        LdpcConfigurator::build(zero_rank, DecoderConfig::default()),
        Err(LdpcError::InvalidEncoderRank { rank: 0, cols: 3 })
    ));

    let full_rank = ParityCheckMatrix::try_from_rows(3, vec![vec![0], vec![1], vec![2]])
        .expect("identity checks are valid");
    assert!(matches!(
        LdpcConfigurator::build(full_rank, DecoderConfig::default()),
        Err(LdpcError::InvalidEncoderRank { rank: 3, cols: 3 })
    ));
}

#[test]
fn parity_check_clone_owns_independent_adjacency_storage() {
    let checks = checks_with_dependent_and_empty_rows();
    let cloned = checks.clone();

    assert_eq!(cloned.rows(), checks.rows());
    assert_eq!(cloned.cols(), checks.cols());
    assert_eq!(cloned.edge_count(), checks.edge_count());
    for row in 0..checks.rows() {
        assert_eq!(cloned.check_bits(row), checks.check_bits(row));
    }
    for bit in 0..checks.cols() {
        assert_eq!(cloned.bit_checks(bit), checks.bit_checks(bit));
    }

    assert_ne!(
        checks.check_bits(0).unwrap().as_ptr(),
        cloned.check_bits(0).unwrap().as_ptr()
    );
    assert_ne!(
        checks.bit_checks(0).unwrap().as_ptr(),
        cloned.bit_checks(0).unwrap().as_ptr()
    );

    drop(checks);
    assert_eq!(cloned.check_bits(2), Some(&[0, 1, 4][..]));
    assert_eq!(cloned.check_bits(3), Some(&[][..]));
    assert_eq!(cloned.bit_checks(0), Some(&[0, 2][..]));
}
