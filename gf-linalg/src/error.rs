//! Ошибки линейной алгебры.

use core::fmt;

/// Ошибка при создании матрицы, разборе текста, преобразовании коэффициентов
/// или выполнении операции линейной алгебры.
///
/// Методы с префиксом `try_` и операторы `+`, `-` и `*` возвращают эту ошибку в
/// [`Result`]. Оператор `?` позволяет передать её вызывающему коду, например из
/// функции с результатом `Result<_, LinalgError>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinalgError {
    /// Размеры матрицы недопустимы.
    InvalidDimensions {
        /// Число строк.
        rows: usize,
        /// Число столбцов.
        cols: usize,
    },
    /// Метод, требующий квадратную матрицу, получил прямоугольную матрицу.
    ///
    /// [`crate::Matrix::try_determinant`] и [`crate::Matrix::try_inverse`]
    /// возвращают этот вариант, если число строк не равно числу столбцов.
    NonSquareMatrix {
        /// Число строк исходной матрицы.
        rows: usize,
        /// Число столбцов исходной матрицы.
        cols: usize,
    },
    /// Квадратная матрица вырождена и не имеет обратной.
    ///
    /// [`crate::Matrix::try_inverse`] возвращает эту ошибку. При этом
    /// [`crate::Matrix::try_determinant`] успешно возвращает нулевой элемент
    /// поля.
    SingularMatrix,
    /// Число элементов не соответствует размерам матрицы.
    ElementCountMismatch {
        /// Ожидаемое число элементов.
        expected: usize,
        /// Фактическое число элементов.
        actual: usize,
    },
    /// Длины складываемых или вычитаемых векторов не совпадают.
    VectorLengthMismatch {
        /// Длина левого вектора.
        left: usize,
        /// Длина правого вектора.
        right: usize,
    },
    /// Формы складываемых матриц не совпадают.
    ///
    /// Для сложения должны совпадать и число строк, и число столбцов; равного
    /// количества элементов недостаточно.
    MatrixShapeMismatch {
        /// Число строк левой матрицы.
        left_rows: usize,
        /// Число столбцов левой матрицы.
        left_cols: usize,
        /// Число строк правой матрицы.
        right_rows: usize,
        /// Число столбцов правой матрицы.
        right_cols: usize,
    },
    /// Формы вычитаемых матриц не совпадают.
    ///
    /// Для вычитания должны совпадать и число строк, и число столбцов; равного
    /// количества элементов недостаточно.
    MatrixSubtractionShapeMismatch {
        /// Число строк левой матрицы.
        left_rows: usize,
        /// Число столбцов левой матрицы.
        left_cols: usize,
        /// Число строк правой матрицы.
        right_rows: usize,
        /// Число столбцов правой матрицы.
        right_cols: usize,
    },
    /// Число столбцов левой матрицы не совпадает с числом строк правой.
    ///
    /// Умножение `A * B` возможно, когда внутренние размеры совпадают:
    /// `A.cols() == B.rows()`. Внешние размеры задают форму произведения.
    MatrixProductMismatch {
        /// Число столбцов левой матрицы.
        left_cols: usize,
        /// Число строк правой матрицы.
        right_rows: usize,
    },
    /// Число столбцов матрицы не совпадает с длиной вектора.
    ///
    /// Для умножения `A * v` длина `v` должна равняться `A.cols()`; результат
    /// содержит по одному элементу для каждой строки `A`.
    MatrixVectorLengthMismatch {
        /// Число столбцов матрицы.
        matrix_cols: usize,
        /// Длина вектора.
        vector_len: usize,
    },
    /// Запрошенная операция пока не реализована.
    NotImplemented {
        /// Название операции, доступное во время всей жизни программы.
        operation: &'static str,
    },
    /// Заголовок текстового значения отсутствует или не совпадает с ожидаемым.
    ///
    /// Форматы требуют заголовки в нижнем регистре: `vector` или `matrix`.
    InvalidTextHeader {
        /// Ожидаемый заголовок, например `vector` или `matrix`.
        expected: &'static str,
    },
    /// Размер текстового значения отсутствует, не является беззнаковым
    /// десятичным ASCII-числом или не помещается в `usize`.
    InvalidTextDimension {
        /// Нулевой индекс ошибочного токена во входном тексте; заголовок имеет индекс 0.
        token_index: usize,
    },
    /// Число токенов элементов не совпадает с объявленным размером.
    ///
    /// Считаются токены после заголовка и размеров. Эта ошибка проверяется до
    /// формата отдельных элементов.
    TextElementCountMismatch {
        /// Ожидаемое число элементных токенов.
        expected: usize,
        /// Фактическое число элементных токенов.
        actual: usize,
    },
    /// Токен элемента не распознан реализацией [`crate::FieldText`].
    ///
    /// Индекс начинается с нуля и следует плоскому порядку элементов матрицы
    /// по строкам; у вектора это порядок элементов.
    InvalidTextElement {
        /// Нулевой индекс ошибочного элемента после заголовка и размера.
        index: usize,
    },
    /// Массив коэффициентов превышает допустимый объём одного многочлена.
    ///
    /// Предел `MAX_POLYNOMIAL_BYTES` учитывает только `len * size_of::<F>()`
    /// массива, а не общий размер строк матрицы.
    PolynomialCoefficientLimitExceeded {
        /// Индекс строки матрицы; `None` обозначает вектор.
        row: Option<usize>,
        /// Число коэффициентов в массиве.
        coefficients: usize,
        /// Максимальное число коэффициентов с учётом размера элемента поля.
        max_coefficients: usize,
    },
    /// Длина строки коэффициентов матрицы отличается от длины первой строки.
    ///
    /// Возвращается, когда очередная допустимая ширина отличается от ширины
    /// первой строки. Строки проверяются по порядку, поэтому более поздние
    /// ширины могут ещё не проверяться. Индекс строки начинается с нуля.
    PolynomialRowLengthMismatch {
        /// Нулевой индекс строки с отличающейся длиной.
        row: usize,
        /// Число коэффициентов в первой строке.
        expected: usize,
        /// Фактическое число коэффициентов в строке `row`.
        actual: usize,
    },
}

