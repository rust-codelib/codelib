//! Постепенное построение базиса системы с несколькими правыми частями.

use core::mem::size_of;

use gf2m::Gf256;

use crate::{FieldElement, LinalgError, Matrix, MAX_MATRIX_DIM};

/// Результат вставки уравнения в [`LinearSystem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquationInsertion {
    /// Новое независимое уравнение увеличило ранг на единицу.
    Independent {
        /// Индекс опорного коэффициента, начиная с нуля.
        pivot_column: usize,
        /// Ранг после вставки.
        rank: usize,
    },
    /// Уравнение следует из сохранённого базиса; система не изменена.
    Dependent,
    /// Уравнение противоречит базису; система не изменена.
    Inconsistent {
        /// Первый противоречивый столбец правой части, начиная с нуля.
        rhs_column: usize,
    },
}

#[derive(Debug)]
struct Row<F> {
    pivot: usize,
    coefficients: Vec<F>,
    rhs: Vec<F>,
}

/// Система `A * X = B`, принимающая по одному уравнению.
///
/// Сохраняет не более `unknowns` независимых нормализованных ступенчатых строк.
/// Коэффициенты и правые части хранятся раздельно; ограничение каждой оси
/// [`MAX_MATRIX_DIM`] сохраняется без создания дополненной матрицы `[A | B]`.
/// Новые опорные столбцы могут приходить в любом порядке. Противоречие —
/// результат вставки, а не состояние, запрещающее дальнейшую работу с базисом.
///
/// Операции заимствуют входы и не меняют их. Память строк выделяется постепенно;
/// недостаток памяти аллокатора обрабатывается так же, как для [`Matrix`].
/// Худшая сложность заполнения базиса — кубическая по числу неизвестных,
/// плюс операции с правыми частями.
///
/// # Пример
///
/// ```
/// use gf_linalg::{EquationInsertion, LinearSystem};
/// use gfpm::Gf5;
///
/// let mut system = LinearSystem::<Gf5>::try_new(2, 1)?;
/// system.insert_equation(&[Gf5::new(1), Gf5::new(1)], &[Gf5::new(1)])?;
/// assert!(system.try_solution()?.is_none());
/// system.insert_equation(&[Gf5::new(2), Gf5::new(1)], &[Gf5::new(3)])?;
/// let solution = system.try_solution()?.unwrap();
/// assert_eq!(solution.as_slice(), &[Gf5::new(2), Gf5::new(4)]);
/// assert_eq!(
///     system.insert_equation(&[Gf5::new(0); 2], &[Gf5::new(1)])?,
///     EquationInsertion::Inconsistent { rhs_column: 0 },
/// );
/// assert_eq!(system.rank(), 2);
/// # Ok::<(), gf_linalg::LinalgError>(())
/// ```
#[derive(Debug)]
pub struct LinearSystem<F: FieldElement = Gf256> {
    unknowns: usize,
    rhs_columns: usize,
    rows: Vec<Row<F>>,
}

fn check_buffer<T>(len: usize) -> Result<(), LinalgError> {
    let bytes = len
        .checked_mul(size_of::<T>())
        .ok_or(LinalgError::SizeOverflow)?;
    if bytes > isize::MAX as usize {
        return Err(LinalgError::SizeOverflow);
    }
    Ok(())
}

impl<F: FieldElement> LinearSystem<F> {
    /// Создаёт пустой базис для заданного числа неизвестных и правых частей.
    ///
    /// # Ошибки
    ///
    /// [`LinalgError::InvalidLinearSystemDimensions`] — если одна из осей вне
    /// `1..=MAX_MATRIX_DIM`. [`LinalgError::SizeOverflow`] — если строка,
    /// таблица строк или буфер решения не адресуются срезом Rust.
    /// Проверяет размеры будущего решения до выделения памяти.
    pub fn try_new(unknowns: usize, rhs_columns: usize) -> Result<Self, LinalgError> {
        if !(1..=MAX_MATRIX_DIM).contains(&unknowns) || !(1..=MAX_MATRIX_DIM).contains(&rhs_columns)
        {
            return Err(LinalgError::InvalidLinearSystemDimensions {
                unknowns,
                rhs_columns,
            });
        }
        let solution_len = unknowns
            .checked_mul(rhs_columns)
            .ok_or(LinalgError::SizeOverflow)?;
        check_buffer::<F>(unknowns)?;
        check_buffer::<F>(rhs_columns)?;
        check_buffer::<F>(solution_len)?;
        check_buffer::<Row<F>>(unknowns)?;
        Ok(Self {
            unknowns,
            rhs_columns,
            rows: Vec::with_capacity(unknowns),
        })
    }

