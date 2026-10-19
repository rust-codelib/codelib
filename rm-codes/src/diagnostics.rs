//! Диагностика различий между двоичными последовательностями.

use crate::{Bit, RmError};

/// Считает число различающихся позиций в слове и эталоне.
///
/// Функция не зависит от параметров кода и подходит для сравнения как
/// кодовых слов, так и сообщений. Пустые последовательности дают `0`.
///
/// # Ошибки
///
/// Возвращает [`RmError::ReferenceLengthMismatch`], если длины различаются.
pub fn count_bit_errors(word: &[Bit], reference: &[Bit]) -> Result<usize, RmError> {
    if word.len() != reference.len() {
        return Err(RmError::ReferenceLengthMismatch {
            expected: word.len(),
            actual: reference.len(),
        });
    }

    Ok(word
        .iter()
        .zip(reference)
        .filter(|(left, right)| left != right)
        .count())
}

#[cfg(test)]
mod tests {
    use super::count_bit_errors;
    use crate::{Bit, RmError};

    fn bits(text: &str) -> Vec<Bit> {
        text.bytes()
            .map(|byte| Bit::try_from(byte - b'0').unwrap())
            .collect()
    }

    #[test]
    fn counts_differences() {
        assert_eq!(count_bit_errors(&bits("1010"), &bits("1001")).unwrap(), 2);
        assert_eq!(count_bit_errors(&[], &[]).unwrap(), 0);
    }

    #[test]
    fn rejects_length_mismatch() {
        assert_eq!(
            count_bit_errors(&bits("10"), &bits("1")),
            Err(RmError::ReferenceLengthMismatch {
                expected: 2,
                actual: 1
            })
        );
    }
}
