//! Известные векторы: значения, выверенные вручную и независимым
//! Python-расчётом (scripts/gen_vectors.py в репозитории проекта).

use gfpm::{Gf, Gf11, Gf125, Gf13, Gf2147483647, Gf25, Gf27, Gf3, Gf49, Gf5, Gf65537, Gf7, Gf9};

#[test]
fn gf9_multiplication_and_cycle() {
    // модуль x^2 + x + 2: alpha = x, примитивен, порядок 8
    let alpha = Gf9::alpha();
    assert_eq!(alpha, Gf9::new(3));

    // полный цикл степеней alpha (сгенерирован независимо)
    let cycle = [3u64, 7, 8, 2, 6, 5, 4, 1];
    for (k, &expected) in cycle.iter().enumerate() {
        assert_eq!(
            alpha.pow(k as u64 + 1),
            Gf9::new(expected),
            "alpha^{}",
            k + 1
        );
    }
    assert_eq!(alpha.pow(9), alpha, "alpha^9 = alpha (показатель mod 8)");

    // произведения
    assert_eq!(Gf9::new(5) * Gf9::new(3), Gf9::new(4)); // (x+2)·x = x+1
    assert_eq!(Gf9::new(5) * Gf9::new(5), Gf9::new(2)); // (x+2)^2 = 2
    assert_eq!(Gf9::new(7) * Gf9::new(8), Gf9::new(6));
    assert_eq!(Gf9::new(4) * Gf9::new(6), Gf9::new(2));
    assert_eq!(Gf9::new(8) * Gf9::new(8), Gf9::new(5));

    // обратные (полная таблица)
    let inverses = [0u64, 1, 2, 4, 3, 7, 8, 5, 6];
    for (v, &inv) in inverses.iter().enumerate() {
        assert_eq!(Gf9::new(v as u64).inv(), Gf9::new(inv), "inv({v})");
    }

    // сложение и вычитание с переносами по модулю 3
    assert_eq!(Gf9::new(5) + Gf9::new(5), Gf9::new(7)); // (x+2)+(x+2) = 2x+1
    assert_eq!(Gf9::new(2) - Gf9::new(5), Gf9::new(6)); // 2 − (x+2) = −x = 2x
    assert_eq!(-Gf9::new(5), Gf9::new(7)); // −(x+2) = 2x + 1
}

#[test]
fn gf9_trace_norm_sqrt() {
    // след и норма (независимый расчёт)
    assert_eq!(Gf9::new(3).trace(), Gf9::new(2)); // Tr(x) = 2
    assert_eq!(Gf9::new(5).trace(), Gf9::new(0)); // Tr(x+2) = 0
    assert_eq!(Gf9::new(7).trace(), Gf9::new(0));
    assert_eq!(Gf9::new(3).norm(), Gf9::new(2)); // N(x) = x^4 = 2
    assert_eq!(Gf9::new(5).norm(), Gf9::new(1)); // N(x+2) = 1
    assert_eq!(Gf9::new(7).norm(), Gf9::new(1));

    // квадраты: образ отображения y ↦ y^2 — в точности {0,1,2,5,7}
    let squares = [0u64, 1, 2, 5, 7];
    for v in 0..9u64 {
        let x = Gf9::new(v);
        assert_eq!(
            x.is_square(),
            squares.contains(&v),
            "is_square({v}) в GF(9)"
        );
    }
    // конкретные корни (их ровно два, ±r)
    let roots_of_2 = [Gf9::new(5), Gf9::new(7)];
    let r = Gf9::new(2).sqrt().expect("2 — квадрат");
    assert!(roots_of_2.contains(&r), "sqrt(2) ∈ {{5, 7}}");
    let roots_of_7 = [Gf9::new(3), Gf9::new(6)];
    let r = Gf9::new(7).sqrt().expect("7 — квадрат");
    assert!(roots_of_7.contains(&r), "sqrt(7) ∈ {{3, 6}}");
    assert_eq!(Gf9::new(3).sqrt(), None, "3 — невычет в GF(9)");
    // невычеты корней не имеют
    for &v in &[3u64, 4, 6, 8] {
        assert_eq!(Gf9::new(v).sqrt(), None, "sqrt({v}) = None");
    }
}

