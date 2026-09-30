//! Рантайм-слой: ошибки конструирования, поиск неприводимых,
//! согласованность с константным ядром, форматирование.

use gfpm::{FieldParams, Gf, GfError, GfRuntime};

#[test]
fn params_validation_errors() {
    // не простое
    assert_eq!(FieldParams::new(4, 1, 0), Err(GfError::NotPrime(4)));
    assert_eq!(FieldParams::new(1, 1, 0), Err(GfError::NotPrime(1)));
    assert_eq!(
        FieldParams::new(2147483649, 1, 0),
        Err(GfError::NotPrime(2147483649))
    );
    // степень
    assert_eq!(
        FieldParams::new(3, 0, 0),
        Err(GfError::UnsupportedDegree(0))
    );
    assert_eq!(
        FieldParams::new(3, 65, 0),
        Err(GfError::UnsupportedDegree(65))
    );
    // порядок
    assert_eq!(
        FieldParams::new(3, 41, 0),
        Err(GfError::OrderTooLarge { p: 3, m: 41 })
    );
    assert_eq!(
        FieldParams::new(2147483647, 3, 0),
        Err(GfError::OrderTooLarge {
            p: 2147483647,
            m: 3
        })
    );
    // структура модуля
    assert_eq!(
        FieldParams::new(3, 2, 100),
        Err(GfError::InvalidPolynomial {
            p: 3,
            m: 2,
            poly: 100
        })
    );
    assert_eq!(
        FieldParams::new(3, 2, 9),
        Err(GfError::InvalidPolynomial {
            p: 3,
            m: 2,
            poly: 9
        })
    );
    // нулевой модуль: при m >= 2 это x^m (приводимый),
    // при m = 1 — модуль x с нулевым alpha()
    assert_eq!(
        FieldParams::new(3, 2, 0),
        Err(GfError::InvalidPolynomial {
            p: 3,
            m: 2,
            poly: 0
        })
    );
    assert_eq!(
        FieldParams::new(5, 1, 0),
        Err(GfError::InvalidPolynomial {
            p: 5,
            m: 1,
            poly: 0
        })
    );
    // Display ошибок
    assert_eq!(
        GfError::NotPrime(4).to_string(),
        "характеристика 4 не является простым числом"
    );
    assert_eq!(
        GfError::UnsupportedDegree(65).to_string(),
        "неподдерживаемая степень расширения: m = 65, допустимо 1..=64"
    );
    assert_eq!(
        GfError::OrderTooLarge { p: 3, m: 41 }.to_string(),
        "порядок поля 3^41 превосходит 2^64"
    );
    assert_eq!(
        GfError::ElementOutOfRange { value: 9, order: 9 }.to_string(),
        "значение 9 не принадлежит полю: должно быть меньше 9"
    );
    // отдельное сообщение для нулевого модуля
    assert_eq!(
        GfError::InvalidPolynomial { p: 3, m: 2, poly: 0 }.to_string(),
        "модуль 0 непригоден для GF(3^2): при m >= 2 это x^2 (приводимый), при m = 1 alpha() даёт нуль"
    );
}

#[test]
fn element_validation_errors() {
    let params = FieldParams::new(3, 2, 5).unwrap(); // GF(9)
    assert_eq!(
        GfRuntime::new(params, 9),
        Err(GfError::ElementOutOfRange { value: 9, order: 9 })
    );
    assert_eq!(
        GfRuntime::new(params, u64::MAX),
        Err(GfError::ElementOutOfRange {
            value: u64::MAX,
            order: 9
        })
    );
    // длина среза коэффициентов
    assert_eq!(
        GfRuntime::from_coeffs(params, &[1]),
        Err(GfError::InvalidSliceLen {
            expected: 2,
            got: 1
        })
    );
    assert_eq!(
        GfRuntime::from_coeffs(params, &[1, 2, 3]),
        Err(GfError::InvalidSliceLen {
            expected: 2,
            got: 3
        })
    );
    // коэффициент вне GF(3)
    assert_eq!(
        GfRuntime::from_coeffs(params, &[3, 1]),
        Err(GfError::CoefficientOutOfRange {
            index: 0,
            coeff: 3,
            p: 3
        })
    );
    // валидное конструирование
    let e = GfRuntime::from_coeffs(params, &[2, 1]).unwrap();
    assert_eq!(e.value(), 5);
    assert_eq!(e.coeffs(), &[2, 1]);
    assert_eq!(e.coeff(0), 2);
    assert_eq!(e.coeff(5), 0);
}

