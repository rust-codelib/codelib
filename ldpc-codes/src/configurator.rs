//! Сборка согласованных кодера и декодера по одной проверочной матрице.

use crate::{DecoderConfig, LdpcError, ParityCheckMatrix, SpaDecoder, SystematicEncoder};

/// Создаёт пару кодера и декодера, согласованную с одной матрицей `H`.
pub struct LdpcConfigurator;

/// Согласованные кодер и декодер одного двоичного LDPC-кода.
///
/// Поля закрыты: пара появляется только после успешной подготовки и кодера,
/// и декодера по одной исходной проверочной матрице.
#[derive(Debug)]
pub struct ConfiguredLdpc {
    encoder: SystematicEncoder,
    decoder: SpaDecoder,
}

impl LdpcConfigurator {
    /// Строит систематический кодер и SPA-декодер по одной матрице `H`.
    ///
    /// Сначала готовится кодер по глубокой копии матрицы. Он вычисляет ранг
    /// одним приведением `H` к RREF и определяет длину сообщения и
    /// информационные позиции. После успешного создания кодера декодер
    /// получает исходную матрицу со всеми строками, включая зависимые и
    /// пустые. Пара возвращается только после успешной подготовки обоих
    /// компонентов.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`LdpcError::InvalidEncoderRank`], если ранг матрицы равен
    /// нулю или числу столбцов, а также ошибки размера или линейной алгебры,
    /// полученные при подготовке компонентов.
    pub fn build(
        checks: ParityCheckMatrix,
        config: DecoderConfig,
    ) -> Result<ConfiguredLdpc, LdpcError> {
        let encoder = SystematicEncoder::try_new(checks.clone())?;
        let decoder = SpaDecoder::try_new(checks, config)?;

        Ok(ConfiguredLdpc { encoder, decoder })
    }
}

impl ConfiguredLdpc {
    /// Возвращает согласованный систематический кодер.
    #[must_use]
    pub fn encoder(&self) -> &SystematicEncoder {
        &self.encoder
    }

    /// Возвращает изменяемую ссылку на согласованный SPA-декодер.
    #[must_use]
    pub fn decoder_mut(&mut self) -> &mut SpaDecoder {
        &mut self.decoder
    }
}
