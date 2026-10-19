use rm_codes::{count_bit_errors, Bit, RmConfig, RmConfigurator, RmDecodeStatus};

fn bits(text: &str) -> Result<Vec<Bit>, rm_codes::RmError> {
    text.bytes()
        .map(|byte| Bit::try_from(byte - b'0'))
        .collect()
}

fn main() -> Result<(), rm_codes::RmError> {
    let config = RmConfig::new(1, 4)?;
    let code = RmConfigurator::build(config);

    let message = bits("10011")?;
    let encoded = code.encode(&message)?;

    for errors in 0..=3 {
        let mut received = encoded.clone();
        for slot in received.iter_mut().take(errors) {
            *slot = slot.flip();
        }

        let result = code.decode(&received, None)?;
        let remaining = count_bit_errors(result.word(), &encoded)?;

        println!(
            "errors = {errors}, status = {:?}, changed = {}, remaining = {}",
            result.status(),
            result.changed_bits(),
            remaining
        );

        if errors == 0 {
            assert_eq!(result.status(), RmDecodeStatus::Valid);
        } else {
            assert_eq!(result.status(), RmDecodeStatus::Corrected);
        }
        assert_eq!(result.message(), message.as_slice());
        assert_eq!(remaining, 0);
    }

    Ok(())
}
