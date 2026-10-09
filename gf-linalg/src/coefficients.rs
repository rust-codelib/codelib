//! Представление векторов и строк матриц массивами коэффициентов.
//!
//! Элемент с индексом `i` представляет коэффициент при `x^i`, поэтому порядок
//! массива соответствует возрастанию степеней. Пустые массивы и нулевые
//! коэффициенты в конце сохраняются: `[a, 0, 0]` остаётся массивом длины 3.
//! Эти преобразования задают только представление данных; отдельного типа
//! `Polynomial` и операций над многочленами в `gf-linalg` нет.
//!
//! [`MAX_POLYNOMIAL_BYTES`] ограничивает память элементов одного выходного
//! массива: `len * size_of::<F>()` не должно превышать 128 КиБ
//! (131 072 байта). Не учитываются метаданные значения `Box`, накладные
//! расходы аллокатора, внешний список строк и его запасная ёмкость. Число
//! коэффициентов вычисляется делением `MAX_POLYNOMIAL_BYTES / size_of::<F>()`
//! до копирования, без умножения, которое могло бы переполниться. Для
//! нулевого размера элемента helper возвращает `usize::MAX`.
//!
//! Длина самого [`Vector`] этим пределом не ограничена. При преобразовании
//! вектора в коэффициенты проверяется длина нового массива; его `Box<[F]>`
//! имеет ровно столько ячеек, сколько элементов, и запасной ёмкости нет.
//! Обратное преобразование принимает массив любой длины, в том числе длиннее
//! предела: оно создаёт вектор, к которому предел не применяется.
//!
//! Матрица представляется `Vec<Box<[F]>>`, по одному массиву на строку.
//! Предел `len * size_of::<F>()` применяется к каждой строке отдельно, а
//! суммарный объём строк может быть больше 128 КиБ. Ширина матрицы не
//! превышает 4096, но для крупного `F` строка всё равно может выйти за предел
//! коэффициентов, хотя сама матрица допустима. Например, строка максимальной
//! ширины с `Gf256` размером 2 байта занимает 8192 байта. При обратном
//! преобразовании предел коэффициентов не проверяется: проверяется только
//! форма. Сначала метод проверяет число строк, затем ширину первой строки;
//! остальные строки проверяются по порядку, сначала на допустимость ширины,
//! затем на совпадение с первой. Первая найденная ошибка возвращается до
//! выделения плоского буфера матрицы.
//!
//! В примере исходный вектор остаётся доступен после копирования коэффициентов.
//! Обратные методы принимают `Box` или `Vec<Box<_>>` во владение: переданные
//! коллекции переходят во владение метода и после вызова отдельно не
//! используются. При обратном преобразовании вектора буфер `Box` может
//! использоваться в `Vec`; матрица собирает новый плоский буфер, копируя в
//! него элементы строк.
//!
//! ```
//! use gf2m::Gf256;
//! use gf_linalg::{LinalgError, Matrix, Vector};
//!
//! fn main() -> Result<(), LinalgError> {
//!     let vector = Vector::new(vec![Gf256::new(7), Gf256::zero(), Gf256::zero()]);
//!     let coefficients = vector.try_to_polynomial_coefficients()?;
//!     assert_eq!(coefficients.as_ref(), &[Gf256::new(7), Gf256::zero(), Gf256::zero()]);
//!     assert_eq!(vector.len(), 3); // Источник не был перемещён или изменён.
//!     let restored = Vector::from_polynomial_coefficients(coefficients);
//!     assert_eq!(restored, vector);
//!
//!     let matrix = Matrix::try_new(
//!         2,
//!         3,
//!         vec![
//!             Gf256::new(1), Gf256::zero(), Gf256::zero(),
//!             Gf256::new(2), Gf256::new(3), Gf256::zero(),
//!         ],
//!     )?;
//!     let rows = matrix.try_to_polynomial_rows()?;
//!     assert_eq!(rows.len(), 2);
//!     assert_eq!(rows[0].as_ref(), &[Gf256::new(1), Gf256::zero(), Gf256::zero()]);
//!     let restored = Matrix::try_from_polynomial_rows(rows)?;
//!     assert_eq!(restored, matrix);
//!     Ok(())
//! }
//! ```

