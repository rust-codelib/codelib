use gf2m::Gf256;
use gf_linalg::{Matrix, Vector};

pub fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

pub fn gf2m() -> Result<(), String> {
    let a = Gf256::new(0x53);
    check(a * a.inv() == Gf256::one(), "GF256 inverse")?;
    check(
        (Gf256::new(0x80) * Gf256::new(2)).value() == 0x1d,
        "GF256 multiplication",
    )?;
    check(gf2m::clmul(3, 3) == 5, "GF2 polynomial multiplication")
}

pub fn gfpm() -> Result<(), String> {
    use gfpm::Gf9;
    check(
        Gf9::new(5) * Gf9::new(3) == Gf9::new(4),
        "GF9 multiplication",
    )?;
    check(Gf9::new(5) * Gf9::new(5).inv() == Gf9::one(), "GF9 inverse")
}

pub fn linalg() -> Result<(), String> {
    let matrix = Matrix::<Gf256>::try_new(
        2,
        2,
        vec![Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::new(4)],
    )
    .map_err(|e| e.to_string())?;
    let product = matrix
        .try_mul_vector(&Vector::new(vec![Gf256::one(), Gf256::new(2)]))
        .map_err(|e| e.to_string())?;
    check(
        product.as_slice() == [Gf256::new(5), Gf256::new(11)],
        "matrix/vector product",
    )?;
    let inverse = matrix.try_inverse().map_err(|e| e.to_string())?;
    let identity = matrix.try_mul(&inverse).map_err(|e| e.to_string())?;
    check(
        identity.as_slice() == [Gf256::one(), Gf256::zero(), Gf256::zero(), Gf256::one()],
        "matrix inverse",
    )
}

pub fn rs_config() -> Result<(), String> {
    let config = rs_codes::RsConfig::new(4, 0).map_err(|e| e.to_string())?;
    check(config.t() == 2, "RS correction bound")?;
    check(
        rs_codes::RsConfig::new(0, 0).is_err(),
        "RS invalid configuration",
    )
}
