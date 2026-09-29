//! Представление векторов и строк матриц массивами коэффициентов.
//!
//! Элемент с индексом `i` представляет коэффициент при `x^i`, поэтому порядок
//! массива соответствует возрастанию степеней. Пустые массивы и нулевые
//! коэффициенты в конце сохраняются: `[a, 0, 0]` остаётся массивом длины 3.
//! Эти преобразования задают только представление данных; отдельного типа
//! `Polynomial` и операций над многочленами в `gf-linalg` нет.
//!
//! [`MAX_POLYNOMIAL_BYTES`] ограничивает память элементов одного выходного
//! массива: `len * size_of::<Gf256>()` не должно превышать 128 КиБ
//! (131 072 байта). Не учитываются метаданные значения `Box`, накладные
//! расходы аллокатора, внешний `Vec<Box<[Gf256]>>` и его запасная ёмкость.
//! Для текущего `Gf256`, размер которого равен 2 байтам, допускается до
//! 65 536 коэффициентов. Предел проверяется делением
//! `MAX_POLYNOMIAL_BYTES / size_of::<Gf256>()` до копирования, без
//! умножения, которое могло бы переполниться.
//!
//! Длина самого [`Vector`] этим пределом не ограничена. При преобразовании
//! вектора в коэффициенты проверяется длина нового массива; его `Box<[Gf256]>`
//! имеет ровно столько ячеек, сколько элементов, и запасной ёмкости нет.
//! Обратное преобразование принимает массив любой длины, в том числе длиннее
//! предела: оно создаёт вектор, к которому предел не применяется.
//!
//! Матрица представляется `Vec<Box<[Gf256]>>`, по одному массиву на строку.
//! Предел применяется к каждой строке отдельно, а сумма размеров строк может
//! быть больше 128 КиБ. Сейчас ширина матрицы ограничена 4096 столбцами, так
//! что массив одной строки занимает не более 8192 байт. При обратном
//! преобразовании сначала проверяется число строк, затем ширина первой строки,
//! после чего остальные строки по порядку: недопустимая ширина даёт
//! `InvalidDimensions`, а допустимая, но отличающаяся —
//! `PolynomialRowLengthMismatch`. Первый найденный дефект возвращается до
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

use crate::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};
use gf2m::Gf256;
use std::mem::size_of;

/// Максимальный объём элементов массива коэффициентов одного многочлена.
///
/// Предел учитывает только `len * size_of::<Gf256>()` для одного массива
/// коэффициентов. Метаданные `Box`, внешний список строк и накладные расходы
/// аллокатора не учитываются. При размере `Gf256` в 2 байта предел допускает
/// 65 536 коэффициентов. Длина самого [`Vector`] этим значением не ограничена.
pub const MAX_POLYNOMIAL_BYTES: usize = 128 * 1024;

fn check_coefficient_limit(row: Option<usize>, coefficients: usize) -> Result<(), LinalgError> {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    if coefficients > max_coefficients {
        return Err(LinalgError::PolynomialCoefficientLimitExceeded {
            row,
            coefficients,
            max_coefficients,
        });
    }

    Ok(())
}

impl Vector {
    /// Создаёт отдельный массив коэффициентов в порядке возрастания степеней.
    ///
    /// Коэффициент по индексу `i` соответствует `x^i`. Пустая длина и
    /// хвостовые нули сохраняются без нормализации; исходный вектор остаётся
    /// неизменным. Предел применяется к длине создаваемого массива, а не к
    /// запасной ёмкости хранящегося вектора. При текущем размере `Gf256` в
    /// 2 байта разрешено не более 65 536 коэффициентов.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::PolynomialCoefficientLimitExceeded`], если
    /// массив коэффициентов превышает [`MAX_POLYNOMIAL_BYTES`].
    pub fn try_to_polynomial_coefficients(&self) -> Result<Box<[Gf256]>, LinalgError> {
        check_coefficient_limit(None, self.len())?;

        Ok(self.as_slice().to_vec().into_boxed_slice())
    }

