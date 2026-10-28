//! Систематическое кодирование и извлечение сообщения.

use coding_core::{Codeword, Message, MessageBytes};

use crate::bits::{bit, bytes_for, set, toggle, valid_padding};
use crate::config::generator_shape;
use crate::generator::validate_generator;
use crate::{BchConfig, BchError};

/// Систематически кодирует сообщение: проверочные биты занимают младшие позиции.
pub fn encode_systematic<const N: usize, const POLY: u32>(
    config: &BchConfig,
    generator: &[u8],
    message: &[u8],
    output: &mut [u8],
) -> Result<(), BchError> {
    let degree = validate_generator::<N, POLY>(config, generator)?;
    let k = N - 1 - degree - config.shortened_by;
    let n = N - 1 - config.shortened_by;
    let expected_message = bytes_for(k);
    let expected_output = bytes_for(n);
    if message.len() != expected_message {
        return Err(BchError::BufferLength {
            expected: expected_message,
            actual: message.len(),
        });
    }
    if output.len() != expected_output {
        return Err(BchError::BufferLength {
            expected: expected_output,
            actual: output.len(),
        });
    }
    if !valid_padding(message, k) {
        return Err(BchError::NonZeroPaddingBits);
    }
    output.fill(0);
    for i in 0..k {
        if bit(message, i) {
            set(output, degree + i, true);
        }
    }
    for i in (degree..n).rev() {
        if bit(output, i) {
            let shift = i - degree;
            for j in 0..=degree {
                if bit(generator, j) {
                    toggle(output, shift + j);
                }
            }
        }
    }
    // Восстанавливаем систематическую часть, затронутую делением.
    for i in 0..k {
        set(output, degree + i, bit(message, i));
    }
    Ok(())
}

/// Извлекает систематическое сообщение из кодового слова.
pub fn extract_message<'out, const N: usize>(
    config: &BchConfig,
    generator: &[u8],
    codeword: Codeword<&[u8]>,
    output: &'out mut [u8],
) -> Result<Message<&'out mut [u8]>, BchError> {
    let degree = generator_shape::<N>(config, generator)?;
    let n = N - 1 - config.shortened_by;
    let k = n - degree;
    MessageBytes::from_bits(k)?;
    let expected_word = bytes_for(n);
    let expected_output = bytes_for(k);
    if codeword.0.len() != expected_word {
        return Err(BchError::BufferLength {
            expected: expected_word,
            actual: codeword.0.len(),
        });
    }
    if output.len() != expected_output {
        return Err(BchError::BufferLength {
            expected: expected_output,
            actual: output.len(),
        });
    }
    if !valid_padding(codeword.0, n) {
        return Err(BchError::NonZeroPaddingBits);
    }
    output.fill(0);
    for i in 0..k {
        if bit(codeword.0, degree + i) {
            set(output, i, true);
        }
    }
    Ok(Message(output))
}
