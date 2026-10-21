use std::{
    hint::black_box,
    time::{Duration, Instant},
};

/// Prepare buffers before this call. Stop at the first Err and retain native
/// results until the clock stops; validation and result disposal are not timed.
pub fn measure<I, O, E>(
    inputs: &mut [I],
    mut operation: impl FnMut(&mut I) -> Result<O, E>,
) -> (Vec<Result<O, E>>, Duration) {
    let mut outputs = Vec::with_capacity(inputs.len());
    let start = Instant::now();
    for input in inputs {
        let result = black_box(operation(black_box(input)));
        let failed = result.is_err();
        outputs.push(result);
        if failed {
            break;
        }
    }
    let elapsed = start.elapsed();
    (outputs, elapsed)
}

pub fn bits_per_second(payload_bits: usize, calls: usize, elapsed: Duration) -> Option<f64> {
    (calls > 0 && !elapsed.is_zero())
        .then(|| payload_bits as f64 * calls as f64 / elapsed.as_secs_f64())
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WordErrors {
    pub corrected: usize,
    pub introduced: usize,
    pub residual: usize,
}

/// One completed attempt; missing word metrics never mean zero errors.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub recovered: bool,
    pub before: Option<usize>,
    pub after: Option<WordErrors>,
    pub iterations: Option<usize>,
}

impl Observation {
    pub fn validate(self, n: usize) -> Result<Self, String> {
        let invalid_before = self.before.is_some_and(|before| before > n);
        let invalid_after = self.after.is_some_and(|after| {
            after.corrected > n
                || after.introduced > n
                || after.residual > n
                || self.recovered != (after.residual == 0)
                || self.before.is_some_and(|before| {
                    after.corrected > before
                        || after.introduced > n.saturating_sub(before)
                        || before - after.corrected + after.introduced != after.residual
                })
        });
        if invalid_before || invalid_after {
            return Err(format!("invalid block observation: {self:?}"));
        }
        Ok(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Counts {
    pub blocks: usize,
    pub recovered: usize,
    pub before: Option<usize>,
    pub after: Option<WordErrors>,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            blocks: 0,
            recovered: 0,
            before: Some(0),
            after: Some(WordErrors::default()),
        }
    }
}

impl Counts {
    pub fn add(&mut self, observation: Observation) {
        self.blocks += 1;
        self.recovered += usize::from(observation.recovered);
        self.before = self.before.zip(observation.before).map(|(a, b)| a + b);
        self.after = self.after.zip(observation.after).map(|(a, b)| WordErrors {
            corrected: a.corrected + b.corrected,
            introduced: a.introduced + b.introduced,
            residual: a.residual + b.residual,
        });
    }
}

/// Errors refer to the whole codeword, in the element type's units.
pub fn count_errors<T: Eq>(
    reference: &[T],
    received: &[T],
    decoded: &[T],
) -> Result<Observation, String> {
    if reference.len() != received.len() || reference.len() != decoded.len() {
        return Err("word lengths differ".into());
    }
    let mut errors_before = 0;
    let mut after = WordErrors::default();
    for ((expected, before), decoded) in reference.iter().zip(received).zip(decoded) {
        let was_wrong = before != expected;
        let is_wrong = decoded != expected;
        errors_before += usize::from(was_wrong);
        after.residual += usize::from(is_wrong);
        after.corrected += usize::from(was_wrong && !is_wrong);
        after.introduced += usize::from(!was_wrong && is_wrong);
    }
    Ok(Observation {
        recovered: after.residual == 0,
        before: Some(errors_before),
        after: Some(after),
        iterations: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_and_error_counts_use_actual_calls_and_reference_words() {
        assert_eq!(bits_per_second(64, 10, Duration::from_secs(2)), Some(320.0));
        assert_eq!(bits_per_second(64, 0, Duration::from_secs(2)), None);
        assert_eq!(bits_per_second(64, 10, Duration::ZERO), None);
        let counts = count_errors(&[0; 4], &[1, 1, 0, 0], &[1, 0, 1, 0]).unwrap();
        assert_eq!(
            counts,
            Observation {
                recovered: false,
                before: Some(2),
                after: Some(WordErrors {
                    corrected: 1,
                    introduced: 1,
                    residual: 2
                }),
                iterations: None,
            }
        );
        assert!(count_errors(&[0], &[], &[0]).is_err());
    }
}
