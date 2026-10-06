//! Проверочная матрица двоичного LDPC-кода в разреженном виде.

use gf_linalg::MAX_MATRIX_DIM;

use crate::LdpcError;

/// Проверочная матрица `H`, хранящая только позиции единиц по строкам.
///
/// Каждая строка содержит отсортированные индексы битов. Пустые и одинаковые
/// строки допустимы; столбцы без единиц также сохраняются в размере матрицы.
/// Внутреннее хранилище занимает место пропорционально числу строк и единиц,
/// без выделения плотной матрицы `rows × cols`.
///
/// ```
/// use ldpc_codes::ParityCheckMatrix;
///
/// let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![2, 0], vec![1, 2]])?;
/// assert_eq!(checks.rows(), 2);
/// assert_eq!(checks.cols(), 3);
/// assert_eq!(checks.check_bits(0), Some(&[0, 2][..]));
/// # Ok::<(), ldpc_codes::LdpcError>(())
/// ```
#[derive(Debug)]
pub struct ParityCheckMatrix {
    check_bits: Vec<Vec<usize>>,
    bits: usize,
}

impl ParityCheckMatrix {
    /// Создаёт разреженную матрицу из списков единиц каждой проверки.
    ///
    /// `bits` задаёт число столбцов, а число строк определяется длиной
    /// `rows`. Каждая ось должна быть в диапазоне `1..=MAX_MATRIX_DIM`.
    /// Индексы битов должны быть меньше `bits` и не повторяться внутри одной
    /// строки. Перед хранением индексы каждой строки сортируются по возрастанию.
    /// Пустые и одинаковые строки допустимы.
    ///
    /// Проверка формы выполняется раньше проверки индексов. Строки
    /// проверяются по входному порядку, а в каждой строке сначала проверяются
    /// все индексы в исходном порядке и только потом повторы. Ошибка возвращается
    /// без создания частично валидной матрицы.
    pub fn try_from_rows(bits: usize, mut rows: Vec<Vec<usize>>) -> Result<Self, LdpcError> {
        let row_count = rows.len();
        if !(1..=MAX_MATRIX_DIM).contains(&row_count) || !(1..=MAX_MATRIX_DIM).contains(&bits) {
            return Err(LdpcError::InvalidDimensions {
                rows: row_count,
                cols: bits,
            });
        }

        let mut edge_count = 0usize;
        for (check, row) in rows.iter_mut().enumerate() {
            if let Some(&bit) = row.iter().find(|&&bit| bit >= bits) {
                return Err(LdpcError::BitIndexOutOfBounds { check, bit, bits });
            }

            row.sort_unstable();
            if let Some(pair) = row.windows(2).find(|pair| pair[0] == pair[1]) {
                return Err(LdpcError::DuplicateBitIndex {
                    check,
                    bit: pair[0],
                });
            }

            edge_count = edge_count
                .checked_add(row.len())
                .ok_or(LdpcError::SizeOverflow)?;
        }

        Ok(Self {
            check_bits: rows,
            bits,
        })
    }

    /// Возвращает число проверок (строк).
    #[must_use]
    pub fn rows(&self) -> usize {
        self.check_bits.len()
    }

    /// Возвращает число битов (столбцов).
    #[must_use]
    pub fn cols(&self) -> usize {
        self.bits
    }

    /// Возвращает отсортированные индексы битов заданной проверки.
    ///
    /// Для существующей проверки без единиц возвращается `Some(&[])`, а для
    /// индекса за границей — `None`.
    #[must_use]
    pub fn check_bits(&self, check: usize) -> Option<&[usize]> {
        self.check_bits.get(check).map(Vec::as_slice)
    }
}
