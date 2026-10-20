//! Небольшие тестовые кодеки для проверки общих интерфейсов.

use coding_core::{
    Candidate, Codeword, DecodeOutcome, Decoder, Encoder, LlrConvention, Message, MessageBytes,
    Observer, TransformInput,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Shape,
    Value,
    Context,
    Model,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checks {
    ParityPassed,
    FrozenPassed,
    TableMember,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    ParityFailed,
    Tie,
    Budget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Completed,
}

/// Задаёт порядок информационных позиций; в тестах используется [2, 0].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    information: [usize; 2],
    parity: usize,
}

impl Layout {
    pub fn try_new(information: [usize; 2]) -> Result<Self, Error> {
        MessageBytes::from_bits(information.len()).map_err(|_| Error::Shape)?;
        if information.iter().any(|&p| p >= 3) || information[0] == information[1] {
            return Err(Error::Context);
        }
        let parity = 3 - information[0] - information[1];
        Ok(Self {
            information,
            parity,
        })
    }

    pub fn extract(&self, word: Codeword<[u8; 3]>) -> Message<[u8; 2]> {
        Message([word.0[self.information[0]], word.0[self.information[1]]])
    }
}

pub struct Block(pub Layout);

impl Encoder<&[u8]> for Block {
    type Output = Codeword<[u8; 3]>;
    type Error = Error;
    type Event = Event;

    fn encode(
        &mut self,
        input: &[u8],
        observer: &mut dyn Observer<Event>,
    ) -> Result<Self::Output, Error> {
        if input.len() != 2 {
            return Err(Error::Shape);
        }
        if input.iter().any(|&b| b > 1) {
            return Err(Error::Value);
        }
        let mut word = [0; 3];
        word[self.0.information[0]] = input[0];
        word[self.0.information[1]] = input[1];
        word[self.0.parity] = input[0] ^ input[1];
        observer.on_event(&Event::Completed);
        Ok(Codeword(word))
    }
}

pub struct Hard;

impl Decoder<&[u8]> for Hard {
    type Value = Codeword<[u8; 3]>;
    type Checks = Checks;
    type NeedMore = usize;
    type StopReason = Stop;
    type Error = Error;
    type Event = Event;

    fn decode(
        &mut self,
        input: &[u8],
        observer: &mut dyn Observer<Event>,
    ) -> Result<DecodeOutcome<Self::Value, Checks, usize, Stop>, Error> {
        if input.len() != 3 {
            return Err(Error::Shape);
        }
        if input.iter().any(|&b| b > 1) {
            return Err(Error::Value);
        }
        if input[0] ^ input[1] ^ input[2] != 0 {
            return Ok(DecodeOutcome::Stopped(Stop::ParityFailed));
        }
        observer.on_event(&Event::Completed);
        Ok(DecodeOutcome::Candidate(Candidate {
            value: Codeword([input[0], input[1], input[2]]),
            checks: Checks::ParityPassed,
        }))
    }
}

/// Мягкие оценки в порядке позиций кодового слова.
///
/// Стираемые позиции дают нулевой вклад, но и на них требуются конечные входные
/// значения. Насыщение не выполняется; оба знака нуля означают отсутствие предпочтения.
pub struct SoftInput<'a> {
    pub layout: Layout,
    pub convention: LlrConvention,
    pub values: &'a [f64],
    pub erased: &'a [bool],
}

#[derive(Clone, Copy)]
pub struct Model {
    pub layout: Layout,
    pub convention: LlrConvention,
    pub gain: f64,
}

pub struct Soft {
    layout: Layout,
    convention: LlrConvention,
    gain: f64,
    budget: usize,
}

