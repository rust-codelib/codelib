use ldpc_codes::{Bit, Encoder, LdpcError, ParityCheckMatrix, SystematicEncoder};

fn main() -> Result<(), LdpcError> {
    // Четвёртая строка — XOR первых трёх, поэтому ранг равен 3.
    let checks = ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )?;
    let encoder = SystematicEncoder::try_new(checks)?;

    let message = [Bit::Zero, Bit::One, Bit::One];
    let word = encoder.encode(&message)?;

    assert_eq!(encoder.message_len(), 3);
    assert_eq!(encoder.codeword_len(), 6);
    assert_eq!(encoder.rank(), 3);
    assert_eq!(encoder.information_positions(), &[3, 4, 5]);
    assert_eq!(encoder.parity_positions(), &[0, 1, 2]);
    assert_eq!(
        word,
        [
            Bit::Zero,
            Bit::Zero,
            Bit::One,
            Bit::Zero,
            Bit::One,
            Bit::One
        ]
    );
    assert_eq!(encoder.extract_message(&word)?, message);

    Ok(())
}
