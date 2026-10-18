//! Контракт поля, над которым строится каскадный код.

use gf_linalg::FieldElement;

/// Конечное поле с нумерацией элементов.
///
/// Кроме арифметики [`FieldElement`] коду нужны порядок поля `q` и биекция
/// между элементами и числами `0..q`. Нумерация задаёт точки матрицы
/// Вандермонда (узел `i` получает точку с номером `i`) и упаковку байтов в
/// символы. Номер нуля поля должен быть равен `0`.
///
/// Реализации предоставлены для `gf2m::Gf<N, POLY>` и `gfpm::Gf<P, M, POLY>`.
pub trait CodeField: FieldElement {
    /// Порядок поля `q`.
    fn order() -> u128;

    /// Элемент с номером `index` или `None`, если `index >= q`.
    fn from_index(index: u64) -> Option<Self>;

    /// Номер элемента в диапазоне `0..q`.
    fn to_index(self) -> u64;
}

impl<const N: usize, const POLY: u32> CodeField for gf2m::Gf<N, POLY> {
    fn order() -> u128 {
        let _ = Self::M; // проверка параметров типа
        N as u128
    }

    fn from_index(index: u64) -> Option<Self> {
        if (index as u128) < Self::order() {
            u16::try_from(index).ok().map(Self::new)
        } else {
            None
        }
    }

    fn to_index(self) -> u64 {
        u64::from(self.value())
    }
}

impl<const P: u32, const M: usize, const POLY: u128> CodeField for gfpm::Gf<P, M, POLY> {
    fn order() -> u128 {
        Self::ORDER
    }

    fn from_index(index: u64) -> Option<Self> {
        if (index as u128) < Self::ORDER {
            Some(Self::new(index))
        } else {
            None
        }
    }

    fn to_index(self) -> u64 {
        self.value()
    }
}

/// `(-1)^neg · value`.
pub(crate) fn signed<F: FieldElement>(neg: bool, value: F) -> F {
    if neg {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gf2m::Gf256;
    use gfpm::{Gf13, Gf9};

    fn round_trip<F: CodeField>() {
        let q = F::order() as u64;
        for i in 0..q {
            let x = F::from_index(i).expect("index inside field");
            assert_eq!(x.to_index(), i);
        }
        assert_eq!(F::from_index(q), None);
        assert_eq!(F::from_index(0), Some(F::zero()));
    }

    #[test]
    fn indices_are_bijective() {
        round_trip::<Gf256>();
        round_trip::<Gf13>();
        round_trip::<Gf9>();
        assert_eq!(Gf256::order(), 256);
        assert_eq!(Gf9::order(), 9);
    }

    #[test]
    fn signed_negates() {
        let x = Gf13::new(5);
        assert_eq!(signed(true, x), Gf13::new(8));
        assert_eq!(signed(false, x), x);
        // В характеристике 2 знак исчезает.
        let y = Gf256::new(7);
        assert_eq!(signed(true, y), y);
    }
}
