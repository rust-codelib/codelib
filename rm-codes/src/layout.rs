//! Внутренняя раскладка коэффициентов кода Рида–Маллера.

use crate::{Bit, RmConfig};

/// Единая раскладка коэффициентов и позиций для `RM(r, m)`.
///
/// Маски хранятся в порядке сообщения: сначала свободный член, затем
/// коэффициенты степени 1, затем степени 2 и так далее. Внутри одной
/// степени порядок лексикографический по номерам переменных `1..=m`.
#[derive(Clone, Debug)]
pub(crate) struct RmLayout {
    n: usize,
    full_mask: u32,
    masks: Vec<u32>,
}

impl RmLayout {
    /// Строит раскладку по проверенным параметрам.
    pub(crate) fn new(config: RmConfig) -> Self {
        let m = config.m();
        let r = config.r();
        let n = config.length();
        let full_mask = (1u32 << m) - 1;

        let mut entries: Vec<(u32, Vec<u8>)> = (0..=full_mask)
            .filter(|mask| mask.count_ones() as usize <= r)
            .map(|mask| {
                let indices = (1..=m)
                    .filter(|&i| mask & (1u32 << (m - i)) != 0)
                    .map(|i| i as u8)
                    .collect();
                (mask, indices)
            })
            .collect();

        entries.sort_by(|(_, left), (_, right)| {
            left.len().cmp(&right.len()).then_with(|| left.cmp(right))
        });

        let masks = entries.into_iter().map(|(mask, _)| mask).collect();

        Self {
            n,
            full_mask,
            masks,
        }
    }

    /// Длина кодового слова.
    pub(crate) fn n(&self) -> usize {
        self.n
    }

    /// Размерность кода.
    pub(crate) fn k(&self) -> usize {
        self.masks.len()
    }

    /// Маска коэффициента с указанным индексом.
    pub(crate) fn mask(&self, index: usize) -> u32 {
        self.masks[index]
    }

    /// Степень коэффициента с указанным индексом.
    pub(crate) fn degree(&self, index: usize) -> usize {
        self.masks[index].count_ones() as usize
    }

    /// Все подмаски маски `mask`, включая саму `mask` и ноль.
    ///
    /// Порядок убывания: `mask`, затем лексикографически меньшие подмаски.
    pub(crate) fn submasks(mask: u32) -> Submasks {
        Submasks {
            mask,
            current: Some(mask),
        }
    }

    /// Базы смежных классов `span(mask)`: все наборы битов вне `mask`.
    pub(crate) fn coset_bases(&self, mask: u32) -> Submasks {
        Self::submasks(self.full_mask ^ mask)
    }

    /// Проверяет, что позиция `point` содержит все биты `mask`.
    pub(crate) fn contains(point: u32, mask: u32) -> bool {
        point & mask == mask
    }

    /// Кодирует сообщение без проверки длины.
    pub(crate) fn encode(&self, message: &[Bit]) -> Vec<Bit> {
        debug_assert_eq!(message.len(), self.k());
        let mut word = vec![Bit::Zero; self.n];

        for (index, &bit) in message.iter().enumerate() {
            if bit.is_zero() {
                continue;
            }

            let mask = self.masks[index];
            for (position, output) in word.iter_mut().enumerate() {
                if Self::contains(position as u32, mask) {
                    *output ^= Bit::One;
                }
            }
        }

        word
    }

    /// Вычисляет коэффициенты АНФ по полному слову.
    ///
    /// Для корректного кодового слова это обратное преобразование Мёбиуса.
    pub(crate) fn extract_coefficients(&self, word: &[Bit]) -> Vec<Bit> {
        debug_assert_eq!(word.len(), self.n);
        let mut coefficients = Vec::with_capacity(self.k());

        for &mask in &self.masks {
            let mut parity = Bit::Zero;
            for submask in Self::submasks(mask) {
                parity ^= word[submask as usize];
            }
            coefficients.push(parity);
        }

        coefficients
    }

    /// Строки генераторной матрицы `G` размера `k × n`.
    pub(crate) fn generator_rows(&self) -> Vec<Vec<Bit>> {
        self.masks
            .iter()
            .map(|&mask| {
                (0..self.n)
                    .map(|position| Bit::from_bool(Self::contains(position as u32, mask)))
                    .collect()
            })
            .collect()
    }
}

/// Итератор по всем подмаскам фиксированной маски.
#[derive(Clone, Debug)]
pub(crate) struct Submasks {
    mask: u32,
    current: Option<u32>,
}

impl Iterator for Submasks {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.current?;
        self.current = if value == 0 {
            None
        } else {
            Some((value - 1) & self.mask)
        };
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{RmLayout, Submasks};
    use crate::{Bit, RmConfig};

    #[test]
    fn coefficient_order_matches_examples() {
        let layout = RmLayout::new(RmConfig::new(2, 4).unwrap());

        assert_eq!(
            layout.masks.as_slice(),
            &[0, 8, 4, 2, 1, 12, 10, 9, 6, 5, 3]
        );
        assert_eq!(layout.degree(0), 0);
        assert_eq!(layout.degree(4), 1);
        assert_eq!(layout.degree(10), 2);
    }

    #[test]
    fn layout_dimension_matches_config() {
        for m in 1..=8 {
            for r in 0..=m {
                let config = RmConfig::new(r, m).unwrap();
                let layout = RmLayout::new(config);
                assert_eq!(layout.k(), config.dimension(), "r = {r}, m = {m}");
                assert_eq!(layout.n(), config.length(), "r = {r}, m = {m}");
            }
        }
    }

    #[test]
    fn submasks_are_yielded_once_in_descending_order() {
        let masks: Vec<_> = Submasks {
            mask: 0b1010,
            current: Some(0b1010),
        }
        .collect();

        assert_eq!(masks, vec![0b1010, 0b1000, 0b0010, 0]);
    }

    #[test]
    fn coset_bases_are_outside_the_mask() {
        let layout = RmLayout::new(RmConfig::new(2, 4).unwrap());
        let bases: Vec<_> = layout.coset_bases(0b1100).collect();

        assert_eq!(bases, vec![0b0011, 0b0010, 0b0001, 0]);
        assert!(bases.iter().all(|base| base & 0b1100 == 0));
    }

    #[test]
    fn generator_rows_and_encoding_agree() {
        let layout = RmLayout::new(RmConfig::new(2, 4).unwrap());
        let message: Vec<_> = [0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1]
            .into_iter()
            .map(|bit| Bit::from_bool(bit == 1))
            .collect();

        let expected = layout.encode(&message);
        let mut from_matrix = vec![Bit::Zero; layout.n()];
        for (message_bit, row) in message.iter().zip(layout.generator_rows()) {
            if message_bit.is_one() {
                for (output, &row_bit) in from_matrix.iter_mut().zip(&row) {
                    *output ^= row_bit;
                }
            }
        }

        assert_eq!(from_matrix, expected);
    }
}
