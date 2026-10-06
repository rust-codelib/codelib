use core::mem::size_of;

use gf_linalg::{Matrix, Vector, MAX_MATRIX_DIM};

use crate::{bits_to_vector, vector_to_bits, Bit, Gf2, LdpcError, ParityCheckMatrix};

/// Общий контракт систематического кодера двоичного блока.
///
/// Длина сообщения и слова определяется конкретным кодером. Методы читают
/// входные срезы и не изменяют ни их, ни состояние кодера.
pub trait Encoder {
    /// Возвращает число информационных битов `k`.
    fn message_len(&self) -> usize;

    /// Возвращает число битов кодового слова `n`.
    fn codeword_len(&self) -> usize;

    /// Кодирует сообщение длины `k` в слово длины `n`.
    ///
    /// Информационные биты сохраняются в позициях, заданных кодером; итоговое
    /// слово возвращается в порядке столбцов исходной проверочной матрицы.
    ///
    /// # Errors
    ///
    /// Возвращает [`LdpcError::MessageLengthMismatch`], если длина сообщения
    /// не равна [`message_len`](Self::message_len), или ошибку линейной
    /// алгебры/преобразования, если соответствующая операция завершилась
    /// ошибкой.
    fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, LdpcError>;

    /// Извлекает сообщение из слова допустимого кодового слова длины `n`.
    ///
    /// Проверяется синдром по исходной проверочной матрице. Метод не исправляет
    /// ошибки и не определяет, какое слово было передано.
    ///
    /// # Errors
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`], если длина слова неверна,
    /// или [`LdpcError::InvalidCodeword`], если слово нарушает исходные
    /// проверки.
    fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, LdpcError>;
}

fn dense_from_checks(checks: &ParityCheckMatrix) -> Result<Matrix<Gf2>, LdpcError> {
    let rows = checks.rows();
    let cols = checks.cols();
    if !(1..=MAX_MATRIX_DIM).contains(&rows) || !(1..=MAX_MATRIX_DIM).contains(&cols) {
        return Err(LdpcError::InvalidDimensions { rows, cols });
    }

    let element_count = rows.checked_mul(cols).ok_or(LdpcError::SizeOverflow)?;
    element_count
        .checked_mul(size_of::<Gf2>())
        .ok_or(LdpcError::SizeOverflow)?;

    let mut data = vec![Gf2::zero(); element_count];
    for row in 0..rows {
        for &bit in checks
            .check_bits(row)
            .expect("row index is within checked matrix dimensions")
        {
            data[row * cols + bit] = Gf2::one();
        }
    }

    Ok(Matrix::try_new(rows, cols, data)?)
}

fn check_buffer_size<T>(len: usize) -> Result<(), LdpcError> {
    len.checked_mul(size_of::<T>())
        .ok_or(LdpcError::SizeOverflow)?;
    Ok(())
}

/// Систематический кодер, подготовленный по проверочной матрице `H`.
///
/// Кодер выбирает непивотные столбцы как информационные, а опорные столбцы —
/// как проверочные. Позиции возвращаются в порядке столбцов исходной `H`.
/// Создать значение можно только через [`try_new`](Self::try_new), который
/// отвергает нулевой и полный ранг.
#[derive(Debug)]
pub struct SystematicEncoder {
    checks: ParityCheckMatrix,
    parity_coefficients: Matrix<Gf2>,
    systematic_to_original: Vec<usize>,
    original_to_systematic: Vec<usize>,
}

impl SystematicEncoder {
    /// Подготавливает систематический кодер по проверочной матрице.
    ///
    /// Вычисляется один RREF матрицы `H`. Непивотные столбцы в возрастающем
    /// порядке становятся информационными, пивотные — проверочными. Строки,
    /// зависимые от предыдущих, учитываются через ранг, а не как отдельные
    /// проверочные биты.
    ///
    /// # Errors
    ///
    /// Возвращает [`LdpcError::InvalidEncoderRank`], если ранг равен нулю или
    /// числу столбцов. Ошибки плотной линейной алгебры и переполнения размеров
    /// передаются как [`LdpcError::LinearAlgebra`] и [`LdpcError::SizeOverflow`].
    pub fn try_new(checks: ParityCheckMatrix) -> Result<Self, LdpcError> {
        let dense = dense_from_checks(&checks)?;
        let reduced = dense.rref();
        let rank = reduced.rank();
        let cols = checks.cols();

        if rank == 0 || rank == cols {
            return Err(LdpcError::InvalidEncoderRank { rank, cols });
        }

        let information_len = cols.checked_sub(rank).ok_or(LdpcError::SizeOverflow)?;
        check_buffer_size::<usize>(information_len)?;
        check_buffer_size::<usize>(cols)?;
        let parity_element_count = rank
            .checked_mul(information_len)
            .ok_or(LdpcError::SizeOverflow)?;
        check_buffer_size::<Gf2>(parity_element_count)?;

        let mut systematic_to_original = Vec::with_capacity(cols);
        let pivot_columns = reduced.pivot_columns();
        for col in 0..cols {
            if pivot_columns.binary_search(&col).is_err() {
                systematic_to_original.push(col);
            }
        }
        systematic_to_original.extend_from_slice(pivot_columns);

        let mut original_to_systematic = vec![0; cols];
        for (systematic, &original) in systematic_to_original.iter().enumerate() {
            original_to_systematic[original] = systematic;
        }

        let mut parity_data = Vec::with_capacity(parity_element_count);
        for row in 0..rank {
            let reduced_row = reduced
                .matrix()
                .row(row)
                .expect("every pivot row is within the reduced matrix");
            for &information_col in &systematic_to_original[..information_len] {
                parity_data.push(reduced_row[information_col]);
            }
        }
        let parity_coefficients = Matrix::try_new(rank, information_len, parity_data)?;

        Ok(Self {
            checks,
            parity_coefficients,
            systematic_to_original,
            original_to_systematic,
        })
    }

    /// Возвращает ранг исходной проверочной матрицы.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.parity_coefficients.rows()
    }

    /// Возвращает исходные позиции информационных столбцов по возрастанию.
    #[must_use]
    pub fn information_positions(&self) -> &[usize] {
        &self.systematic_to_original[..self.parity_coefficients.cols()]
    }

    /// Возвращает исходные позиции проверочных (опорных) столбцов.
    #[must_use]
    pub fn parity_positions(&self) -> &[usize] {
        &self.systematic_to_original[self.parity_coefficients.cols()..]
    }
}

impl Encoder for SystematicEncoder {
    fn message_len(&self) -> usize {
        self.parity_coefficients.cols()
    }

    fn codeword_len(&self) -> usize {
        self.checks.cols()
    }

    fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, LdpcError> {
        let expected = self.message_len();
        if message.len() != expected {
            return Err(LdpcError::MessageLengthMismatch {
                expected,
                actual: message.len(),
            });
        }

        let information = bits_to_vector(message);
        let parity = self.parity_coefficients.try_mul_vector(&information)?;
        let systematic_len = information
            .len()
            .checked_add(parity.len())
            .ok_or(LdpcError::SizeOverflow)?;
        check_buffer_size::<Gf2>(systematic_len)?;

        let mut systematic_data = Vec::with_capacity(systematic_len);
        systematic_data.extend_from_slice(information.as_slice());
        systematic_data.extend_from_slice(parity.as_slice());
        let systematic_word = vector_to_bits(&Vector::<Gf2>::new(systematic_data))?;

        let mut word = vec![Bit::Zero; self.codeword_len()];
        for (original, &systematic) in self.original_to_systematic.iter().enumerate() {
            word[original] = systematic_word[systematic];
        }

        Ok(word)
    }

    fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, LdpcError> {
        let expected = self.codeword_len();
        if word.len() != expected {
            return Err(LdpcError::WordLengthMismatch {
                expected,
                actual: word.len(),
            });
        }

        let unsatisfied_checks = self
            .checks
            .syndrome(word)?
            .iter()
            .filter(|&&bit| bit == Bit::One)
            .count();
        if unsatisfied_checks != 0 {
            return Err(LdpcError::InvalidCodeword { unsatisfied_checks });
        }

        Ok(self.systematic_to_original[..self.message_len()]
            .iter()
            .map(|&position| word[position])
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use gf_linalg::Matrix;

    use crate::{encoder::dense_from_checks, Gf2, LdpcError, ParityCheckMatrix, SystematicEncoder};

    #[test]
    fn dense_conversion_preserves_shape_entries_and_sparse_source() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![2, 0], vec![], vec![0, 2], vec![3]])
                .unwrap();

        let dense = dense_from_checks(&checks).unwrap();

        assert_eq!(dense.rows(), 4);
        assert_eq!(dense.cols(), 4);
        assert_eq!(
            dense.as_slice(),
            &[
                Gf2::one(),
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::one(),
            ]
        );

        assert_eq!(checks.check_bits(0), Some(&[0, 2][..]));
        assert_eq!(checks.check_bits(1), Some(&[][..]));
        assert_eq!(checks.check_bits(2), Some(&[0, 2][..]));
        assert_eq!(checks.bit_checks(0), Some(&[0, 2][..]));
        assert_eq!(checks.bit_checks(1), Some(&[][..]));
        assert_eq!(checks.bit_checks(2), Some(&[0, 2][..]));
        assert_eq!(checks.bit_checks(3), Some(&[3][..]));
    }

    #[test]
    fn dense_conversion_keeps_rectangular_dimensions_and_isolated_columns() {
        let checks = ParityCheckMatrix::try_from_rows(5, vec![vec![4, 1], vec![2]]).unwrap();

        let dense = dense_from_checks(&checks).unwrap();

        assert_eq!(
            dense,
            Matrix::try_new(
                2,
                5,
                vec![
                    Gf2::zero(),
                    Gf2::one(),
                    Gf2::zero(),
                    Gf2::zero(),
                    Gf2::one(),
                    Gf2::zero(),
                    Gf2::zero(),
                    Gf2::one(),
                    Gf2::zero(),
                    Gf2::zero(),
                ]
            )
            .unwrap()
        );
        assert_eq!(checks.bit_checks(0), Some(&[][..]));
        assert_eq!(checks.bit_checks(3), Some(&[][..]));
    }

    #[test]
    fn preparation_builds_exact_coefficients_and_inverse_permutations_for_main_example() {
        let checks = ParityCheckMatrix::try_from_rows(
            6,
            vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
        )
        .unwrap();

        let encoder = SystematicEncoder::try_new(checks).unwrap();

        assert_eq!(
            encoder.parity_coefficients,
            Matrix::try_new(3, 3, [0, 1, 1, 1, 1, 1, 1, 0, 1].map(Gf2::new).to_vec()).unwrap()
        );
        assert_eq!(encoder.systematic_to_original, [3, 4, 5, 0, 1, 2]);
        assert_eq!(encoder.original_to_systematic, [3, 4, 5, 0, 1, 2]);
        assert_permutations_are_inverse(&encoder);
    }

    #[test]
    fn preparation_builds_exact_coefficients_and_distinct_inverse_permutations_for_cycle_three() {
        let checks =
            ParityCheckMatrix::try_from_rows(5, vec![vec![0, 1, 4], vec![2, 3, 4]]).unwrap();

        let encoder = SystematicEncoder::try_new(checks).unwrap();

        assert_eq!(
            encoder.parity_coefficients,
            Matrix::try_new(2, 3, [1, 0, 1, 0, 1, 1].map(Gf2::new).to_vec()).unwrap()
        );
        assert_eq!(encoder.systematic_to_original, [1, 3, 4, 0, 2]);
        assert_eq!(encoder.original_to_systematic, [3, 0, 4, 1, 2]);
        assert_permutations_are_inverse(&encoder);
    }

    #[test]
    fn preparation_distinguishes_swap_from_identity_permutation() {
        let swap = SystematicEncoder::try_new(
            ParityCheckMatrix::try_from_rows(3, vec![vec![1, 2]]).unwrap(),
        )
        .unwrap();
        assert_eq!(swap.systematic_to_original, [0, 2, 1]);
        assert_eq!(swap.original_to_systematic, [0, 2, 1]);

        let identity =
            SystematicEncoder::try_new(ParityCheckMatrix::try_from_rows(3, vec![vec![2]]).unwrap())
                .unwrap();
        assert_eq!(identity.systematic_to_original, [0, 1, 2]);
        assert_eq!(identity.original_to_systematic, [0, 1, 2]);
    }

    fn assert_permutations_are_inverse(encoder: &SystematicEncoder) {
        for (systematic, &original) in encoder.systematic_to_original.iter().enumerate() {
            assert_eq!(encoder.original_to_systematic[original], systematic);
        }
        for (original, &systematic) in encoder.original_to_systematic.iter().enumerate() {
            assert_eq!(encoder.systematic_to_original[systematic], original);
        }
    }

    #[test]
    fn buffer_size_helper_detects_byte_count_overflow() {
        assert_eq!(
            super::check_buffer_size::<u64>(usize::MAX),
            Err(LdpcError::SizeOverflow)
        );
    }
}
