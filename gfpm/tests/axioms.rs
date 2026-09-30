//! Аксиомы поля: полный перебор элементов малых полей.
//!
//! Для каждого псевдонима проверяются: коммутативность,
//! ассоциативность, дистрибутивность, тождества, сокращение
//! вычитанием, обратные элементы (все ненулевые), `x^(p^m) = x`,
//! след/норма в простом подполе, согласованность `is_square`
//! и `sqrt`. Тройки — только для полей порядка ≤ 200.

use gfpm::FiniteField;

macro_rules! field_checks {
    ($name:ident, $ty:ty) => {
        #[test]
        fn $name() {
            let one = <$ty>::one();
            let order = one.field_order();
            let p = one.characteristic() as u64;
            let elements: Vec<$ty> = (0..order as u64)
                .map(|v| {
                    let e = one.from_int(v);
                    // round trip упакованного представления
                    assert_eq!(e.to_int(), v, "to_int(from_int({v}))");
                    e
                })
                .collect();

            for &a in &elements {
                // тождества и нуль
                assert_eq!(a.add(a.zero()), a, "a + 0 = a");
                assert_eq!(a.mul(a.one()), a, "a · 1 = a");
                assert_eq!(a.sub(a), a.zero(), "a − a = 0");
                assert_eq!(a.neg().add(a), a.zero(), "−a + a = 0");
                assert_eq!(a.neg().neg(), a, "−(−a) = a");
                assert_eq!(a.add(a.zero()).mul(a.one()), a);
                // обратный и деление
                if a.is_zero() {
                    assert_eq!(a.inv(), a.zero(), "inv(0) = 0");
                    assert_eq!(a.div(a.one()), a.zero(), "0 / 1 = 0");
                } else {
                    assert_eq!(a.inv().mul(a), a.one(), "inv(a)·a = 1");
                    assert_eq!(a.mul(a.one()).div(a), a.one(), "a·1/a = 1");
                    assert_eq!(a.div(a.one()), a, "a / 1 = a");
                }
                // степени
                assert_eq!(a.pow(0), a.one(), "a^0 = 1");
                assert_eq!(a.pow(1), a, "a^1 = a");
                assert_eq!(a.sqr(), a.mul(a), "a^2 = a·a");
                assert_eq!(a.pow(order as u64), a, "a^q = a");
                if !a.is_zero() {
                    assert_eq!(a.pow(order as u64 - 1), a.one(), "a^(q−1) = 1");
                }
                // фробениус
                assert_eq!(a.frobenius(), a.pow(p), "a^p");
                // след и норма лежат в простом подполе
                assert!(a.trace().to_int() < p, "Tr(x) ∈ GF(p)");
                assert!(a.norm().to_int() < p, "N(x) ∈ GF(p)");
                // мультипликативность нормы: N(a·a) = N(a)^2
                assert_eq!(a.sqr().norm(), a.norm().sqr(), "N(a^2) = N(a)^2");
                // аддитивность следа: Tr(a + a) = 2·Tr(a)
                assert_eq!(a.add(a).trace(), a.trace().add(a.trace()));
                // корни
                match a.sqrt() {
                    Some(r) => {
                        assert!(a.is_square(), "sqrt есть ⟹ квадрат");
                        assert_eq!(r.sqr(), a, "sqrt(a)^2 = a");
                    }
                    None => assert!(!a.is_square(), "sqrt нет ⟹ невычет"),
                }
                assert_eq!(a.is_square(), a.sqrt().is_some(), "is_square ⟺ sqrt");
            }

            // пары: коммутативность, сокращение, деление.
            // Поля до ~2^12 элементов — полный перебор, больше —
            // детерминированная выборка LCG (полный перебор пар
            // для GF(65537) занял бы часы).
            let run_pair = |a: $ty, b: $ty| {
                assert_eq!(a.add(b), b.add(a), "a + b = b + a");
                assert_eq!(a.mul(b), b.mul(a), "a·b = b·a");
                assert_eq!(a.sub(b).add(b), a, "(a − b) + b = a");
                if !b.is_zero() {
                    assert_eq!(a.mul(b).div(b), a, "a·b/b = a");
                }
                assert_eq!(
                    a.mul(b).inv(),
                    a.inv().mul(b.inv()),
                    "inv(ab) = inv(a)inv(b)"
                );
                // фробениус — автоморфизм
                assert_eq!(a.add(b).frobenius(), a.frobenius().add(b.frobenius()));
                assert_eq!(a.mul(b).frobenius(), a.frobenius().mul(b.frobenius()));
            };
            if elements.len() <= 4096 {
                for &a in &elements {
                    for &b in &elements {
                        run_pair(a, b);
                    }
                }
            } else {
                let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
                let mut next = || {
                    s = s
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    s >> 33
                };
                for _ in 0..4096 {
                    let a = one.from_int(next() % order as u64);
                    let b = one.from_int(next() % order as u64);
                    run_pair(a, b);
                }
            }

            // тройки: ассоциативность и дистрибутивность
            if elements.len() <= 200 {
                for &a in &elements {
                    for &b in &elements {
                        for &c in &elements {
                            assert_eq!(
                                a.add(b).add(c),
                                a.add(b.add(c)),
                                "(a + b) + c = a + (b + c)"
                            );
                            assert_eq!(a.mul(b).mul(c), a.mul(b.mul(c)), "(a·b)·c = a·(b·c)");
                            assert_eq!(
                                a.mul(b.add(c)),
                                a.mul(b).add(a.mul(c)),
                                "a·(b + c) = a·b + a·c"
                            );
                        }
                    }
                }
            }

            // след линеен и норма мультипликативна (выборка троек LCG)
            if elements.len() > 200 {
                let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
                let mut next = || {
                    s = s
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    s >> 11
                };
                for _ in 0..256 {
                    let a = one.from_int(next() % order as u64);
                    let b = one.from_int(next() % order as u64);
                    assert_eq!(
                        a.add(b).trace(),
                        a.trace().add(b.trace()),
                        "Tr(a + b) = Tr(a) + Tr(b)"
                    );
                    assert_eq!(
                        a.mul(b).norm(),
                        a.norm().mul(b.norm()),
                        "N(a·b) = N(a)·N(b)"
                    );
                }
            }
        }
    };
}

