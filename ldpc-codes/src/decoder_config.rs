//! Настройки декодирования LDPC-кода.

use crate::LdpcError;

const MAX_LLR_LIMIT: f64 = 20.0;

/// Неизменяемые параметры декодера.
///
/// Бюджет итераций может быть равен нулю. Предел LLR должен быть конечным
/// числом из диапазона `(0, 20]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoderConfig {
    max_iterations: usize,
    llr_limit: f64,
}

impl DecoderConfig {
    /// Создаёт конфигурацию с заданным бюджетом итераций и пределом LLR.
    ///
    /// Возвращает [`LdpcError::InvalidLlrLimit`], если `llr_limit` не является
    /// конечным числом из диапазона `(0, 20]`. Значение `max_iterations = 0`
    /// допустимо.
    pub fn try_new(max_iterations: usize, llr_limit: f64) -> Result<Self, LdpcError> {
        if !llr_limit.is_finite() || llr_limit <= 0.0 || llr_limit > MAX_LLR_LIMIT {
            return Err(LdpcError::InvalidLlrLimit);
        }

        Ok(Self {
            max_iterations,
            llr_limit,
        })
    }

    /// Возвращает максимальное число итераций декодера.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// Возвращает предел, которым декодер ограничивает значения LLR.
    pub fn llr_limit(&self) -> f64 {
        self.llr_limit
    }
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            llr_limit: MAX_LLR_LIMIT,
        }
    }
}
