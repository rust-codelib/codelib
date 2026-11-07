use polar_codes::{Bit, Combine, PolarCode, PolarError};

fn main() -> Result<(), PolarError> {
    let code = PolarCode::from_bec(8, 4, 0.5)?;
    let message = [Bit::One, Bit::Zero, Bit::One, Bit::One];
    let word = code.encode(&message)?;
    let llrs: Vec<f64> = word
        .iter()
        .map(|bit| match bit {
            Bit::Zero => 5.0,
            Bit::One => -5.0,
        })
        .collect();

    let decision = code.decode(&llrs, Combine::MinSum)?;
    println!("Информационные позиции: {:?}", code.information_positions());
    println!("Кодовое слово: {:?}", word);
    println!("Оценка сообщения: {:?}", decision.message());
    assert_eq!(decision.message(), &message);
    assert_eq!(decision.word(), word.as_slice());
    Ok(())
}