    /// Принимает массив коэффициентов во владение и создаёт вектор.
    ///
    /// Индекс `i` становится коэффициентом при `x^i`. Порядок, пустой массив
    /// и нули в конце сохраняются. Дополнительного ограничения длины нет:
    /// массив больше [`MAX_POLYNOMIAL_BYTES`] также будет принят. Метод
    /// забирает `Box` во владение, а возвращённый вектор владеет данными.
    pub fn from_polynomial_coefficients(coefficients: Box<[Gf256]>) -> Self {
        Self::new(Vec::from(coefficients))
    }
}

impl Matrix {
    /// Создаёт по одному массиву коэффициентов на каждую строку матрицы.
    ///
    /// Коэффициенты в строке идут по возрастанию степеней, а строки остаются
    /// в исходном порядке. Длина каждой строки точно равна числу столбцов;
    /// хвостовые нули сохраняются. Для каждого создаваемого массива отдельно
    /// проверяется [`MAX_POLYNOMIAL_BYTES`]. Общий объём всех строк этим
    /// пределом не ограничен. При `Gf256` ширина 4096 столбцов даёт массив
    /// строки размером 8192 байта. Исходная матрица не изменяется.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::PolynomialCoefficientLimitExceeded`] с
    /// индексом строки, если её массив коэффициентов превышает предел.
    /// Матрицы ограничены [`MAX_MATRIX_DIM`] столбцами, поэтому с текущим
    /// `Gf256` этот предел не может быть превышен, но проверка сохранена для
    /// согласованности контракта преобразования.
    pub fn try_to_polynomial_rows(&self) -> Result<Vec<Box<[Gf256]>>, LinalgError> {
        for row in 0..self.rows() {
            check_coefficient_limit(Some(row), self.cols())?;
        }

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
    /// Для текущего `Gf256` максимальная ширина ограничивает массив каждой
    /// строки 8192 байтами, что меньше [`MAX_POLYNOMIAL_BYTES`]. Общий объём
    /// всех строк может превышать этот предел.
    ///
    /// Строка с недопустимой длиной возвращает
    /// [`LinalgError::InvalidDimensions`] с числом строк во входе и длиной этой
    /// строки. Для допустимой, но отличающейся длины возвращается
    /// [`LinalgError::PolynomialRowLengthMismatch`] с индексом и обеими длинами.
    /// Проверки идут по порядку входных строк до выделения плоского буфера.
    /// Метод забирает входной `Vec<Box<[Gf256]>>` во владение и копирует
    /// значения строк в новый плоский буфер матрицы.
    ///
    /// # Errors
    ///
    /// Возвращает [`LinalgError::InvalidDimensions`] для пустого набора,
    /// превышения предела строк, недопустимой ширины первой строки или
    /// недопустимой ширины строки, найденной раньше рваной строки.
    /// Возвращает [`LinalgError::PolynomialRowLengthMismatch`] при первой
    /// найденной допустимой, но отличающейся ширине.
    pub fn try_from_polynomial_rows(rows: Vec<Box<[Gf256]>>) -> Result<Self, LinalgError> {
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
            let actual = row.len();
            if !(1..=MAX_MATRIX_DIM).contains(&actual) {
                return Err(LinalgError::InvalidDimensions {
                    rows: row_count,
                    cols: actual,
                });
            }
            if actual != cols {
                return Err(LinalgError::PolynomialRowLengthMismatch {
                    row: row_index,
                    expected: cols,
                    actual,
                });
            }
        }

        let element_count = row_count
            .checked_mul(cols)
            .ok_or(LinalgError::InvalidDimensions {
                rows: row_count,
                cols,
            })?;
        let mut data = Vec::with_capacity(element_count);
        for row in rows {
            data.extend_from_slice(&row);
        }

        Matrix::try_new(row_count, cols, data)
    }
}
