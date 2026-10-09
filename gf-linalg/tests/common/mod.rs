use gf2m::Gf256;

pub(crate) fn rectangular_values() -> Vec<Gf256> {
    vec![
        Gf256::new(1),
        Gf256::new(2),
        Gf256::new(3),
        Gf256::new(4),
        Gf256::new(5),
        Gf256::zero(),
    ]
}
