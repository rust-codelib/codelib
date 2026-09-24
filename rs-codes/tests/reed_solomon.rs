//! Тесты скелета Рида–Соломона.
//!
//! Контрактные `stubs_*` выполняются всегда: сигнатуры заморожены,
//! тела — `todo!()` (паника «not yet implemented» и ожидается).
//! Приёмочные `round_trip_*` помечены `#[ignore]` и включатся после
//! реализации; они обязаны компилироваться уже сейчас — это и есть
//! проверка заморозки контрактов.

use gf2m::Gf16;
use gf2m::Gf256;
use rs_codes::{
    berlekamp_massey, correct, encode_parity, error_magnitudes, error_positions, euclid,
    generator_poly, syndromes, KeyEquationSolver, RsConfig, RsError,
};

// ==== Контрактные тесты заглушек ====

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_generator_poly() {
    let mut out = [Gf256::new(0); 5];
    let _ = generator_poly(4, 0, &mut out);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_encode_parity() {
    let msg = [Gf256::new(1); 251];
    let gen = [Gf256::new(1); 5];
    let mut parity = [Gf256::new(0); 4];
    let _ = encode_parity(&msg, &gen, &mut parity);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_syndromes() {
    let received = [Gf256::new(0); 255];
    let mut out = [Gf256::new(0); 4];
    let _ = syndromes(&received, 4, 0, &mut out);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_berlekamp_massey() {
    let syn = [Gf256::new(1); 4];
    let mut locator = [Gf256::new(0); 4];
    let _ = berlekamp_massey(&syn, &mut locator);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_euclid() {
    let syn = [Gf256::new(1); 4];
    let mut locator = [Gf256::new(0); 4];
    let mut evaluator = [Gf256::new(0); 4];
    let _ = euclid(&syn, &mut locator, &mut evaluator);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_error_positions() {
    let locator = [Gf256::new(1); 3];
    let mut positions = [Gf256::new(0); 2];
    let _ = error_positions(&locator, 2, 255, &mut positions);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_error_magnitudes() {
    let evaluator = [Gf256::new(0); 3];
    let locator = [Gf256::new(1); 3];
    let positions = [Gf256::new(1); 2];
    let mut magnitudes = [Gf256::new(0); 2];
    let _ = error_magnitudes(&evaluator, &locator, 2, &positions, 0, &mut magnitudes);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn stubs_correct() {
    let mut received = [Gf256::new(0); 255];
    let cfg = RsConfig::new(4, 0).unwrap();
    let mut workspace = [Gf256::new(0); 14];
    let _ = correct(&mut received, &cfg, &mut workspace);
}

#[test]
fn stubs_config_is_real_code() {
    // Конфигурация — не заглушка: валидация и доступ работают.
    let cfg = RsConfig::new(4, 0).unwrap();
    assert_eq!(cfg.nsym, 4);
    assert_eq!(cfg.first_root, 0);
    assert_eq!(cfg.t(), 2);
    assert_eq!(cfg.solver, KeyEquationSolver::BerlekampMassey);

    let cfg = cfg.with_solver(KeyEquationSolver::Euclid);
    assert_eq!(cfg.solver, KeyEquationSolver::Euclid);

    assert_eq!(RsConfig::new(0, 0), Err(RsError::InvalidParameters));
    assert_eq!(
        RsError::InvalidParameters.to_string(),
        "недопустимые параметры кода Рида–Соломона"
    );
    assert_eq!(
        RsError::TooManyErrors.to_string(),
        "число ошибок превышает корректирующую способность кода"
    );
}

// ==== Приёмочные тесты конвейера (включить после реализации) ====

#[test]
#[ignore = "включить после реализации todo!()-заглушек"]
fn round_trip_gf256_two_errors() {
    // (255, 251) над GF(256): nsym = 4, t = 2
    let cfg = RsConfig::new(4, 0).unwrap();

    let mut gen = [Gf256::new(0); 5];
    generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

    let msg: [Gf256; 251] = core::array::from_fn(|i| Gf256::new(((i * 31 + 7) & 0xFF) as u16));
    let mut parity = [Gf256::new(0); 4];
    encode_parity(&msg, &gen, &mut parity).unwrap();

    let mut word = [Gf256::new(0); 255];
    word[..251].copy_from_slice(&msg);
    word[251..].copy_from_slice(&parity);

    // две ошибки
    word[3] += Gf256::new(0x55);
    word[200] += Gf256::new(0xAA);

    let mut workspace = [Gf256::new(0); 14]; // 3 * nsym + 2, nsym = 4
    let fixed = correct(&mut word, &cfg, &mut workspace).unwrap();
    assert_eq!(fixed, 2);
    assert_eq!(&word[..251], &msg);
    assert_eq!(&word[251..], &parity);
}

#[test]
#[ignore = "включить после реализации todo!()-заглушек"]
fn round_trip_gf256_no_errors() {
    let cfg = RsConfig::new(4, 0).unwrap();
    let mut gen = [Gf256::new(0); 5];
    generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

    let msg: [Gf256; 251] = core::array::from_fn(|i| Gf256::new(((i * 17 + 3) & 0xFF) as u16));
    let mut parity = [Gf256::new(0); 4];
    encode_parity(&msg, &gen, &mut parity).unwrap();

    let mut word = [Gf256::new(0); 255];
    word[..251].copy_from_slice(&msg);
    word[251..].copy_from_slice(&parity);

    let mut workspace = [Gf256::new(0); 14]; // 3 * nsym + 2, nsym = 4
    let fixed = correct(&mut word, &cfg, &mut workspace).unwrap();
    assert_eq!(fixed, 0);
    assert_eq!(&word[..251], &msg);
}

#[test]
#[ignore = "включить после реализации todo!()-заглушек"]
fn round_trip_gf256_too_many_errors() {
    // t = 2: три ошибки невосстановимы
    let cfg = RsConfig::new(4, 0).unwrap();
    let mut gen = [Gf256::new(0); 5];
    generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

    let msg: [Gf256; 251] = core::array::from_fn(|i| Gf256::new(((i * 13 + 1) & 0xFF) as u16));
    let mut parity = [Gf256::new(0); 4];
    encode_parity(&msg, &gen, &mut parity).unwrap();

    let mut word = [Gf256::new(0); 255];
    word[..251].copy_from_slice(&msg);
    word[251..].copy_from_slice(&parity);
    for i in [5, 100, 250] {
        word[i] += Gf256::new(0x7E);
    }

    let mut workspace = [Gf256::new(0); 14]; // 3 * nsym + 2, nsym = 4
    assert_eq!(
        correct(&mut word, &cfg, &mut workspace),
        Err(RsError::TooManyErrors)
    );
}

#[test]
#[ignore = "включить после реализации todo!()-заглушек"]
fn round_trip_gf16_shortened() {
    // (15, 11) над GF(16): nsym = 4, t = 2
    let cfg = RsConfig::new(4, 0).unwrap();
    let mut gen = [Gf16::new(0); 5];
    generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

    let msg: [Gf16; 11] = core::array::from_fn(|i| Gf16::new((i * 7 % 16) as u16));
    let mut parity = [Gf16::new(0); 4];
    encode_parity(&msg, &gen, &mut parity).unwrap();

    let mut word = [Gf16::new(0); 15];
    word[..11].copy_from_slice(&msg);
    word[11..].copy_from_slice(&parity);
    word[0] += Gf16::new(5);
    word[9] += Gf16::new(11);

    let mut workspace = [Gf16::new(0); 14]; // 3 * nsym + 2, nsym = 4
    let fixed = correct(&mut word, &cfg, &mut workspace).unwrap();
    assert_eq!(fixed, 2);
    assert_eq!(&word[..11], &msg);
}

#[test]
#[ignore = "включить после реализации todo!()-заглушек"]
fn round_trip_solver_variants_agree() {
    // Оба решателя ключевого уравнения обязаны давать одинаковый
    // результат на одном и том же слове.
    let base = RsConfig::new(4, 0).unwrap();

    for solver in [
        KeyEquationSolver::BerlekampMassey,
        KeyEquationSolver::Euclid,
    ] {
        let cfg = base.with_solver(solver);
        let mut gen = [Gf256::new(0); 5];
        generator_poly(cfg.nsym, cfg.first_root, &mut gen).unwrap();

        let msg: [Gf256; 251] = core::array::from_fn(|i| Gf256::new(((i * 29 + 5) & 0xFF) as u16));
        let mut parity = [Gf256::new(0); 4];
        encode_parity(&msg, &gen, &mut parity).unwrap();

        let mut word = [Gf256::new(0); 255];
        word[..251].copy_from_slice(&msg);
        word[251..].copy_from_slice(&parity);
        word[42] += Gf256::new(0x11);
        word[99] += Gf256::new(0x22);

        let mut workspace = [Gf256::new(0); 14]; // 3 * nsym + 2, nsym = 4
        let fixed = correct(&mut word, &cfg, &mut workspace).unwrap();
        assert_eq!(fixed, 2, "solver = {solver:?}");
        assert_eq!(&word[..251], &msg, "solver = {solver:?}");
    }
}
