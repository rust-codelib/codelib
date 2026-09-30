//! Перекрёстная проверка слоёв: рантайм против константно-генерикового
//! ядра на всех степенях m = 2..=16.

use gf2m::GfRuntime;

/// Сравнение всех операций рантайм-слоя с соответствующим
/// инстанцированием константного ядра.
macro_rules! cross_field {
    ($name:ident, $m:expr, $gf:ty) => {
        #[test]
        fn $name() {
            let m: u32 = $m;
            type F = $gf;
            let order: u32 = 1 << m;
            let mut s: u32 = 0xDEAD_BEEF;
            let mut next = || {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                if m < 16 {
                    ((s >> 16) as u16) & (order as u16 - 1)
                } else {
                    (s >> 16) as u16
                }
            };
            for _ in 0..2048 {
                let (av, bv) = (next(), next());
                let a = GfRuntime::new(m, av).unwrap();
                let b = GfRuntime::new(m, bv).unwrap();
                let (ca, cb) = (F::new(av), F::new(bv));

                assert_eq!(a.add(b).value(), ca.add(cb).value(), "add m = {m}");
                assert_eq!(a.mul(b).value(), ca.mul(cb).value(), "mul m = {m}");
                assert_eq!(a.inv().value(), ca.inv().value(), "inv m = {m}");
                assert_eq!(a.div(b).value(), ca.div(cb).value(), "div m = {m}");
                assert_eq!(a.sqr().value(), ca.sqr().value(), "sqr m = {m}");
                assert_eq!(a.sqrt().value(), ca.sqrt().value(), "sqrt m = {m}");

                let k = (av as u32) % 97;
                assert_eq!(a.pow(k).value(), ca.pow(k).value(), "pow m = {m}");
                assert_eq!(a.trace().value(), ca.trace().value(), "trace m = {m}");

                // цех-логарифм: self трактуется как показатель k
                let kk = (bv as u32) % (order - 1);
                let zr = GfRuntime::new(m, kk as u16).unwrap().zech();
                let zc = F::zech(kk);
                assert_eq!(
                    zr.map(|x| x.value()),
                    zc.map(|x| x.value()),
                    "zech m = {m}, k = {kk}"
                );
            }
        }
    };
}

cross_field!(cross_gf4, 2, gf2m::Gf4);
cross_field!(cross_gf8, 3, gf2m::Gf8);
cross_field!(cross_gf16, 4, gf2m::Gf16);
cross_field!(cross_gf32, 5, gf2m::Gf32);
cross_field!(cross_gf64, 6, gf2m::Gf64);
cross_field!(cross_gf128, 7, gf2m::Gf128);
cross_field!(cross_gf256, 8, gf2m::Gf256);
cross_field!(cross_gf512, 9, gf2m::Gf512);
cross_field!(cross_gf1024, 10, gf2m::Gf1024);
cross_field!(cross_gf2048, 11, gf2m::Gf2048);
cross_field!(cross_gf4096, 12, gf2m::Gf4096);
cross_field!(cross_gf8192, 13, gf2m::Gf8192);
cross_field!(cross_gf16384, 14, gf2m::Gf16384);
cross_field!(cross_gf32768, 15, gf2m::Gf32768);
cross_field!(cross_gf65536, 16, gf2m::Gf<65536, 0x1002D>);
