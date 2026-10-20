mod support;
use fountain_codes::*;
fn main() -> Result<(), FountainError> {
    let encoder = support::encoder()?;
    // The receiver has only parity IDs 3,4,5. All source IDs 0,1,2 were lost.
    let parts = [3, 4, 5]
        .map(|id| encoder.symbol_unobserved(id))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut decoder =
        RepairableDecoder::try_new(encoder.descriptor().clone(), ResourceLimits::default())?;
    for symbol in &parts {
        decoder.push_unobserved(symbol)?;
    }
    let expected = [
        0x80, 0x01, 0xff, 0x00, 0x34, 0x12, 0x00, 0x80, 0xaa, 0x55, 0xcc,
    ];
    assert_eq!(decoder.message(), Some(expected.as_slice())); // Checking code only.
    println!(
        "lost=[0,1,2], available=[3,4,5], rank={}, recovered={:02x?}, original comparison=OK",
        decoder.rank(),
        decoder.message().unwrap()
    );
    Ok(())
}
