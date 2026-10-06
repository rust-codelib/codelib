use gf2m::Gf256;

use super::Matrix;
use crate::{FieldElement, LinalgError};

/// Результат приведения матрицы к приведённому ступенчатому виду.
///
/// Матрица и опорные столбцы хранятся вместе, чтобы их форма и метаданные
/// оставались согласованными. Создать или изменить поля напрямую нельзя.
/// Тип поля по умолчанию — `gf2m::Gf256`.
#[derive(Debug, PartialEq, Eq)]
pub struct RrefResult<F: FieldElement = Gf256> {
    matrix: Matrix<F>,
    pivot_columns: Vec<usize>,
}

fn find_pivot_row<F: FieldElement>(
    data: &[F],
    cols: usize,
    start_row: usize,
    row_count: usize,
    pivot_col: usize,
) -> Option<usize> {
    (start_row..row_count).find(|&row| !data[row * cols + pivot_col].is_zero())
}

fn swap_rows<F>(data: &mut [F], cols: usize, first_row: usize, second_row: usize) {
    for col in 0..cols {
        data.swap(first_row * cols + col, second_row * cols + col);
    }
}

fn scale_row<F: FieldElement>(data: &mut [F], cols: usize, row: usize, factor: F) {
    let row_offset = row * cols;
    for col in 0..cols {
        data[row_offset + col] = data[row_offset + col] * factor;
    }
}

fn subtract_row_multiple<F: FieldElement>(
    data: &mut [F],
    cols: usize,
    target_row: usize,
    pivot_row: usize,
    start_col: usize,
    factor: F,
) {
    let target_offset = target_row * cols;
    let pivot_offset = pivot_row * cols;
    for col in start_col..cols {
        let pivot_value = data[pivot_offset + col];
        data[target_offset + col] = data[target_offset + col] - factor * pivot_value;
    }
}

impl<F: FieldElement> RrefResult<F> {
    /// Возвращает приведённую матрицу без передачи владения.
    pub fn matrix(&self) -> &Matrix<F> {
        &self.matrix
    }

    /// Возвращает возрастающие индексы опорных столбцов, начиная с нуля.
    ///
    /// Для каждой опорной строки этот столбец содержит единицу, а остальные
    /// строки — нули.
    pub fn pivot_columns(&self) -> &[usize] {
        &self.pivot_columns
    }

    /// Возвращает ранг — число опорных столбцов.
    pub fn rank(&self) -> usize {
        self.pivot_columns.len()
    }

    /// Передаёт приведённую матрицу во владение вызывающему коду.
    ///
    /// После передачи матрицы список опорных столбцов отбрасывается.
    pub fn into_matrix(self) -> Matrix<F> {
        self.matrix
    }
}

impl<F: FieldElement> Matrix<F> {
    /// Возвращает приведённый ступенчатый вид матрицы и опорные столбцы.
    ///
    /// Преобразования строк оставляют столбцы в исходном порядке. Результат
    /// сохраняет форму матрицы; зависимые строки становятся нулевыми и
    /// располагаются внизу. Метод подходит квадратным и прямоугольным,
    /// вырожденным и нулевым матрицам. Исходная матрица остаётся неизменной.
    ///
    /// Ранг равен числу опорных столбцов. Индексы начинаются с нуля и
    /// перечисляются слева направо.
    ///
    /// # Examples
    ///
    /// ```
    /// use gf_linalg::{LinalgError, Matrix, RrefResult};
    /// use gfpm::Gf;
    ///
    /// type Gf2 = Gf<2, 1, 1>;
    ///
    /// fn main() -> Result<(), LinalgError> {
    ///     let matrix = Matrix::<Gf2>::try_new(
    ///         2,
    ///         3,
    ///         [1, 1, 0, 1, 0, 1].map(Gf2::new).to_vec(),
    ///     )?;
    ///     let result: RrefResult<Gf2> = matrix.rref();
    ///     let expected = Matrix::<Gf2>::try_new(
    ///         2,
    ///         3,
    ///         [1, 0, 1, 0, 1, 1].map(Gf2::new).to_vec(),
    ///     )?;
    ///
    ///     assert_eq!(result.matrix(), &expected);
    ///     assert_eq!(result.pivot_columns(), &[0, 1]);
    ///     assert_eq!(result.rank(), 2);
    ///     assert_eq!(matrix.rank(), 2);
    ///     assert_eq!(matrix.get(0, 1), Some(Gf2::one()));
    ///     Ok(())
    /// }
    /// ```
    pub fn rref(&self) -> RrefResult<F> {
        let mut data = self.data.clone();
        let mut pivot_columns = Vec::with_capacity(self.rows.min(self.cols));
        let mut pivot_row = 0;

        for pivot_col in 0..self.cols {
            if pivot_row == self.rows {
                break;
            }

            let Some(found_row) = find_pivot_row(&data, self.cols, pivot_row, self.rows, pivot_col)
            else {
                continue;
            };

            if found_row != pivot_row {
                swap_rows(&mut data, self.cols, found_row, pivot_row);
            }

            let pivot_offset = pivot_row * self.cols;
            let pivot_index = pivot_offset + pivot_col;
            let pivot_inverse = data[pivot_index].inv();
            scale_row(&mut data, self.cols, pivot_row, pivot_inverse);

            for row in 0..self.rows {
                if row == pivot_row {
                    continue;
                }

                let row_offset = row * self.cols;
                let factor = data[row_offset + pivot_col];
                if factor.is_zero() {
                    continue;
                }

                subtract_row_multiple(&mut data, self.cols, row, pivot_row, 0, factor);
            }

            pivot_columns.push(pivot_col);
            pivot_row += 1;
        }

        RrefResult {
            matrix: Self {
                rows: self.rows,
                cols: self.cols,
                data,
            },
            pivot_columns,
        }
    }