#[test]
fn find_produces_irreducible_moduli() {
    // эталонные значения поиска (порядок — по возрастанию упакованного;
    // упаковка gfpm — без старшей единицы: x^4 + x + 1 → 3, а не 0x13)
    let cases: &[(u32, u32, u128)] = &[
        (2, 4, 3),  // x^4 + x + 1 — тот же модуль, что у gf2m::Gf16
        (2, 8, 27), // x^8 + x^4 + x^3 + x + 1
        (3, 2, 1),  // x^2 + 1
        (3, 3, 7),  // x^3 + 2x + 1
        (3, 4, 5),  // x^4 + x + 2
        (5, 2, 2),  // x^2 + 2
        (5, 3, 6),  // x^3 + x + 1
        (7, 2, 1),  // x^2 + 1
        (11, 2, 1), // x^2 + 1
        (13, 2, 2), // x^2 + 2
    ];
    for &(p, m, expected) in cases {
        let params = FieldParams::find(p, m).unwrap();
        assert_eq!(params.poly(), expected, "find({p}, {m})");
        assert_eq!(params.p(), p);
        assert_eq!(params.m(), m);
        // найденный модуль неприводим
        let mut f = [0u32; 65];
        gfpm::poly::unpack_poly(params.poly(), p, m, &mut f[..m as usize]);
        f[m as usize] = 1;
        assert!(gfpm::poly::is_irreducible(&f[..=m as usize], p));
    }
    // ошибки поиска — те же, что у new
    assert_eq!(FieldParams::find(4, 2), Err(GfError::NotPrime(4)));
    assert_eq!(FieldParams::find(3, 0), Err(GfError::UnsupportedDegree(0)));
    assert_eq!(
        FieldParams::find(3, 41),
        Err(GfError::OrderTooLarge { p: 3, m: 41 })
    );
}

#[test]
fn runtime_matches_const_layer() {
    macro_rules! agree {
        ($ty:ty, $p:expr, $m:expr, $poly:expr) => {{
            let params = FieldParams::new($p, $m, $poly).unwrap();
            let order = params.order() as u64;
            for a in 0..order {
                for b in 0..order {
                    let x = GfRuntime::new(params, a).unwrap();
                    let y = GfRuntime::new(params, b).unwrap();
                    let xc = <$ty>::new(a);
                    let yc = <$ty>::new(b);
                    assert_eq!((x + y).value(), (xc + yc).value(), "add({a},{b})");
                    assert_eq!((x - y).value(), (xc - yc).value(), "sub({a},{b})");
                    assert_eq!((x * y).value(), (xc * yc).value(), "mul({a},{b})");
                    assert_eq!((x / y).value(), (xc / yc).value(), "div({a},{b})");
                }
            }
            for a in 0..order {
                let x = GfRuntime::new(params, a).unwrap();
                let xc = <$ty>::new(a);
                assert_eq!(x.inv().value(), xc.inv().value(), "inv({a})");
                assert_eq!(x.sqr().value(), xc.sqr().value(), "sqr({a})");
                assert_eq!(x.pow(11).value(), xc.pow(11).value(), "pow({a})");
                assert_eq!(x.trace().value(), xc.trace().value(), "trace({a})");
                assert_eq!(x.norm().value(), xc.norm().value(), "norm({a})");
                assert_eq!(x.frobenius().value(), xc.frobenius().value(), "frob({a})");
                assert_eq!(
                    x.sqrt(),
                    xc.sqrt()
                        .map(|r| GfRuntime::new(params, r.value()).unwrap()),
                    "sqrt({a})"
                );
                assert_eq!(x.is_square(), xc.is_square(), "is_square({a})");
                assert_eq!(x.value(), xc.value());
            }
        }};
    }
    agree!(Gf<3, 2, 5>, 3, 2, 5); // GF(9)
    agree!(Gf<5, 2, 7>, 5, 2, 7); // GF(25)
    agree!(Gf<3, 3, 7>, 3, 3, 7); // GF(27)
    agree!(Gf<7, 2, 10>, 7, 2, 10); // GF(49)
    agree!(Gf<2, 4, 3>, 2, 4, 3); // GF(16), модуль x^4 + x + 1
}

