use coding_core::{Codeword, Observer};
use gf_linalg::Matrix;

use crate::bit::count_ones;
use crate::size::check_buffer_bytes;
use crate::{bits_to_vector, vector_to_bits, Bit, Gf2, LdpcError, ParityCheckMatrix};

fn dense_from_checks(checks: &ParityCheckMatrix) -> Result<Matrix<Gf2>, LdpcError> {
    let rows = checks.rows();
    let cols = checks.cols();

    let element_count = rows.checked_mul(cols).ok_or(LdpcError::SizeOverflow)?;
    check_buffer_bytes::<Gf2>(element_count)?;

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

/// Систематический кодер, подготовленный по проверочной матрице `H`.
///
/// Кодер выбирает непивотные столбцы как информационные, а опорные столбцы —
/// как проверочные. Позиции возвращаются в порядке столбцов исходной `H`, и
/// `message[j]` помещается в `word[information_positions()[j]]`.
/// Создать значение можно только через [`try_new`](Self::try_new), который
/// отвергает нулевой и полный ранг.
#[derive(Debug)]
pub struct SystematicEncoder {
    checks: ParityCheckMatrix,
    parity_coefficients: Matrix<Gf2>,
    systematic_to_original: Vec<usize>,
}

impl SystematicEncoder {
    /// Подготавливает систематический кодер по проверочной матрице.
    ///
    /// Забирает `checks` по значению и хранит исходную матрицу внутри кодера;
    /// дополнительных настраиваемых параметров нет.
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
        check_buffer_bytes::<usize>(information_len)?;
        check_buffer_bytes::<usize>(cols)?;
        let parity_element_count = rank
            .checked_mul(information_len)
            .ok_or(LdpcError::SizeOverflow)?;
        check_buffer_bytes::<Gf2>(parity_element_count)?;

        let mut systematic_to_original = Vec::with_capacity(cols);
        let pivot_columns = reduced.pivot_columns();
        for col in 0..cols {
            if pivot_columns.binary_search(&col).is_err() {
                systematic_to_original.push(col);
            }
        }
        systematic_to_original.extend_from_slice(pivot_columns);

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
        })
    }

    /// Возвращает число информационных битов `k`.
    #[must_use]
    pub fn message_len(&self) -> usize {
        self.parity_coefficients.cols()
    }

    /// Возвращает число битов кодового слова `n`.
    #[must_use]
    pub fn codeword_len(&self) -> usize {
        self.checks.cols()
    }

    /// Извлекает сообщение из допустимого кодового слова длины `n`.
    ///
    /// Сначала проверяется длина слова, затем его синдром по исходной
    /// проверочной матрице. Метод не исправляет ошибки и не определяет, какое
    /// допустимое слово было передано. Любое другое слово с нулевым синдромом
    /// также принимается.
    ///
    /// # Errors
    ///
    /// Возвращает [`LdpcError::WordLengthMismatch`], если длина слова неверна,
    /// или [`LdpcError::InvalidCodeword`], если слово нарушает исходные
    /// проверки.
    pub fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, LdpcError> {
        let expected = self.codeword_len();
        if word.len() != expected {
            return Err(LdpcError::WordLengthMismatch {
                expected,
                actual: word.len(),
            });
        }

        let unsatisfied_checks = count_ones(&self.checks.syndrome(word)?);
        if unsatisfied_checks != 0 {
            return Err(LdpcError::InvalidCodeword { unsatisfied_checks });
        }

        Ok(self.systematic_to_original[..self.message_len()]
            .iter()
            .map(|&position| word[position])
            .collect())
    }

    /// Возвращает ранг исходной проверочной матрицы.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.parity_coefficients.rows()
    }

    pub(crate) fn checks(&self) -> &ParityCheckMatrix {
        &self.checks
    }

    /// Возвращает исходные позиции информационных столбцов по возрастанию.
    /// Бит `message[j]` при кодировании помещается в
    /// `word[information_positions()[j]]`.
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

/// Кодирует сообщение длины `k` в слово длины `n`.
///
/// Информационные биты сохраняются в позициях, заданных кодером; итоговое
/// слово возвращается в порядке столбцов исходной проверочной матрицы. Бит
/// `message[j]` находится в
/// `word[SystematicEncoder::information_positions()[j]]`. Общий кодер не
/// отправляет событий, поэтому переданный наблюдатель не вызывается.
///
/// # Errors
///
/// Возвращает [`LdpcError::MessageLengthMismatch`], если длина сообщения не
/// равна [`SystematicEncoder::message_len`], или ошибку линейной
/// алгебры/преобразования, если соответствующая операция завершилась ошибкой.
impl<'a> coding_core::Encoder<&'a [Bit]> for SystematicEncoder {
    type Output = Codeword<Vec<Bit>>;
    type Error = LdpcError;
    type Event = ();

    fn encode(
        &mut self,
        message: &'a [Bit],
        _observer: &mut dyn Observer<()>,
    ) -> Result<Self::Output, Self::Error> {
        let expected = self.message_len();
        if message.len() != expected {
            return Err(LdpcError::MessageLengthMismatch {
                expected,
                actual: message.len(),
            });
        }

        let information = bits_to_vector(message);
        let parity = self.parity_coefficients.try_mul_vector(&information)?;
        let parity_bits = vector_to_bits(&parity)?;
        let codeword_len = self.codeword_len();
        check_buffer_bytes::<Bit>(codeword_len)?;

        let mut word = vec![Bit::Zero; codeword_len];
        for (&bit, &position) in message.iter().zip(self.information_positions()) {
            word[position] = bit;
        }
        for (&bit, &position) in parity_bits.iter().zip(self.parity_positions()) {
            word[position] = bit;
        }

        Ok(Codeword(word))
    }
}

#[cfg(test)]
#[path = "../tests/unit/encoder.rs"]
mod tests;
