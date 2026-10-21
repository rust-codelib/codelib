use std::{borrow::Cow, convert::Infallible};

use codelib_testkit::{
    candidate, report::PointReport, BlockLayout, BlockScenario, Channel, CodecOps, Decoded,
};
use coding_core::{
    Candidate, Codeword, DecodeOutcome, Decoder as CoreDecoder, Encoder as CoreEncoder,
    NoopObserver, Observer,
};
use ldpc_codes::{
    Bit, ConfiguredLdpc, DecodeInput, DecodeResult, DecodeStatus, Encoder, LdpcConfigurator,
    LdpcError, ParityCheckMatrix,
};
use rand::Rng;

use crate::{
    config::{Config, LdpcConfig},
    smoke::check,
};

pub const N: usize = 128;

// Delete this local bridge when ldpc-codes implements coding-core natively.
struct LegacyLdpc(ConfiguredLdpc);

type Outcome = DecodeOutcome<DecodeResult, DecodeStatus, Infallible, Infallible>;

impl CoreEncoder<&[Bit]> for LegacyLdpc {
    type Output = Codeword<Vec<Bit>>;
    type Error = LdpcError;
    type Event = ();

    fn encode(
        &mut self,
        message: &[Bit],
        _: &mut dyn Observer<()>,
    ) -> Result<Self::Output, LdpcError> {
        self.0.encoder().encode(message).map(Codeword)
    }
}

impl CoreDecoder<DecodeInput<'_>> for LegacyLdpc {
    type Value = DecodeResult;
    type Checks = DecodeStatus;
    type NeedMore = Infallible;
    type StopReason = Infallible;
    type Error = LdpcError;
    type Event = ();

    fn decode(
        &mut self,
        input: DecodeInput<'_>,
        _: &mut dyn Observer<()>,
    ) -> Result<Outcome, LdpcError> {
        // An iteration limit still yields a word, which is needed for BER.
        self.0.decode(input, None).map(|value| {
            DecodeOutcome::Candidate(Candidate {
                checks: value.status(),
                value,
            })
        })
    }
}

struct Ldpc<'a> {
    code: LegacyLdpc,
    checks: ParityCheckMatrix,
    config: &'a LdpcConfig,
}

pub fn run(config: &Config) -> Result<Vec<PointReport>, String> {
    let rows = (0..64).map(|i| vec![i, (i + 1) % 64, 64 + i]).collect();
    let checks = ParityCheckMatrix::try_from_rows(N, rows).map_err(|e| e.to_string())?;
    run_code(config, checks)
}

fn run_code(config: &Config, checks: ParityCheckMatrix) -> Result<Vec<PointReport>, String> {
    config.ldpc.validate(checks.cols())?;
    let code = LdpcConfigurator::build(checks.clone(), config.ldpc.decoder()?)
        .map_err(|e| e.to_string())?;
    let k = code.encoder().message_len();
    let layout = BlockLayout {
        message_len: k,
        codeword_len: checks.cols(),
        payload_bits: k,
        error_unit: "bits",
    };
    log::info!(
        "LDPC n={}, k={k}, repetitions={}",
        checks.cols(),
        config.repetitions
    );
    let mut scenario = BlockScenario::new(
        Ldpc {
            code: LegacyLdpc(code),
            checks,
            config: &config.ldpc,
        },
        layout,
        Channel::BitFlips(|bit| {
            if *bit == Bit::Zero {
                Bit::One
            } else {
                Bit::Zero
            }
        }),
        CodecOps {
            symbol: |rng| {
                if rng.random::<bool>() {
                    Bit::One
                } else {
                    Bit::Zero
                }
            },
            buffers: |_, _| (),
            encode: |c, trial| {
                CoreEncoder::encode(&mut c.code, trial.message.as_slice(), &mut NoopObserver)
            },
            encoded: |c, trial, encoded| {
                let word = &encoded.as_ref().map_err(ToString::to_string)?.0;
                check(
                    c.checks.is_codeword(word).map_err(|e| e.to_string())?
                        && c.code
                            .0
                            .encoder()
                            .extract_message(word)
                            .map_err(|e| e.to_string())?
                            == trial.message,
                    "encoder produced an invalid word or changed the message",
                )?;
                Ok(word)
            },
            prepare: |c, _, _, received| {
                received
                    .iter()
                    .map(|bit| match bit {
                        Some(Bit::Zero) => Ok(c.config.llr_magnitude),
                        Some(Bit::One) => Ok(-c.config.llr_magnitude),
                        None => Err("LDPC bit-flip input contains an erasure".into()),
                    })
                    .collect::<Result<Vec<_>, String>>()
            },
            decode: |c, llrs| {
                CoreDecoder::decode(
                    &mut c.code,
                    DecodeInput {
                        llrs,
                        erasures: &[],
                    },
                    &mut NoopObserver,
                )
            },
            decoded: |c, _, decoded| {
                let Some(candidate) = candidate(decoded)? else {
                    return Ok(None);
                };
                let result = &candidate.value;
                let valid = c
                    .checks
                    .is_codeword(result.word())
                    .map_err(|e| e.to_string())?;
                check(
                    (result.status() == DecodeStatus::ParitySatisfied) == valid
                        && result.iterations() <= c.config.max_iterations,
                    "inconsistent decoder status or iterations",
                )?;
                let message = if valid {
                    Some(Cow::Owned(
                        c.code
                            .0
                            .encoder()
                            .extract_message(result.word())
                            .map_err(|e| e.to_string())?,
                    ))
                } else {
                    None
                };
                Ok(Some(Decoded {
                    word: Some(result.word()),
                    message,
                    iterations: Some(result.iterations()),
                }))
            },
        },
    );
    scenario.run(
        "ldpc-codes",
        config.seed,
        config.repetitions,
        config.warmup,
        &config.ldpc.error_counts.points(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repetition_code_has_known_results_without_iterations() {
        let mut config = Config {
            warmup: 2,
            repetitions: 8,
            ..Config::default()
        };
        config.ldpc.error_counts = crate::config::ChannelCounts::Many(vec![3, 1, 0, 2]);
        config.ldpc.max_iterations = 0;
        let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).unwrap();
        let points = run_code(&config, checks).unwrap();
        assert_eq!(points.len(), 4);
        for (errors, point) in points.iter().enumerate() {
            assert_eq!(point.error_count, Some(errors));
            assert_eq!(point.status, "passed");
            assert_eq!(point.completed_trials, Some(8));
            assert_eq!(
                point.recovered_blocks,
                Some(if errors == 0 { 8 } else { 0 })
            );
            assert_eq!(
                (point.errors_before, point.residual),
                (Some(8 * errors), Some(8 * errors))
            );
            assert_eq!((point.corrected, point.introduced), (Some(0), Some(0)));
            assert_eq!(point.error_rate_after, Some(errors as f64 / 3.0));
            assert_eq!(point.fer, Some(if errors == 0 { 0.0 } else { 1.0 }));
            assert_eq!(point.mean_iterations, Some(0.0));
        }
    }
}
