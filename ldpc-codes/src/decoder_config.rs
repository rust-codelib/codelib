//! Настройки декодирования LDPC-кода.

use crate::LdpcError;

const MAX_LLR_LIMIT: f64 = 20.0;

/// Неизменяемые параметры SPA.
///
/// Значения по умолчанию: бюджет 50 итераций и предел LLR `20.0`. Бюджет
/// может быть равен нулю. Предел должен быть конечным числом из диапазона
/// `(0, 20]`. Публичный [`crate::spa_step`] выполняет ровно один шаг при любом
/// бюджете; `max_iterations` предназначен для цикла полного декодера.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoderConfig {
    max_iterations: usize,
    llr_limit: f64,
}

impl DecoderConfig {
    /// Создаёт конфигурацию с бюджетом итераций и пределом LLR.
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

    /// Возвращает бюджет итераций полного декодера.
    ///
    /// [`crate::spa_step`] этот бюджет не применяет и всегда выполняет один шаг.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// Возвращает предел насыщения канальных LLR и сообщений SPA.
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
