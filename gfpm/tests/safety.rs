//! Граничные случаи: максимальный порядок 2^64, большие простые,
//! GF(2), соглашения нуля, переполнения промежуточных вычислений.
//!
//! Тесты исполняются и в debug (панелей от переполнений нет),
//! и в release (CI гоняет оба режима).

use gfpm::{Gf, GfRuntime};

/// GF(2^64): максимальный допустимый порядок — ровно 2^64.
/// Модуль x^64 + x^4 + x^3 + x + 1 (0x1B) — первый неприводимый
/// степени 64 над GF(2).
#[test]
fn gf2_64_boundary() {
    type F = Gf<2, 64, 0x1B>;
    assert_eq!(F::ORDER, 1u128 << 64);

    // все u64 — корректные упакованные значения
    let max = F::new(u64::MAX);
    assert_eq!(max.value(), u64::MAX);
    assert_eq!(F::new(0).value(), 0);

    // x^64 = x^4 + x^3 + x + 1 (редукция старшей степени)
    let alpha = F::alpha();
    assert_eq!(alpha.pow(64), F::new(0b11011)); // 27
    assert_eq!(alpha.pow(65), F::new(54)); // x·(x^4+x^3+x+1)

    // арифметика на максимальных значениях не переполняется
    let a = F::new(u64::MAX);
    let b = F::new(0x7FFF_FFFF_FFFF_FFFD);
    assert_eq!(a.mul(b).mul(b.inv()), a, "a·b/b = a");
    assert_ne!(a.mul(b), F::zero());
    assert_eq!(a.mul(a.inv()), F::one());

    // показатель u64::MAX ≡ 0 (mod 2^64 − 1) → единица
    assert_eq!(alpha.pow(u64::MAX), F::one());
    assert_eq!(alpha.pow(0), F::one());

    // в характеристике 2 всё — квадраты
    assert!(a.is_square());
    assert_eq!(a.sqrt().unwrap().sqr(), a);

    // след/норма/фробениус не паникуют и лежат в подполе
    assert!(a.trace().value() <= 1);
    assert!(a.norm().value() <= 1);
    // 64-кратная композиция фробениуса — тождественный автоморфизм:
    // sigma^64(x) = x^(2^64) = x. (a.frobenius().pow(64) здесь не годится —
    // это (x^2)^64 = x^128, а не композиция.)
    let mut f = a;
    for _ in 0..64 {
        f = f.frobenius();
    }
    assert_eq!(f, a, "фробениус^64 = id");

    // reduce максимального u64
    assert_eq!(F::reduce(u64::MAX), F::new(u64::MAX)); // степень 63 < 64
}

/// GF(p^2) для p = 2^31 − 1: порядок ~2^62, коэффициенты ~2^31 —
//  умножения u32·u32 в u64 без переполнения.
#[test]
fn mersenne_squared_boundary() {
    type F = Gf<2147483647, 2, 1>; // x^2 + 1
    let p = 2147483647u64;
    assert_eq!(F::ORDER, (p * p) as u128);

    let alpha = F::alpha();
    assert_eq!(alpha.sqr(), F::new(p - 1)); // x^2 = −1

    // большие коэффициенты
    let a = F::from_coeffs([(p - 1) as u32, (p - 1) as u32]);
    let b = F::from_coeffs([(p - 2) as u32, (p - 1) as u32]);
    // (−1 − x)(−2 − x) = 2 + 3x + x^2 = 1 + 3x (x^2 = −1)
    assert_eq!(
        a.mul(b),
        F::from_coeffs([1, 3]),
        "(−1−x)·(−2−x) = 1 + 3x по модулю x^2 + 1"
    );
    assert_eq!(a.mul(a.inv()), F::one());
    assert_eq!(a.mul(b).div(b), a);
    // норма N(a + bi) = a^2 + b^2
    let n = a.norm();
    assert!(n.value() < p);
    // (p−1)^2 + (p−1)^2 mod p = 1 + 1 = 2
    assert_eq!(n, F::new(2));

    // sqrt: элемент p−1 (= −1 = x^2) имеет корень x
    let r = F::new(p - 1).sqrt().expect("x^2 = −1 — квадрат");
    assert_eq!(r.sqr(), F::new(p - 1));
    // согласованность для произвольного элемента
    let c = F::new(2);
    if c.is_square() {
        assert_eq!(c.sqrt().unwrap().sqr(), c);
    } else {
        assert_eq!(c.sqrt(), None);
    }
}

/// Наибольшее 32-битное простое 4294967291: арифметика у границы u32.
#[test]
fn largest_32bit_prime() {
    type F = Gf<4294967291, 1, 1>;
    let p = 4294967291u64;

    let a = F::new(p - 1); // −1
    assert_eq!(a.add(a), F::new(p - 2)); // −2
    assert_eq!(a.mul(a), F::one()); // (−1)^2 = 1
    assert_eq!(a.inv(), a);
    assert_eq!(a.pow(p - 1), F::one()); // Ферма
    assert_eq!(a.pow((p - 1) / 2), F::new(p - 1)); // −1 — невычет
    assert!(!a.is_square());
    assert_eq!(a.sqrt(), None);
    // 2^32 mod p = 5
    assert_eq!(F::new(2).pow(32), F::new(5));
    // reduce любого u64
    assert!(F::reduce(u64::MAX).value() < p);
}

