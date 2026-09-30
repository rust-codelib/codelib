//! Матрицы над конечным полем.

use gf2m::Gf256;
use std::ops::{Add, Mul, Sub};

use crate::{FieldElement, LinalgError, Vector, MAX_MATRIX_DIM};

/// Прямоугольная матрица элементов поля `F`, хранящая значения по строкам.
///
/// Размер каждой оси находится в диапазоне `1..=MAX_MATRIX_DIM`. Матрица
/// принимает `Vec` во владение и сохраняет порядок элементов, включая нули.
/// Данные доступны для чтения через срез; изменение элемента не может поменять
/// длину хранилища. Равенство сравнивает форму и все элементы, поэтому матрицы
/// разных форм не равны, даже если их плоские данные совпадают.
///
/// Методы операций заимствуют матрицы и возвращают новые значения, не меняя
/// исходные. Операторы доступны и для принадлежащих значений (`a + b`, `a - b`,
/// `a * b`), и для ссылок (`&a + &b`, `&a - &b`, `&a * &b`); варианты со
/// ссылками оставляют операнды доступными после операции. Результат этих
/// операторов имеет тип [`Result`], потому что размеры могут оказаться
/// несовместимы.
///
/// Параметр поля по умолчанию — `gf2m::Gf256`.
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
///
/// Операции требуют одного и того же типа поля. Следующие выражения не
/// компилируются, хотя эти операции доступны для матриц и векторов одного поля:
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::Matrix;
/// use gfpm::Gf9;
///
/// let binary = Matrix::<Gf256>::try_new(1, 1, vec![Gf256::one()]).unwrap();
/// let ternary = Matrix::<Gf9>::try_new(1, 1, vec![Gf9::one()]).unwrap();
/// let _ = &binary + &binary;
/// let _ = binary.try_add(&ternary);
/// let _ = &binary + &ternary;
/// ```
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::Matrix;
/// use gfpm::Gf9;
///
/// let binary = Matrix::<Gf256>::try_new(1, 1, vec![Gf256::one()]).unwrap();
/// let ternary = Matrix::<Gf9>::try_new(1, 1, vec![Gf9::one()]).unwrap();
/// let _ = &binary - &binary;
/// let _ = binary.try_sub(&ternary);
/// let _ = &binary - &ternary;
/// ```
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::{Matrix, Vector};
/// use gfpm::Gf9;
///
/// let binary = Matrix::<Gf256>::try_new(1, 1, vec![Gf256::one()]).unwrap();
/// let binary_vector = Vector::<Gf256>::new(vec![Gf256::one()]);
/// let ternary = Vector::<Gf9>::new(vec![Gf9::one()]);
/// let _ = &binary * &binary_vector;
/// let _ = binary.try_mul_vector(&ternary);
/// let _ = &binary * &ternary;
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Matrix<F: FieldElement = Gf256> {
    rows: usize,
    cols: usize,
    data: Vec<F>,
}

