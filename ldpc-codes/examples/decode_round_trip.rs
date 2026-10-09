use ldpc_codes::{
    bits_to_vector, count_bit_errors, vector_to_bits, Bit, DecodeInput, DecodeStatus,
    DecoderConfig, Encoder, LdpcConfigurator, LdpcError, ParityCheckMatrix,
};

fn main() -> Result<(), LdpcError> {
    let source_bits = [Bit::One];
    let source_vector = bits_to_vector(&source_bits);
    let message = vector_to_bits(&source_vector)?;

    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
    let mut configured = LdpcConfigurator::build(checks, DecoderConfig::try_new(1, 20.0)?)?;
    let encoded_word = configured.encoder().encode(&message)?;
    let llrs = [-2.0, 1.0, -3.0];
    let result = configured.decode(
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
        None,
    )?;

    println!("Статус декодирования: {:?}", result.status());
    println!("Итоговое слово: {:?}", result.word());
    println!("Число итераций: {}", result.iterations());
    println!(
        "ParitySatisfied подтверждает выполнение H, но не гарантирует совпадение с переданным словом."
    );
    if result.status() != DecodeStatus::ParitySatisfied {
        println!("Синдром ненулевой; сообщение не извлекается.");
        return Ok(());
    }

    let recovered_message = configured.encoder().extract_message(result.word())?;
    let recovered_vector = bits_to_vector(&recovered_message);
    let word_errors = count_bit_errors(result.word(), &encoded_word)?;
    let message_errors = count_bit_errors(&recovered_message, &message)?;

    println!("Сообщение до передачи: {:?}", source_vector.as_slice());
    println!(
        "Сообщение после декодирования: {:?}",
        recovered_vector.as_slice()
    );
    println!("Ошибок кодового слова (n = 3): {word_errors}");
    println!("Ошибок сообщения (k = 1): {message_errors}");

    assert_eq!(result.word(), encoded_word);
    assert_eq!(recovered_vector.as_slice(), source_vector.as_slice());
    Ok(())
}
