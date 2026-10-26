//! Вычисление синдромов принятого двоичного слова.

use crate::bits::bit;
use crate::Gf;

pub(crate) fn syndromes<const N: usize, const POLY: u32>(
    received: &[u8],
    n: usize,
    out: &mut [Gf<N, POLY>],
) -> bool {
    let mut all_zero = true;
    for (s, value) in out.iter_mut().enumerate() {
        let x = Gf::<N, POLY>::alpha().pow((s + 1) as u32);
        let mut acc = Gf::zero();
        for i in (0..n).rev() {
            acc = acc * x
                + if bit(received, i) {
                    Gf::one()
                } else {
                    Gf::zero()
                };
        }
        *value = acc;
        all_zero &= acc.is_zero();
    }
    all_zero
}
