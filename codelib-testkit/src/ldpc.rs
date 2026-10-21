use ldpc_codes::{
    Bit, ConfiguredLdpc, DecodeInput, DecodeResult, DecodeStatus, Encoder, LdpcConfigurator,
    ParityCheckMatrix,
};
use rand::{seq::SliceRandom, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    config::Config,
    measure::{bits_per_second, count_errors, measure, Counts},
    smoke::check,
};

pub const N: usize = 128;

pub struct Report {
    pub counts: Counts,
    pub encode_bps: Option<f64>,
    pub decode_bps: Option<f64>,
}

#[derive(Debug, PartialEq)]
struct Trial {
    message: Vec<Bit>,
    positions: Vec<usize>,
}

fn corpus(seed: u64, n: usize, k: usize, repetitions: usize) -> Vec<Trial> {
    let mut root = ChaCha8Rng::seed_from_u64(seed);
    let mut messages = ChaCha8Rng::seed_from_u64(root.next_u64());
    let mut positions = ChaCha8Rng::seed_from_u64(root.next_u64());
    (0..repetitions)
        .map(|_| {
            let message = (0..k)
                .map(|_| {
                    if messages.random::<bool>() {
                        Bit::One
                    } else {
                        Bit::Zero
                    }
                })
                .collect();
            let mut permutation: Vec<_> = (0..n).collect();
            permutation.shuffle(&mut positions);
            Trial {
                message,
                positions: permutation,
            }
        })
        .collect()
}

fn channel(word: &[Bit], positions: &[usize], errors: usize, magnitude: f64) -> Vec<f64> {
    let mut llrs: Vec<_> = word
        .iter()
        .map(|&b| {
            if b == Bit::Zero {
                magnitude
            } else {
                -magnitude
            }
        })
        .collect();
    for &position in &positions[..errors] {
        llrs[position] = -llrs[position];
    }
    llrs
}

fn input(llrs: &[f64]) -> DecodeInput<'_> {
    DecodeInput {
        llrs,
        erasures: &[],
    }
}

fn validate_encoded(
    code: &ConfiguredLdpc,
    checks: &ParityCheckMatrix,
    trial: &Trial,
    word: &[Bit],
) -> Result<(), String> {
    check(
        checks.is_codeword(word).map_err(|e| e.to_string())?
            && code
                .encoder()
                .extract_message(word)
                .map_err(|e| e.to_string())?
                == trial.message,
        "encoder produced an invalid word or changed the message",
    )
}

fn observe(
    code: &ConfiguredLdpc,
    checks: &ParityCheckMatrix,
    trial: &Trial,
    reference: &[Bit],
    llrs: &[f64],
    result: &DecodeResult,
    config: &Config,
) -> Result<Counts, String> {
    let valid = checks
        .is_codeword(result.word())
        .map_err(|e| e.to_string())?;
    check(
        (result.status() == DecodeStatus::ParitySatisfied) == valid
            && result.iterations() <= config.ldpc.max_iterations,
        "inconsistent decoder status or iterations",
    )?;
    let received: Vec<_> = llrs
        .iter()
        .map(|&x| if x >= 0.0 { Bit::Zero } else { Bit::One })
        .collect();
    let counts = count_errors(reference, &received, result.word())?;
    check(
        counts.before == config.ldpc.errors,
        "incorrect number of injected errors",
    )?;
    if valid {
        let recovered = code
            .encoder()
            .extract_message(result.word())
            .map_err(|e| e.to_string())?
            == trial.message;
        check(
            recovered == (counts.recovered == 1),
            "word and message recovery disagree",
        )?;
    }
    check(
        config.ldpc.errors != 0 || counts.recovered == 1,
        "clean block was not recovered",
    )?;
    Ok(counts)
}

pub fn run(config: &Config) -> Result<Report, String> {
    let rows = (0..64).map(|i| vec![i, (i + 1) % 64, 64 + i]).collect();
    let checks = ParityCheckMatrix::try_from_rows(N, rows).map_err(|e| e.to_string())?;
    run_code(config, checks)
}

