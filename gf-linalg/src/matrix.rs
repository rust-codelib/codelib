//! Матрицы над полем GF(256).

use gf2m::Gf256;
use std::ops::{Add, Mul};

use crate::{LinalgError, Vector, MAX_MATRIX_DIM};

/// Прямоугольная матрица элементов GF(256), хранящая значения по строкам.
///
/// Размер каждой оси находится в диапазоне `1..=MAX_MATRIX_DIM`. Матрица
/// принимает `Vec` во владение и сохраняет порядок элементов, включая нули.
/// Данные доступны для чтения через срез; изменение элемента не может поменять
/// длину хранилища. Равенство сравнивает форму и все элементы, поэтому матрицы
/// разных форм не равны, даже если их плоские данные совпадают.
///
/// Методы операций заимствуют матрицы и возвращают новые значения, не меняя
/// исходные. Операторы доступны и для принадлежащих значений (`a + b`, `a * b`),
/// и для ссылок (`&a + &b`, `&a * &b`); варианты со ссылками оставляют операнды
/// доступными после операции. Результат `+` и `*` имеет тип [`Result`], потому
/// что размеры могут оказаться несовместимы.
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
#[derive(Debug, PartialEq, Eq)]
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

    /// Складывает матрицу с матрицей той же формы по правилам GF(256).
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
    pub fn try_add(&self, rhs: &Self) -> Result<Self, LinalgError> {
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

    /// Умножает матрицы по правилам GF(256).
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
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, LinalgError> {
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
                let mut sum = Gf256::zero();
                for inner in 0..self.cols {
                    let left = self.data[row * self.cols + inner];
                    let right = rhs.data[inner * rhs.cols + col];
                    sum += left * right;
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

    /// Умножает матрицу на вектор по правилам GF(256).
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
    pub fn try_mul_vector(&self, rhs: &Vector) -> Result<Vector, LinalgError> {
        if self.cols != rhs.len() {
            return Err(LinalgError::MatrixVectorLengthMismatch {
                matrix_cols: self.cols,
                vector_len: rhs.len(),
            });
        }

        let mut data = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut sum = Gf256::zero();
            for col in 0..self.cols {
                let matrix_value = self.data[row * self.cols + col];
                let vector_value = rhs.as_slice()[col];
                sum += matrix_value * vector_value;
            }
            data.push(sum);
        }

        Ok(Vector::new(data))
    }

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

/// Сложение двух матриц, переданных оператору во владение.
///
/// Оба значения перемещаются в оператор, поэтому после выражения их нельзя
/// использовать. Результат имеет тип `Result<Matrix, LinalgError>` и содержит
/// сумму той же формы либо [`LinalgError::MatrixShapeMismatch`]. Чтобы сохранить
/// операнды доступными, используйте вариант `&Matrix + &Matrix`.
impl Add for Matrix {
    type Output = Result<Matrix, LinalgError>;

    fn add(self, rhs: Self) -> Self::Output {
        self.try_add(&rhs)
    }
}

/// Сложение заимствованных матриц без передачи владения операндами.
///
/// Выражение `&a + &b` возвращает `Result<Matrix, LinalgError>`; обе исходные
/// матрицы остаются доступными. Несовпадающие формы дают
/// [`LinalgError::MatrixShapeMismatch`].
impl Add<&Matrix> for &Matrix {
    type Output = Result<Matrix, LinalgError>;

    fn add(self, rhs: &Matrix) -> Self::Output {
        self.try_add(rhs)
    }
}

/// Умножение двух матриц, переданных оператору во владение.
///
/// Оба значения перемещаются в оператор, поэтому после выражения их нельзя
/// использовать. Результат имеет тип `Result<Matrix, LinalgError>` и содержит
/// матрицу с числом строк левого операнда и числом столбцов правого либо
/// ошибку несовместимых внутренних размеров. Чтобы сохранить операнды
/// доступными, используйте вариант `&Matrix * &Matrix`.
impl Mul for Matrix {
    type Output = Result<Matrix, LinalgError>;

    fn mul(self, rhs: Self) -> Self::Output {
        self.try_mul(&rhs)
    }
}

/// Умножение заимствованных матриц без передачи владения операндами.
///
/// Выражение `&a * &b` возвращает `Result<Matrix, LinalgError>`; обе исходные
/// матрицы остаются доступными. Число столбцов `a` должно совпасть с числом
/// строк `b`.
impl Mul<&Matrix> for &Matrix {
    type Output = Result<Matrix, LinalgError>;

    fn mul(self, rhs: &Matrix) -> Self::Output {
        self.try_mul(rhs)
    }
}

/// Умножение матрицы и вектора, переданных оператору во владение.
///
/// Матрица и вектор перемещаются в оператор, поэтому после выражения их нельзя
/// использовать. Результат имеет тип `Result<Vector, LinalgError>` и содержит
/// вектор длины `matrix.rows()` либо ошибку несовпадения длины вектора с числом
/// столбцов матрицы. Чтобы сохранить операнды доступными, используйте
/// вариант `&Matrix * &Vector`.
impl Mul<Vector> for Matrix {
    type Output = Result<Vector, LinalgError>;

    fn mul(self, rhs: Vector) -> Self::Output {
        self.try_mul_vector(&rhs)
    }
}

/// Умножение заимствованных матрицы и вектора без передачи владения.
///
/// Выражение `&matrix * &vector` возвращает `Result<Vector, LinalgError>`;
/// матрица и вектор остаются доступными. Длина вектора должна совпасть с числом
/// столбцов матрицы, а длина результата равна числу её строк.
impl Mul<&Vector> for &Matrix {
    type Output = Result<Vector, LinalgError>;

    fn mul(self, rhs: &Vector) -> Self::Output {
        self.try_mul_vector(rhs)
    }
}
