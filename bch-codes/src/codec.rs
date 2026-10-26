//! Адаптер блочного BCH-кода к унифицированным ролям `coding-core`.

use core::convert::Infallible;

use coding_core::{
    Candidate, Codeword, DecodeOutcome, Decoder, Encoder, Message, MessageBytes, Observer,
};

use crate::config::generator_degree;
use crate::decode::{correct, BchDecodeWorkspace};
use crate::encode::{encode_systematic, extract_message};
use crate::generator::validate_generator;
use crate::{BchChecks, BchConfig, BchCorrection, BchError, BchEvent, BchStopReason};

/// Вход унифицированного кодера.
pub struct BchEncodeInput<'a, 'out> {
    /// Сообщение в упакованном представлении.
    pub message: &'a [u8],
    /// Выходной буфер кодового слова.
    pub output: &'out mut [u8],
}

/// Вход унифицированного декодера.
pub struct BchDecodeInput<'word, 'work, const N: usize, const POLY: u32> {
    /// Принятое слово, исправляемое на месте только при успехе.
    pub received: &'word mut [u8],
    /// Рабочая память одного вызова.
    pub workspace: BchDecodeWorkspace<'work, N, POLY>,
}

/// Подготовленный BCH-кодек, заимствующий порождающий многочлен.
pub struct BchCodec<'g, const N: usize, const POLY: u32> {
    config: BchConfig,
    generator: &'g [u8],
}

impl<'g, const N: usize, const POLY: u32> BchCodec<'g, N, POLY> {
    /// Проверяет конфигурацию и порождающий многочлен.
    pub fn new(config: BchConfig, generator: &'g [u8]) -> Result<Self, BchError> {
        let degree = validate_generator::<N, POLY>(&config, generator)?;
        MessageBytes::from_bits(N - 1 - degree - config.shortened_by)?;
        Ok(Self { config, generator })
    }

    /// Возвращает конфигурацию.
    #[must_use]
    pub const fn config(&self) -> BchConfig {
        self.config
    }

    /// Возвращает длину кодового слова в битах.
    #[must_use]
    pub const fn codeword_bits(&self) -> usize {
        N - 1 - self.config.shortened_by
    }

    /// Возвращает длину сообщения в битах.
    #[must_use]
    pub fn message_bits(&self) -> usize {
        N - 1 - generator_degree::<N>(&self.config).unwrap_or(0) - self.config.shortened_by
    }

    /// Извлекает сообщение в буфер вызывающего кода.
    pub fn extract_message<'out>(
        &self,
        word: Codeword<&[u8]>,
        output: &'out mut [u8],
    ) -> Result<Message<&'out mut [u8]>, BchError> {
        extract_message::<N>(&self.config, self.generator, word, output)
    }
}

impl<'g, 'a, 'out, const N: usize, const POLY: u32> Encoder<BchEncodeInput<'a, 'out>>
    for BchCodec<'g, N, POLY>
{
    type Output = Codeword<&'out mut [u8]>;
    type Error = BchError;
    type Event = BchEvent;

    fn encode(
        &mut self,
        input: BchEncodeInput<'a, 'out>,
        observer: &mut dyn Observer<Self::Event>,
    ) -> Result<Self::Output, Self::Error> {
        encode_systematic::<N, POLY>(&self.config, self.generator, input.message, input.output)?;
        observer.on_event(&BchEvent::Encoded);
        Ok(Codeword(input.output))
    }
}

impl<'g, 'word, 'work, const N: usize, const POLY: u32>
    Decoder<BchDecodeInput<'word, 'work, N, POLY>> for BchCodec<'g, N, POLY>
{
    type Value = Codeword<&'word mut [u8]>;
    type Checks = BchChecks;
    type NeedMore = Infallible;
    type StopReason = BchStopReason;
    type Error = BchError;
    type Event = BchEvent;

    fn decode(
        &mut self,
        mut input: BchDecodeInput<'word, 'work, N, POLY>,
        observer: &mut dyn Observer<Self::Event>,
    ) -> Result<
        DecodeOutcome<Self::Value, Self::Checks, Self::NeedMore, Self::StopReason>,
        Self::Error,
    > {
        let result = correct::<N, POLY>(
            &self.config,
            self.generator,
            input.received,
            &mut input.workspace,
        )?;
        match result {
            BchCorrection::Candidate(checks) => {
                observer.on_event(&BchEvent::SyndromesComputed {
                    all_zero: checks.initial_syndromes_zero,
                });
                observer.on_event(&BchEvent::DecodingFinished {
                    corrected_bits: checks.corrected_bits,
                    verified: checks.final_syndromes_zero,
                });
                Ok(DecodeOutcome::Candidate(Candidate {
                    value: Codeword(input.received),
                    checks,
                }))
            }
            BchCorrection::Stopped(reason) => {
                observer.on_event(&BchEvent::DecodingFinished {
                    corrected_bits: 0,
                    verified: false,
                });
                Ok(DecodeOutcome::Stopped(reason))
            }
        }
    }
}
