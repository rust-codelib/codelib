//! Сборка согласованных кодера и декодера по одной проверочной матрице.

use crate::{
    DecodeInput, DecodeObserver, DecodeResult, Decoder, DecoderConfig, LdpcError,
    ParityCheckMatrix, SpaDecoder, SystematicEncoder,
};

/// Создаёт пару кодера и декодера, согласованную с одной матрицей `H`.
pub struct LdpcConfigurator;

/// Согласованные кодер и декодер одного двоичного LDPC-кода.
///
/// Поля закрыты: пара появляется только после успешной подготовки и кодера,
/// и декодера по одной исходной проверочной матрице. Декодировать блок можно
/// через [`Self::decode`]; внутренний декодер нельзя заменить отдельно.
///
/// Попытка получить изменяемую ссылку на внутренний декодер не компилируется:
///
/// ```compile_fail,E0599
/// use ldpc_codes::{DecoderConfig, LdpcConfigurator, ParityCheckMatrix, SpaDecoder};
///
/// fn main() -> Result<(), ldpc_codes::LdpcError> {
///     let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
///     let mut code = LdpcConfigurator::build(checks, DecoderConfig::default())?;
///     let other_checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0]])?;
///     *code.decoder_mut() = SpaDecoder::try_new(other_checks, DecoderConfig::default())?;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct ConfiguredLdpc {
    encoder: SystematicEncoder,
    decoder: SpaDecoder,
}

impl LdpcConfigurator {
    /// Строит систематический кодер и SPA-декодер по одной матрице `H`.
    ///
    /// Сначала готовится кодер, который сохраняет исходную матрицу, вычисляет
    /// ранг одним приведением `H` к RREF и определяет длину сообщения и
    /// информационные позиции. После успешного создания кодера декодер
    /// строит граф по ссылке на сохранённую матрицу, включая зависимые и
    /// пустые строки. Пара возвращается только после успешной подготовки обоих
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
        let encoder = SystematicEncoder::try_new(checks)?;
        let decoder = SpaDecoder::try_new_borrowed(encoder.checks(), config)?;

        Ok(ConfiguredLdpc { encoder, decoder })
    }
}

impl ConfiguredLdpc {
    /// Возвращает согласованный систематический кодер.
    #[must_use]
    pub fn encoder(&self) -> &SystematicEncoder {
        &self.encoder
    }

    /// Декодирует один блок с SPA-декодером, согласованным с кодером.
    ///
    /// Декодер остаётся связан с исходной проверочной матрицей пары и
    /// повторно использует своё состояние между вызовами.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибки некорректного входа, определённые контрактом
    /// [`Decoder::decode`].
    pub fn decode(
        &mut self,
        input: DecodeInput<'_>,
        observer: Option<&mut dyn DecodeObserver>,
    ) -> Result<DecodeResult, LdpcError> {
        self.decoder.decode(input, observer)
    }
}