fn run_code(config: &Config, checks: ParityCheckMatrix) -> Result<Report, String> {
    config.ldpc.validate(checks.cols())?;
    let mut code = LdpcConfigurator::build(checks.clone(), config.ldpc.decoder()?)
        .map_err(|e| e.to_string())?;
    let k = code.encoder().message_len();
    let mut corpus = corpus(config.seed, checks.cols(), k, config.repetitions);
    log::info!(
        "LDPC n={}, k={k}, errors={}, repetitions={}",
        checks.cols(),
        config.ldpc.errors,
        config.repetitions
    );

    // A clean control block must work even when the measured channel is noisy.
    let control = code
        .encoder()
        .encode(&corpus[0].message)
        .map_err(|e| e.to_string())?;
    validate_encoded(&code, &checks, &corpus[0], &control)?;
    let clean = channel(&control, &[], 0, config.ldpc.llr_magnitude);
    let decoded = code
        .decode(input(&clean), None)
        .map_err(|e| e.to_string())?;
    check(
        decoded.word() == control
            && code
                .encoder()
                .extract_message(decoded.word())
                .map_err(|e| e.to_string())?
                == corpus[0].message,
        "clean control block was not recovered",
    )?;

    for index in 0..config.warmup {
        let trial = &corpus[index % corpus.len()];
        let word = code
            .encoder()
            .encode(&trial.message)
            .map_err(|e| e.to_string())?;
        validate_encoded(&code, &checks, trial, &word)?;
        let llrs = channel(
            &word,
            &trial.positions,
            config.ldpc.errors,
            config.ldpc.llr_magnitude,
        );
        let result = code.decode(input(&llrs), None).map_err(|e| e.to_string())?;
        observe(&code, &checks, trial, &word, &llrs, &result, config)?;
    }

    let (encoded, encode_time) =
        measure(&mut corpus, |trial| code.encoder().encode(&trial.message));
    let words: Vec<_> = encoded
        .into_iter()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let mut llrs = Vec::with_capacity(corpus.len());
    for (trial, word) in corpus.iter().zip(&words) {
        validate_encoded(&code, &checks, trial, word)?;
        llrs.push(channel(
            word,
            &trial.positions,
            config.ldpc.errors,
            config.ldpc.llr_magnitude,
        ));
    }
    let (decoded, decode_time) = measure(&mut llrs, |llrs| code.decode(input(llrs), None));
    let mut counts = Counts::default();
    for (index, result) in decoded.into_iter().enumerate() {
        let result = result.map_err(|e| format!("sample {index}: {e}"))?;
        let observed = observe(
            &code,
            &checks,
            &corpus[index],
            &words[index],
            &llrs[index],
            &result,
            config,
        )
        .map_err(|e| format!("sample {index}: {e}"))?;
        log::debug!(
            "sample {index}: recovered={}, residual={}, iterations={}",
            observed.recovered,
            observed.residual,
            result.iterations()
        );
        counts.add(observed);
    }
    Ok(Report {
        counts,
        encode_bps: bits_per_second(k, corpus.len(), encode_time),
        decode_bps: bits_per_second(k, corpus.len(), decode_time),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_is_reproducible_and_errors_have_distinct_positions() {
        let trials = corpus(42, 12, 6, 4);
        assert_eq!(trials, corpus(42, 12, 6, 4));
        assert_eq!(trials, corpus(42, 12, 6, 8)[..4]);
        assert_ne!(trials, corpus(43, 12, 6, 4));
        let word = vec![Bit::Zero; 12];
        for trial in trials {
            for errors in 0..=12 {
                let llrs = channel(&word, &trial.positions, errors, 3.0);
                assert_eq!(llrs.iter().filter(|&&x| x < 0.0).count(), errors);
            }
        }
    }

    #[test]
    fn repetition_code_has_known_results_without_iterations() {
        for errors in 0..=3 {
            let mut config = Config {
                warmup: 2,
                repetitions: 8,
                ..Config::default()
            };
            config.ldpc.errors = errors;
            config.ldpc.max_iterations = 0;
            let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).unwrap();
            let result = run_code(&config, checks).unwrap();
            assert_eq!(
                result.counts,
                Counts {
                    blocks: 8,
                    recovered: if errors == 0 { 8 } else { 0 },
                    before: 8 * errors,
                    corrected: 0,
                    introduced: 0,
                    residual: 8 * errors
                }
            );
        }
    }
}