impl fmt::Display for LinalgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { rows, cols } => {
                write!(f, "недопустимые размеры матрицы: {rows} × {cols}")
            }
            Self::NonSquareMatrix { rows, cols } => {
                write!(f, "матрица не квадратная: {rows} × {cols}")
            }
            Self::SingularMatrix => {
                write!(f, "матрица вырождена: обратная матрица не существует")
            }
            Self::ElementCountMismatch { expected, actual } => {
                write!(
                    f,
                    "неверное число элементов матрицы: ожидалось {expected}, получено {actual}"
                )
            }
            Self::VectorLengthMismatch { left, right } => {
                write!(
                    f,
                    "длины векторов не совпадают: слева {left}, справа {right}"
                )
            }
            Self::MatrixShapeMismatch {
                left_rows,
                left_cols,
                right_rows,
                right_cols,
            } => write!(
                f,
                "размеры матриц для сложения не совпадают: слева {left_rows} × {left_cols}, справа {right_rows} × {right_cols}"
            ),
            Self::MatrixSubtractionShapeMismatch {
                left_rows,
                left_cols,
                right_rows,
                right_cols,
            } => write!(
                f,
                "размеры матриц для вычитания не совпадают: слева {left_rows} × {left_cols}, справа {right_rows} × {right_cols}"
            ),
            Self::MatrixProductMismatch {
                left_cols,
                right_rows,
            } => write!(
                f,
                "размеры матриц для умножения не совпадают: число столбцов слева {left_cols}, число строк справа {right_rows}"
            ),
            Self::MatrixVectorLengthMismatch {
                matrix_cols,
                vector_len,
            } => write!(
                f,
                "умножение матрицы на вектор невозможно: число столбцов матрицы {matrix_cols}, длина вектора {vector_len}"
            ),
            Self::NotImplemented { operation } => {
                write!(f, "операция не реализована: {operation}")
            }
            Self::InvalidTextHeader { expected } => write!(
                f,
                "неверный заголовок текстового формата: ожидалось `{expected}`"
            ),
            Self::InvalidTextDimension { token_index } => {
                write!(f, "неверный размер в токене {token_index}")
            }
            Self::TextElementCountMismatch { expected, actual } => write!(
                f,
                "неверное число элементов текста: ожидалось {expected}, получено {actual}"
            ),
            Self::InvalidTextElement { index } => {
                write!(f, "неверный текстовый элемент с индексом {index}")
            }
            Self::PolynomialCoefficientLimitExceeded {
                row,
                coefficients,
                max_coefficients,
            } => match row {
                None => write!(
                    f,
                    "превышен предел коэффициентов многочлена вектора: {coefficients} > {max_coefficients}"
                ),
                Some(row) => write!(
                    f,
                    "превышен предел коэффициентов многочлена в строке матрицы с индексом {row}: {coefficients} > {max_coefficients}"
                ),
            },
            Self::PolynomialRowLengthMismatch {
                row,
                expected,
                actual,
            } => write!(
                f,
                "длина коэффициентов строки матрицы с индексом {row} не совпадает: ожидалось {expected}, получено {actual}"
            ),
        }
    }
}

impl std::error::Error for LinalgError {}