use crate::matrix::checked_element_count;
use crate::{FieldElement, LinalgError, Matrix, Vector, MAX_MATRIX_DIM};
use std::mem::size_of;

/// Максимальный объём элементов массива коэффициентов одного многочлена.
///
/// Предел учитывает только `len * size_of::<F>()` для одного массива
/// коэффициентов. Для конкретного `F` максимальная длина равна
/// `MAX_POLYNOMIAL_BYTES / size_of::<F>()`; метаданные `Box`, внешний список
/// строк и накладные расходы аллокатора не учитываются. Например, для
/// `Gf256`, занимающего 2 байта, предел допускает 65 536 коэффициентов.
/// Длина самого [`Vector`] и обратное преобразование этим пределом не ограничены.
pub const MAX_POLYNOMIAL_BYTES: usize = 128 * 1024;

fn max_coefficients_for_size(element_size: usize) -> usize {
    MAX_POLYNOMIAL_BYTES
        .checked_div(element_size)
        .unwrap_or(usize::MAX)
}

fn check_coefficient_limit<F: FieldElement>(
    row: Option<usize>,
    coefficients: usize,
) -> Result<(), LinalgError> {
    let max_coefficients = max_coefficients_for_size(size_of::<F>());
    if coefficients > max_coefficients {
        return Err(LinalgError::PolynomialCoefficientLimitExceeded {
            row,
            coefficients,
            max_coefficients,
        });
    }

    Ok(())
}

impl<F: FieldElement> Vector<F> {
    /// Создаёт отдельный массив коэффициентов в порядке возрастания степеней.
    ///
    /// Коэффициент по индексу `i` соответствует `x^i`. Пустая длина и
    /// хвостовые нули сохраняются без нормализации; исходный вектор остаётся
    /// неизменным. Предел применяется к длине создаваемого массива, а не к
    /// запасной ёмкости хранящегося вектора. Максимум зависит от
    /// `size_of::<F>()`.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::PolynomialCoefficientLimitExceeded`], если
    /// массив коэффициентов превышает [`MAX_POLYNOMIAL_BYTES`].
    pub fn try_to_polynomial_coefficients(&self) -> Result<Box<[F]>, LinalgError> {
        check_coefficient_limit::<F>(None, self.len())?;

        Ok(self.as_slice().to_vec().into_boxed_slice())
    }

    /// Принимает массив коэффициентов во владение и создаёт вектор.
    ///
    /// Индекс `i` становится коэффициентом при `x^i`. Порядок, пустой массив
    /// и нули в конце сохраняются. Дополнительного ограничения длины нет:
    /// массив больше [`MAX_POLYNOMIAL_BYTES`] также будет принят. Метод
    /// забирает `Box` во владение, а возвращённый вектор владеет данными.
    pub fn from_polynomial_coefficients(coefficients: Box<[F]>) -> Self {
        Self::new(Vec::from(coefficients))
    }
}

impl<F: FieldElement> Matrix<F> {
    /// Создаёт по одному массиву коэффициентов на каждую строку матрицы.
    ///
    /// Коэффициенты в строке идут по возрастанию степеней, а строки остаются
    /// в исходном порядке. Длина каждой строки точно равна числу столбцов;
    /// хвостовые нули сохраняются. Для каждого создаваемого массива отдельно
    /// проверяется [`MAX_POLYNOMIAL_BYTES`] с учётом `size_of::<F>()`. Общий
    /// объём всех строк этим пределом не ограничен. Например, при `Gf256`
    /// ширина 4096 столбцов даёт массив строки размером 8192 байта; у крупного
    /// элемента допустимая матрица может содержать строку больше предела.
    /// Исходная матрица не изменяется.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::PolynomialCoefficientLimitExceeded`] с
    /// индексом строки, если её массив коэффициентов превышает предел.
    /// Ограничение оси матрицы не отменяет проверку размера строки для `F`.
    pub fn try_to_polynomial_rows(&self) -> Result<Vec<Box<[F]>>, LinalgError> {
        check_coefficient_limit::<F>(Some(0), self.cols())?;

        Ok(self
            .as_slice()
            .chunks_exact(self.cols())
            .map(|row| row.to_vec().into_boxed_slice())
            .collect())
    }

