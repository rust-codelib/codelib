mod support;
use coding_core::IncrementalDecoder;
use fountain_codes::*;
fn main() -> Result<(), FountainError> {
    let encoder = support::encoder()?;
    let mut decoder =
        RepairableDecoder::try_new(encoder.descriptor().clone(), ResourceLimits::default())?;
    for id in [3, 3, 4, 5, 6] {
        decoder.push(
            &encoder.symbol_unobserved(id)?,
            &mut |event: &DecodeEvent| println!("{event:?}"),
        )?;
    }
    assert_eq!(
        decoder.message(),
        Some([0x80, 0x01, 0xff, 0x00, 0x34, 0x12, 0x00, 0x80, 0xaa, 0x55, 0xcc].as_slice())
    );
    Ok(())
}
