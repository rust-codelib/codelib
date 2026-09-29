//! Текстовый формат [`Vector`] и [`Matrix`].
//!
//! Заголовок должен быть ровно `vector` или `matrix` в нижнем регистре.
//! Размеры — беззнаковые десятичные ASCII-числа; ведущие нули допустимы.
//! Элементы записываются как `0xNN`: на входе шестнадцатеричные цифры могут
//! быть строчными или прописными, префикс `0x` остаётся строчным, а вывод
//! всегда использует строчные цифры. Векторы могут иметь нулевую длину;
//! каждая ось матрицы должна быть в диапазоне `1..=MAX_MATRIX_DIM`.
//!
//! Между токенами принимаются только ASCII-разделители: пробел, табуляция,
//! перевод строки, вертикальная табуляция, перевод страницы и возврат каретки
//! (`U+0009`–`U+000D` и `U+0020`). В частности, вертикальная табуляция также
//! разделяет токены.
//!
//! Проверки выполняются по порядку: заголовок, наличие и формат размеров,
//! допустимость осей матрицы, точное число токенов элементов, затем значения
//! элементов. Поэтому ошибка количества элементов возникает раньше ошибки
//! формата отдельного элемента. Индексы размеров относятся к токенам всего
//! входа и начинаются с нуля; индекс элемента — плоский индекс в порядке по
//! строкам, также начиная с нуля. У вектора нет предела длины при чтении.
//!
//! `Display` выводит вектор в одну строку, а матрицу — с одной строкой на ряд
//! и без завершающего перевода строки. Оба типа можно разобрать обратно через
//! `str::parse`:
//!
//! ```
//! use gf2m::Gf256;
//! use gf_linalg::Vector;
//!
//! let vector = Vector::new(vec![Gf256::new(0x2a), Gf256::zero()]);
//! let text = vector.to_string();
//! assert_eq!(text, "vector 2 0x2a 0x00");
//! let parsed: Vector = text.parse().unwrap();
//! assert_eq!(parsed, vector); // Длина и хвостовой ноль сохранены.
//! ```
//!
//! ```
//! use gf2m::Gf256;
//! use gf_linalg::{LinalgError, Matrix};
//!
//! let matrix = Matrix::try_new(
//!     2,
//!     2,
//!     vec![Gf256::new(1), Gf256::zero(), Gf256::new(2), Gf256::new(0xff)],
//! )?;
//! let text = matrix.to_string();
//! assert_eq!(text, "matrix 2 2\n0x01 0x00\n0x02 0xff");
//! assert!(!text.ends_with('\n'));
//! let parsed: Matrix = text.parse()?;
//! assert_eq!(parsed, matrix);
//! # Ok::<(), LinalgError>(())
//! ```
//!
//! Разбор возвращает [`LinalgError`] в `Result`, поэтому вызывающий код может
//! обработать неверный ввод без паники:
//!
//! ```
//! use gf_linalg::{LinalgError, Vector};
//!
//! let result = "vector 2 0x01".parse::<Vector>();
//! assert_eq!(
//!     result,
//!     Err(LinalgError::TextElementCountMismatch {
//!         expected: 2,
//!         actual: 1,
//!     })
//! );
//! ```

use core::fmt;
use gf2m::Gf256;
use std::str::FromStr;

use crate::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};

impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vector {}", self.len())?;
        for element in self.as_slice() {
            write!(f, " 0x{:02x}", element.value())?;
        }
        Ok(())
    }
}

impl FromStr for Vector {
    type Err = LinalgError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut tokens = ascii_whitespace_tokens(text);

        if tokens.next() != Some("vector") {
            return Err(LinalgError::InvalidTextHeader { expected: "vector" });
        }

        let dimension = tokens
            .next()
            .ok_or(LinalgError::InvalidTextDimension { token_index: 1 })?;
        let length = parse_ascii_usize(dimension, 1)?;

        // Count before allocating the element buffer. In particular, a short
        // input declaring usize::MAX fails here without attempting that size.
        let actual = tokens.count();
        if actual != length {
            return Err(LinalgError::TextElementCountMismatch {
                expected: length,
                actual,
            });
        }

        let mut elements = Vec::with_capacity(length);
        for (index, token) in ascii_whitespace_tokens(text).skip(2).enumerate() {
            let value = parse_hex_byte(token).ok_or(LinalgError::InvalidTextElement { index })?;
            elements.push(Gf256::new(u16::from(value)));
        }

        Ok(Vector::new(elements))
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "matrix {} {}", self.rows(), self.cols())?;
        for row in self.as_slice().chunks(self.cols()) {
            writeln!(f)?;
            for (col, element) in row.iter().enumerate() {
                if col > 0 {
                    write!(f, " ")?;
                }
                write!(f, "0x{:02x}", element.value())?;
            }
        }
        Ok(())
    }
}

impl FromStr for Matrix {
    type Err = LinalgError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut tokens = ascii_whitespace_tokens(text);

        if tokens.next() != Some("matrix") {
            return Err(LinalgError::InvalidTextHeader { expected: "matrix" });
        }

        let rows_token = tokens
            .next()
            .ok_or(LinalgError::InvalidTextDimension { token_index: 1 })?;
        let rows = parse_ascii_usize(rows_token, 1)?;
        let cols_token = tokens
            .next()
            .ok_or(LinalgError::InvalidTextDimension { token_index: 2 })?;
        let cols = parse_ascii_usize(cols_token, 2)?;

        if !(1..=MAX_MATRIX_DIM).contains(&rows) || !(1..=MAX_MATRIX_DIM).contains(&cols) {
            return Err(LinalgError::InvalidDimensions { rows, cols });
        }

        let expected = rows
            .checked_mul(cols)
            .ok_or(LinalgError::InvalidDimensions { rows, cols })?;

        // Count every element token before allocating a buffer or parsing any
        // values, so malformed elements cannot hide a size mismatch.
        let actual = tokens.count();
        if actual != expected {
            return Err(LinalgError::TextElementCountMismatch { expected, actual });
        }

        let mut elements = Vec::with_capacity(expected);
        for (index, token) in ascii_whitespace_tokens(text).skip(3).enumerate() {
            let value = parse_hex_byte(token).ok_or(LinalgError::InvalidTextElement { index })?;
            elements.push(Gf256::new(u16::from(value)));
        }

        Matrix::try_new(rows, cols, elements)
    }
}

fn ascii_whitespace_tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| matches!(character, '\t'..='\r' | ' '))
        .filter(|token| !token.is_empty())
}

fn parse_ascii_usize(token: &str, token_index: usize) -> Result<usize, LinalgError> {
    if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(LinalgError::InvalidTextDimension { token_index });
    }

    token
        .parse::<usize>()
        .map_err(|_| LinalgError::InvalidTextDimension { token_index })
}

fn parse_hex_byte(token: &str) -> Option<u8> {
    let bytes = token.as_bytes();
    if bytes.len() != 4 || bytes[0] != b'0' || bytes[1] != b'x' {
        return None;
    }

    let high = hex_nibble(bytes[2])?;
    let low = hex_nibble(bytes[3])?;
    Some((high << 4) | low)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