impl<F: FieldElement> Matrix<F> {
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
    pub fn try_new(rows: usize, cols: usize, data: Vec<F>) -> Result<Self, LinalgError> {
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
            let Some(pivot_row) = (pivot_col..n).find(|&row| !work[row * n + pivot_col].is_zero())
            else {
                return Ok(F::zero());
            };

            if pivot_row != pivot_col {
                determinant = -determinant;
                for col in 0..n {
                    work.swap(pivot_row * n + col, pivot_col * n + col);
                }
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

                let pivot_offset = pivot_col * n;
                for col in pivot_col + 1..n {
                    let pivot_value = work[pivot_offset + col];
                    work[row_offset + col] = work[row_offset + col] - factor * pivot_value;
                }
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
            let Some(pivot_row) = (pivot_col..n).find(|&row| !left[row * n + pivot_col].is_zero())
            else {
                return Err(LinalgError::SingularMatrix);
            };

            if pivot_row != pivot_col {
                for col in 0..n {
                    left.swap(pivot_row * n + col, pivot_col * n + col);
                    right.swap(pivot_row * n + col, pivot_col * n + col);
                }
            }

            let pivot_inverse = left[pivot_col * n + pivot_col].inv();
            for col in 0..n {
                left[pivot_col * n + col] = left[pivot_col * n + col] * pivot_inverse;
                right[pivot_col * n + col] = right[pivot_col * n + col] * pivot_inverse;
            }

            let pivot_offset = pivot_col * n;
            for row in 0..n {
                if row == pivot_col {
                    continue;
                }

                let row_offset = row * n;
                let factor = left[row_offset + pivot_col];
                if factor.is_zero() {
                    continue;
                }

                for col in 0..n {
                    let left_pivot_value = left[pivot_offset + col];
                    let right_pivot_value = right[pivot_offset + col];
                    left[row_offset + col] = left[row_offset + col] - factor * left_pivot_value;
                    right[row_offset + col] = right[row_offset + col] - factor * right_pivot_value;
                }
            }
        }

        Ok(Self {
            rows: n,
            cols: n,
            data: right,
        })
    }
}

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
    pub fn as_slice(&self) -> &[F] {
        &self.data
    }

    /// Возвращает срез строки с индексом, начиная с нуля.
    ///
    /// `Some(срез)` содержит ровно `cols()` элементов. `None` возвращается,
    /// если индекс строки находится за границами матрицы, в том числе если он
    /// равен `usize::MAX`; паники не происходит.
    pub fn row(&self, row: usize) -> Option<&[F]> {
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
    pub fn get(&self, row: usize, col: usize) -> Option<F> {
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
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut F> {
        if row >= self.rows || col >= self.cols {
            return None;
        }

        self.data.get_mut(row * self.cols + col)
    }
}

/// Сложение двух матриц, переданных оператору во владение.
///
/// Оба значения перемещаются в оператор. Результат содержит сумму той же
/// формы либо [`LinalgError::MatrixShapeMismatch`].
impl<F: FieldElement> Add for Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn add(self, rhs: Self) -> Self::Output {
        self.try_add(&rhs)
    }
}

/// Сложение заимствованных матриц без передачи владения операндами.
///
/// Выражение `&a + &b` оставляет обе исходные матрицы доступными. Несовпадающие
/// формы дают [`LinalgError::MatrixShapeMismatch`].
impl<F: FieldElement> Add<&Matrix<F>> for &Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn add(self, rhs: &Matrix<F>) -> Self::Output {
        self.try_add(rhs)
    }
}

/// Вычитание двух матриц, переданных оператору во владение.
///
/// Оба значения перемещаются в оператор. Результат содержит разность той же
/// формы либо [`LinalgError::MatrixSubtractionShapeMismatch`].
impl<F: FieldElement> Sub for Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.try_sub(&rhs)
    }
}

/// Вычитание заимствованных матриц без передачи владения операндами.
///
/// Выражение `&a - &b` оставляет обе исходные матрицы доступными. Несовпадающие
/// формы дают [`LinalgError::MatrixSubtractionShapeMismatch`].
impl<F: FieldElement> Sub<&Matrix<F>> for &Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn sub(self, rhs: &Matrix<F>) -> Self::Output {
        self.try_sub(rhs)
    }
}

/// Умножение двух матриц, переданных оператору во владение.
///
/// Результат имеет число строк левого операнда и число столбцов правого либо
/// ошибку несовместимых внутренних размеров.
impl<F: FieldElement> Mul for Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn mul(self, rhs: Self) -> Self::Output {
        self.try_mul(&rhs)
    }
}

/// Умножение заимствованных матриц без передачи владения операндами.
///
/// Выражение `&a * &b` оставляет исходные матрицы доступными. Число столбцов
/// `a` должно совпасть с числом строк `b`.
impl<F: FieldElement> Mul<&Matrix<F>> for &Matrix<F> {
    type Output = Result<Matrix<F>, LinalgError>;

    fn mul(self, rhs: &Matrix<F>) -> Self::Output {
        self.try_mul(rhs)
    }
}

/// Умножение матрицы и вектора, переданных оператору во владение.
///
/// Результат содержит вектор длины `matrix.rows()` либо ошибку несовпадения
/// длины вектора с числом столбцов матрицы.
impl<F: FieldElement> Mul<Vector<F>> for Matrix<F> {
    type Output = Result<Vector<F>, LinalgError>;

    fn mul(self, rhs: Vector<F>) -> Self::Output {
        self.try_mul_vector(&rhs)
    }
}

/// Умножение заимствованных матрицы и вектора без передачи владения.
///
/// Выражение `&matrix * &vector` оставляет исходные значения доступными. Длина
/// вектора должна совпасть с числом столбцов матрицы.
impl<F: FieldElement> Mul<&Vector<F>> for &Matrix<F> {
    type Output = Result<Vector<F>, LinalgError>;

    fn mul(self, rhs: &Vector<F>) -> Self::Output {
        self.try_mul_vector(rhs)
    }
}
