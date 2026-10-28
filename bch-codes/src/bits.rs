//! Упакованное двоичное представление полиномов.

pub(crate) const fn bytes_for(bits: usize) -> usize {
    bits.div_ceil(8)
}

pub(crate) fn bit(data: &[u8], index: usize) -> bool {
    data[index / 8] & (1 << (index % 8)) != 0
}

pub(crate) fn toggle(data: &mut [u8], index: usize) {
    data[index / 8] ^= 1 << (index % 8);
}

pub(crate) fn set(data: &mut [u8], index: usize, value: bool) {
    let mask = 1 << (index % 8);
    if value {
        data[index / 8] |= mask;
    } else {
        data[index / 8] &= !mask;
    }
}

pub(crate) fn valid_padding(data: &[u8], bits: usize) -> bool {
    if bits == 0 || bits % 8 == 0 {
        return true;
    }
    data.last()
        .is_some_and(|v| *v & !((1u8 << (bits % 8)) - 1) == 0)
}
