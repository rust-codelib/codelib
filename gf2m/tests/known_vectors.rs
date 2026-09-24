//! Внешние контрольные векторы: FIPS-197 (поле AES 0x11B) и
//! стандартное поле Рида–Соломона 0x11D (QR-коды).

use gf2m::{Gf, Gf16, Gf256, Gf4};

/// Поле AES x^8 + x^4 + x^3 + x + 1 (0x11B) — разовое
/// инстанцирование без псевдонима: псевдоним `Gf256` закреплён
/// за полем Рида–Соломона 0x11D.
type Aes256 = Gf<256, 0x11B>;

#[test]
fn aes_field_fips197_vectors() {
    // FIPS-197, обоснование S-box: inv(0x53) = 0xCA
    assert_eq!(Aes256::new(0x53).inv(), Aes256::new(0xCA));
    assert_eq!(Aes256::new(0x53) * Aes256::new(0xCA), Aes256::new(1));
    // Учебник Rijndael: {57} · {13} = {FE}
    assert_eq!(Aes256::new(0x57) * Aes256::new(0x13), Aes256::new(0xFE));
    // {57} · {02} = {AE} — сдвиг без переполнения, верно в любом поле
    assert_eq!(Aes256::new(0x57) * Aes256::new(0x02), Aes256::new(0xAE));
}

#[test]
fn rs_field_0x11d_vectors() {
    // Gf256 — поле Рида–Соломона 0x11D (QR-коды и большинство
    // реализаций RS)
    // сдвиг без переполнения — верно в любом GF(256)
    assert_eq!(Gf256::new(0x57) * Gf256::new(0x02), Gf256::new(0xAE));
    // редукция из спецификации QR: t^8 = t^4 + t^3 + t^2 + 1 = 0x1D
    assert_eq!(Gf256::new(0x80) * Gf256::new(0x02), Gf256::new(0x1D));
    assert_eq!(Gf256::alpha().pow(8), Gf256::new(0x1D));
    // продолжение цепочки степеней α
    assert_eq!(Gf256::new(0x1D) * Gf256::new(0x02), Gf256::new(0x3A));
    // α примитивен в 0x11D: порядок группы 255
    assert_eq!(Gf256::alpha().pow(255), Gf256::new(1));
}

#[test]
fn gf4_vectors() {
    // x^2 + x + 1: t^2 = t + 1, (t + 1)^2 = t, t · (t + 1) = 1
    assert_eq!(Gf4::new(2) * Gf4::new(2), Gf4::new(3));
    assert_eq!(Gf4::new(3) * Gf4::new(3), Gf4::new(2));
    assert_eq!(Gf4::new(2).inv(), Gf4::new(3));
    assert_eq!(Gf4::new(3).inv(), Gf4::new(2));
}

#[test]
fn gf16_alpha_powers() {
    // x^4 + x + 1: α = t имеет порядок 15
    let a = Gf16::alpha();
    assert_eq!(a.pow(4), Gf16::new(3)); // t^4 = t + 1
    assert_eq!(a.pow(5), Gf16::new(6)); // t^5 = t^2 + t
    assert_eq!(a.pow(14), Gf16::new(9)); // t^14 = t^3 + 1
    assert_eq!(a.pow(15), Gf16::new(1));
    assert_eq!(Gf16::new(3) * Gf16::new(3), Gf16::new(5)); // (t+1)^2 = t^2 + 1
    assert_eq!(Gf16::new(3).inv(), Gf16::new(14)); // 3 = α^4, inv = α^11 = t^3+t^2+t
}

#[test]
fn gf16_inverse_cross_check() {
    // инверсия согласована со степенями α: inv(α^k) = α^(15−k)
    let a = Gf16::alpha();
    for k in 1u32..15 {
        let x = a.pow(k);
        assert_eq!(x.inv(), a.pow(15 - k), "inv(α^{k})");
    }
}

// Псевдоним Gf65536 существует только на целях с указателем >= 32 бит.
#[cfg(not(target_pointer_width = "16"))]
#[test]
fn gf65536_generator_order() {
    use gf2m::Gf65536;
    // α = t имеет полный порядок 65535 = 3 · 5 · 17 · 257
    let a = Gf65536::alpha();
    assert_eq!(a.pow(65535), Gf65536::new(1));
    for p in [3u32, 5, 17, 257] {
        assert_ne!(a.pow(65535 / p), Gf65536::new(1), "α^(65535/{p}) == 1");
    }
}