    /// Принимает строки коэффициентов во владение и создаёт прямоугольную матрицу.
    ///
    /// Нужна хотя бы одна строка, а число строк и длина каждой строки должны
    /// быть в диапазоне `1..=MAX_MATRIX_DIM`. Все строки должны иметь
    /// одинаковую длину. После проверки непустого набора метод проверяет число
    /// строк, затем ширину первой строки. Остальные строки проверяются по
    /// порядку: ширина вне диапазона `1..=MAX_MATRIX_DIM` даёт
    /// [`LinalgError::InvalidDimensions`], а допустимая ширина, отличающаяся
    /// от первой, сразу даёт [`LinalgError::PolynomialRowLengthMismatch`] с
    /// нулевым индексом строки. Поэтому возвращается первая найденная ошибка
    /// среди строк; более поздняя ошибка не проверяется. Нулевые коэффициенты
    /// в конце не удаляются.
    ///
    /// Предел размера коэффициентов здесь не применяется: проверяется только
    /// форма строк. Поэтому допустимая по оси строка может быть длиннее
    /// [`MAX_POLYNOMIAL_BYTES`] в байтах для крупных элементов `F`.
    ///
    /// Строка с недопустимой длиной возвращает
    /// [`LinalgError::InvalidDimensions`] с числом строк во входе и длиной этой
    /// строки. Для допустимой, но отличающейся длины возвращается
    /// [`LinalgError::PolynomialRowLengthMismatch`] с индексом и обеими длинами.
    /// Проверки идут по порядку входных строк до выделения плоского буфера.
    /// Метод забирает входной `Vec<Box<[F]>>` во владение и копирует
    /// значения строк в новый плоский буфер матрицы.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::InvalidDimensions`] для пустого набора,
    /// превышения предела строк, недопустимой ширины первой строки или
    /// недопустимой ширины строки, найденной раньше рваной строки.
    /// Возвращает [`LinalgError::PolynomialRowLengthMismatch`] при первой
    /// найденной допустимой, но отличающейся ширине.
    pub fn try_from_polynomial_rows(rows: Vec<Box<[F]>>) -> Result<Self, LinalgError> {
        let row_count = rows.len();
        let Some(first_row) = rows.first() else {
            return Err(LinalgError::InvalidDimensions { rows: 0, cols: 0 });
        };

        let cols = first_row.len();
        if row_count > MAX_MATRIX_DIM {
            return Err(LinalgError::InvalidDimensions {
                rows: row_count,
                cols,
            });
        }
        if !(1..=MAX_MATRIX_DIM).contains(&cols) {
            return Err(LinalgError::InvalidDimensions {
                rows: row_count,
                cols,
            });
        }

        for (row_index, row) in rows.iter().enumerate().skip(1) {
            let actual_cols = row.len();
            if !(1..=MAX_MATRIX_DIM).contains(&actual_cols) {
                return Err(LinalgError::InvalidDimensions {
                    rows: row_count,
                    cols: actual_cols,
                });
            }
            if actual_cols != cols {
                return Err(LinalgError::PolynomialRowLengthMismatch {
                    row: row_index,
                    expected: cols,
                    actual: actual_cols,
                });
            }
        }

        let element_count = checked_element_count(row_count, cols)?;
        let mut data = Vec::with_capacity(element_count);
        for row in rows {
            data.extend_from_slice(&row);
        }

        Self::try_new(row_count, cols, data)
    }
}

#[cfg(test)]
mod tests {
    use super::max_coefficients_for_size;

    #[test]
    fn zero_sized_elements_have_no_coefficient_count_limit() {
        assert_eq!(max_coefficients_for_size(0), usize::MAX);
    }
}
