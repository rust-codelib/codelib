//! Текстовая запись массива коэффициентов с сохранением его длины.

use core::fmt;
use std::str::FromStr;

use crate::polynomial::check_coefficient_limit;
use crate::{FieldText, Polynomial, PolynomialError};

struct ElementDisplay<F: FieldText>(F);

impl<F: FieldText> fmt::Display for ElementDisplay<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        F::fmt_element(self.0, f)
    }
}

/// Выводит `polynomial N c0 c1 ...` с токенами [`FieldText`],
/// сохраняя длину и хвостовые нули. Поле выбирается типом при чтении.
impl<F: FieldText> fmt::Display for Polynomial<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "polynomial {}", self.coefficients().len())?;
        for &coefficient in self.coefficients() {
            write!(f, " {}", ElementDisplay(coefficient))?;
        }
        Ok(())
    }
}

/// Читает `polynomial N c0 c1 ...` с ASCII-разделителями между токенами.
///
/// `N` — десятичные ASCII-цифры, в том числе с ведущими нулями. Коэффициент —
/// токен [`FieldText`] с необязательным знаком вплотную к нему. Минус отрицает
/// элемент поля; в `gfpm` это отрицание упакованного элемента, не целого числа.
/// Собственный разбор `FieldText` имеет приоритет перед обработкой знака.
///
/// Порядок проверок: заголовок, формат длины, предел памяти, число токенов,
/// коэффициенты слева направо. Буфер выделяется после проверки числа токенов.
impl<F: FieldText> FromStr for Polynomial<F> {
    type Err = PolynomialError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut tokens = ascii_whitespace_tokens(text);
        if tokens.next() != Some("polynomial") {
            return Err(PolynomialError::InvalidTextHeader);
        }
        let length_token = tokens.next().ok_or(PolynomialError::InvalidTextLength)?;
        if !length_token.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(PolynomialError::InvalidTextLength);
        }
        let length = length_token
            .parse::<usize>()
            .map_err(|_| PolynomialError::InvalidTextLength)?;
        check_coefficient_limit::<F>(length)?;

        let actual = tokens.count();
        if actual != length {
            return Err(PolynomialError::TextCoefficientCountMismatch {
                expected: length,
                actual,
            });
        }

        let mut coefficients = Vec::with_capacity(length);
        for (index, token) in ascii_whitespace_tokens(text).skip(2).enumerate() {
            let coefficient = parse_signed_element::<F>(token)
                .ok_or(PolynomialError::InvalidTextCoefficient { index })?;
            coefficients.push(coefficient);
        }
        Self::try_new(coefficients)
    }
}

fn ascii_whitespace_tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| matches!(character, '\t'..='\r' | ' '))
        .filter(|token| !token.is_empty())
}

fn parse_signed_element<F: FieldText>(token: &str) -> Option<F> {
    // Пользовательский канонический токен сам может начинаться со знака.
    if let Some(element) = F::parse_element(token) {
        return Some(element);
    }
    let (negative, magnitude) = match token.as_bytes().first() {
        Some(b'-') => (true, &token[1..]),
        Some(b'+') => (false, &token[1..]),
        _ => return None,
    };
    if magnitude.is_empty() {
        return None;
    }
    let element = F::parse_element(magnitude)?;
    Some(if negative { -element } else { element })
}
