use crate::{walsh, Kerdock, KerdockError};

/// The result of bounded-distance decoding for one block.
///
/// Owns the message and recovered word; subsequent codec calls do not change
/// the result or affect its lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeResult {
    message: Vec<bool>,
    word: Vec<bool>,
    distance: usize,
}

impl DecodeResult {
    /// The recovered message bits, in the same order as the input to `encode`.
    #[must_use]
    pub fn message(&self) -> &[bool] {
        &self.message
    }

    /// The nearest codeword.
    #[must_use]
    pub fn word(&self) -> &[bool] {
        &self.word
    }

    /// The Hamming distance from the received word to the returned codeword.
    ///
    /// Equals the number of corrected errors only within the guaranteed radius;
    /// by itself, this does not identify the channel error count.
    #[must_use]
    pub fn distance(&self) -> usize {
        self.distance
    }
}

struct Candidate {
    a: usize,
    b: usize,
    c: bool,
    distance: usize,
}

impl Candidate {
    fn message(&self, m: u32) -> Vec<bool> {
        (0..m - 1)
            .map(|i| self.a & (1 << i) != 0)
            .chain((0..m).map(|i| self.b & (1 << i) != 0))
            .chain([self.c])
            .collect()
    }
}

impl Kerdock {
    /// Recovers the nearest codeword within the correction radius.
    ///
    /// Recovers the original message if at most [`Self::correction_radius`]
    /// transmitted bits were flipped. With more errors, decoding may successfully
    /// return a different message: success does not prove which word was
    /// transmitted. The input slice is unchanged.
    ///
    /// Searches all quadratic cosets using integer Walsh transforms:
    /// `O(n² log n)` operations and `O(n)` additional memory.
    ///
    /// # Errors
    /// Returns [`KerdockError::WordLengthMismatch`] for an incorrect block length.
    /// Returns [`KerdockError::NoCodewordWithinRadius`] if the nearest word is
    /// outside the radius. The error includes the actual nearest distance;
    /// ties outside the radius are also rejected.
    pub fn decode(&self, received: &[bool]) -> Result<DecodeResult, KerdockError> {
        let candidate = self.nearest(received)?;
        let radius = self.correction_radius();
        if candidate.distance > radius {
            return Err(KerdockError::NoCodewordWithinRadius {
                nearest_distance: candidate.distance,
                radius,
            });
        }
        let message = candidate.message(self.m);
        let word = self.encode(&message)?;
        Ok(DecodeResult {
            message,
            word,
            distance: candidate.distance,
        })
    }

    /// Extracts a message only from an exact codeword, without correction.
    ///
    /// The input slice is unchanged. Uses the same search as [`Self::decode`],
    /// but accepts only distance zero.
    ///
    /// # Errors
    /// Returns [`KerdockError::WordLengthMismatch`] for an incorrect block length.
    /// Returns [`KerdockError::InvalidCodeword`] if the word is not in the code,
    /// even if decoding could correct the corruption.
    pub fn extract_message(&self, word: &[bool]) -> Result<Vec<bool>, KerdockError> {
        let candidate = self.nearest(word)?;
        if candidate.distance != 0 {
            return Err(KerdockError::InvalidCodeword);
        }
        Ok(candidate.message(self.m))
    }

    fn nearest(&self, received: &[bool]) -> Result<Candidate, KerdockError> {
        let n = self.codeword_len();
        if received.len() != n {
            return Err(KerdockError::WordLengthMismatch {
                expected: n,
                actual: received.len(),
            });
        }
        let mut values = vec![0i32; n];
        let mut best = Candidate {
            a: 0,
            b: 0,
            c: false,
            distance: n,
        };
        let mut best_magnitude = -1;
        for a in 0..n / 2 {
            for (j, (value, &bit)) in values.iter_mut().zip(received).enumerate() {
                *value = if bit ^ self.quadratics.bit(a, j) {
                    -1
                } else {
                    1
                };
            }
            walsh::transform(&mut values);
            for (b, &coefficient) in values.iter().enumerate() {
                let magnitude = coefficient.abs();
                // Strict comparison preserves the first candidate on a tie.
                if magnitude > best_magnitude {
                    best_magnitude = magnitude;
                    best = Candidate {
                        a,
                        b,
                        c: coefficient < 0,
                        distance: (n - magnitude as usize) / 2,
                    };
                }
            }
        }
        Ok(best)
    }
}
