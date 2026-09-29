//! Матрицы над полем GF(256).

use gf2m::Gf256;

use crate::{LinalgError, MAX_MATRIX_DIM};

/// Прямоугольная матрица элементов GF(256), хранящая значения по строкам.
///
/// Размер каждой оси находится в диапазоне `1..=MAX_MATRIX_DIM`. Матрица
/// принимает `Vec` во владение и сохраняет порядок элементов, включая нули.
/// Данные доступны для чтения через срез; изменение элемента не может поменять
/// длину хранилища.
///
/// ```
/// use gf2m::Gf256;
/// use gf_linalg::{Matrix, MAX_MATRIX_DIM};
///
/// let values = vec![Gf256::new(1), Gf256::new(2), Gf256::new(3), Gf256::zero()];
/// let matrix = Matrix::try_new(2, 2, values)?;
/// assert_eq!(matrix.rows(), 2);
/// assert_eq!(matrix.cols(), 2);
/// assert_eq!(matrix.row(0), Some(&[Gf256::new(1), Gf256::new(2)][..]));
/// assert_eq!(matrix.as_slice().len(), 4);
/// assert_eq!(MAX_MATRIX_DIM, 4096);
/// # Ok::<(), gf_linalg::LinalgError>(())
/// ```
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<Gf256>,
}

impl Matrix {
    /// Создаёт матрицу и принимает `Vec` во владение без копирования элементов.
    ///
    /// Число строк и столбцов должно быть в диапазоне
    /// `1..=MAX_MATRIX_DIM`. Элементы должны идти по строкам: сначала вся
    /// первая строка, затем вторая и так далее. Длина `data` должна точно
    /// равняться `rows * cols`; порядок и хвостовые нули сохраняются.
    ///
    /// Возвращает [`LinalgError::InvalidDimensions`], если хотя бы одна ось
    /// выходит за допустимый диапазон, и [`LinalgError::ElementCountMismatch`],
    /// если число элементов не совпадает с ожидаемым. Размеры проверяются до
    /// длины `data`.
    pub fn try_new(rows: usize, cols: usize, data: Vec<Gf256>) -> Result<Self, LinalgError> {
        if !(1..=MAX_MATRIX_DIM).contains(&rows) || !(1..=MAX_MATRIX_DIM).contains(&cols) {
            return Err(LinalgError::InvalidDimensions { rows, cols });
        }

        let expected = rows
            .checked_mul(cols)
            .ok_or(LinalgError::InvalidDimensions { rows, cols })?;
        if data.len() != expected {
            return Err(LinalgError::ElementCountMismatch {
                expected,
                actual: data.len(),
            });
        }

        Ok(Self { rows, cols, data })
    }

    /// Возвращает число строк.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Возвращает число столбцов.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Возвращает срез всех элементов в порядке по строкам.
    ///
    /// Срез не передаёт владение данными и не позволяет изменить длину
    /// внутреннего `Vec`.
    pub fn as_slice(&self) -> &[Gf256] {
        &self.data
    }

    /// Возвращает срез строки с индексом, начиная с нуля.
    ///
    /// `Some(срез)` содержит ровно `cols()` элементов. `None` возвращается,
    /// если индекс строки находится за границами матрицы, в том числе если он
    /// равен `usize::MAX`; паники не происходит.
    pub fn row(&self, row: usize) -> Option<&[Gf256]> {
        if row >= self.rows {
            return None;
        }

        let start = row * self.cols;
        Some(&self.data[start..start + self.cols])
    }

    /// Возвращает копию элемента в строке и столбце с индексами, начиная с нуля.
    ///
    /// Возвращает `Some(элемент)`, если обе координаты находятся в границах,
    /// и `None` в противном случае. Обе координаты проверяются до вычисления
    /// плоского индекса, поэтому выход за пределы столбцов не переходит в
    /// следующую строку. Индексы `usize::MAX` также безопасно дают `None`.
    pub fn get(&self, row: usize, col: usize) -> Option<Gf256> {
        if row >= self.rows || col >= self.cols {
            return None;
        }

        Some(self.data[row * self.cols + col])
    }

    /// Возвращает изменяемую ссылку на элемент в строке и столбце с нулевыми индексами.
    ///
    /// Через возвращённую ссылку можно заменить значение, не меняя длину
    /// матрицы. `None` возвращается без паники, если любая координата находится
    /// за границами, включая значение `usize::MAX`.
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Gf256> {
        if row >= self.rows || col >= self.cols {
            return None;
        }

        self.data.get_mut(row * self.cols + col)
    }
}
