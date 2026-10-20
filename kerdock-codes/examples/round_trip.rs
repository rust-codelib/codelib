use kerdock_codes::{Kerdock, KerdockError};

fn main() -> Result<(), KerdockError> {
    let codec = Kerdock::try_new(6)?;
    // a = 5 (5 bits), b = 19 (6 bits), c = 1; least-significant bit first.
    let message = [
        true, false, true, false, false, true, true, false, false, true, false, true,
    ];
    let word = codec.encode(&message)?;
    assert_eq!(word.len(), 64);
    assert_eq!(codec.extract_message(&word)?, message);

    let positions = [0, 3, 7, 11, 16, 20, 25, 31, 38, 44, 50, 57, 63];
    assert_eq!(positions.len(), codec.correction_radius());
    let mut received = word.clone();
    for position in positions {
        received[position] = !received[position];
    }

    let decoded = codec.decode(&received)?;
    assert_eq!(decoded.message(), message);
    assert_eq!(decoded.word(), word);
    assert_eq!(decoded.distance(), positions.len());
    println!("Восстановлены 12 битов сообщения; исправлено 13 ошибок в блоке из 64 битов.");
    Ok(())
}
