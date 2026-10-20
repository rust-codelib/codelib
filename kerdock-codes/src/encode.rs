use crate::{Kerdock, KerdockError};

impl Kerdock {
    /// Encodes one message of [`Self::message_len`] bits into a new codeword.
    ///
    /// The first `m-1` bits encode field element `a`, and the next `m` encode
    /// mask `b`, both least-significant bit first. The final bit is constant `c`.
    /// Coordinate `j` is `Q_a(j) XOR parity(b & j) XOR c`.
    /// The input slice is unchanged; the result contains exactly
    /// [`Self::codeword_len`] bits. Encoding takes `O(n)` work after preparation.
    ///
    /// # Errors
    /// Returns [`KerdockError::MessageLengthMismatch`] if the length is not `2m`.
    pub fn encode(&self, message: &[bool]) -> Result<Vec<bool>, KerdockError> {
        if message.len() != self.message_len() {
            return Err(KerdockError::MessageLengthMismatch {
                expected: self.message_len(),
                actual: message.len(),
            });
        }
        let s = self.m as usize - 1;
        let from_bits = |bits: &[bool]| {
            bits.iter()
                .enumerate()
                .fold(0usize, |value, (i, &bit)| value | (usize::from(bit) << i))
        };
        let a = from_bits(&message[..s]);
        let b = from_bits(&message[s..s + self.m as usize]);
        let c = message[message.len() - 1];
        Ok((0..self.codeword_len())
            .map(|j| self.quadratics.bit(a, j) ^ ((b & j).count_ones() % 2 == 1) ^ c)
            .collect())
    }
}
