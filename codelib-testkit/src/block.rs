use std::{borrow::Cow, fmt::Display};

use coding_core::{Candidate, DecodeOutcome};
use rand::{seq::SliceRandom, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    benchmark::{self, BenchmarkCase},
    measure::{count_errors, Observation},
    report::{self, ChannelKind, PointReport},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Trial<M, B = ()> {
    pub message: Vec<M>,
    pub positions: Vec<usize>,
    pub buffers: B,
}

pub struct BlockLayout {
    pub message_len: usize,
    pub codeword_len: usize,
    pub payload_bits: usize,
    pub error_unit: &'static str,
}

pub enum Channel<W> {
    BitFlips(fn(&W) -> W),
    Erasures,
}

impl<W: Clone> Channel<W> {
    fn kind(&self) -> ChannelKind {
        match self {
            Self::BitFlips(_) => ChannelKind::BitFlips,
            Self::Erasures => ChannelKind::Erasures,
        }
    }

    fn apply(&self, word: &[W], positions: &[usize], count: usize) -> Vec<Option<W>> {
        let mut received: Vec<_> = word.iter().cloned().map(Some).collect();
        for &index in &positions[..count] {
            received[index] = match self {
                Self::BitFlips(flip) => Some(flip(&word[index])),
                Self::Erasures => None,
            };
        }
        received
    }
}

/// Comparable data, borrowed from API results/buffers or extracted after timing.
pub struct Decoded<'a, M: Clone, W> {
    pub word: Option<&'a [W]>,
    pub message: Option<Cow<'a, [M]>>,
    pub iterations: Option<usize>,
}

/// Interpret coding-core outcomes without treating ordinary non-recovery as Err.
pub fn candidate<V, C, N, S, E: Display>(
    result: &Result<DecodeOutcome<V, C, N, S>, E>,
) -> Result<Option<&Candidate<V, C>>, String> {
    match result.as_ref().map_err(ToString::to_string)? {
        DecodeOutcome::Candidate(value) => Ok(Some(value)),
        DecodeOutcome::NeedMore(_) | DecodeOutcome::Stopped(_) => Ok(None),
    }
}

/// Only encode/decode run under the clock. Keep native results until projection.
/// Borrowed API returns can be reduced to metadata while data stays in buffers.
pub struct CodecOps<C, M: Clone, W, B, E, I, D> {
    pub symbol: fn(&mut ChaCha8Rng) -> M,
    pub buffers: fn(&C, &[M]) -> B,
    pub encode: fn(&mut C, &mut Trial<M, B>) -> E,
    pub encoded: EncodedViewFn<C, M, W, B, E>,
    pub prepare: PrepareFn<C, M, W, B, E, I>,
    pub decode: fn(&mut C, &mut I) -> D,
    pub decoded: DecodedViewFn<C, M, W, I, D>,
}

type EncodedViewFn<C, M, W, B, E> =
    for<'a> fn(&C, &'a Trial<M, B>, &'a E) -> Result<&'a [W], String>;
type PrepareFn<C, M, W, B, E, I> = fn(&C, &Trial<M, B>, &E, &[Option<W>]) -> Result<I, String>;
type DecodedViewFn<C, M, W, I, D> =
    for<'a> fn(&C, &'a I, &'a D) -> Result<Option<Decoded<'a, M, W>>, String>;

pub struct BlockScenario<C, M: Clone, W, B, E, I, D> {
    pub codec: C,
    layout: BlockLayout,
    channel: Channel<W>,
    ops: CodecOps<C, M, W, B, E, I, D>,
    count: usize,
}

pub(crate) struct Prepared<W, I> {
    received: Vec<Option<W>>,
    native: I,
}

impl<C, M: Clone + Eq, W: Clone + Eq, B, E, I, D> BlockScenario<C, M, W, B, E, I, D> {
    pub fn new(
        codec: C,
        layout: BlockLayout,
        channel: Channel<W>,
        ops: CodecOps<C, M, W, B, E, I, D>,
    ) -> Self {
        Self {
            codec,
            layout,
            channel,
            ops,
            count: 0,
        }
    }

    fn corpus(&self, seed: u64, repetitions: usize) -> Vec<Trial<M, B>> {
        let mut root = ChaCha8Rng::seed_from_u64(seed);
        let mut messages = ChaCha8Rng::seed_from_u64(root.next_u64());
        let mut positions = ChaCha8Rng::seed_from_u64(root.next_u64());
        (0..repetitions)
            .map(|_| {
                let message: Vec<_> = (0..self.layout.message_len)
                    .map(|_| (self.ops.symbol)(&mut messages))
                    .collect();
                let mut permutation: Vec<_> = (0..self.layout.codeword_len).collect();
                permutation.shuffle(&mut positions);
                let buffers = (self.ops.buffers)(&self.codec, &message);
                Trial {
                    message,
                    positions: permutation,
                    buffers,
                }
            })
            .collect()
    }
}

