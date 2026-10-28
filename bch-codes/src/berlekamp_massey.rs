//! Построение полинома локаторов алгоритмом Берлекампа—Мэсси.

use crate::Gf;

pub(crate) fn locator<const N: usize, const POLY: u32>(
    syndromes: &[Gf<N, POLY>],
    current: &mut [Gf<N, POLY>],
    previous: &mut [Gf<N, POLY>],
    temporary: &mut [Gf<N, POLY>],
) -> usize {
    let t = current.len() - 1;
    current.fill(Gf::zero());
    previous.fill(Gf::zero());
    temporary.fill(Gf::zero());
    current[0] = Gf::one();
    previous[0] = Gf::one();
    let mut degree = 0usize;
    let mut shift = 1usize;
    let mut last_discrepancy = Gf::one();
    for step in 0..2 * t {
        let mut discrepancy = syndromes[step];
        for i in 1..=degree {
            discrepancy += current[i] * syndromes[step - i];
        }
        if discrepancy.is_zero() {
            shift += 1;
            continue;
        }
        temporary.copy_from_slice(current);
        let factor = discrepancy / last_discrepancy;
        if shift <= t {
            for j in 0..=t - shift {
                current[j + shift] += factor * previous[j];
            }
        }
        if 2 * degree <= step {
            degree = step + 1 - degree;
            previous.copy_from_slice(temporary);
            last_discrepancy = discrepancy;
            shift = 1;
        } else {
            shift += 1;
        }
    }
    degree
}
