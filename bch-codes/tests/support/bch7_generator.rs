use bch_codes::{build_generator, BchConfig};
use gf2m::Gf8;

pub fn bch7_generator(config: &BchConfig) -> [u8; 1] {
    let mut generator = [0u8; 1];
    let mut workspace = [Gf8::zero(); 32];
    let degree = build_generator::<8, 0xB>(config, &mut generator, &mut workspace).unwrap();
    assert_eq!(degree, 3);
    generator
}
