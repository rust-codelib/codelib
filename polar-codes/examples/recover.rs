use polar_codes::{Bit, Combine, PolarCode, PolarError};

/// Уверенность нестёртого бита. Знак берётся из принятого бита, не из отправленного.
const CONFIDENCE: f64 = 5.0;

fn main() -> Result<(), PolarError> {
    let code = PolarCode::from_bec(8, 4, 0.5)?;
    let message = [Bit::One, Bit::Zero, Bit::One, Bit::One];
    let word = code.encode(&message)?;

    println!("Информационные позиции: {:?}", code.information_positions());
    println!("Отправленное слово:     {:?}", word);

    recover_flipped_bit(&code, &message, &word, 0)?;
    recover_erasures(&code, &message, &word, &[0, 1])?;
    Ok(())
}

fn recover_flipped_bit(
    code: &PolarCode,
    message: &[Bit],
    word: &[Bit],
    index: usize,
) -> Result<(), PolarError> {
    let mut received = word.to_vec();
    received[index] = flip(received[index]);
    let llrs: Vec<f64> = received.iter().copied().map(hard_llr).collect();

    println!();
    println!("Переворот бита {index}: принятый бит противоположен отправленному.");
    println!("LLR = +{CONFIDENCE}, если принят 0, и -{CONFIDENCE}, если принята 1.");
    print_recovery(code, message, word, Some(&received), &llrs)
}

fn recover_erasures(
    code: &PolarCode,
    message: &[Bit],
    word: &[Bit],
    erased: &[usize],
) -> Result<(), PolarError> {
    let mut llrs: Vec<f64> = word.iter().copied().map(hard_llr).collect();
    for &index in erased {
        llrs[index] = 0.0;
    }

    println!();
    println!("Стирания {erased:?}: у стёртого бита нет наблюдения, LLR = 0.");
    print_recovery(code, message, word, None, &llrs)
}

fn print_recovery(
    code: &PolarCode,
    message: &[Bit],
    word: &[Bit],
    received: Option<&[Bit]>,
    llrs: &[f64],
) -> Result<(), PolarError> {
    let decision = code.decode(llrs, Combine::MinSum)?;
    if let Some(received) = received {
        println!("Принятые биты:          {received:?}");
    }
    println!("LLR:                    {llrs:?}");
    println!("Оценка сообщения:       {:?}", decision.message());
    assert_eq!(decision.message(), message);
    assert_eq!(decision.word(), word);
    Ok(())
}

fn hard_llr(bit: Bit) -> f64 {
    match bit {
        Bit::Zero => CONFIDENCE,
        Bit::One => -CONFIDENCE,
    }
}

fn flip(bit: Bit) -> Bit {
    match bit {
        Bit::Zero => Bit::One,
        Bit::One => Bit::Zero,
    }
}