// Полный перебор: расширения
field_checks!(gf9_axioms, gfpm::Gf9);
field_checks!(gf25_axioms, gfpm::Gf25);
field_checks!(gf27_axioms, gfpm::Gf27);
field_checks!(gf49_axioms, gfpm::Gf49);
field_checks!(gf81_axioms, gfpm::Gf81);
field_checks!(gf121_axioms, gfpm::Gf121);
field_checks!(gf125_axioms, gfpm::Gf125);
field_checks!(gf169_axioms, gfpm::Gf169);
// Полный перебор пар: простые поля (тройки через выборку)
field_checks!(gf3_axioms, gfpm::Gf3);
field_checks!(gf5_axioms, gfpm::Gf5);
field_checks!(gf7_axioms, gfpm::Gf7);
field_checks!(gf11_axioms, gfpm::Gf11);
field_checks!(gf13_axioms, gfpm::Gf13);
field_checks!(gf251_axioms, gfpm::Gf251);
// Большое простое Ферма: только выборка пар
field_checks!(gf65537_axioms, gfpm::Gf65537);

/// Простое Мерсенна GF(2^31 − 1): выборка пар LCG (полный перебор
/// 2^31 элементов неуместен, но законы обязаны держаться).
#[test]
fn gf2147483647_axioms_sampled() {
    use gfpm::Gf2147483647;
    let one = Gf2147483647::one();
    let order = one.field_order();
    let mut s: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let mut next = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        s >> 33
    };
    for _ in 0..4096 {
        let a = Gf2147483647::new(next());
        let b = Gf2147483647::new(next());
        let c = Gf2147483647::new(next());
        assert_eq!(a.add(b), b.add(a));
        assert_eq!(a.mul(b), b.mul(a));
        assert_eq!(a.add(b).add(c), a.add(b.add(c)));
        assert_eq!(a.mul(b).mul(c), a.mul(b.mul(c)));
        assert_eq!(a.mul(b.add(c)), a.mul(b).add(a.mul(c)));
        assert_eq!(a.sub(b).add(b), a);
        if !b.is_zero() {
            assert_eq!(a.mul(b).div(b), a);
            assert_eq!(b.inv().mul(b), one);
        }
        assert_eq!(a.pow(order as u64 - 1), one, "Ферма: a^(p−1) = 1");
        assert_eq!(a.trace(), a, "M = 1: след — тождество");
        assert_eq!(a.norm(), a, "M = 1: норма — тождество");
        assert_eq!(a.frobenius(), a, "M = 1: фробениус — тождество");
    }
    let _ = order;
}

