//! Тестовый кодек с буферами вызывающего кода и символами Gf256.

use coding_core::{
    Candidate, DecodeOutcome, Decoder, Encoder, Message, MessageBytes, Observer, SizeError,
};
use gf2m::Gf256;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Size(SizeError),
    Shape,
}

pub struct EncodeInto<'a, 'b> {
    pub message: &'a [Gf256],
    pub output: &'b mut [Gf256],
    pub scratch: &'b mut [Gf256],
}
pub struct FieldCodec;

impl<'a, 'b> Encoder<EncodeInto<'a, 'b>> for FieldCodec {
    type Output = &'b mut [Gf256];
    type Error = Error;
    type Event = ();

    fn encode(
        &mut self,
        input: EncodeInto<'a, 'b>,
        _: &mut dyn Observer<()>,
    ) -> Result<Self::Output, Error> {
        // Символ Gf256 представляет один передаваемый байт, хотя занимает два в памяти.
        MessageBytes::try_new(input.message.len()).map_err(Error::Size)?;
        let required = input
            .message
            .len()
            .checked_add(1)
            .ok_or(Error::Size(SizeError::Overflow))?;
        if input.output.len() != required || input.scratch.len() != 1 {
            return Err(Error::Shape);
        }
        input.scratch[0] = input
            .message
            .iter()
            .copied()
            .fold(Gf256::new(0), |a, b| a + b);
        input.output[..input.message.len()].copy_from_slice(input.message);
        input.output[input.message.len()] = input.scratch[0];
        Ok(input.output)
    }
}

pub struct DecodeInto<'a, 'b> {
    pub word: &'a [Gf256],
    pub original_len: usize,
    pub output: &'b mut [Gf256],
}
impl<'a, 'b> Decoder<DecodeInto<'a, 'b>> for FieldCodec {
    type Value = Message<&'b mut [Gf256]>;
    type Checks = ();
    type NeedMore = ();
    type StopReason = ();
    type Error = Error;
    type Event = ();

    fn decode(
        &mut self,
        input: DecodeInto<'a, 'b>,
        _: &mut dyn Observer<()>,
    ) -> Result<DecodeOutcome<Self::Value, (), (), ()>, Error> {
        MessageBytes::try_new(input.original_len).map_err(Error::Size)?;
        let required = input
            .original_len
            .checked_add(1)
            .ok_or(Error::Size(SizeError::Overflow))?;
        if input.word.len() != required || input.output.len() != input.original_len {
            return Err(Error::Shape);
        }
        if input.word.iter().copied().fold(Gf256::new(0), |a, b| a + b) != Gf256::new(0) {
            return Ok(DecodeOutcome::Stopped(()));
        }
        input
            .output
            .copy_from_slice(&input.word[..input.original_len]);
        Ok(DecodeOutcome::Candidate(Candidate {
            value: Message(input.output),
            checks: (),
        }))
    }
}