    /// Возвращает ранг матрицы — число опорных столбцов её RREF.
    ///
    /// Поддерживаются квадратные и прямоугольные, нулевые и вырожденные
    /// матрицы. Исходная матрица не изменяется.
    pub fn rank(&self) -> usize {
        self.rref().rank()
    }
}

impl<F: FieldElement> Matrix<F> {
    /// Вычисляет определитель квадратной матрицы методом Гаусса над полем `F`.
    ///
    /// Метод определён только для квадратных матриц. Для квадратной вырожденной
    /// матрицы результатом будет `Ok(F::zero())`. Прямоугольная матрица
    /// приводит к [`LinalgError::NonSquareMatrix`] с её фактическими размерами.
    ///
    /// Метод заимствует исходную матрицу и не меняет её.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::NonSquareMatrix`], если число строк не равно
    /// числу столбцов.
    pub fn try_determinant(&self) -> Result<F, LinalgError> {
        if self.rows != self.cols {
            return Err(LinalgError::NonSquareMatrix {
                rows: self.rows,
                cols: self.cols,
            });
        }

        let n = self.rows;
        let mut work = self.data.clone();
        let mut determinant = F::one();

        for pivot_col in 0..n {
            let Some(pivot_row) = find_pivot_row(&work, n, pivot_col, n, pivot_col) else {
                return Ok(F::zero());
            };

            if pivot_row != pivot_col {
                determinant = -determinant;
                swap_rows(&mut work, n, pivot_row, pivot_col);
            }

            let pivot = work[pivot_col * n + pivot_col];
            determinant = determinant * pivot;

            // Pivot гарантированно ненулевой после поиска выше.
            let pivot_inverse = pivot.inv();
            for row in pivot_col + 1..n {
                let row_offset = row * n;
                let factor = work[row_offset + pivot_col] * pivot_inverse;
                if factor.is_zero() {
                    continue;
                }

                subtract_row_multiple(&mut work, n, row, pivot_col, pivot_col + 1, factor);
                work[row_offset + pivot_col] = F::zero();
            }
        }

        Ok(determinant)
    }
}

impl<F: FieldElement> Matrix<F> {
    /// Возвращает обратную матрицу квадратной матрицы методом Гаусса–Жордана
    /// над полем `F`.
    ///
    /// Метод определён только для квадратных матриц. Если квадратная матрица
    /// вырождена, обратной матрицы нет и возвращается
    /// [`LinalgError::SingularMatrix`]. Для прямоугольной матрицы возвращается
    /// [`LinalgError::NonSquareMatrix`] с её фактическими размерами.
    /// Метод заимствует исходную матрицу и не меняет её.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::NonSquareMatrix`], если число строк не равно
    /// числу столбцов, или [`LinalgError::SingularMatrix`], если квадратная
    /// матрица вырождена.
    ///
    /// # Examples
    ///
    /// ```
    /// use gf2m::Gf256;
    /// use gf_linalg::{LinalgError, Matrix};
    ///
    /// fn main() -> Result<(), LinalgError> {
    ///     let matrix = Matrix::try_new(
    ///         2,
    ///         2,
    ///         vec![Gf256::one(), Gf256::one(), Gf256::one(), Gf256::zero()],
    ///     )?;
    ///
    ///     let determinant = matrix.try_determinant()?;
    ///     assert_eq!(determinant, Gf256::one());
    ///
    ///     let inverse = matrix.try_inverse()?;
    ///     let identity = Matrix::try_new(
    ///         2,
    ///         2,
    ///         vec![Gf256::one(), Gf256::zero(), Gf256::zero(), Gf256::one()],
    ///     )?;
    ///     assert_eq!(matrix.try_mul(&inverse)?, identity);
    ///
    ///     // Методы принимают &matrix, поэтому исходная матрица остаётся доступной.
    ///     assert_eq!(matrix.get(0, 0), Some(Gf256::one()));
    ///     Ok(())
    /// }
    /// ```
    pub fn try_inverse(&self) -> Result<Self, LinalgError> {
        if self.rows != self.cols {
            return Err(LinalgError::NonSquareMatrix {
                rows: self.rows,
                cols: self.cols,
            });
        }

        let n = self.rows;
        let element_count = n
            .checked_mul(n)
            .ok_or(LinalgError::InvalidDimensions { rows: n, cols: n })?;

        let mut left = self.data.clone();
        let mut right = vec![F::zero(); element_count];
        for index in 0..n {
            right[index * n + index] = F::one();
        }

        for pivot_col in 0..n {
            let Some(pivot_row) = find_pivot_row(&left, n, pivot_col, n, pivot_col) else {
                return Err(LinalgError::SingularMatrix);
            };

            if pivot_row != pivot_col {
                swap_rows(&mut left, n, pivot_row, pivot_col);
                swap_rows(&mut right, n, pivot_row, pivot_col);
            }

            let pivot_inverse = left[pivot_col * n + pivot_col].inv();
            scale_row(&mut left, n, pivot_col, pivot_inverse);
            scale_row(&mut right, n, pivot_col, pivot_inverse);

            for row in 0..n {
                if row == pivot_col {
                    continue;
                }

                let factor = left[row * n + pivot_col];
                if factor.is_zero() {
                    continue;
                }

                subtract_row_multiple(&mut left, n, row, pivot_col, 0, factor);
                subtract_row_multiple(&mut right, n, row, pivot_col, 0, factor);
            }
        }

        Ok(Self {
            rows: n,
            cols: n,
            data: right,
        })
    }
}
