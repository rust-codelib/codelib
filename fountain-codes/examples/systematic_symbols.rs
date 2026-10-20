mod support;
use fountain_codes::*;
fn main() -> Result<(), FountainError> {
    let encoder = support::encoder()?;
    for id in 0..encoder.descriptor().source_symbols() as u64 {
        let symbol = encoder.symbol_unobserved(id)?;
        let wire = symbol.to_wire()?;
        assert_eq!(parse_symbol(&wire)?, symbol);
        println!(
            "source ID={id}, payload={:02x?}, wire={} bytes",
            symbol.payload(),
            wire.len()
        );
    }
    Ok(())
}
