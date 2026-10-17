//! Сборка согласованных кодера и декодера Рида–Маллера.

use crate::{
    Bit, Encoder, RmConfig, RmDecodeObserver, RmDecodeResult, RmDecoder, RmEncoder, RmError,
};

/// Создаёт пару кодера и декодера для одних параметров `RM(r, m)`.
pub struct RmConfigurator;

/// Согласованные кодер и декодер одного кода Рида–Маллера.
///
/// Поля закрыты: пара появляется только после проверки параметров
/// [`RmConfig`].
#[derive(Clone, Debug)]
pub struct ConfiguredRm {
    encoder: RmEncoder,
    decoder: RmDecoder,
}

impl RmConfigurator {
    /// Строит кодер и декодер по проверенным параметрам.
    #[must_use]
    pub fn build(config: RmConfig) -> ConfiguredRm {
        ConfiguredRm {
            encoder: RmEncoder::new(config),
            decoder: RmDecoder::new(config),
        }
    }
}

impl ConfiguredRm {
    /// Возвращает согласованный кодер.
    #[must_use]
    pub fn encoder(&self) -> &RmEncoder {
        &self.encoder
    }

    /// Возвращает согласованный декодер.
    #[must_use]
    pub fn decoder(&self) -> &RmDecoder {
        &self.decoder
    }

    /// Кодирует сообщение длины `k`.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::MessageLengthMismatch`] при неверной длине.
    pub fn encode(&self, message: &[Bit]) -> Result<Vec<Bit>, RmError> {
        self.encoder.encode(message)
    }

    /// Извлекает сообщение из допустимого кодового слова.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::WordLengthMismatch`] или
    /// [`RmError::InvalidCodeword`].
    pub fn extract_message(&self, word: &[Bit]) -> Result<Vec<Bit>, RmError> {
        self.encoder.extract_message(word)
    }

    /// Декодирует слово мажоритарным алгоритмом Рида.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::WordLengthMismatch`] при неверной длине.
    pub fn decode(
        &self,
        word: &[Bit],
        observer: Option<&mut dyn RmDecodeObserver>,
    ) -> Result<RmDecodeResult, RmError> {
        self.decoder.decode(word, observer)
    }
}
