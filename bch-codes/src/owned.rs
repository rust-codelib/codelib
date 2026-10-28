//! Владеющий BCH-кодек с автоматическим выбором конечного поля.

use alloc::vec;
use alloc::vec::Vec;
use core::convert::Infallible;

use coding_core::{Candidate, Codeword, DecodeOutcome, Decoder, Encoder, Message, Observer};

use crate::{
    build_generator, construction_workspace_len, correct, decode_workspace_len, encode_systematic,
    extract_message, generator_bytes, generator_degree, BchChecks, BchCodec, BchConfig,
    BchCorrection, BchDecodeWorkspace, BchEncodeInput, BchError, BchEvent, BchStopReason, Gf,
};

/// Владеющий результат декодирования сообщения.
pub type OwnedBchDecodeOutcome =
    DecodeOutcome<Message<Vec<u8>>, BchChecks, Infallible, BchStopReason>;

/// Вход владеющего декодера в унифицированном API.
///
/// Рабочая память выделяется внутри вызова и не передаётся пользователем.
pub struct OwnedBchDecodeInput<'word> {
    /// Принятое слово, исправляемое на месте только при успехе.
    pub received: &'word mut [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Gf4,
    Gf8,
    Gf16,
    Gf32,
    Gf64,
    Gf128,
    Gf256,
    Gf512,
    Gf1024,
    Gf2048,
    Gf4096,
    Gf8192,
    Gf16384,
    Gf32768,
    Gf65536,
}

/// BCH-кодек, который сам выбирает поле и владеет порождающим многочленом.
///
/// Пользователь задаёт только длину сообщения в битах и гарантированно
/// исправляемое число ошибок. Временная память выделяется отдельно для каждого
/// вызова кодирования или декодирования.
pub struct OwnedBchCodec {
    config: BchConfig,
    generator: Vec<u8>,
    field: Field,
    message_bits: usize,
    codeword_bits: usize,
}

impl OwnedBchCodec {
    /// Выбирает наименьшее поддерживаемое поле, вмещающее сообщение.
    pub fn new(message_bits: usize, correctable_errors: usize) -> Result<Self, BchError> {
        if message_bits == 0 {
            return Err(BchError::InvalidParameters);
        }
        let base = BchConfig::new(correctable_errors, 0)?;

        macro_rules! select {
            ($(($variant:ident, $n:literal, $poly:literal)),+ $(,)?) => {
                $(
                    if let Some(shortened_by) = shortening::<$n>(&base, message_bits) {
                        return Self::prepare::<$n, $poly>(
                            Field::$variant,
                            BchConfig::new(correctable_errors, shortened_by)?,
                            message_bits,
                        );
                    }
                )+
            };
        }

        select!(
            (Gf4, 4, 0b1_11),                          // x^2 + x + 1.
            (Gf8, 8, 0b1_011),                         // x^3 + x + 1.
            (Gf16, 16, 0b1_0011),                      // x^4 + x + 1.
            (Gf32, 32, 0b1_00101),                     // x^5 + x^2 + 1.
            (Gf64, 64, 0b1_000011),                    // x^6 + x + 1.
            (Gf128, 128, 0b1_0000011),                 // x^7 + x + 1.
            (Gf256, 256, 0b1_0001_1101),               // x^8 + x^4 + x^3 + x^2 + 1.
            (Gf512, 512, 0b10_0001_0001),              // x^9 + x^4 + 1.
            (Gf1024, 1024, 0b100_0000_1001),           // x^10 + x^3 + 1.
            (Gf2048, 2048, 0b1000_0000_0101),          // x^11 + x^2 + 1.
            (Gf4096, 4096, 0b1_0000_0101_0011),        // x^12 + x^6 + x^4 + x + 1.
            (Gf8192, 8192, 0b10_0000_0001_1011),       // x^13 + x^4 + x^3 + x + 1.
            (Gf16384, 16384, 0b100_0000_0010_1011),    // x^14 + x^5 + x^3 + x + 1.
            (Gf32768, 32768, 0b1000_0000_0000_0011),   // x^15 + x + 1.
            (Gf65536, 65536, 0b1_0000_0000_0010_1101), // x^16 + x^5 + x^3 + x^2 + 1.
        );
        Err(BchError::InvalidParameters)
    }

    fn prepare<const N: usize, const POLY: u32>(
        field: Field,
        config: BchConfig,
        message_bits: usize,
    ) -> Result<Self, BchError> {
        let mut generator = vec![0; generator_bytes::<N>(&config)?];
        let mut workspace = vec![Gf::<N, POLY>::zero(); construction_workspace_len::<N>(&config)?];
        build_generator::<N, POLY>(&config, &mut generator, &mut workspace)?;
        let codec = BchCodec::<N, POLY>::new(config, &generator)?;
        let codeword_bits = codec.codeword_bits();
        Ok(Self {
            config,
            generator,
            field,
            message_bits,
            codeword_bits,
        })
    }

    /// Возвращает длину сообщения в битах.
    #[must_use]
    pub const fn message_bits(&self) -> usize {
        self.message_bits
    }

    /// Возвращает длину кодового слова в битах.
    #[must_use]
    pub const fn codeword_bits(&self) -> usize {
        self.codeword_bits
    }

    /// Возвращает гарантированно исправляемое число ошибок.
    #[must_use]
    pub const fn correctable_errors(&self) -> usize {
        self.config.correctable_errors
    }

