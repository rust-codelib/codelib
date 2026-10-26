use bch_codes::{correct, BchConfig, BchCorrection, BchDecodeWorkspace};
use gf2m::Gf8;

pub fn correct_bch7(config: &BchConfig, generator: &[u8], received: &mut [u8]) -> BchCorrection {
    let mut field = [Gf8::zero(); 32];
    let mut positions = [0usize; 4];
    let mut workspace = BchDecodeWorkspace {
        field: &mut field,
        positions: &mut positions,
    };
    correct::<8, 0xB>(config, generator, received, &mut workspace).unwrap()
}
