//! Проверочная матрица двоичного LDPC-кода в разреженном виде.

use gf_linalg::MAX_MATRIX_DIM;

use crate::bit::bit_parity;
use crate::size::check_buffer_bytes;
use crate::{Bit, LdpcError};

/// Проверочная матрица `H`, хранящая только позиции единиц по строкам.
///
/// Каждая строка содержит отсортированные индексы битов. Пустые и одинаковые
/// строки допустимы; столбцы без единиц также сохраняются в размере матрицы.
/// Хранилище обеих смежностей занимает `O(m + n + E)`, без выделения плотной
/// матрицы `rows × cols`, где `m` — число проверок, `n` — число битов, а `E` —
/// число единиц.
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
#[derive(Debug, Clone)]
pub struct ParityCheckMatrix {
    check_bits: Vec<Vec<usize>>,
    bit_checks: Vec<Vec<usize>>,
    bits: usize,
    edge_count: usize,
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

        // Проверяем размеры выделяемых буферов до построения обратных списков.
        check_buffer_bytes::<Vec<usize>>(bits)?;
        check_buffer_bytes::<usize>(edge_count)?;
        let mut bit_checks = Vec::with_capacity(bits);
        bit_checks.resize_with(bits, Vec::new);
        for (check, row) in rows.iter().enumerate() {
            for &bit in row {
                bit_checks[bit].push(check);
            }
        }

        Ok(Self {
            check_bits: rows,
            bit_checks,
            bits,
            edge_count,
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

    /// Возвращает общее число единиц матрицы.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    /// Возвращает отсортированные индексы битов заданной проверки.
    ///
    /// Для существующей проверки без единиц возвращается `Some(&[])`, а для
    /// индекса за границей — `None`.
    #[must_use]
    pub fn check_bits(&self, check: usize) -> Option<&[usize]> {
        self.check_bits.get(check).map(Vec::as_slice)
    }

    /// Возвращает отсортированные индексы проверок, содержащих заданный бит.
    ///
    /// Для существующего бита без единиц возвращается `Some(&[])`, а для
    /// индекса за границей — `None`.
    #[must_use]
    pub fn bit_checks(&self, bit: usize) -> Option<&[usize]> {
        self.bit_checks.get(bit).map(Vec::as_slice)
    }

    /// Вычисляет синдром слова в порядке проверок исходной матрицы.
    ///
    /// Длина `word` должна совпадать с числом столбцов. Каждый бит результата
    /// равен XOR битов, входящих в соответствующую строку `H`; пустая строка
    /// даёт [`Bit::Zero`]. Матрица и входное слово не изменяются.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`], если длина слова не равна
    /// [`cols`](Self::cols).
    pub fn syndrome(&self, word: &[Bit]) -> Result<Vec<Bit>, LdpcError> {
        if word.len() != self.bits {
            return Err(LdpcError::WordLengthMismatch {
                expected: self.bits,
                actual: word.len(),
            });
        }

        Ok(self
            .check_bits
            .iter()
            .map(|bits| bit_parity(bits.iter().map(|&bit| word[bit])))
            .collect())
    }

    /// Проверяет, удовлетворяет ли слово всем строкам проверочной матрицы.
    ///
    /// Возвращает `true`, если синдром нулевой. Длина `word` должна совпадать
    /// с числом столбцов; матрица и входное слово не изменяются.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`], если длина слова не равна
    /// [`cols`](Self::cols).
    pub fn is_codeword(&self, word: &[Bit]) -> Result<bool, LdpcError> {
        Ok(self.syndrome(word)?.iter().all(|&bit| bit == Bit::Zero))
    }
}