#[test]
fn gf25_vectors() {
    // модуль x^2 + x + 2 над GF(5), alpha = x примитивен (порядок 24)
    let alpha = Gf25::alpha();
    assert_eq!(alpha, Gf25::new(5));
    assert_eq!(alpha.pow(24), Gf25::one());
    assert_eq!(alpha.pow(12), Gf25::new(4)); // единственный элемент порядка 2: −1
    assert_eq!(alpha.sqr(), Gf25::new(23)); // x^2 = 4x + 3

    // произведения
    assert_eq!(Gf25::new(5) * Gf25::new(6), Gf25::new(3));
    assert_eq!(Gf25::new(12) * Gf25::new(13), Gf25::new(8));

    // след и норма alpha
    assert_eq!(alpha.trace(), Gf25::new(4)); // Tr(x) = −1
    assert_eq!(alpha.norm(), Gf25::new(2)); // N(x) = x^6 = 2

    // корни: 7 — квадрат (корни 10 и 15), 24 = −1 — невычет
    let r = Gf25::new(7).sqrt().expect("7 — квадрат в GF(25)");
    assert!(r == Gf25::new(10) || r == Gf25::new(15));
    assert_eq!(Gf25::new(24).sqrt(), None);
    assert!(!Gf25::new(24).is_square());
}

#[test]
fn gf27_vectors() {
    // модуль x^3 + 2x + 1 над GF(3), alpha примитивен (порядок 26)
    let alpha = Gf27::alpha();
    assert_eq!(alpha, Gf27::new(3));
    assert_eq!(alpha.pow(26), Gf27::one());
    assert_eq!(alpha.pow(13), Gf27::new(2)); // элемент порядка 2: −1
                                             // x^3 = −2x − 1 = x + 2 → упакованно 2 + 1·3 = 5
    assert_eq!(alpha.pow(3), Gf27::new(5));
}

#[test]
fn gf49_vectors() {
    // модуль x^2 + x + 3 над GF(7)
    let alpha = Gf49::alpha();
    assert_eq!(alpha, Gf49::new(7));
    assert_eq!(alpha.pow(48), Gf49::one());
    assert_eq!(Gf49::new(10) * Gf49::new(11), Gf49::new(44));
    // x^2 = −x − 3 = 6x + 4 → упакованно 4 + 6·7 = 46
    assert_eq!(alpha.sqr(), Gf49::new(46));
}

#[test]
fn prime_field_vectors() {
    // GF(3)
    assert_eq!(Gf3::new(2) + Gf3::new(2), Gf3::new(1));
    assert_eq!(Gf3::new(2) * Gf3::new(2), Gf3::new(1));
    assert_eq!(Gf3::new(2).inv(), Gf3::new(2));
    assert_eq!(Gf3::new(2), Gf3::alpha()); // alpha = 2

    // GF(5)
    assert_eq!(Gf5::new(2) * Gf5::new(3), Gf5::new(1));
    assert_eq!(Gf5::new(3).inv(), Gf5::new(2));
    assert_eq!(Gf5::new(3) * Gf5::new(3), Gf5::new(4));
    assert_eq!(Gf5::new(2).pow(4), Gf5::one());
    // корни: 4 = 2^2 = 3^2, 2 и 3 — невычеты
    let r = Gf5::new(4).sqrt().expect("4 — квадрат в GF(5)");
    assert!(r == Gf5::new(2) || r == Gf5::new(3), "sqrt(4) ∈ {{2, 3}}");
    assert_eq!(Gf5::new(2).sqrt(), None);
    assert!(!Gf5::new(2).is_square());

    // GF(7): 3·5 = 1, sqrt(2) = ±3
    assert_eq!(Gf7::new(3) * Gf7::new(5), Gf7::one());
    assert_eq!(Gf7::new(2).pow(3), Gf7::new(1)); // 8 = 1 mod 7
    let r = Gf7::new(2).sqrt().expect("2 — квадрат mod 7");
    assert!(r == Gf7::new(3) || r == Gf7::new(4));
    assert_eq!(Gf7::new(3).sqrt(), None); // 3 — невычет mod 7

    // GF(11), GF(13)
    assert_eq!(Gf11::new(3) * Gf11::new(4), Gf11::one());
    assert_eq!(Gf13::new(5) * Gf13::new(8), Gf13::one());
    let r = Gf13::new(12)
        .sqrt()
        .expect("−1 — квадрат mod 13 (13 ≡ 1 mod 4)");
    assert!(r == Gf13::new(5) || r == Gf13::new(8));
}

