//! Аксиомы поля для всех степеней m = 2..=16.
//!
//! Малые поля (порядок ≤ 512) проверяются полным перебором пар,
//! большие — детерминированной выборкой; тернарные законы — выборкой
//! троек для всех полей.

use gf2m::{
    Gf, Gf1024, Gf128, Gf16, Gf16384, Gf2048, Gf256, Gf32, Gf32768, Gf4, Gf4096, Gf512, Gf64, Gf8,
    Gf8192,
};

/// Прогон всех аксиом для одного поля.
///
/// Аргументы: имя модуля, тип, порядок поля и список различных
/// простых делителей порядка `N − 1` мультипликативной группы
/// (для проверки того, что α — примитивный элемент).
macro_rules! field_axioms {
    ($mod_name:ident, $gf:ty, $order:expr, [$($p:expr),*]) => {
        mod $mod_name {
            use super::*;

            type F = $gf;
            const ORDER: usize = $order;

            /// Детерминированный LCG по значениям поля.
            fn lcg(s: &mut u32) -> u16 {
                *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                ((*s >> 16) as u16 as usize % ORDER) as u16
            }

            fn each_element(mut f: impl FnMut(F)) {
                for v in 0..ORDER {
                    f(F::new(v as u16));
                }
            }

            fn each_pair(mut f: impl FnMut(F, F)) {
                if ORDER <= 512 {
                    for a in 0..ORDER {
                        for b in 0..ORDER {
                            f(F::new(a as u16), F::new(b as u16));
                        }
                    }
                } else {
                    let mut s: u32 = 0xBEEF;
                    for _ in 0..4096 {
                        let (a, b) = (lcg(&mut s), lcg(&mut s));
                        f(F::new(a), F::new(b));
                    }
                }
            }

            fn each_triple(mut f: impl FnMut(F, F, F)) {
                let mut s: u32 = 0xC0FFEE;
                for _ in 0..1024 {
                    let (a, b, c) = (lcg(&mut s), lcg(&mut s), lcg(&mut s));
                    f(F::new(a), F::new(b), F::new(c));
                }
            }

            #[test]
            fn addition_is_abelian_group() {
                each_element(|a| {
                    assert_eq!(a + F::zero(), a);
                    assert_eq!(a + a, F::zero());
                    assert_eq!(-a, a);
                });
                each_pair(|a, b| assert_eq!(a + b, b + a, "{a} + {b}"));
                each_triple(|a, b, c| {
                    assert_eq!((a + b) + c, a + (b + c), "({a} + {b}) + {c}");
                });
            }

            #[test]
            fn nonzero_form_abelian_group_under_mul() {
                each_element(|a| {
                    if !a.is_zero() {
                        assert_eq!(a * F::one(), a);
                        assert_eq!(a * a.inv(), F::one());
                        assert_eq!(a / a, F::one());
                    }
                });
                each_pair(|a, b| {
                    if !a.is_zero() && !b.is_zero() {
                        assert_eq!(a * b, b * a, "{a} · {b}");
                        assert_eq!(a * b * b.inv(), a);
                        assert_eq!(a / b * b, a);
                    }
                });
                each_triple(|a, b, c| {
                    assert_eq!((a * b) * c, a * (b * c), "({a} · {b}) · {c}");
                });
            }

            #[test]
            fn multiplication_distributes_over_addition() {
                each_triple(|a, b, c| {
                    assert_eq!(a * (b + c), a * b + a * c, "{a} · ({b} + {c})");
                });
            }

            #[test]
            fn frobenius_and_square_roots() {
                each_element(|a| {
                    assert_eq!(a.sqr(), a * a, "{a}^2");
                    assert_eq!(a.sqrt().sqr(), a, "sqrt({a})^2");
                    assert_eq!(a.sqr().sqrt(), a, "sqrt({a}^2)");
                });
            }

            #[test]
            fn trace_is_gf2_valued_and_linear() {
                each_element(|a| {
                    let t = a.trace();
                    assert!(t == F::zero() || t == F::one(), "Tr({a}) = {t}");
                    assert_eq!(a.sqr().trace(), t, "Tr({a}^2)");
                });
                each_pair(|a, b| {
                    assert_eq!((a + b).trace(), a.trace() + b.trace(), "Tr({a} + {b})");
                });
            }

            #[test]
            fn pow_laws() {
                each_element(|a| {
                    assert_eq!(a.pow(0), F::one(), "{a}^0");
                    assert_eq!(a.pow(1), a, "{a}^1");
                    assert_eq!(a.pow(3), a * a * a, "{a}^3");
                    assert_eq!(a.pow(7), a.pow(3) * a.pow(4), "{a}^7");
                });
            }

            #[test]
            fn alpha_is_primitive() {
                let order = (ORDER - 1) as u32;
                let a = F::alpha();
                assert_eq!(a.pow(order), F::one(), "α^{order}");
                $(
                    let d = order / $p;
                    assert_ne!(a.pow(d), F::one(), "α^{d} == 1: порядок α делит {d}");
                )*
            }
        }
    };
}

field_axioms!(gf4, Gf4, 4, [3]);
field_axioms!(gf8, Gf8, 8, [7]);
field_axioms!(gf16, Gf16, 16, [3, 5]);
field_axioms!(gf32, Gf32, 32, [31]);
field_axioms!(gf64, Gf64, 64, [3, 7]);
field_axioms!(gf128, Gf128, 128, [127]);
field_axioms!(gf256, Gf256, 256, [3, 5, 17]);
field_axioms!(gf512, Gf512, 512, [7, 73]);
field_axioms!(gf1024, Gf1024, 1024, [3, 11, 31]);
field_axioms!(gf2048, Gf2048, 2048, [23, 89]);
field_axioms!(gf4096, Gf4096, 4096, [3, 5, 7, 13]);
field_axioms!(gf8192, Gf8192, 8192, [8191]);
field_axioms!(gf16384, Gf16384, 16384, [3, 43, 127]);
field_axioms!(gf32768, Gf32768, 32768, [7, 31, 151]);
field_axioms!(gf65536, Gf<65536, 0x1002D>, 65536, [3, 5, 17, 257]);
