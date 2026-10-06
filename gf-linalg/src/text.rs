//! Текстовый формат [`Vector`] и [`Matrix`].
//!
//! Заголовок должен быть ровно `vector` или `matrix` в нижнем регистре.
//! Размеры — беззнаковые десятичные ASCII-числа; ведущие нули допустимы.
//! Формат токена элемента задаётся [`FieldText`]. Для семейства `gf2m` это
//! `0x` и фиксированное число шестнадцатеричных ASCII-цифр; цифры на входе
//! могут быть строчными или прописными, префикс `0x` остаётся строчным, а
//! вывод использует строчные цифры. Векторы могут иметь нулевую длину;
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
use std::str::FromStr;

use crate::matrix::checked_element_count;
use crate::{FieldText, LinalgError, Matrix, Vector};

struct ElementDisplay<F: FieldText>(F);

impl<F: FieldText> fmt::Display for ElementDisplay<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_element(f)
    }
}

impl<F: FieldText> fmt::Display for Vector<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vector {}", self.len())?;
        for element in self.as_slice() {
            write!(f, " {}", ElementDisplay(*element))?;
        }
        Ok(())
    }
}

impl<F: FieldText> FromStr for Vector<F> {
    type Err = LinalgError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut tokens = ascii_whitespace_tokens(text);

        if tokens.next() != Some("vector") {
            return Err(LinalgError::InvalidTextHeader { expected: "vector" });
        }

        let length_token = tokens
            .next()
            .ok_or(LinalgError::InvalidTextDimension { token_index: 1 })?;
        let length = parse_ascii_usize(length_token, 1)?;

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
            let element =
                F::parse_element(token).ok_or(LinalgError::InvalidTextElement { index })?;
            elements.push(element);
        }

        Ok(Vector::<F>::new(elements))
    }
}

impl<F: FieldText> fmt::Display for Matrix<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "matrix {} {}", self.rows(), self.cols())?;
        for row in self.as_slice().chunks(self.cols()) {
            writeln!(f)?;
            for (col, element) in row.iter().enumerate() {
                if col > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", ElementDisplay(*element))?;
            }
        }
        Ok(())
    }
}

impl<F: FieldText> FromStr for Matrix<F> {
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

        let expected = checked_element_count(rows, cols)?;

        // Count every element token before allocating a buffer or parsing any
        // values, so malformed elements cannot hide a size mismatch.
        let actual = tokens.count();
        if actual != expected {
            return Err(LinalgError::TextElementCountMismatch { expected, actual });
        }

        let mut elements = Vec::with_capacity(expected);
        for (index, token) in ascii_whitespace_tokens(text).skip(3).enumerate() {
            let element =
                F::parse_element(token).ok_or(LinalgError::InvalidTextElement { index })?;
            elements.push(element);
        }

        Matrix::<F>::try_new(rows, cols, elements)
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
