//! Поиск корней локатора и их отображение на битовые позиции.

use crate::{BchStopReason, Gf};

pub(crate) fn error_positions<const N: usize, const POLY: u32>(
    locator: &[Gf<N, POLY>],
    degree: usize,
    transmitted_len: usize,
    positions: &mut [usize],
) -> Result<usize, BchStopReason> {
    let mut roots = 0usize;
    let mut shortened_root = None;
    for position in 0..N - 1 {
        // Λ(α^-j) = 0 означает ошибку коэффициента при x^j.
        let x = Gf::<N, POLY>::alpha().pow(((N - 1 - position) % (N - 1)) as u32);
        let mut value = Gf::zero();
        for i in (0..=degree).rev() {
            value = value * x + locator[i];
        }
        if value.is_zero() {
            if position >= transmitted_len {
                shortened_root = Some(position);
            } else if roots < positions.len() {
                positions[roots] = position;
            }
            roots += 1;
        }
    }
    if let Some(position) = shortened_root {
        return Err(BchStopReason::ShortenedPosition { position });
    }
    if roots != degree || roots > positions.len() {
        return Err(BchStopReason::InvalidLocator { degree, roots });
    }
    Ok(roots)
}
