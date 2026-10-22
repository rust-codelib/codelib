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
/// Поля закрыты: пара появляется только после успешной подготовки обоих
/// компонентов по одной исходной `H`. Пара реализует
/// [`coding_core::Encoder<&[Bit]>`] и [`coding_core::Decoder<DecodeInput<'_>>`];
/// внутренние компоненты нельзя заменить отдельно.
///
/// Кодировать нужно саму пару через `self.encode(...)`: [`Self::encoder`]
/// возвращает неизменяемую ссылку для метаданных и извлечения сообщения.
///
/// Получение изменяемой ссылки на внутренний декодер не компилируется:
///
/// ```compile_fail,E0599
/// use ldpc_codes::{ConfiguredLdpc, SpaDecoder};
///
/// fn replace_decoder(code: &mut ConfiguredLdpc, decoder: SpaDecoder) {
///     *code.decoder_mut() = decoder;
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
    /// Пара возвращается только после успешной подготовки обоих компонентов.
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
        // Кодер сохраняет исходную H; декодер строит граф по ссылке на неё,
        // включая зависимые и пустые строки.
        let encoder = SystematicEncoder::try_new(checks)?;
        let decoder = SpaDecoder::try_new_borrowed(encoder.checks(), config)?;

        Ok(ConfiguredLdpc { encoder, decoder })
    }
}

impl ConfiguredLdpc {
    /// Возвращает согласованный систематический кодер.
    ///
    /// Ссылка неизменяемая: через неё доступны размеры, информационные позиции
    /// и извлечение сообщения. Для кодирования используйте
    /// [`coding_core::Encoder::encode`] у самой пары.
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
/// Контракт совпадает с [`SystematicEncoder`], включая порядок слова и
/// [`SystematicEncoder::information_positions`]. Событий нет, поэтому
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
/// Контракт совпадает с [`SpaDecoder`], включая кандидат при исчерпании
/// бюджета (в том числе `0`), повторное использование состояния и отсутствие
/// событий при ошибке входа.
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
