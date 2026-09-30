//! Логарифмические и экспоненциальные таблицы для ходовых полей.
//!
//! Таблицы строятся на этапе компиляции (`const fn`) и попадают в
//! `.rodata`: умножение и деление — два просмотра массива вместо
//! `O(m)` сдвигов. [`gf256`] доступен всегда; [`gf65536`] требует
//! цель с указателем не менее 32 бит (две таблицы по 128 КиБ).

/// Таблицы GF(256) с многочленом AES x^8 + x^4 + x^3 + x^2 + 1 (0x11D).
///
/// `LOG[0] = 0` — контрольное значение: логарифм нуля не определён,
/// операции с нулём обязаны отсекаться до обращения к таблице.
pub mod gf256 {
    /// Образующий многочлен поля (примитивный, из FIPS-197).
    pub const POLY: u32 = 0x11D;

    /// Экспоненты: `EXP[i] = α^i`, `i` в 0..256 (`EXP[255] = 1`).
    pub const EXP: [u16; 256] = super::build::<256, POLY>().0;

    /// Логарифмы: `LOG[α^i] = i` для `i` в 0..255; `LOG[0] = 0` (сентинел).
    pub const LOG: [u16; 256] = super::build::<256, POLY>().1;

    /// Умножение через таблицы: O(1). `0 · x = 0`.
    #[must_use]
    pub const fn mul(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let la = LOG[a as usize] as usize;
            let lb = LOG[b as usize] as usize;
            EXP[(la + lb) % 255] as u8
        }
    }

    /// Деление через таблицы: O(1). `0 / x = 0`, `x / 0 = 0`.
    #[must_use]
    pub const fn div(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let la = LOG[a as usize] as usize;
            let lb = LOG[b as usize] as usize;
            EXP[(la + 255 - lb) % 255] as u8
        }
    }

    /// Обратный элемент: O(1). Соглашение: `inv(0) = 0`.
    #[must_use]
    pub const fn inv(a: u8) -> u8 {
        if a == 0 {
            0
        } else {
            EXP[255 - LOG[a as usize] as usize] as u8
        }
    }
}

/// Таблицы GF(65536) с многочленом x^16 + x^5 + x^3 + x^2 + 1 (0x1002D).
///
/// Доступен только на целях с указателем не менее 32 бит: две таблицы
/// по 65536 элементов `u16` (256 КиБ `.rodata`) неадресуемы на 16-битных
/// целях.
#[cfg(not(target_pointer_width = "16"))]
pub mod gf65536 {
    /// Образующий многочлен поля (примитивный).
    pub const POLY: u32 = 0x1002D;

    /// Экспоненты: `EXP[i] = α^i`, `i` в 0..65536 (`EXP[65535] = 1`).
    // 128 КиБ в .rodata — намеренно: таблицы строятся на компиляции.
    #[allow(clippy::large_const_arrays)]
    pub const EXP: [u16; 65536] = super::build::<65536, POLY>().0;

    /// Логарифмы: `LOG[α^i] = i` для `i` в 0..65534; `LOG[0] = 0` (сентинел).
    // 128 КиБ в .rodata — намеренно: таблицы строятся на компиляции.
    #[allow(clippy::large_const_arrays)]
    pub const LOG: [u16; 65536] = super::build::<65536, POLY>().1;

    /// Умножение через таблицы: O(1). `0 · x = 0`.
    #[must_use]
    pub const fn mul(a: u16, b: u16) -> u16 {
        if a == 0 || b == 0 {
            0
        } else {
            let la = LOG[a as usize] as usize;
            let lb = LOG[b as usize] as usize;
            EXP[(la + lb) % 65535]
        }
    }

    /// Деление через таблицы: O(1). `0 / x = 0`, `x / 0 = 0`.
    #[must_use]
    pub const fn div(a: u16, b: u16) -> u16 {
        if a == 0 || b == 0 {
            0
        } else {
            let la = LOG[a as usize] as usize;
            let lb = LOG[b as usize] as usize;
            EXP[(la + 65535 - lb) % 65535]
        }
    }

    /// Обратный элемент: O(1). Соглашение: `inv(0) = 0`.
    #[must_use]
    pub const fn inv(a: u16) -> u16 {
        if a == 0 {
            0
        } else {
            EXP[65535 - LOG[a as usize] as usize]
        }
    }
}

/// Компиляторная сборка пары (EXP, LOG) для поля порядка `N = 2^m`
/// с образующим многочленом `POLY` (старший член включён).
const fn build<const N: usize, const POLY: u32>() -> ([u16; N], [u16; N]) {
    let mut exp = [0u16; N];
    let mut log = [0u16; N];
    let mut x: u32 = 1; // α^0
    let mut i: usize = 0;
    while i < N - 1 {
        exp[i] = x as u16;
        log[x as usize] = i as u16;
        // умножение на α = t: сдвиг и приведение по модулю POLY
        x <<= 1;
        if x >= N as u32 {
            x ^= POLY; // старший бит гасится членом x^m из POLY
        }
        i += 1;
    }
    exp[N - 1] = 1; // α^(N−1) = 1
    (exp, log)
}
