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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub blocks: usize,
    pub recovered: usize,
    pub before: usize,
    pub corrected: usize,
    pub introduced: usize,
    pub residual: usize,
}

impl Counts {
    pub fn add(&mut self, other: Self) {
        self.blocks += other.blocks;
        self.recovered += other.recovered;
        self.before += other.before;
        self.corrected += other.corrected;
        self.introduced += other.introduced;
        self.residual += other.residual;
    }
}

/// Errors refer to the whole codeword, in the element type's units.
pub fn count_errors<T: Eq>(
    reference: &[T],
    received: &[T],
    decoded: &[T],
) -> Result<Counts, String> {
    if reference.len() != received.len() || reference.len() != decoded.len() {
        return Err("word lengths differ".into());
    }
    let mut counts = Counts {
        blocks: 1,
        ..Default::default()
    };
    for ((expected, before), after) in reference.iter().zip(received).zip(decoded) {
        let was_wrong = before != expected;
        let is_wrong = after != expected;
        counts.before += usize::from(was_wrong);
        counts.residual += usize::from(is_wrong);
        counts.corrected += usize::from(was_wrong && !is_wrong);
        counts.introduced += usize::from(!was_wrong && is_wrong);
    }
    counts.recovered = usize::from(counts.residual == 0);
    Ok(counts)
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
            Counts {
                blocks: 1,
                recovered: 0,
                before: 2,
                corrected: 1,
                introduced: 1,
                residual: 2
            }
        );
        assert!(count_errors(&[0], &[], &[0]).is_err());
    }
}
