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

    let received = bits("1001100110011110")?;
    let result = code.decode(&received, None)?;

    println!("received  = {}", render(&received));
    println!("corrected = {}", render(result.word()));
    println!("message   = {}", render(result.message()));
    println!("status    = {:?}", result.status());
    println!("changed   = {}", result.changed_bits());
    println!("positions = {:?}", result.corrected_positions());

    Ok(())
}