    /// Кодирует сообщение и возвращает новый буфер кодового слова.
    pub fn encode(&self, message: &[u8]) -> Result<Vec<u8>, BchError> {
        let mut output = vec![0; bytes_for(self.codeword_bits)];
        self.dispatch_encode(message, &mut output)?;
        Ok(output)
    }

    /// Исправляет принятое слово и извлекает сообщение в новый буфер.
    pub fn decode(&self, received: &[u8]) -> Result<OwnedBchDecodeOutcome, BchError> {
        let mut word = received.to_vec();
        let correction = self.dispatch_correct(&mut word)?;
        match correction {
            BchCorrection::Candidate(checks) => {
                let mut message = vec![0; bytes_for(self.message_bits)];
                self.dispatch_extract(&word, &mut message)?;
                Ok(DecodeOutcome::Candidate(Candidate {
                    value: Message(message),
                    checks,
                }))
            }
            BchCorrection::Stopped(reason) => Ok(DecodeOutcome::Stopped(reason)),
        }
    }

    fn dispatch_encode(&self, message: &[u8], output: &mut [u8]) -> Result<(), BchError> {
        macro_rules! dispatch {
            ($(($variant:ident, $n:literal, $poly:literal)),+ $(,)?) => {
                match self.field {
                    $(Field::$variant => encode_systematic::<$n, $poly>(
                        &self.config, &self.generator, message, output,
                    ),)+
                }
            };
        }
        fields!(dispatch)
    }

    fn dispatch_correct(&self, received: &mut [u8]) -> Result<BchCorrection, BchError> {
        macro_rules! dispatch {
            ($(($variant:ident, $n:literal, $poly:literal)),+ $(,)?) => {
                match self.field {
                    $(Field::$variant => correct_alloc::<$n, $poly>(
                        &self.config, &self.generator, received,
                    ),)+
                }
            };
        }
        fields!(dispatch)
    }

    fn dispatch_extract(&self, word: &[u8], output: &mut [u8]) -> Result<(), BchError> {
        macro_rules! dispatch {
            ($(($variant:ident, $n:literal, $poly:literal)),+ $(,)?) => {
                match self.field {
                    $(Field::$variant => extract_message::<$n>(
                        &self.config, &self.generator, Codeword(word), output,
                    ).map(|_| ()),)+
                }
            };
        }
        fields!(dispatch)
    }
}

macro_rules! fields {
    ($macro:ident) => {
        $macro!(
            (Gf4, 4, 0b1_11),
            (Gf8, 8, 0b1_011),
            (Gf16, 16, 0b1_0011),
            (Gf32, 32, 0b1_00101),
            (Gf64, 64, 0b1_000011),
            (Gf128, 128, 0b1_0000011),
            (Gf256, 256, 0b1_0001_1101),
            (Gf512, 512, 0b10_0001_0001),
            (Gf1024, 1024, 0b100_0000_1001),
            (Gf2048, 2048, 0b1000_0000_0101),
            (Gf4096, 4096, 0b1_0000_0101_0011),
            (Gf8192, 8192, 0b10_0000_0001_1011),
            (Gf16384, 16384, 0b100_0000_0010_1011),
            (Gf32768, 32768, 0b1000_0000_0000_0011),
            (Gf65536, 65536, 0b1_0000_0000_0010_1101),
        )
    };
}
use fields;

fn shortening<const N: usize>(config: &BchConfig, requested_message_bits: usize) -> Option<usize> {
    let degree = generator_degree::<N>(config).ok()?;
    let full_message_bits = N - 1 - degree;
    if requested_message_bits <= full_message_bits {
        Some(full_message_bits - requested_message_bits)
    } else {
        None
    }
}

fn correct_alloc<const N: usize, const POLY: u32>(
    config: &BchConfig,
    generator: &[u8],
    received: &mut [u8],
) -> Result<BchCorrection, BchError> {
    let mut field = vec![Gf::<N, POLY>::zero(); decode_workspace_len(config)];
    let mut positions = vec![0; config.correctable_errors];
    correct::<N, POLY>(
        config,
        generator,
        received,
        &mut BchDecodeWorkspace {
            field: &mut field,
            positions: &mut positions,
        },
    )
}

const fn bytes_for(bits: usize) -> usize {
    bits.saturating_add(7) / 8
}

impl<'a, 'out> Encoder<BchEncodeInput<'a, 'out>> for OwnedBchCodec {
    type Output = Codeword<&'out mut [u8]>;
    type Error = BchError;
    type Event = BchEvent;

    fn encode(
        &mut self,
        input: BchEncodeInput<'a, 'out>,
        observer: &mut dyn Observer<Self::Event>,
    ) -> Result<Self::Output, Self::Error> {
        self.dispatch_encode(input.message, input.output)?;
        observer.on_event(&BchEvent::Encoded);
        Ok(Codeword(input.output))
    }
}

impl<'word> Decoder<OwnedBchDecodeInput<'word>> for OwnedBchCodec {
    type Value = Codeword<&'word mut [u8]>;
    type Checks = BchChecks;
    type NeedMore = Infallible;
    type StopReason = BchStopReason;
    type Error = BchError;
    type Event = BchEvent;

    fn decode(
        &mut self,
        input: OwnedBchDecodeInput<'word>,
        observer: &mut dyn Observer<Self::Event>,
    ) -> Result<DecodeOutcome<Self::Value, Self::Checks, Self::NeedMore, Self::StopReason>, BchError>
    {
        let result = self.dispatch_correct(input.received)?;
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
