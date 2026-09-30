use gf2m::{Gf16, Gf256};
use gf_linalg::FieldElement;
use gfpm::{Gf125, Gf25, Gf9};

fn assert_field_basics<F: FieldElement>(zero: F, one: F, nonzero: F) {
    assert_eq!(F::zero(), zero);
    assert_eq!(F::one(), one);
    assert!(F::zero().is_zero());
    assert!(!one.is_zero());
    assert_eq!(nonzero * nonzero.inv(), one);
}

#[test]
fn static_field_adapters_cover_both_parameterized_families() {
    assert_field_basics::<Gf256>(Gf256::zero(), Gf256::one(), Gf256::new(7));
    assert_field_basics::<Gf16>(Gf16::zero(), Gf16::one(), Gf16::new(3));
    assert_field_basics::<Gf9>(Gf9::zero(), Gf9::one(), Gf9::new(3));
    assert_field_basics::<Gf25>(Gf25::zero(), Gf25::one(), Gf25::new(3));
    assert_field_basics::<Gf125>(Gf125::zero(), Gf125::one(), Gf125::new(3));

    // Оба прямых типа проверяют поддержку семейств без именованных псевдонимов.
    type DirectGf2m = gf2m::Gf<64, 0x43>;
    type DirectGfpm = gfpm::Gf<7, 1, 4>;
    assert_field_basics::<DirectGf2m>(DirectGf2m::zero(), DirectGf2m::one(), DirectGf2m::new(3));
    assert_field_basics::<DirectGfpm>(DirectGfpm::zero(), DirectGfpm::one(), DirectGfpm::new(3));
}

#[test]
fn gf9_adapter_preserves_characteristic_three_arithmetic() {
    let one = Gf9::one();
    let two = Gf9::new(2);
    let three = Gf9::new(3);
    let four = Gf9::new(4);

    assert_eq!(one + one, two);
    assert_eq!(one - two, two);
    assert_eq!(-one, two);
    assert_eq!(three * four, one);
    assert_eq!(three.inv(), four);
}

#[test]
fn gf256_adapter_preserves_characteristic_two_arithmetic() {
    let one = Gf256::one();
    let value = Gf256::new(0x57);

    assert_eq!(one + one, Gf256::zero());
    assert_eq!(one - one, Gf256::zero());
    assert_eq!(-value, value);
    assert_eq!(value + value, Gf256::zero());
}