#[test]
fn large_prime_vectors() {
    // GF(65537): простое Ферма
    let a = Gf65537::new(2);
    assert_eq!(a.pow(16), Gf65537::new(65536)); // 2^16 = 65536 = −1
    assert_eq!(a.pow(32), Gf65537::one()); // 2^32 = 1
    assert_eq!(Gf65537::new(3).inv(), Gf65537::new(21846)); // 3·21846 = 1
    assert_eq!(Gf65537::alpha(), Gf65537::new(3)); // 3 — первообразный корень

    // GF(2^31 − 1): простое Мерсенна
    let b = Gf2147483647::new(2);
    assert_eq!(b.pow(31), Gf2147483647::one()); // 2^31 ≡ 1 (mod 2^31 − 1)
    assert_eq!(Gf2147483647::new(2).inv(), Gf2147483647::new(1073741824));
    assert_eq!(Gf2147483647::alpha(), Gf2147483647::new(7));
    // −1 — невычет: p ≡ 3 (mod 4)
    assert!(!Gf2147483647::new(2147483646).is_square());
    assert_eq!(Gf2147483647::new(2147483646).sqrt(), None);
}

#[test]
fn mersenne_quadratic_extension() {
    // GF((2^31 − 1)^2), модуль x^2 + 1 (неприводим: p ≡ 3 mod 4)
    type F = Gf<2147483647, 2, 1>;
    assert_eq!(F::ORDER, 2147483647u128 * 2147483647);

    let alpha = F::alpha(); // x
    assert_eq!(alpha.sqr(), F::new(2147483646)); // x^2 = −1
    assert_eq!(alpha.pow(4), F::one());
    // inv(x) = −x = 2147483646·x → упакованно 0 + 2147483646·p
    let inv_alpha = F::from_coeffs([0, 2147483646]);
    assert_eq!(alpha.inv(), inv_alpha);
    assert_eq!(alpha.mul(inv_alpha), F::one());
}

#[test]
fn display_formatting() {
    assert_eq!(Gf9::from_coeffs([2, 1]).to_string(), "x + 2");
    assert_eq!(Gf9::from_coeffs([1, 2]).to_string(), "2x + 1");
    assert_eq!(Gf9::from_coeffs([0, 1]).to_string(), "x");
    assert_eq!(Gf9::from_coeffs([2, 0]).to_string(), "2");
    assert_eq!(Gf9::zero().to_string(), "0");
    assert_eq!(Gf125::from_coeffs([3, 0, 1]).to_string(), "x^2 + 3");
    assert_eq!(Gf27::from_coeffs([1, 0, 2]).to_string(), "2x^2 + 1");
    assert_eq!(Gf5::new(3).to_string(), "3");
    // Debug показывает коэффициенты
    assert_eq!(format!("{:?}", Gf9::from_coeffs([2, 1])), "Gf([2, 1])");
}

#[test]
fn reduce_out_of_range() {
    // GF(9): 100 = (1, 0, 2, 0, 1)_3 = x^4 + 2x^2 + 1 ≡ x + 2
    assert_eq!(Gf9::reduce(100), Gf9::new(5));
    // 9 = (0, 0, 1)_3 = x^2 ≡ −(x + 2) = 2x + 1
    assert_eq!(Gf9::reduce(9), Gf9::new(7));
    // значения в диапазоне не меняются
    for v in 0..9u64 {
        assert_eq!(Gf9::reduce(v), Gf9::new(v));
    }
    // GF(5), модуль x − 2 (POLY = 3): reduce(v) = значение цифр в точке 2
    // 6 = (1, 1)_5 = x + 1 → x ≡ 2 → 2 + 1 = 3
    assert_eq!(Gf5::reduce(6), Gf5::new(3));
    assert_eq!(Gf5::reduce(3), Gf5::new(3));
    // огромное значение не паникует и даёт элемент поля
    let x = Gf9::reduce(u64::MAX);
    assert!(x.value() < 9);
}
