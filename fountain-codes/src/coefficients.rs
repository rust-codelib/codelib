use crate::FountainError;
use coding_core::MessageBytes;
use gf2m::Gf256;
use gf_linalg::Vector;

/// One GF(256) coefficient per original byte. The existing gf-linalg array
/// conversion preserves length and trailing zeros; this is not a Polynomial.
pub fn message_to_coefficients(message: &[u8]) -> Result<Box<[Gf256]>, FountainError> {
    MessageBytes::try_new(message.len())?;
    Ok(
        Vector::new(message.iter().map(|&b| Gf256::new(b as u16)).collect())
            .try_to_polynomial_coefficients()?,
    )
}
/// Exact inverse for a message-sized GF(256) coefficient array.
pub fn coefficients_to_message(coefficients: &[Gf256]) -> Result<Vec<u8>, FountainError> {
    MessageBytes::try_new(coefficients.len())?;
    Ok(coefficients.iter().map(|x| x.value() as u8).collect())
}
