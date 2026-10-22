//! Сборка согласованных кодера и декодера по одной проверочной матрице.

use coding_core::{Codeword, Observer};

use crate::{
    Bit, DecodeChecks, DecodeEvent, DecodeInput, DecodeResult, DecoderConfig, LdpcDecodeOutcome,
    LdpcError, ParityCheckMatrix, SpaDecoder, SystematicEncoder,
};

/// Создаёт пару кодера и декодера, согласованную с одной матрицей `H`.
pub struct LdpcConfigurator;

/// Согласованные кодер и декодер одного двоичного LDPC-кода.
///
/// Поля закрыты: пара появляется только после успешной подготовки и кодера,
/// и декодера по одной исходной проверочной матрице. Пара реализует общие
/// [`coding_core::Encoder<&[Bit]>`] и [`coding_core::Decoder<DecodeInput<'_>>`];
/// внутренние компоненты нельзя заменить отдельно.
///
/// Кодирование выполняют через `self.encode(...)`, а не через неизменяемый
/// [`Self::encoder`]: общему кодеру нужна изменяемая ссылка на пару. Метаданные
/// и извлечение сообщения остаются доступны через [`Self::encoder`].
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
    ///
    /// Ссылка неизменяемая: через неё доступны размеры, информационные позиции
    /// и извлечение сообщения. Кодировать нужно саму пару через
    /// [`coding_core::Encoder::encode`], иначе внутренний кодер нельзя было бы
    /// изменяемо позаимствовать при сохранении закрытой пары.
    #[must_use]
    pub fn encoder(&self) -> &SystematicEncoder {
        &self.encoder
    }

    /// Возвращает число информационных битов `k` согласованного кода.
    #[must_use]
    pub fn message_len(&self) -> usize {
        self.encoder.message_len()
    }

    /// Возвращает число битов кодового слова `n` согласованного кода.
    #[must_use]
    pub fn codeword_len(&self) -> usize {
        self.encoder.codeword_len()
    }
}

/// Кодирует сообщение согласованным систематическим кодером.
///
/// Слово возвращается в порядке столбцов исходной `H`, информационные биты
/// стоят в [`SystematicEncoder::information_positions`]. Событий нет, поэтому
/// наблюдатель не вызывается.
impl<'a> coding_core::Encoder<&'a [Bit]> for ConfiguredLdpc {
    type Output = Codeword<Vec<Bit>>;
    type Error = LdpcError;
    type Event = ();

    fn encode(
        &mut self,
        message: &'a [Bit],
        observer: &mut dyn Observer<()>,
    ) -> Result<Self::Output, Self::Error> {
        self.encoder.encode(message, observer)
    }
}

/// Декодирует один блок SPA-декодером, согласованным с кодером.
///
/// Контракт совпадает с реализацией у [`SpaDecoder`]: кандидат возвращается и
/// при исчерпании бюджета, включая бюджет `0`. Декодер остаётся связан с
/// исходной проверочной матрицей пары и повторно использует своё состояние
/// между вызовами. Ошибки входа не отправляют событий и не портят следующий
/// корректный вызов.
impl<'a> coding_core::Decoder<DecodeInput<'a>> for ConfiguredLdpc {
    type Value = DecodeResult;
    type Checks = DecodeChecks;
    type NeedMore = core::convert::Infallible;
    type StopReason = core::convert::Infallible;
    type Error = LdpcError;
    type Event = DecodeEvent;

    fn decode(
        &mut self,
        input: DecodeInput<'a>,
        observer: &mut dyn Observer<DecodeEvent>,
    ) -> Result<LdpcDecodeOutcome, LdpcError> {
        self.decoder.decode(input, observer)
    }
}
