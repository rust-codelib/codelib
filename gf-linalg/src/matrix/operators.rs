use std::ops::{Add, Mul, Sub};

use super::Matrix;
use crate::{FieldElement, LinalgError, Vector};

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
