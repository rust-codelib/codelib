//! Текстовое представление элементов статического поля.

use core::fmt;

use crate::FieldElement;

/// Форматирование и разбор одного элемента поля как токена.
///
/// Текстовый контракт отделён от [`FieldElement`], чтобы арифметические
/// алгоритмы могли работать с полем без требования строкового представления.
/// Форматирующая реализация должна выводить один непустой токен без ASCII
/// пробельных символов. Разбор должен проверять формат и диапазон до
/// конструирования элемента и возвращать `None` для неверного токена.
/// Канонический токен каждого допустимого элемента должен разбираться обратно
/// в тот же элемент.
///
/// Оба встроенных семейства реализуют этот трейт для всех своих статических
/// параметров. Для `gf2m::Gf` токен имеет вид `0x` и фиксированное число
/// шестнадцатеричных цифр. Для `gfpm::Gf` токен — десятичное представление
/// упакованного `u64`, возвращаемого `value()`, со значением меньше `ORDER`;
/// это упакованный код элемента, а не десятичная запись элемента поля.
/// Разбор для `gfpm::Gf` принимает только ASCII-цифры `0`–`9`, в том числе с
/// ведущими нулями; форматирование выводит каноническое десятичное значение без
/// ведущих нулей. После разбора `u64` его значение сравнивается с `ORDER` как
/// `u128` до создания элемента.
/// `Display`/`FromStr` контейнеров требуют `FieldText`, тогда как арифметика и
/// преобразования коэффициентов от текста не зависят.
pub trait FieldText: FieldElement {
    /// Записывает канонический токен элемента непосредственно в форматтер.
    fn fmt_element(self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Разбирает один токен, возвращая `None`, если формат или значение неверны.
    fn parse_element(token: &str) -> Option<Self>;
}

impl<const N: usize, const POLY: u32> FieldText for gf2m::Gf<N, POLY> {
    fn fmt_element(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = (Self::M as usize).div_ceil(4).max(2);
        write!(f, "0x{:0width$x}", self.value(), width = width)
    }

    fn parse_element(token: &str) -> Option<Self> {
        let width = (Self::M as usize).div_ceil(4).max(2);
        let bytes = token.as_bytes();
        if bytes.len() != width + 2 || bytes[..2] != *b"0x" {
            return None;
        }

        let mut value = 0u16;
        for &digit in &bytes[2..] {
            let nibble = match digit {
                b'0'..=b'9' => digit - b'0',
                b'a'..=b'f' => digit - b'a' + 10,
                b'A'..=b'F' => digit - b'A' + 10,
                _ => return None,
            };
            value = value.checked_mul(16)?.checked_add(u16::from(nibble))?;
        }

        if usize::from(value) >= N {
            return None;
        }

        Some(gf2m::Gf::<N, POLY>::new(value))
    }
}

impl<const P: u32, const M: usize, const POLY: u128> FieldText for gfpm::Gf<P, M, POLY> {
    fn fmt_element(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }

    fn parse_element(token: &str) -> Option<Self> {
        let bytes = token.as_bytes();
        if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
            return None;
        }

        let packed = token.parse::<u64>().ok()?;
        if u128::from(packed) >= Self::ORDER {
            return None;
        }

        Some(gfpm::Gf::<P, M, POLY>::new(packed))
    }
}
