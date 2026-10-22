use super::SpaState;
use crate::Bit;

#[derive(Debug, PartialEq, Eq)]
/// Snapshot of semantic state; the validation-only erasure mask is excluded.
pub(super) struct StateSnapshot {
    channel: Vec<u64>,
    q: Vec<u64>,
    r: Vec<u64>,
    posterior: Vec<u64>,
    word: Vec<Bit>,
    tanh: Vec<u64>,
    prefix: Vec<u64>,
    suffix: Vec<u64>,
}

impl SpaState {
    pub(super) fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            channel: float_bits(&self.channel),
            q: float_bits(&self.q),
            r: float_bits(&self.r),
            posterior: float_bits(&self.posterior),
            word: self.word.clone(),
            tanh: float_bits(&self.tanh),
            prefix: float_bits(&self.prefix),
            suffix: float_bits(&self.suffix),
        }
    }
}

pub(super) fn float_bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}
