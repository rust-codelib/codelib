use super::Matrix;
use crate::{FieldElement, LinalgError, Vector};

impl<F: FieldElement> Matrix<F> {
    /// Возвращает новую матрицу с переставленными строками и столбцами.
    ///
    /// Элементы копируются в порядке по строкам транспонированной матрицы;
    /// число строк результата равно числу столбцов исходной матрицы, а число
    /// столбцов — числу строк. Исходная матрица остаётся неизменной.
    pub fn transpose(&self) -> Self {
        let mut data = Vec::with_capacity(self.data.len());

        for col in 0..self.cols {
            for row in 0..self.rows {
                data.push(self.data[row * self.cols + col]);
            }
        }

        Self {
            rows: self.cols,
            cols: self.rows,
            data,
        }
    }
}

impl<F: FieldElement> Matrix<F> {
    /// Складывает матрицу с матрицей той же формы по правилам поля `F`.
    ///
    /// При несовпадении числа строк или столбцов возвращает
    /// [`LinalgError::MatrixShapeMismatch`] с формами обоих операндов. При
    /// успехе создаёт матрицу той же формы, складывая элементы по позициям и
    /// сохраняя порядок по строкам; исходные матрицы не изменяются.
    ///
    /// # Ошибка
    ///
    /// Возвращает [`LinalgError::MatrixShapeMismatch`], если строки или столбцы
    /// операндов различаются. Сравниваются обе оси, даже когда общее число
    /// элементов у матриц одинаково.
    pub fn try_add(&self, rhs: &Matrix<F>) -> Result<Matrix<F>, LinalgError> {
        if self.rows != rhs.rows || self.cols != rhs.cols {
            return Err(LinalgError::MatrixShapeMismatch {
                left_rows: self.rows,
                left_cols: self.cols,
                right_rows: rhs.rows,
                right_cols: rhs.cols,
            });
        }

        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(&left, &right)| left + right)
            .collect();

        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    /// Вычитает матрицу той же формы по правилам поля `F`.
    ///
    /// При несовпадении числа строк или столбцов возвращает
    /// [`LinalgError::MatrixSubtractionShapeMismatch`] с формами обоих
    /// операндов. При успехе создаёт матрицу той же формы, вычисляя разность
    /// элементов по позициям; исходные матрицы не изменяются.
    ///
    /// # Ошибка
    ///
    /// Возвращает [`LinalgError::MatrixSubtractionShapeMismatch`], если строки
    /// или столбцы операндов различаются. Проверяются обе оси, даже когда
    /// общее число элементов у матриц одинаково.
    pub fn try_sub(&self, rhs: &Matrix<F>) -> Result<Matrix<F>, LinalgError> {
        if self.rows != rhs.rows || self.cols != rhs.cols {
            return Err(LinalgError::MatrixSubtractionShapeMismatch {
                left_rows: self.rows,
                left_cols: self.cols,
                right_rows: rhs.rows,
                right_cols: rhs.cols,
            });
        }

        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(&left, &right)| left - right)
            .collect();

        Ok(Matrix::<F> {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    /// Умножает матрицы по правилам поля `F`.
    ///
    /// Число столбцов `self` должно совпадать с числом строк `rhs`; иначе
    /// возвращается [`LinalgError::MatrixProductMismatch`] с размерами в
    /// порядке операндов. Результат имеет форму `self.rows() × rhs.cols()` и
    /// хранится по строкам. Исходные матрицы не изменяются.
    ///
    /// # Ошибка
    ///
    /// Возвращает [`LinalgError::MatrixProductMismatch`], когда внутренние
    /// размеры не совпадают. Если расчёт числа элементов результата переполняет
    /// `usize`, возвращается [`LinalgError::InvalidDimensions`].
    pub fn try_mul(&self, rhs: &Matrix<F>) -> Result<Matrix<F>, LinalgError> {
        if self.cols != rhs.rows {
            return Err(LinalgError::MatrixProductMismatch {
                left_cols: self.cols,
                right_rows: rhs.rows,
            });
        }

        let result_len = self
            .rows
            .checked_mul(rhs.cols)
            .ok_or(LinalgError::InvalidDimensions {
                rows: self.rows,
                cols: rhs.cols,
            })?;
        let mut data = Vec::with_capacity(result_len);

        for row in 0..self.rows {
            for col in 0..rhs.cols {
                let mut sum = F::zero();
                for inner in 0..self.cols {
                    let left = self.data[row * self.cols + inner];
                    let right = rhs.data[inner * rhs.cols + col];
                    sum = sum + left * right;
                }
                data.push(sum);
            }
        }

        Ok(Self {
            rows: self.rows,
            cols: rhs.cols,
            data,
        })
    }

    /// Умножает матрицу на вектор по правилам поля `F`.
    ///
    /// Длина `rhs` должна совпадать с числом столбцов матрицы; при
    /// несовпадении возвращается [`LinalgError::MatrixVectorLengthMismatch`]
    /// до вычисления результата. Успешный результат содержит по одному
    /// элементу на строку матрицы. Исходные матрица и вектор не изменяются.
    ///
    /// # Ошибка
    ///
    /// Возвращает [`LinalgError::MatrixVectorLengthMismatch`], если длина
    /// вектора не совпадает с числом столбцов матрицы.
    pub fn try_mul_vector(&self, rhs: &Vector<F>) -> Result<Vector<F>, LinalgError> {
        if self.cols != rhs.len() {
            return Err(LinalgError::MatrixVectorLengthMismatch {
                matrix_cols: self.cols,
                vector_len: rhs.len(),
            });
        }

        let mut data = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut sum = F::zero();
            for col in 0..self.cols {
                let matrix_value = self.data[row * self.cols + col];
                let vector_value = rhs.as_slice()[col];
                sum = sum + matrix_value * vector_value;
            }
            data.push(sum);
        }

        Ok(Vector::<F>::new(data))
    }
}