impl<C, M: Clone + Eq, W: Clone + Eq, B, E, EE, I, D, DE>
    BlockScenario<C, M, W, B, Result<E, EE>, I, Result<D, DE>>
{
    pub fn run(
        &mut self,
        codec: &str,
        seed: u64,
        repetitions: usize,
        warmup: usize,
        points: &[usize],
    ) -> Result<Vec<PointReport>, String> {
        let mut points = points.to_vec();
        points.sort_unstable();
        if repetitions == 0
            || self.layout.codeword_len == 0
            || points.is_empty()
            || points.iter().any(|&count| count > self.layout.codeword_len)
            || points.windows(2).any(|p| p[0] == p[1])
        {
            return Err(
                "expected positive trials/word length and distinct channel counts within 0..=n"
                    .into(),
            );
        }
        repetitions
            .checked_mul(points.len())
            .and_then(|_| repetitions.checked_mul(self.layout.codeword_len))
            .ok_or("trial counters overflow")?;
        let mut trials = self.corpus(seed, repetitions);
        self.count = 0;
        benchmark::check(self, &mut trials[0]).map_err(|e| format!("clean control: {e}"))?;
        Ok(report::series(
            codec,
            self.channel.kind(),
            &points,
            repetitions,
            |count| {
                self.count = count;
                log::info!("{codec} {:?}={count}", self.channel.kind());
                benchmark::run(self, &mut trials, warmup)
            },
        ))
    }
}

