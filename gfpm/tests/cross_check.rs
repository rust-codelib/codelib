//! Сверка с крейтом `gf2m`: для характеристики 2 упаковка элементов
//! gfpm совпадает с битовой из gf2m, поэтому один и тот же модуль
//! даёт одинаковую арифметику. Разница конвенций: gf2m хранит
//! модуль СО старшим битом (0x13 = x^4 + x + 1), gfpm — БЕЗ него
//! (1 + 2 = 3), поэтому константы ниже — это gf2m-полиномы с
//! обнулённым старшим битом. Полный перебор для малых полей,
//! выборка LCG для GF(65536).

use gf2m::{Gf as Gf2m, Gf16, Gf256, Gf4, Gf8};
use gfpm::{Gf, GfRuntime};

macro_rules! cross_full {
    ($name:ident, $gfpm_ty:ty, $gf2m_ty:ty, $n:expr) => {
        #[test]
        fn $name() {
            for a in 0..$n {
                for b in 0..$n {
                    let x = <$gfpm_ty>::new(a);
                    let y = <$gfpm_ty>::new(b);
                    let xr = <$gf2m_ty>::new(a as u16);
                    let yr = <$gf2m_ty>::new(b as u16);
                    assert_eq!(x.value(), u64::from(xr.value()), "value({a})");
                    assert_eq!(
                        (x + y).value(),
                        u64::from((xr + yr).value()),
                        "add({a},{b})"
                    );
                    assert_eq!(
                        (x - y).value(),
                        u64::from((xr - yr).value()),
                        "sub({a},{b})"
                    );
                    assert_eq!(
                        (x * y).value(),
                        u64::from((xr * yr).value()),
                        "mul({a},{b})"
                    );
                    assert_eq!(x.inv().value(), u64::from(xr.inv().value()), "inv({a})");
                    assert_eq!(
                        (x / y).value(),
                        u64::from((xr / yr).value()),
                        "div({a},{b})"
                    );
                }
            }
            for a in 0..$n {
                let x = <$gfpm_ty>::new(a);
                let xr = <$gf2m_ty>::new(a as u16);
                assert_eq!(x.sqr().value(), u64::from(xr.sqr().value()), "sqr({a})");
                assert_eq!(
                    x.sqrt().unwrap().value(),
                    u64::from(xr.sqrt().value()),
                    "sqrt({a})"
                );
                assert_eq!(
                    x.trace().value(),
                    u64::from(xr.trace().value()),
                    "trace({a})"
                );
                assert_eq!(x.pow(7).value(), u64::from(xr.pow(7).value()), "pow({a})");
            }
        }
    };
}

// GF(2^2), модуль gf2m 0x7 = x^2 + x + 1 → gfpm POLY = 3
cross_full!(cross_gf4, Gf<2, 2, 3>, Gf4, 4);
// GF(2^3), модуль gf2m 0xB = x^3 + x + 1 → gfpm POLY = 3
cross_full!(cross_gf8, Gf<2, 3, 3>, Gf8, 8);
// GF(2^4), модуль gf2m 0x13 = x^4 + x + 1 → gfpm POLY = 3
cross_full!(cross_gf16, Gf<2, 4, 3>, Gf16, 16);
// GF(2^8), модуль gf2m 0x11D = x^8 + x^4 + x^3 + x^2 + 1 → gfpm POLY = 0x1D
cross_full!(cross_gf256, Gf<2, 8, 0x1D>, Gf256, 256);

/// GF(2^16), модуль gf2m 0x1002D = x^16 + x^5 + x^3 + x^2 + 1 →
/// gfpm POLY = 0x2D. Выборка пар LCG (полный перебор 2^32 пар
/// избыточен: умножение уже сверено полным перебором
/// на четырёх меньших полях).
#[test]
fn cross_gf65536_sampled() {
    type Pm = Gf<2, 16, 0x2D>;
    type Rr = Gf2m<65536, 0x1002D>;

    let mut s: u64 = 0x0123_4567_89AB_CDEF;
    let mut next = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 48) as u16
    };
    for _ in 0..4096 {
        let a = u64::from(next());
        let b = u64::from(next());
        let x = Pm::new(a);
        let y = Pm::new(b);
        let xr = Rr::new(a as u16);
        let yr = Rr::new(b as u16);
        assert_eq!(
            (x * y).value(),
            u64::from((xr * yr).value()),
            "mul({a},{b})"
        );
        assert_eq!(x.inv().value(), u64::from(xr.inv().value()), "inv({a})");
        assert_eq!(
            (x / y).value(),
            u64::from((xr / yr).value()),
            "div({a},{b})"
        );
        assert_eq!(
            x.sqrt().unwrap().value(),
            u64::from(xr.sqrt().value()),
            "sqrt({a})"
        );
        assert_eq!(
            x.trace().value(),
            u64::from(xr.trace().value()),
            "trace({a})"
        );
    }
}

/// Рантайм-слой gfpm против константного ядра gf2m для GF(256).
#[test]
fn cross_runtime_gf256() {
    let params = gfpm::FieldParams::new(2, 8, 0x1D).unwrap();
    for a in 0..256u64 {
        for b in 0..256u64 {
            let x = GfRuntime::new(params, a).unwrap();
            let y = GfRuntime::new(params, b).unwrap();
            let xr = Gf256::new(a as u16);
            let yr = Gf256::new(b as u16);
            assert_eq!(x.value(), u64::from(xr.value()), "value({a})");
            assert_eq!(
                (x * y).value(),
                u64::from((xr * yr).value()),
                "mul({a},{b})"
            );
            assert_eq!(
                (x + y).value(),
                u64::from((xr + yr).value()),
                "add({a},{b})"
            );
            assert_eq!(x.inv().value(), u64::from(xr.inv().value()), "inv({a})");
        }
    }
}

/// GF(2^4) через gfpm vs gf2m — структурные факты характеристики 2.
#[test]
fn char2_structural_facts() {
    type Pm16 = Gf<2, 4, 3>;
    for a in 0..16u64 {
        let x = Pm16::new(a);
        assert!(x.is_square(), "в GF(2^m) всё — квадраты");
        assert_eq!(x.sqrt().unwrap().sqr(), x);
        let t = x.trace().value();
        assert!(t <= 1, "след ∈ GF(2)");
    }
}