/// Псевдонимы используют примитивные модули: `alpha()` порождает
/// мультипликативную группу (порядок `alpha` равен `p^m − 1`).
#[test]
fn primitive_aliases() {
    // собственный порядок alpha: alpha^(group/ell) != 1 для всех
    // простых ell | group, и alpha^(group) = 1
    macro_rules! prim {
        ($ty:ty) => {{
            let alpha = <$ty>::alpha();
            let group = <$ty>::ORDER - 1;
            assert_ne!(alpha, <$ty>::zero(), "alpha не нуль");
            assert_eq!(alpha.pow(group as u64), <$ty>::one(), "alpha^(q−1) = 1");
            let mut g = group;
            let mut d: u128 = 2;
            while d * d <= g {
                if g % d == 0 {
                    assert_ne!(
                        alpha.pow((group / d) as u64),
                        <$ty>::one(),
                        "alpha^({}/{}) != 1 — alpha примитивен",
                        group,
                        d
                    );
                    while g % d == 0 {
                        g /= d;
                    }
                }
                d += 1;
            }
            if g > 1 {
                assert_ne!(
                    alpha.pow((group / g) as u64),
                    <$ty>::one(),
                    "alpha примитивен (степень простого делителя {g})"
                );
            }
        }};
    }
    prim!(gfpm::Gf3);
    prim!(gfpm::Gf5);
    prim!(gfpm::Gf7);
    prim!(gfpm::Gf11);
    prim!(gfpm::Gf13);
    prim!(gfpm::Gf251);
    prim!(gfpm::Gf65537);
    prim!(gfpm::Gf2147483647);
    prim!(gfpm::Gf9);
    prim!(gfpm::Gf27);
    prim!(gfpm::Gf81);
    prim!(gfpm::Gf25);
    prim!(gfpm::Gf125);
    prim!(gfpm::Gf49);
    prim!(gfpm::Gf121);
    prim!(gfpm::Gf169);
}

/// Модули псевдонимов-расширений неприводимы (тест Рабина).
#[test]
fn alias_polys_are_irreducible() {
    use gfpm::poly::is_irreducible;

    let cases: &[(u32, u128)] = &[
        (3, 5),   // Gf9
        (3, 7),   // Gf27
        (3, 5),   // Gf81 (x^4 + x + 2)
        (5, 7),   // Gf25
        (5, 17),  // Gf125
        (7, 10),  // Gf49
        (11, 18), // Gf121
        (13, 15), // Gf169
    ];
    let degrees = [2, 3, 4, 2, 3, 2, 2, 2];
    for (&(p, poly), &m) in cases.iter().zip(degrees.iter()) {
        let mut f = [0u32; 65];
        gfpm::poly::unpack_poly(poly, p, m, &mut f[..m as usize]);
        f[m as usize] = 1;
        assert!(
            is_irreducible(&f[..=m as usize], p),
            "модуль {poly} поля GF({p}^{m}) неприводим"
        );
    }
}