impl Soft {
    /// Проверяет совместимость подготовленной модели и настроек декодера.
    pub fn try_new(
        layout: Layout,
        convention: LlrConvention,
        model: Model,
        budget: usize,
    ) -> Result<Self, Error> {
        if model.layout != layout || model.convention != convention {
            return Err(Error::Context);
        }
        if !model.gain.is_finite() || model.gain <= 0.0 {
            return Err(Error::Model);
        }
        Ok(Self {
            layout,
            convention,
            gain: model.gain,
            budget,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoftResult {
    pub word: Codeword<[u8; 3]>,
    /// Канальные оценки после смены знака, стираний и масштабирования моделью.
    /// Это не апостериорные оценки декодера.
    pub scores: [f64; 3],
}

impl Decoder<SoftInput<'_>> for Soft {
    type Value = SoftResult;
    type Checks = Checks;
    type NeedMore = usize;
    type StopReason = Stop;
    type Error = Error;
    type Event = Event;

    fn decode(
        &mut self,
        input: SoftInput<'_>,
        observer: &mut dyn Observer<Event>,
    ) -> Result<DecodeOutcome<SoftResult, Checks, usize, Stop>, Error> {
        if input.layout != self.layout || input.convention != self.convention {
            return Err(Error::Context);
        }
        if input.values.len() != 3 || input.erased.len() != 3 {
            return Err(Error::Shape);
        }
        if input.values.iter().any(|v| !v.is_finite()) {
            return Err(Error::Value);
        }
        let mut scores = [0.0; 3];
        for (i, score) in scores.iter_mut().enumerate() {
            *score = if input.erased[i] {
                0.0
            } else {
                input
                    .convention
                    .convert(input.values[i], LlrConvention::PositiveForZero)
                    * self.gain
            };
            if !score.is_finite() {
                return Err(Error::Value);
            }
        }
        if self.budget < 4 {
            return Ok(DecodeOutcome::Stopped(Stop::Budget));
        }
        // Для маленького кода достаточно перебрать четыре слова и сравнить их оценки.
        let mut best = f64::NEG_INFINITY;
        let mut winner = Codeword([0; 3]);
        let mut tied = false;
        for message in [[0, 0], [0, 1], [1, 0], [1, 1]] {
            let word = Block(self.layout).encode(&message, &mut coding_core::NoopObserver)?;
            let metric: f64 = word
                .0
                .iter()
                .zip(scores)
                .map(|(&b, s)| if b == 0 { s } else { -s })
                .sum();
            if !metric.is_finite() {
                return Err(Error::Value);
            }
            if metric > best {
                best = metric;
                winner = word;
                tied = false;
            } else if metric == best {
                tied = true;
            }
        }
        if tied {
            return Ok(DecodeOutcome::Stopped(Stop::Tie));
        }
        observer.on_event(&Event::Completed);
        Ok(DecodeOutcome::Candidate(Candidate {
            value: SoftResult {
                word: winner,
                scores,
            },
            checks: Checks::ParityPassed,
        }))
    }
}

/// Тестовый декодер, возвращающий вход преобразования с нулём в позиции 1.
pub struct Transform;
impl Decoder<[u8; 3]> for Transform {
    type Value = TransformInput<[u8; 3]>;
    type Checks = Checks;
    type NeedMore = ();
    type StopReason = Stop;
    type Error = Error;
    type Event = ();

    fn decode(
        &mut self,
        input: [u8; 3],
        _: &mut dyn Observer<()>,
    ) -> Result<DecodeOutcome<Self::Value, Checks, (), Stop>, Error> {
        if input.iter().any(|&b| b > 1) {
            return Err(Error::Value);
        }
        if input[1] != 0 {
            return Ok(DecodeOutcome::Stopped(Stop::ParityFailed));
        }
        Ok(DecodeOutcome::Candidate(Candidate {
            value: TransformInput(input),
            checks: Checks::FrozenPassed,
        }))
    }
}

/// Нелинейный набор кодовых слов: не содержит нулевого слова и не замкнут по XOR.
pub const TABLE: [[u8; 3]; 4] = [[0, 0, 1], [0, 1, 0], [1, 0, 0], [1, 1, 1]];
pub struct Nonlinear;
impl Encoder<u8> for Nonlinear {
    type Output = Codeword<[u8; 3]>;
    type Error = Error;
    type Event = ();

    fn encode(&mut self, input: u8, _: &mut dyn Observer<()>) -> Result<Self::Output, Error> {
        TABLE
            .get(usize::from(input))
            .copied()
            .map(Codeword)
            .ok_or(Error::Value)
    }
}
impl Decoder<Codeword<[u8; 3]>> for Nonlinear {
    type Value = Message<u8>;
    type Checks = Checks;
    type NeedMore = ();
    type StopReason = Stop;
    type Error = Error;
    type Event = ();

    fn decode(
        &mut self,
        input: Codeword<[u8; 3]>,
        _: &mut dyn Observer<()>,
    ) -> Result<DecodeOutcome<Message<u8>, Checks, (), Stop>, Error> {
        if input.0.iter().any(|&b| b > 1) {
            return Err(Error::Value);
        }
        Ok(match TABLE.iter().position(|&w| w == input.0) {
            Some(index) => DecodeOutcome::Candidate(Candidate {
                value: Message(index as u8),
                checks: Checks::TableMember,
            }),
            None => DecodeOutcome::Stopped(Stop::ParityFailed),
        })
    }
}