#[test]
fn runtime_display_and_accessors() {
    let params = FieldParams::new(3, 2, 5).unwrap();
    let e = GfRuntime::new(params, 5).unwrap();
    assert_eq!(e.to_string(), "x + 2");
    assert_eq!(GfRuntime::zero(params).to_string(), "0");
    assert_eq!(
        GfRuntime::from_coeffs(params, &[1, 2]).unwrap().to_string(),
        "2x + 1"
    );
    // описания поля
    assert_eq!(e.p(), 3);
    assert_eq!(e.m(), 2);
    assert_eq!(e.poly(), 5);
    assert_eq!(e.order(), 9);
    assert_eq!(e.params(), params);
    // alpha и единица/нуль
    assert_eq!(GfRuntime::alpha(params), GfRuntime::new(params, 3).unwrap());
    assert_eq!(GfRuntime::one(params), GfRuntime::new(params, 1).unwrap());
    assert!(GfRuntime::zero(params).is_zero());
    // m = 1: alpha = −q_0 (модуль x + 1 → alpha = 2 в GF(3))
    let p1 = FieldParams::new(3, 1, 1).unwrap();
    assert_eq!(GfRuntime::alpha(p1), GfRuntime::new(p1, 2).unwrap());
}

#[test]
fn runtime_reduce_total() {
    let params = FieldParams::new(3, 2, 5).unwrap();
    // то же, что константное ядро: 100 ≡ x + 2
    assert_eq!(
        GfRuntime::reduce(params, 100),
        GfRuntime::new(params, 5).unwrap()
    );
    assert!(
        GfRuntime::reduce(params, u64::MAX).value() < 9,
        "любое u64 приводится в диапазон"
    );
    // элемент другого поля не равен этому же значению
    let other = FieldParams::new(5, 2, 7).unwrap();
    assert_ne!(
        GfRuntime::new(params, 3).unwrap(),
        GfRuntime::new(other, 3).unwrap()
    );
}

/// Трейт [`gfpm::FiniteField`] работает поверх обоих слоёв:
/// мини-гауссово исключение над GF(9).
///
/// Система (коэффициенты — константы GF(3) ⊂ GF(9)):
/// `2x + 2y = 0`, `2x + y = 2` — решение `(x, y) = (2, 1)`.
#[test]
fn generic_gauss_over_both_layers() {
    use gfpm::FiniteField;

    fn solve<F: FiniteField>(one: F) -> Option<(F, F)> {
        let f = |v: u64| one.from_int(v);
        // [[2, 2], [2, 1]] · (x, y) = (0, 2)
        let a11 = f(2);
        let a12 = f(2);
        let a21 = f(2);
        let a22 = f(1);
        let b1 = f(0);
        let b2 = f(2);
        if a11.is_zero() {
            return None;
        }
        let inv = a11.inv();
        let a12n = a12.mul(inv);
        let b1n = b1.mul(inv);
        let factor = a21;
        let a22n = a22.sub(a12n.mul(factor));
        let b2n = b2.sub(b1n.mul(factor));
        if a22n.is_zero() {
            return None;
        }
        let y = b2n.mul(a22n.inv());
        let x = b1n.sub(a12n.mul(y));
        Some((x, y))
    }

    fn check<F: FiniteField>(one: F) {
        let f = |v: u64| one.from_int(v);
        let (x, y) = solve(one).expect("система невырождена");
        assert_eq!(x.to_int(), 2, "x = 2");
        assert_eq!(y.to_int(), 1, "y = 1");
        // подстановка в исходные уравнения
        assert_eq!(f(2).mul(x).add(f(2).mul(y)), f(0), "2x + 2y = 0");
        assert_eq!(f(2).mul(x).add(y), f(2), "2x + y = 2");
    }

    check(gfpm::Gf9::one());
    check(GfRuntime::one(FieldParams::new(3, 2, 5).unwrap()));
}
