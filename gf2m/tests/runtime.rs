//! Тесты рантайм-слоя: конструирование, ошибки и их Display-строки,
//! соглашения нуля, таблица стандартных многочленов.

use gf2m::{GfError, GfRuntime};

#[test]
fn new_rejects_unsupported_degree() {
    for m in [0, 1, 17, 18, 32, u32::MAX] {
        assert_eq!(GfRuntime::new(m, 0), Err(GfError::UnsupportedDegree(m)));
    }
    for m in 2..=16 {
        assert!(
            GfRuntime::new(m, 0).is_ok(),
            "m = {m} должен поддерживаться"
        );
    }
}

#[test]
fn new_rejects_out_of_range_value() {
    assert_eq!(
        GfRuntime::new(8, 256),
        Err(GfError::ElementOutOfRange { m: 8, value: 256 })
    );
    assert_eq!(
        GfRuntime::new(2, 4),
        Err(GfError::ElementOutOfRange { m: 2, value: 4 })
    );
    assert!(GfRuntime::new(8, 255).is_ok());
    assert!(GfRuntime::new(16, u16::MAX).is_ok());
    assert!(GfRuntime::new(16, u16::MAX - 1).is_ok());
}

#[test]
fn error_display_strings() {
    // Строки — часть контракта; проверяются дословно.
    assert_eq!(
        GfError::UnsupportedDegree(1).to_string(),
        "неподдерживаемая степень поля: m = 1, допустимо 2..=16"
    );
    assert_eq!(
        GfError::ElementOutOfRange { m: 8, value: 256 }.to_string(),
        "значение 0x100 не принадлежит GF(2^8): должно быть меньше 2^8"
    );
    assert_eq!(
        GfError::ElementOutOfRange { m: 2, value: 4 }.to_string(),
        "значение 0x4 не принадлежит GF(2^2): должно быть меньше 2^2"
    );
}

#[test]
fn zero_conventions_hold() {
    // Соглашения нуля: арифметика не паникует.
    let z = GfRuntime::new(8, 0).unwrap();
    let x = GfRuntime::new(8, 0x53).unwrap();
    assert_eq!(z.inv(), z);
    assert_eq!(x.div(z), GfRuntime::new(8, 0).unwrap());
    assert_eq!(z.div(z), z);
    assert_eq!(z.pow(0), GfRuntime::new(8, 1).unwrap());
    assert_eq!(z.pow(5), z);
    assert_eq!(z.sqrt(), z);
    assert_eq!(z.trace(), z);
}

#[test]
fn zech_none_cases() {
    // Z(0) не существует: 1 + α^0 = 0; k ≡ 0 (mod 2^m − 1) — тоже.
    for m in 2..=16 {
        let order = (1u32 << m) - 1;
        for k in [0u16, order as u16] {
            let e = GfRuntime::new(m, k).unwrap();
            assert!(
                e.zech().is_none(),
                "zech(m = {m}, k = {k}) должен быть None"
            );
        }
    }
}

#[test]
fn standard_poly_table() {
    assert_eq!(GfRuntime::standard_poly(2).unwrap(), 0x7);
    assert_eq!(GfRuntime::standard_poly(8).unwrap(), 0x11D);
    assert_eq!(GfRuntime::standard_poly(16).unwrap(), 0x1002D);
    assert_eq!(
        GfRuntime::standard_poly(1),
        Err(GfError::UnsupportedDegree(1))
    );
    assert_eq!(gf2m::POLYS.len(), 15);
    // все многочлены содержат старший член степени m
    for (i, &poly) in gf2m::POLYS.iter().enumerate() {
        let m = (i + 2) as u32;
        assert_eq!(gf2m::poly_degree(poly), m, "POLYS[{i}] для m = {m}");
    }
}

#[test]
fn accessors_and_display() {
    let e = GfRuntime::new(11, 0x5AB).unwrap();
    assert_eq!(e.m(), 11);
    assert_eq!(e.value(), 0x5AB);
    assert_eq!(e.to_string(), "0x5ab");
    assert_eq!(GfRuntime::one(4).unwrap().value(), 1);
    assert_eq!(GfRuntime::alpha(4).unwrap().value(), 2);
    assert_eq!(GfRuntime::zero(4).unwrap().value(), 0);
}

#[test]
fn itoh_tsujii_matches_fermat() {
    // Косвенная проверка Ито–Цудзии: inv согласован с pow по Ферма.
    let mut s: u32 = 7;
    for m in 2u32..=16 {
        let order = (1u32 << m) - 1;
        for _ in 0..64 {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = if m < 16 {
                ((s >> 16) as u16) & ((1u16 << m) - 1)
            } else {
                (s >> 16) as u16
            };
            let a = GfRuntime::new(m, v).unwrap();
            if a.value() == 0 {
                continue;
            }
            assert_eq!(a.inv(), a.pow(order - 1), "inv(m = {m}, {a})");
        }
    }
}
