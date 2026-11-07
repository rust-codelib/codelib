//! Декодирование последовательным исключением.

use crate::transform::transform_in_place;
use crate::{Bit, PolarError};

/// Абсолютные значения LLR внутри декодера не превышают этот порог.
const LLR_ABS_MAX: f64 = 1.0e6;

/// Способ вычисления f-функции на факторе полярного графа.
///
/// g-функция в обоих вариантах одна: `L2 + (1 - 2û) L1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combine {
    /// Точная формула `2 atanh(tanh(a/2) tanh(b/2))` в устойчивой записи.
    #[default]
    Exact,
    /// Приближение min-sum: знак произведения и минимум модулей.
    MinSum,
}

/// Результат последовательного исключения.
///
/// [`source`](Self::source) — оценка информационных и замороженных битов `û`.
/// [`word`](Self::word) — кодовое слово `x̂ = û G_N`, а не жёсткое решение по канальным LLR.
/// [`message`](Self::message) — информационные биты `û` в порядке [`PolarCode::information_positions`](crate::PolarCode::information_positions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScDecision {
    source: Vec<Bit>,
    word: Vec<Bit>,
    message: Vec<Bit>,
}

impl ScDecision {
    /// Возвращает оценку вектора `u`, включая замороженные нули.
    #[must_use]
    pub fn source(&self) -> &[Bit] {
        &self.source
    }

    /// Возвращает кодовое слово, полученное повторным кодированием оценки `u`.
    #[must_use]
    pub fn word(&self) -> &[Bit] {
        &self.word
    }

    /// Возвращает информационные биты оценки `u`.
    #[must_use]
    pub fn message(&self) -> &[Bit] {
        &self.message
    }
}

pub(crate) fn successive_cancellation(
    frozen: &[bool],
    information: &[usize],
    llrs: &[f64],
    rule: Combine,
) -> Result<ScDecision, PolarError> {
    if llrs.len() != frozen.len() {
        return Err(PolarError::LlrLengthMismatch {
            expected: frozen.len(),
            actual: llrs.len(),
        });
    }
    if let Some(index) = llrs.iter().position(|llr| !llr.is_finite()) {
        return Err(PolarError::NonFiniteLlr { index });
    }

    let channel: Vec<f64> = llrs.iter().copied().map(clamp_llr).collect();
    let source = decode_node(&channel, frozen, 0, rule);
    let message = information.iter().map(|&index| source[index]).collect();
    let mut word = source.clone();
    transform_in_place(&mut word);

    Ok(ScDecision {
        source,
        word,
        message,
    })
}

fn decode_node(llr: &[f64], frozen: &[bool], offset: usize, rule: Combine) -> Vec<Bit> {
    if llr.len() == 1 {
        let bit = if frozen[offset] {
            Bit::Zero
        } else {
            hard_decision(llr[0])
        };
        return vec![bit];
    }

    let half = llr.len() / 2;
    let mut left_llr = vec![0.0; half];
    for index in 0..half {
        left_llr[index] = combine_f(llr[index], llr[index + half], rule);
    }
    let left = decode_node(&left_llr, frozen, offset, rule);

    // Частичные суммы для g — это полярный образ уже решённой левой половины u.
    let mut partial = left.clone();
    transform_in_place(&mut partial);

    let mut right_llr = vec![0.0; half];
    for index in 0..half {
        right_llr[index] = combine_g(llr[index], llr[index + half], partial[index]);
    }
    let right = decode_node(&right_llr, frozen, offset + half, rule);

    let mut source = left;
    source.extend(right);
    source
}

fn hard_decision(llr: f64) -> Bit {
    if llr >= 0.0 {
        Bit::Zero
    } else {
        Bit::One
    }
}

fn clamp_llr(llr: f64) -> f64 {
    llr.clamp(-LLR_ABS_MAX, LLR_ABS_MAX)
}

fn combine_f(left: f64, right: f64, rule: Combine) -> f64 {
    let magnitude = left.abs().min(right.abs());
    if magnitude == 0.0 {
        return 0.0;
    }

    let negative = (left < 0.0) ^ (right < 0.0);
    let value = match rule {
        Combine::MinSum => {
            if negative {
                -magnitude
            } else {
                magnitude
            }
        }
        Combine::Exact => {
            let base = if negative { -magnitude } else { magnitude };
            // log1p(exp(-|a+b|)) - log1p(exp(-|a-b|)) — точная поправка к min-sum.
            let sum = (left + right).abs();
            let difference = (left - right).abs();
            base + (-sum).exp().ln_1p() - (-difference).exp().ln_1p()
        }
    };
    clamp_llr(value)
}

fn combine_g(left: f64, right: f64, bit: Bit) -> f64 {
    let value = match bit {
        Bit::Zero => right + left,
        Bit::One => right - left,
    };
    if value.is_nan() {
        0.0
    } else {
        clamp_llr(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{combine_f, Combine};

    #[test]
    fn exact_boxplus_matches_the_closed_form_for_equal_magnitudes() {
        let expected = 10.0 - 2.0_f64.ln();
        let same = combine_f(10.0, 10.0, Combine::Exact);
        let opposite = combine_f(10.0, -10.0, Combine::Exact);
        assert!((same - expected).abs() < 1.0e-8);
        assert!((opposite + expected).abs() < 1.0e-8);
        assert_eq!(combine_f(0.0, 5.0, Combine::Exact), 0.0);
        assert!((combine_f(40.0, 40.0, Combine::Exact) - (40.0 - 2.0_f64.ln())).abs() < 1.0e-9);
    }

    #[test]
    fn min_sum_keeps_the_sign_product_and_the_smaller_magnitude() {
        assert_eq!(combine_f(10.0, -4.0, Combine::MinSum), -4.0);
        assert_eq!(combine_f(-3.0, -8.0, Combine::MinSum), 3.0);
        assert_eq!(combine_f(0.0, -8.0, Combine::MinSum), 0.0);
        assert_eq!(combine_f(-0.0, 2.0, Combine::MinSum), 0.0);
    }
}
