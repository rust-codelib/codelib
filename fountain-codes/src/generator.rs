use crate::resources::buffer;
use crate::{BlockDescriptor, FountainError, Profile};

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        mix64(self.0)
    }
}
fn sample(k: u64, next: &mut impl FnMut() -> u64) -> usize {
    let threshold = 0u64.wrapping_sub(k) % k;
    loop {
        let r = next();
        if r >= threshold {
            return (r % k) as usize;
        }
    }
}

/// Sorted (source index, field representation) pairs. Zeros are removed without
/// replacement. This low-level pure generator does not apply local draw limits.
pub fn canonical_support(
    desc: &BlockDescriptor,
    id: u64,
) -> Result<Vec<(usize, u16)>, FountainError> {
    desc.check_nonempty()?;
    if id < desc.k as u64 {
        return Ok(vec![(id as usize, 1)]);
    }
    buffer::<usize>(desc.d as usize)?;
    buffer::<(usize, u16)>((desc.d as usize).min(desc.k))?;
    let mut indices = Vec::with_capacity(desc.d as usize);
    let mut index = SplitMix64(mix64(desc.seed ^ mix64(id) ^ 0x524643315f494458));
    let mut coeff = SplitMix64(mix64(desc.seed ^ mix64(id) ^ 0x524643315f434f46));
    for _ in 0..desc.d {
        indices.push(sample(desc.k as u64, &mut || index.next()));
    }
    indices.sort_unstable();
    indices.dedup();
    Ok(indices
        .into_iter()
        .filter_map(|i| {
            let c = match desc.profile {
                Profile::Gf65536 => coeff.next() as u16,
                Profile::Gf256 => (coeff.next() as u8) as u16,
            };
            (c != 0).then_some((i, c))
        })
        .collect())
}

/// Experimental tuning helper; no universal `c` is assumed.
pub fn logarithmic_draws(k: usize, c: f64) -> Result<u16, FountainError> {
    if k > gf_linalg::MAX_MATRIX_DIM || !c.is_finite() || c <= 0.0 {
        return Err(FountainError::InvalidLogarithmicDraws);
    }
    if k <= 1 {
        return Ok(k as u16);
    }
    let d = (c * (k as f64).ln()).ceil();
    if !(1.0..=u16::MAX as f64).contains(&d) {
        return Err(FountainError::InvalidLogarithmicDraws);
    }
    Ok(d as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splitmix_and_rejection_vectors() {
        let mut rng = SplitMix64(0);
        assert_eq!(
            [rng.next(), rng.next(), rng.next()],
            [0xe220a8397b1dcdaf, 0x6e789e6aa1b965f4, 0x06c45d188009454f]
        );
        let mut words = [0, 1, 2, u64::MAX].into_iter();
        assert_eq!(
            [
                sample(3, &mut || words.next().unwrap()),
                sample(3, &mut || words.next().unwrap()),
                sample(3, &mut || words.next().unwrap())
            ],
            [1, 2, 0]
        );
    }
}
