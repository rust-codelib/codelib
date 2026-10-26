use bch_codes::{build_generator, BchConfig};
use gf2m::Gf16;

pub fn bch15_generator(config: &BchConfig) -> [u8; 2] {
    let mut generator = [0u8; 2];
    let mut workspace = [Gf16::zero(); 96];
    let degree = build_generator::<16, 0x13>(config, &mut generator, &mut workspace).unwrap();
    assert_eq!(degree, 8);
    generator
}
