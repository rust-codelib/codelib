//! Свойства полей на случайных элементах (proptest):
//! законы поля, согласованность степеней, фробениус, след, норма.

use gfpm::{FieldParams, FiniteField, Gf25, Gf65537, Gf81, GfRuntime};
use proptest::prelude::*;

proptest! {
    /// GF(3^4): ассоциативность, дистрибутивность, обратные,
    /// законы степеней на случайных элементах.
    #[test]
    fn gf81_field_laws(
        va in 0u64..81, vb in 0u64..81, vc in 0u64..81, k in 0u64..200
    ) {
        let one = Gf81::one();
        let a = one.from_int(va);
        let b = one.from_int(vb);
        let c = one.from_int(vc);
        prop_assert_eq!((a + b) + c, a + (b + c));
        prop_assert_eq!((a * b) * c, a * (b * c));
        prop_assert_eq!(a * (b + c), a * b + a * c);
        prop_assert_eq!(a - b + b, a);
        prop_assert_eq!(-(-a), a);
        prop_assert_eq!(a - a, a.zero());
        if !b.is_zero() {
            prop_assert_eq!(a * b / b, a);
            prop_assert_eq!(b.inv() * b, one);
        }
        // законы степеней (с приведением показателя по модулю 80)
        prop_assert_eq!(a.pow(k) * a, a.pow(k + 1));
        prop_assert_eq!(a.pow(k).pow(3), a.pow(k * 3));
        if !a.is_zero() && !c.is_zero() {
            prop_assert_eq!((a * c).pow(k), a.pow(k) * c.pow(k));
        }
        // фробениус — гомоморфизм
        prop_assert_eq!((a + b).frobenius(), a.frobenius() + b.frobenius());
        prop_assert_eq!((a * b).frobenius(), a.frobenius() * b.frobenius());
        prop_assert_eq!(a.frobenius().frobenius().frobenius().frobenius(), a);
        // след линеен, норма мультипликативна, обе лежат в GF(3)
        prop_assert_eq!((a + b).trace(), a.trace() + b.trace());
        prop_assert_eq!((a * b).norm(), a.norm() * b.norm());
        prop_assert!(a.trace().to_int() < 3);
        prop_assert!(a.norm().to_int() < 3);
        // корни
        prop_assert_eq!(a.sqrt().map(|r| r.sqr()), a.is_square().then_some(a));
        if let Some(r) = a.sqrt() {
            prop_assert_eq!(r.sqr(), a);
        }
    }

    /// GF(5^2): то же самое над другим полем.
    #[test]
    fn gf25_field_laws(
        va in 0u64..25, vb in 0u64..25, vc in 0u64..25, k in 0u64..200
    ) {
        let one = Gf25::one();
        let a = one.from_int(va);
        let b = one.from_int(vb);
        let c = one.from_int(vc);
        prop_assert_eq!((a + b) + c, a + (b + c));
        prop_assert_eq!((a * b) * c, a * (b * c));
        prop_assert_eq!(a * (b + c), a * b + a * c);
        if !b.is_zero() {
            prop_assert_eq!(a * b / b, a);
        }
        prop_assert_eq!(a.pow(k) * a, a.pow(k + 1));
        prop_assert_eq!((a + b).frobenius(), a.frobenius() + b.frobenius());
        prop_assert_eq!((a * b).norm(), a.norm() * b.norm());
        // x^(q) = x для всех x
        prop_assert_eq!(a.pow(25), a);
    }

    /// Большое простое Ферма GF(65537).
    #[test]
    fn gf65537_field_laws(
        va in 0u64..65537, vb in 0u64..65537, k in 0u64..1000
    ) {
        let one = Gf65537::one();
        let a = one.from_int(va);
        let b = one.from_int(vb);
        prop_assert_eq!(a * b, b * a);
        prop_assert_eq!((a - b) + b, a);
        if !b.is_zero() {
            prop_assert_eq!(a * b / b, a);
            prop_assert_eq!(b.inv() * b, one);
        }
        prop_assert_eq!(a.pow(k) * a, a.pow(k + 1));
        prop_assert_eq!(a.pow(65537), a);
        prop_assert_eq!(a.trace(), a); // M = 1
        prop_assert_eq!(a.norm(), a);
        // корни согласованы
        if let Some(r) = a.sqrt() {
            prop_assert_eq!(r.sqr(), a);
        } else {
            prop_assert!(!a.is_zero());
            prop_assert!(!a.is_square());
        }
    }

    /// Рантайм-слой GF(5^3) с найденным модулем x^3 + x + 1.
    #[test]
    fn runtime_gf125_field_laws(
        va in 0u64..125, vb in 0u64..125, vc in 0u64..125, k in 0u64..200
    ) {
        let params = FieldParams::find(5, 3).unwrap();
        let one = GfRuntime::one(params);
        let a = one.from_int(va);
        let b = one.from_int(vb);
        let c = one.from_int(vc);
        prop_assert_eq!((a + b) + c, a + (b + c));
        prop_assert_eq!((a * b) * c, a * (b * c));
        prop_assert_eq!(a * (b + c), a * b + a * c);
        if !b.is_zero() {
            prop_assert_eq!(a * b / b, a);
            prop_assert_eq!(b.inv() * b, one);
        }
        prop_assert_eq!(a.pow(k) * a, a.pow(k + 1));
        prop_assert_eq!(a.pow(125), a);
        prop_assert_eq!((a + b).frobenius(), a.frobenius() + b.frobenius());
        prop_assert_eq!((a * b).norm(), a.norm() * b.norm());
        prop_assert!(a.trace().to_int() < 5);
        prop_assert!(a.norm().to_int() < 5);
    }
}
