//! Property-based тесты аксиом поля (proptest).

use gf2m::{Gf16, Gf256, GfRuntime};
use proptest::prelude::*;

fn gf256_elem() -> impl Strategy<Value = Gf256> {
    (0u16..256).prop_map(Gf256::new)
}

proptest! {
    #[test]
    fn mul_is_commutative(a in gf256_elem(), b in gf256_elem()) {
        prop_assert_eq!(a * b, b * a);
    }

    #[test]
    fn mul_is_associative(a in gf256_elem(), b in gf256_elem(), c in gf256_elem()) {
        prop_assert_eq!((a * b) * c, a * (b * c));
    }

    #[test]
    fn mul_distributes_over_add(a in gf256_elem(), b in gf256_elem(), c in gf256_elem()) {
        prop_assert_eq!(a * (b + c), a * b + a * c);
    }

    #[test]
    fn inv_is_involution(a in gf256_elem()) {
        prop_assert_eq!(a.inv().inv(), a);
        prop_assert_eq!(a * a.inv() * a, a);
    }

    #[test]
    fn div_undo_mul(a in gf256_elem(), b in 1u16..256) {
        let b = Gf256::new(b);
        prop_assert_eq!((a / b) * b, a);
    }

    #[test]
    fn sqrt_undo_sqr(a in gf256_elem()) {
        prop_assert_eq!(a.sqr().sqrt(), a);
    }

    #[test]
    fn pow_adds_exponents(a in gf256_elem(), i in 0u32..300, j in 0u32..300) {
        prop_assert_eq!(a.pow(i) * a.pow(j), a.pow(i + j));
    }

    #[test]
    fn trace_is_linear_gf16(a in 0u16..16, b in 0u16..16) {
        let (a, b) = (Gf16::new(a), Gf16::new(b));
        prop_assert_eq!((a + b).trace(), a.trace() + b.trace());
    }
}

fn runtime_elem() -> impl Strategy<Value = GfRuntime> {
    (2u32..=16).prop_flat_map(|m| {
        (0u32..(1u32 << m)).prop_map(move |v| GfRuntime::new(m, v as u16).unwrap())
    })
}

proptest! {
    #[test]
    fn runtime_inv_roundtrip(x in runtime_elem()) {
        prop_assert_eq!(x * x.inv() * x, x);
    }

    #[test]
    fn runtime_sqrt_undo_sqr(x in runtime_elem()) {
        prop_assert_eq!(x.sqr().sqrt(), x);
    }

    #[test]
    fn runtime_display_roundtrip(x in runtime_elem()) {
        let s = x.to_string();
        prop_assert!(s.starts_with("0x"));
        prop_assert_eq!(u16::from_str_radix(&s[2..], 16).unwrap(), x.value());
    }
}