impl<C, M: Clone + Eq, W: Clone + Eq, B, E, EE, I, D, DE> BenchmarkCase
    for BlockScenario<C, M, W, B, Result<E, EE>, I, Result<D, DE>>
{
    type Trial = Trial<M, B>;
    type Encoded = E;
    type EncodeError = EE;
    type Input = Prepared<W, I>;
    type Decoded = D;
    type DecodeError = DE;

    fn payload_bits(&self) -> usize {
        self.layout.payload_bits
    }
    fn codeword_len(&self) -> usize {
        self.layout.codeword_len
    }
    fn error_unit(&self) -> &'static str {
        self.layout.error_unit
    }

    fn encode(&mut self, trial: &mut Self::Trial) -> Result<E, EE> {
        (self.ops.encode)(&mut self.codec, trial)
    }

    fn channel(&self, trial: &Self::Trial, encoded: &Result<E, EE>) -> Result<Self::Input, String> {
        let word = (self.ops.encoded)(&self.codec, trial, encoded)?;
        if word.len() != self.layout.codeword_len {
            return Err("encoded word has incorrect length".into());
        }
        let received = self.channel.apply(word, &trial.positions, self.count);
        if word
            .iter()
            .zip(&received)
            .filter(|(a, b)| b.as_ref() != Some(a))
            .count()
            != self.count
        {
            return Err("incorrect number of injected errors".into());
        }
        let native = (self.ops.prepare)(&self.codec, trial, encoded, &received)?;
        Ok(Prepared { received, native })
    }

    fn decode(&mut self, input: &mut Self::Input) -> Result<D, DE> {
        (self.ops.decode)(&mut self.codec, &mut input.native)
    }

    fn observe(
        &self,
        trial: &Self::Trial,
        encoded: &Result<E, EE>,
        input: &Self::Input,
        decoded: &Result<D, DE>,
    ) -> Result<Observation, String> {
        let mut observation = Observation {
            before: matches!(self.channel, Channel::BitFlips(_)).then_some(self.count),
            ..Default::default()
        };
        if let Some(view) = (self.ops.decoded)(&self.codec, &input.native, decoded)? {
            let word_recovered = if let Some(word) = view.word {
                if word.len() != self.layout.codeword_len {
                    return Err("decoded word has incorrect length".into());
                }
                let reference = (self.ops.encoded)(&self.codec, trial, encoded)?;
                if matches!(self.channel, Channel::BitFlips(_)) {
                    let received: Vec<_> = input.received.iter().flatten().cloned().collect();
                    observation = count_errors(reference, &received, word)?;
                }
                Some(reference == word)
            } else {
                None
            };
            let message_recovered = if let Some(message) = view.message {
                if message.len() != trial.message.len() {
                    return Err("decoded message has incorrect length".into());
                }
                Some(message.as_ref() == trial.message)
            } else {
                None
            };
            if word_recovered
                .zip(message_recovered)
                .is_some_and(|(w, m)| w != m)
            {
                return Err("word and message recovery disagree".into());
            }
            observation.recovered = word_recovered
                .or(message_recovered)
                .ok_or("candidate contains neither word nor message")?;
            observation.iterations = view.iterations;
        }
        if self.count == 0 && !observation.recovered {
            return Err("clean block was not recovered".into());
        }
        Ok(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_core::{Codeword, Decoder, Encoder, NoopObserver, Observer};
    use rand::Rng;

    type Outcome = DecodeOutcome<(), usize, (), ()>;
    type ApiResult<T> = Result<T, &'static str>;
    type Case = BlockScenario<Repeat, u8, u8, [u8; 3], ApiResult<()>, Input, ApiResult<Outcome>>;

    struct Input {
        received: Vec<Option<u8>>,
        decoded: [u8; 3],
    }

    #[derive(Default)]
    struct Repeat {
        calls: [usize; 2],
        mode: u8,
    }

    impl<'a> Encoder<(&[u8], &'a mut [u8; 3])> for Repeat {
        type Output = Codeword<&'a mut [u8]>;
        type Error = &'static str;
        type Event = ();

        fn encode(
            &mut self,
            (message, buffer): (&[u8], &'a mut [u8; 3]),
            _: &mut dyn Observer<()>,
        ) -> Result<Self::Output, Self::Error> {
            self.calls[0] += 1;
            buffer.fill(message[0]);
            Ok(Codeword(buffer))
        }
    }

    impl<'a> Decoder<&'a mut Input> for Repeat {
        type Value = Codeword<&'a mut [u8]>;
        type Checks = usize;
        type NeedMore = ();
        type StopReason = ();
        type Error = &'static str;
        type Event = ();

        fn decode(
            &mut self,
            input: &'a mut Input,
            _: &mut dyn Observer<()>,
        ) -> Result<DecodeOutcome<Self::Value, usize, (), ()>, Self::Error> {
            self.calls[1] += 1;
            let zeros = input.received.iter().filter(|&&x| x == Some(0)).count();
            let ones = input.received.iter().filter(|&&x| x == Some(1)).count();
            if self.mode == 4 || zeros + ones == 0 {
                return Ok(DecodeOutcome::NeedMore(()));
            }
            if zeros != 3 && ones != 3 {
                match self.mode {
                    1 => return Ok(DecodeOutcome::NeedMore(())),
                    2 => return Ok(DecodeOutcome::Stopped(())),
                    3 => return Err("native decode failure"),
                    _ => {}
                }
            }
            input.decoded.fill(u8::from(ones > zeros));
            Ok(DecodeOutcome::Candidate(Candidate {
                value: Codeword(&mut input.decoded),
                checks: 1,
            }))
        }
    }

    fn repeat(channel: Channel<u8>) -> Case {
        BlockScenario::new(
            Repeat::default(),
            BlockLayout {
                message_len: 1,
                codeword_len: 3,
                payload_bits: 1,
                error_unit: "bits",
            },
            channel,
            CodecOps {
                symbol: |rng| u8::from(rng.random::<bool>()),
                buffers: |_, _| [9; 3],
                encode: |c, trial| {
                    Encoder::encode(
                        c,
                        (trial.message.as_slice(), &mut trial.buffers),
                        &mut NoopObserver,
                    )
                    .map(|_| ())
                },
                encoded: |_, trial, encoded| {
                    encoded.as_ref().map_err(ToString::to_string)?;
                    Ok(&trial.buffers)
                },
                prepare: |_, _, _, received| {
                    Ok(Input {
                        received: received.to_vec(),
                        decoded: [9; 3],
                    })
                },
                decode: |c, input| {
                    Decoder::decode(c, input, &mut NoopObserver).map(|outcome| match outcome {
                        DecodeOutcome::Candidate(c) => DecodeOutcome::Candidate(Candidate {
                            value: (),
                            checks: c.checks,
                        }),
                        DecodeOutcome::NeedMore(n) => DecodeOutcome::NeedMore(n),
                        DecodeOutcome::Stopped(s) => DecodeOutcome::Stopped(s),
                    })
                },
                decoded: |_, input, decoded| {
                    Ok(candidate(decoded)?.map(|c| Decoded {
                        word: Some(&input.decoded),
                        message: Some(Cow::Borrowed(&input.decoded[..1])),
                        iterations: Some(c.checks),
                    }))
                },
            },
        )
    }

    #[test]
    fn corpus_and_both_channels_preserve_reproducibility_and_distinct_positions() {
        let mut case = repeat(Channel::BitFlips(|b| b ^ 1));
        case.layout.message_len = 6;
        case.layout.codeword_len = 12;
        let trials = case.corpus(42, 4);
        assert_eq!(trials, case.corpus(42, 4));
        assert_eq!(trials, case.corpus(42, 8)[..4]);
        assert_ne!(trials, case.corpus(43, 4));
        for trial in &trials {
            let mut sorted = trial.positions.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..12).collect::<Vec<_>>());
            for channel in [Channel::BitFlips(|b| b ^ 1), Channel::Erasures] {
                for count in 0..=12 {
                    let received = channel.apply(&[0; 12], &trial.positions, count);
                    for (index, symbol) in received.iter().enumerate() {
                        assert_eq!(
                            *symbol == Some(0),
                            !trial.positions[..count].contains(&index)
                        );
                    }
                }
            }
        }
        case.layout.codeword_len = 3;
        for points in [&[][..], &[1, 1], &[4]] {
            assert!(case.run("repeat", 42, 4, 0, points).is_err());
        }
        assert!(case.run("repeat", 42, 0, 0, &[0]).is_err());
    }

    #[test]
    fn outcomes_and_failures_have_shared_statistics_and_exclude_warmup() {
        for warmup in [0, 3] {
            let mut case = repeat(Channel::BitFlips(|b| b ^ 1));
            let points = case.run("repeat", 42, 4, warmup, &[3, 1, 0, 2]).unwrap();
            assert_eq!(case.codec.calls, [1 + 4 * (4 + warmup); 2]);
            for (count, point) in points.iter().enumerate() {
                assert_eq!(point.error_count, Some(count));
                assert_eq!(point.completed_trials, Some(4));
                assert_eq!(point.status, "passed");
                assert_eq!(point.fer, Some(if count < 2 { 0.0 } else { 1.0 }));
                assert_eq!(point.error_rate_before, Some(count as f64 / 3.0));
                assert_eq!(
                    point.error_rate_after,
                    Some(if count < 2 { 0.0 } else { 1.0 })
                );
            }
            let erased = repeat(Channel::Erasures)
                .run("repeat", 42, 4, warmup, &[0, 1, 3])
                .unwrap();
            for (point, count) in erased.iter().zip([0, 1, 3]) {
                assert_eq!(point.error_count, None);
                assert_eq!(point.erasure_count, Some(count));
                assert_eq!(point.fer, Some(if count < 3 { 0.0 } else { 1.0 }));
                assert_eq!(
                    (point.errors_before, point.residual, point.error_rate_after),
                    (None, None, None)
                );
            }
        }
        for mode in 1..=3 {
            let mut case = repeat(Channel::BitFlips(|b| b ^ 1));
            case.codec.mode = mode;
            let points = case.run("repeat", 42, 4, 0, &[0, 1, 2]).unwrap();
            if mode == 3 {
                assert_eq!(points.len(), 2);
                assert_eq!(points[1].status, "failed");
                assert_eq!(
                    points[1].error.as_deref(),
                    Some("sample 0: native decode failure")
                );
                assert_eq!(points[1].completed_trials, None);
                assert_eq!(
                    case.run("repeat", 42, 4, 1, &[1]).unwrap()[0]
                        .error
                        .as_deref(),
                    Some("warmup 0: native decode failure")
                );
            } else {
                assert_eq!(points[1].completed_trials, Some(4));
                assert_eq!(points[1].fer, Some(1.0));
                assert_eq!(
                    (points[1].error_rate_after, points[1].mean_iterations),
                    (None, None)
                );
            }
        }
        let mut case = repeat(Channel::BitFlips(|b| b ^ 1));
        case.codec.mode = 4;
        assert_eq!(
            case.run("repeat", 42, 4, 0, &[1]).unwrap_err(),
            "clean control: clean block was not recovered"
        );
    }

    #[test]
    fn borrowed_api_results_stay_in_caller_buffers_without_timed_copies() {
        let mut case = repeat(Channel::BitFlips(|b| b ^ 1));
        case.ops.decoded = |_, input, decoded| {
            Ok(candidate(decoded)?.map(|c| Decoded {
                word: None,
                message: Some(Cow::Borrowed(&input.decoded[..1])),
                iterations: Some(c.checks),
            }))
        };
        let mut trials = case.corpus(42, 2);
        case.count = 1;
        let report = benchmark::run(&mut case, &mut trials, 3).unwrap();
        assert_eq!(report.mean_iterations, Some(1.0));
        assert_eq!((report.counts.blocks, report.counts.recovered), (2, 2));
        assert_eq!(report.counts.before, Some(2));
        assert_eq!(report.counts.after, None);
        assert!(trials.iter().all(|t| t.buffers == [t.message[0]; 3]));
    }
}
