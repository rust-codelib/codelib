//! Выбор информационных позиций по параметрам Бхаттачарии стирающего канала.

use crate::length::check_length;
use crate::{PolarCode, PolarError};

/// Считает параметры Бхаттачарии синтетических каналов двоичного стирающего канала.
///
/// Для вероятности стирания `ε` рекурсия точна: `Z(W-) = 2Z - Z²` и `Z(W+) = Z²`.
/// Индекс `i` соответствует биту источника `u_i` в естественном порядке кодера.
/// Меньшее значение означает более надёжный канал.
///
/// # Errors
///
/// Возвращает [`PolarError::InvalidLength`], если длина недопустима, или
/// [`PolarError::InvalidErasureProbability`], если вероятность вне отрезка `[0, 1]`.
pub fn bhattacharyya_bec(length: usize, erasure_probability: f64) -> Result<Vec<f64>, PolarError> {
    check_length(length)?;
    if !(0.0..=1.0).contains(&erasure_probability) {
        return Err(PolarError::InvalidErasureProbability);
    }

    let mut score = vec![erasure_probability];
    while score.len() < length {
        let mut next = vec![0.0; score.len() * 2];
        for (index, &value) in score.iter().enumerate() {
            next[2 * index] = (2.0 - value) * value;
            next[2 * index + 1] = value * value;
        }
        score = next;
    }
    Ok(score)
}

impl PolarCode {
    /// Строит код, оставляя `message_len` синтетических каналов с наименьшим `Z`.
    ///
    /// При равенстве параметров предпочтение получает меньший индекс. Для
    /// `message_len = 0` все биты заморожены.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку длины, вероятности стирания или слишком большого
    /// `message_len`.
    pub fn from_bec(
        length: usize,
        message_len: usize,
        erasure_probability: f64,
    ) -> Result<Self, PolarError> {
        if message_len > length {
            return Err(PolarError::InvalidMessageLength {
                message_len,
                length,
            });
        }

        let score = bhattacharyya_bec(length, erasure_probability)?;
        let mut order: Vec<usize> = (0..length).collect();
        order.sort_by(|&left, &right| score[left].total_cmp(&score[right]).then(left.cmp(&right)));
        order.truncate(message_len);
        Self::try_new(length, &order)
    }
}
