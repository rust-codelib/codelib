use ldpc_codes::{spa_step, Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

fn main() -> Result<(), LdpcError> {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
    let channel = [2.0, -1.0, 3.0];

    let result = spa_step(
        &checks,
        DecoderConfig::try_new(0, 20.0)?,
        DecodeInput {
            llrs: &channel,
            erasures: &[],
        },
    )?;

    assert_eq!(result.posterior_llrs(), &[1.0, 4.0, 2.0]);
    assert_eq!(result.word(), &[Bit::Zero, Bit::Zero, Bit::Zero]);
    assert_eq!(checks.syndrome(result.word())?, vec![Bit::Zero, Bit::Zero]);

    println!("posterior LLR: {:?}", result.posterior_llrs());
    println!("word: {:?}", result.word());
    println!("syndrome: {:?}", checks.syndrome(result.word())?);
    Ok(())
}