/// GF(2) — минимальное поле: соглашения нуля и вырожденные степени.
/// Модуль x + 1 (POLY = 0 — модуль x — запрещён параметрами типа:
/// alpha() в нём нулевой).
#[test]
fn gf2_minimal() {
    type F = Gf<2, 1, 1>;
    assert_eq!(F::ORDER, 2);
    let zero = F::zero();
    let one = F::one();
    assert_eq!(one.add(one), zero);
    assert_eq!(one.mul(one), one);
    assert_eq!(one.inv(), one);
    assert_eq!(one.sub(one), zero);
    assert_eq!(zero.inv(), zero); // соглашение
    assert_eq!(one.div(zero), zero); // соглашение
    assert_eq!(zero.pow(0), one); // 0^0 = 1
    assert_eq!(zero.pow(7), zero);
    assert_eq!(one.pow(u64::MAX), one);
    assert_eq!(one.trace(), one); // M = 1: тождества
    assert_eq!(one.norm(), one);
    assert_eq!(one.frobenius(), one);
    assert!(zero.is_square());
    assert_eq!(zero.sqrt(), Some(zero));
    assert_eq!(one.sqrt(), Some(one));
    // модуль x + 1 ⟹ alpha = −1 = 1 в GF(2) — порождает GF(2)*
    assert_eq!(F::alpha(), one);
}

/// Соглашения нуля на всех слоях.
#[test]
fn zero_conventions() {
    macro_rules! conv {
        ($ty:ty) => {{
            let z = <$ty>::zero();
            let o = <$ty>::one();
            assert_eq!(z.inv(), z, "inv(0) = 0");
            assert_eq!(o.div(z), z, "x / 0 = 0");
            assert_eq!(z.div(z), z, "0 / 0 = 0");
            assert_eq!(z.pow(0), o, "0^0 = 1");
            assert_eq!(z.pow(1), z);
            assert!(z.is_square());
            assert_eq!(z.sqrt(), Some(z));
        }};
    }
    conv!(gfpm::Gf9);
    conv!(gfpm::Gf25);
    conv!(gfpm::Gf27);
    conv!(gfpm::Gf5);
    conv!(gfpm::Gf251);

    // рантайм-слой
    let params = gfpm::FieldParams::new(3, 2, 5).unwrap();
    let z = GfRuntime::zero(params);
    let o = GfRuntime::one(params);
    assert_eq!(z.inv(), z);
    assert_eq!(o.div(z), z);
    assert_eq!(z.pow(0), o);
    assert_eq!(z.sqrt(), Some(z));
    assert!(z.is_square());
}

/// Степени с большими показателями согласованы с редукцией
/// показателя по модулю порядка группы.
#[test]
fn pow_exponent_reduction() {
    let alpha = gfpm::Gf9::alpha();
    // порядок группы 8: alpha^k = alpha^(k mod 8) для k > 0
    for k in [1u64, 7, 8, 9, 15, 16, 100, 1000, u64::MAX - 1, u64::MAX] {
        let reduced = 1 + (k - 1) % 8; // в 1..=8
        assert_eq!(alpha.pow(k), alpha.pow(reduced), "alpha^{k}");
    }
    // нуль: 0^k = 0 для k > 0
    assert_eq!(gfpm::Gf9::zero().pow(u64::MAX), gfpm::Gf9::zero());
    assert_eq!(gfpm::Gf9::zero().pow(0), gfpm::Gf9::one());
}

/// Рантайм-слой на границе: GF(2^64) через find.
#[test]
fn runtime_gf2_64() {
    let params = gfpm::FieldParams::find(2, 64).unwrap();
    assert_eq!(params.order(), 1u128 << 64);
    assert_eq!(params.poly(), 0x1B);

    let alpha = GfRuntime::alpha(params);
    assert_eq!(alpha, GfRuntime::new(params, 2).unwrap());
    assert_eq!(alpha.pow(64), GfRuntime::new(params, 27).unwrap());
    let a = GfRuntime::new(params, u64::MAX).unwrap();
    assert_eq!(a.value(), u64::MAX);
    assert_eq!(a.mul(a.inv()), GfRuntime::one(params));
    assert_eq!(a.sqrt().unwrap().sqr(), a);
    // ошибка диапазона с порядком 2^64
    assert_eq!(
        GfRuntime::new(params, u64::MAX),
        Ok(a) // u64::MAX < 2^64 — корректно
    );
}

/// Рантайм-слой, большое чётное расширение GF((2^31−1)^2): sqrt
/// через generic [`field::sqrt`](gfpm::field::sqrt).
///
/// Регрессия: при чётном m все элементы простого подполя — квадраты,
/// и поиск невычета, стартующий с 2, перебирал бы ~2^31 элементов
/// (часы). Теперь перебор начинается с x (упакованное p).
#[test]
fn runtime_mersenne_squared_sqrt() {
    let params = gfpm::FieldParams::new(2147483647, 2, 1).unwrap(); // модуль x^2 + 1
    let neg_one = GfRuntime::new(params, 2147483646).unwrap(); // −1
                                                               // −1 = x^2 — квадрат; корень ищется generic-Тонелли–Шенксом
    let r = neg_one.sqrt().expect("−1 = x^2 — квадрат");
    assert_eq!(r.sqr(), neg_one);
    // сам x — корень из −1 (цифры [0, 1] → упакованное значение p)
    let e = GfRuntime::new(params, 2147483647).unwrap(); // x
    assert_eq!(e.sqr(), neg_one);
    // x + 2 — невычет (сверено независимым расчётом): корня нет
    let non_square = GfRuntime::new(params, 2147483649).unwrap();
    assert!(!non_square.is_square());
    assert_eq!(non_square.sqrt(), None);
}

/// Debug-формат GfRuntime компактен (коэффициенты, а не массив 64).
#[test]
fn runtime_debug_format() {
    let params = gfpm::FieldParams::new(3, 2, 5).unwrap();
    let e = GfRuntime::new(params, 5).unwrap();
    let s = format!("{e:?}");
    assert!(s.contains("coeffs"), "{s}");
    assert!(s.contains("params"), "{s}");
}