    /// Число неизвестных, равное числу столбцов коэффициентов.
    pub fn unknowns(&self) -> usize {
        self.unknowns
    }

    /// Число столбцов правой части.
    pub fn rhs_columns(&self) -> usize {
        self.rhs_columns
    }

    /// Число независимых уравнений в базисе.
    pub fn rank(&self) -> usize {
        self.rows.len()
    }

    /// Добавляет уравнение, если оно независимо от сохранённых строк.
    ///
    /// Зависимое или противоречивое уравнение возвращает соответствующий
    /// [`EquationInsertion`] без изменения базиса. Проверяет число коэффициентов
    /// раньше ширины правой части; все вычисления выполняет на копиях входов.
    ///
    /// # Ошибки
    ///
    /// [`LinalgError::EquationCoefficientCountMismatch`] или
    /// [`LinalgError::EquationRhsCountMismatch`] при неверной форме.
    /// При этих ошибках состояние не изменяется.
    pub fn insert_equation(
        &mut self,
        coefficients: &[F],
        rhs: &[F],
    ) -> Result<EquationInsertion, LinalgError> {
        if coefficients.len() != self.unknowns {
            return Err(LinalgError::EquationCoefficientCountMismatch {
                expected: self.unknowns,
                actual: coefficients.len(),
            });
        }
        if rhs.len() != self.rhs_columns {
            return Err(LinalgError::EquationRhsCountMismatch {
                expected: self.rhs_columns,
                actual: rhs.len(),
            });
        }
        let mut coefficients = coefficients.to_vec();
        let mut rhs = rhs.to_vec();
        for row in &self.rows {
            let factor = coefficients[row.pivot];
            if factor.is_zero() {
                continue;
            }
            for (value, &basis) in coefficients[row.pivot..]
                .iter_mut()
                .zip(&row.coefficients[row.pivot..])
            {
                *value = *value - factor * basis;
            }
            for (value, &basis) in rhs.iter_mut().zip(&row.rhs) {
                *value = *value - factor * basis;
            }
        }
        let Some(pivot) = coefficients.iter().position(|value| !value.is_zero()) else {
            return Ok(match rhs.iter().position(|value| !value.is_zero()) {
                Some(rhs_column) => EquationInsertion::Inconsistent { rhs_column },
                None => EquationInsertion::Dependent,
            });
        };
        let inverse = coefficients[pivot].inv();
        for value in &mut coefficients[pivot..] {
            *value = *value * inverse;
        }
        for value in &mut rhs {
            *value = *value * inverse;
        }
        let position = self.rows.partition_point(|row| row.pivot < pivot);
        self.rows.insert(
            position,
            Row {
                pivot,
                coefficients,
                rhs,
            },
        );
        Ok(EquationInsertion::Independent {
            pivot_column: pivot,
            rank: self.rank(),
        })
    }

    /// Возвращает единственное решение размера `unknowns × rhs_columns`.
    ///
    /// При недостаточном ранге возвращает `Ok(None)`. При полном ранге
    /// выполняет обратный ход, сохраняя порядок неизвестных. Базис не меняется;
    /// ранее отклонённые противоречия не препятствуют решению сохранённой системы.
    ///
    /// # Ошибки
    ///
    /// Ошибки размера передаются от [`Matrix::try_new`]; для корректно созданной
    /// системы размеры решения проверены конструктором заранее.
    pub fn try_solution(&self) -> Result<Option<Matrix<F>>, LinalgError> {
        if self.rank() < self.unknowns {
            return Ok(None);
        }
        let mut data = vec![F::zero(); self.unknowns * self.rhs_columns];
        for row in self.rows.iter().rev() {
            let offset = row.pivot * self.rhs_columns;
            data[offset..offset + self.rhs_columns].copy_from_slice(&row.rhs);
            for column in row.pivot + 1..self.unknowns {
                let factor = row.coefficients[column];
                if factor.is_zero() {
                    continue;
                }
                for rhs_column in 0..self.rhs_columns {
                    let known = data[column * self.rhs_columns + rhs_column];
                    data[offset + rhs_column] = data[offset + rhs_column] - factor * known;
                }
            }
        }
        Matrix::try_new(self.unknowns, self.rhs_columns, data).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::check_buffer;
    use crate::LinalgError;

    #[test]
    fn buffer_addressability_is_checked_without_allocating() {
        assert_eq!(
            check_buffer::<u16>(usize::MAX),
            Err(LinalgError::SizeOverflow)
        );
        assert_eq!(
            check_buffer::<u8>(isize::MAX as usize + 1),
            Err(LinalgError::SizeOverflow)
        );
        assert_eq!(check_buffer::<u8>(isize::MAX as usize), Ok(()));
    }
}
