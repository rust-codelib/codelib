use rm_codes::{Bit, RmConfig, RmConfigurator};

fn bits(text: &str) -> Result<Vec<Bit>, rm_codes::RmError> {
    text.bytes()
        .map(|byte| Bit::try_from(byte - b'0'))
        .collect()
}

fn render(bits: &[Bit]) -> String {
    bits.iter()
        .map(|bit| if bit.is_one() { '1' } else { '0' })
        .collect()
}

fn main() -> Result<(), rm_codes::RmError> {
    let config = RmConfig::new(1, 4)?;
    let code = RmConfigurator::build(config);

    let message = bits("10011")?;
    let word = code.encode(&message)?;

    println!("RM({}, {})", config.r(), config.m());
    println!("n = {}, k = {}", config.length(), config.dimension());
    println!("message = {}", render(&message));
    println!("word    = {}", render(&word));

    Ok(())
}
