//! Конфигуратор кода по параметрам системы.

use crate::{CascadeCode, CascadeError, CascadeParams, CodeField, DecoderConfig};

/// Пошаговая настройка каскадного кода.
///
/// По умолчанию выбирается режим MSR (`mu = k`) и настройки декодера
/// [`DecoderConfig::default`].
///
/// ```rust
/// use cascade_codes::{CascadeConfigurator, Encoder};
/// use gf2m::Gf256;
///
/// let code = CascadeConfigurator::new(8, 4, 6).mbr().build::<Gf256>()?;
/// assert_eq!(code.params().alpha(), 6);
/// assert_eq!(code.message_len(), 18);
/// # Ok::<(), cascade_codes::CascadeError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeConfigurator {
    n: usize,
    k: usize,
    d: usize,
    mu: Option<usize>,
    decoder: DecoderConfig,
}

impl CascadeConfigurator {
    /// Начинает настройку системы `(n, k, d)`.
    #[must_use]
    pub fn new(n: usize, k: usize, d: usize) -> Self {
        Self {
            n,
            k,
            d,
            mu: None,
            decoder: DecoderConfig::default(),
        }
    }

    /// Выбирает режим `mu` (`1..=k`).
    #[must_use]
    pub fn mode(mut self, mu: usize) -> Self {
        self.mu = Some(mu);
        self
    }

    /// Режим минимальной полосы восстановления (`mu = 1`).
    #[must_use]
    pub fn mbr(self) -> Self {
        self.mode(1)
    }

    /// Режим минимального объёма хранения (`mu = k`).
    #[must_use]
    pub fn msr(mut self) -> Self {
        self.mu = None;
        self
    }

    /// Настройки восстановления данных.
    #[must_use]
    pub fn decoder_config(mut self, config: DecoderConfig) -> Self {
        self.decoder = config;
        self
    }

    /// Проверенные параметры без построения кода.
    ///
    /// # Ошибки
    ///
    /// См. [`CascadeParams::try_new`].
    pub fn params(&self) -> Result<CascadeParams, CascadeError> {
        CascadeParams::try_new(self.n, self.k, self.d, self.mu.unwrap_or(self.k))
    }

    /// Строит код над полем `F`.
    ///
    /// # Ошибки
    ///
    /// См. [`CascadeParams::try_new`] и [`CascadeCode::try_new`].
    pub fn build<F: CodeField>(&self) -> Result<CascadeCode<F>, CascadeError> {
        Ok(CascadeCode::try_new(self.params()?)?.with_decoder_config(self.decoder))
    }
}
