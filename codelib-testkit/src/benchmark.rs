use crate::measure::{bits_per_second, measure, Counts, Observation};

pub(crate) trait BenchmarkCase {
    type Trial;
    type Encoded;
    type EncodeError;
    type Input;
    type Decoded;
    type DecodeError;

    fn payload_bits(&self) -> usize;
    fn codeword_len(&self) -> usize;
    fn error_unit(&self) -> &'static str;
    fn encode(&mut self, trial: &mut Self::Trial) -> Result<Self::Encoded, Self::EncodeError>;
    fn decode(&mut self, input: &mut Self::Input) -> Result<Self::Decoded, Self::DecodeError>;
    fn channel(
        &self,
        trial: &Self::Trial,
        encoded: &Result<Self::Encoded, Self::EncodeError>,
    ) -> Result<Self::Input, String>;
    fn observe(
        &self,
        trial: &Self::Trial,
        encoded: &Result<Self::Encoded, Self::EncodeError>,
        input: &Self::Input,
        decoded: &Result<Self::Decoded, Self::DecodeError>,
    ) -> Result<Observation, String>;
}

#[derive(Debug)]
pub struct BenchmarkReport {
    pub counts: Counts,
    pub error_unit: &'static str,
    pub codeword_len: usize,
    pub payload_bits: usize,
    pub mean_iterations: Option<f64>,
    pub encode_bps: Option<f64>,
    pub decode_bps: Option<f64>,
}

impl BenchmarkReport {
    pub fn block_rate(&self, count: usize) -> Option<f64> {
        (self.counts.blocks > 0).then(|| count as f64 / self.counts.blocks as f64)
    }

    pub fn error_rate(&self, count: Option<usize>) -> Option<f64> {
        let count = count?;
        (self.counts.blocks > 0 && self.codeword_len > 0)
            .then(|| count as f64 / (self.counts.blocks as f64 * self.codeword_len as f64))
    }
}

/// Untimed API and observation checks for clean controls and warmup.
pub fn check<C: BenchmarkCase>(case: &mut C, trial: &mut C::Trial) -> Result<(), String> {
    let encoded = case.encode(trial);
    let mut input = case.channel(trial, &encoded)?;
    let decoded = case.decode(&mut input);
    case.observe(trial, &encoded, &input, &decoded)
        .and_then(|o| o.validate(case.codeword_len()))
        .map(|_| ())
}

pub fn run<C: BenchmarkCase>(
    case: &mut C,
    trials: &mut [C::Trial],
    warmup: usize,
) -> Result<BenchmarkReport, String> {
    let calls = trials.len();
    if calls == 0 {
        return Err("benchmark corpus must not be empty".into());
    }
    let payload_bits = case.payload_bits();
    for index in 0..warmup {
        check(case, &mut trials[index % calls]).map_err(|e| format!("warmup {index}: {e}"))?;
    }

    let (encoded, encode_time) = measure(trials, |trial| case.encode(trial));
    let mut inputs: Vec<_> = trials
        .iter()
        .zip(&encoded)
        .enumerate()
        .map(|(index, (trial, word))| {
            case.channel(trial, word)
                .map_err(|e| format!("sample {index}: {e}"))
        })
        .collect::<Result<_, _>>()?;
    let (decoded, decode_time) = measure(&mut inputs, |input| case.decode(input));
    let mut counts = Counts::default();
    let mut iterations = Some(0.0);
    for (index, result) in decoded.iter().enumerate() {
        let observed = case
            .observe(&trials[index], &encoded[index], &inputs[index], result)
            .and_then(|o| o.validate(case.codeword_len()))
            .map_err(|e| format!("sample {index}: {e}"))?;
        log::debug!(
            "sample {index}: recovered={}, word_errors={:?}",
            observed.recovered,
            observed.after
        );
        iterations = iterations
            .zip(observed.iterations)
            .map(|(sum, value)| sum + value as f64);
        counts.add(observed);
    }
    Ok(BenchmarkReport {
        counts,
        error_unit: case.error_unit(),
        codeword_len: case.codeword_len(),
        payload_bits,
        mean_iterations: iterations.map(|sum| sum / calls as f64),
        encode_bps: bits_per_second(payload_bits, calls, encode_time),
        decode_bps: bits_per_second(payload_bits, calls, decode_time),
    })
}
