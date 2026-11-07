//! Двоичный полярный код заданной длины и множества информационных позиций.

use crate::length::check_length;
use crate::transform::transform_in_place;
use crate::{Bit, PolarError};

/// Двоичный полярный код длины `N = 2^m`.
///
/// Замороженные биты источника равны нулю. Бит `message[j]` записывается в
/// [`information_positions`](Self::information_positions)`[j]`. Позиции хранятся
/// в порядке возрастания, даже если при построении их передали в другом порядке.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolarCode {
    information: Vec<usize>,
    frozen: Vec<bool>,
}

impl PolarCode {
    /// Строит код по длине блока и индексам информационных битов.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если длина недопустима, индекс выходит за границы
    /// или информационная позиция повторяется.
    pub fn try_new(length: usize, information: &[usize]) -> Result<Self, PolarError> {
        check_length(length)?;

        let mut seen = vec![false; length];
        for &index in information {
            if index >= length {
                return Err(PolarError::IndexOutOfBounds { index, length });
            }
            if seen[index] {
                return Err(PolarError::DuplicateInformationIndex { index });
            }
            seen[index] = true;
        }

        let mut information = information.to_vec();
        information.sort_unstable();
        let frozen = seen.iter().map(|is_information| !is_information).collect();

        Ok(Self {
            information,
            frozen,
        })
    }

    /// Возвращает длину кодового слова `N`.
    #[must_use]
    pub fn length(&self) -> usize {
        self.frozen.len()
    }

    /// Возвращает число информационных битов `K`.
    #[must_use]
    pub fn message_len(&self) -> usize {
        self.information.len()
    }

    /// Возвращает информационные позиции в порядке возрастания.
    #[must_use]
    pub fn information_positions(&self) -> &[usize] {
        &self.information
    }

    /// Возвращает замороженные позиции в порядке возрастания.
    #[must_use]
    pub fn frozen_positions(&self) -> Vec<usize> {
        self.frozen
            .iter()
            .enumerate()
            .filter_map(|(index, frozen)| frozen.then_some(index))
            .collect()
    }

    /// Кодирует сообщение длины `K` в слово длины `N`.
    ///
    /// Замороженные биты источника равны нулю, затем применяется [`polar_transform`](crate::polar_transform).
    ///
    /// # Errors
    ///
    /// Возвращает [`PolarError::MessageLengthMismatch`], если длина сообщения
    /// не равна [`message_len`](Self::message_len).
    pub fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, PolarError> {
        if message.len() != self.information.len() {
            return Err(PolarError::MessageLengthMismatch {
                expected: self.information.len(),
                actual: message.len(),
            });
        }

        let mut source = vec![Bit::Zero; self.frozen.len()];
        for (&index, &bit) in self.information.iter().zip(message) {
            source[index] = bit;
        }
        transform_in_place(&mut source);
        Ok(source)
    }

    /// Извлекает сообщение из кодового слова.
    ///
    /// Слово проходит обратное полярное преобразование. Результат принимается
    /// только если на всех замороженных позициях стоит ноль.
    ///
    /// # Errors
    ///
    /// Возвращает [`PolarError::WordLengthMismatch`], если длина слова неверна,
    /// или [`PolarError::InvalidCodeword`], если заморозка нарушена.
    pub fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, PolarError> {
        if word.len() != self.frozen.len() {
            return Err(PolarError::WordLengthMismatch {
                expected: self.frozen.len(),
                actual: word.len(),
            });
        }

        let mut source = word.to_vec();
        transform_in_place(&mut source);
        let frozen_ones = self
            .frozen
            .iter()
            .zip(&source)
            .filter(|&(frozen, bit)| *frozen && *bit == Bit::One)
            .count();
        if frozen_ones > 0 {
            return Err(PolarError::InvalidCodeword { frozen_ones });
        }

        Ok(self
            .information
            .iter()
            .map(|&index| source[index])
            .collect())
    }
}
