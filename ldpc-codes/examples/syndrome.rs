use ldpc_codes::{bits_to_vector, vector_to_bits, Bit, LdpcError, ParityCheckMatrix};

fn main() -> Result<(), LdpcError> {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 2], vec![1, 2]])?;

    let word_with_unsatisfied_check = [Bit::One, Bit::Zero, Bit::Zero];
    assert_eq!(
        checks.syndrome(&word_with_unsatisfied_check)?,
        vec![Bit::One, Bit::Zero]
    );
    assert!(!checks.is_codeword(&word_with_unsatisfied_check)?);

    let codeword = [Bit::One, Bit::One, Bit::One];
    assert_eq!(checks.syndrome(&codeword)?, vec![Bit::Zero, Bit::Zero]);
    assert!(checks.is_codeword(&codeword)?);

    let bits = [Bit::One, Bit::Zero, Bit::One, Bit::Zero];
    let vector = bits_to_vector(&bits);
    assert_eq!(vector.len(), bits.len());
    assert_eq!(vector_to_bits(&vector)?, bits);

    Ok(())
}
