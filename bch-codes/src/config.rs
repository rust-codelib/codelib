//! Параметры кода и размеры буферов.

use crate::bits::{bit, bytes_for, valid_padding};
use crate::BchError;

/// Конфигурация примитивного узкосмыслового BCH-кода.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BchConfig {
    /// Гарантированно исправляемое число ошибок.
    pub correctable_errors: usize,
    /// Число удалённых старших информационных позиций.
    pub shortened_by: usize,
}

impl BchConfig {
    /// Создаёт конфигурацию. Ограничения, зависящие от поля, проверяются
    /// функциями и конструктором кодека.
    pub fn new(correctable_errors: usize, shortened_by: usize) -> Result<Self, BchError> {
        if correctable_errors == 0 {
            return Err(BchError::InvalidParameters);
        }
        Ok(Self {
            correctable_errors,
            shortened_by,
        })
    }
}

pub(crate) fn validate_config<const N: usize>(config: &BchConfig) -> Result<(), BchError> {
    let n = N.checked_sub(1).ok_or(BchError::InvalidParameters)?;
    if !N.is_power_of_two()
        || !(4..=65536).contains(&N)
        || config.correctable_errors == 0
        || config
            .correctable_errors
            .checked_mul(2)
            .and_then(|v| v.checked_add(1))
            .map_or(true, |d| d > n)
    {
        return Err(BchError::InvalidParameters);
    }
    let degree = generator_degree::<N>(config)?;
    let k = n.checked_sub(degree).ok_or(BchError::InvalidParameters)?;
    if config.shortened_by > k {
        return Err(BchError::InvalidParameters);
    }
    Ok(())
}

fn coset_leader<const N: usize>(start: usize) -> usize {
    let n = N - 1;
    let mut x = start % n;
    let mut leader = x;
    loop {
        x = (x * 2) % n;
        if x < leader {
            leader = x;
        }
        if x == start % n {
            break;
        }
    }
    leader
}

fn coset_len<const N: usize>(start: usize) -> usize {
    let n = N - 1;
    let mut x = (start * 2) % n;
    let mut len = 1;
    while x != start % n {
        x = (x * 2) % n;
        len += 1;
    }
    len
}

/// Возвращает степень порождающего многочлена.
pub fn generator_degree<const N: usize>(config: &BchConfig) -> Result<usize, BchError> {
    let n = N.checked_sub(1).ok_or(BchError::InvalidParameters)?;
    if !N.is_power_of_two()
        || !(4..=65536).contains(&N)
        || config.correctable_errors == 0
        || config
            .correctable_errors
            .saturating_mul(2)
            .saturating_add(1)
            > n
    {
        return Err(BchError::InvalidParameters);
    }
    let mut degree = 0;
    for i in 1..=2 * config.correctable_errors {
        if coset_leader::<N>(i) == i {
            degree += coset_len::<N>(i);
        }
    }
    Ok(degree)
}

/// Возвращает размер упакованного порождающего многочлена в байтах.
pub fn generator_bytes<const N: usize>(config: &BchConfig) -> Result<usize, BchError> {
    Ok(bytes_for(generator_degree::<N>(config)? + 1))
}

/// Возвращает число элементов поля в рабочем буфере построения генератора.
pub fn construction_workspace_len<const N: usize>(config: &BchConfig) -> Result<usize, BchError> {
    validate_config::<N>(config)?;
    Ok(N + 2 * (N.trailing_zeros() as usize + 1))
}

/// Возвращает число элементов поля в рабочем буфере декодирования.
#[must_use]
pub const fn decode_workspace_len(config: &BchConfig) -> usize {
    config
        .correctable_errors
        .saturating_mul(5)
        .saturating_add(3)
}

pub(crate) fn generator_shape<const N: usize>(
    config: &BchConfig,
    generator: &[u8],
) -> Result<usize, BchError> {
    validate_config::<N>(config)?;
    let degree = generator_degree::<N>(config)?;
    let expected = bytes_for(degree + 1);
    if generator.len() != expected {
        return Err(BchError::BufferLength {
            expected,
            actual: generator.len(),
        });
    }
    if !valid_padding(generator, degree + 1) || !bit(generator, degree) || !bit(generator, 0) {
        return Err(BchError::InvalidGenerator);
    }
    Ok(degree)
}

/// Возвращает число передаваемых битов кодового слова.
pub fn codeword_bits<const N: usize>(
    config: &BchConfig,
    generator: &[u8],
) -> Result<usize, BchError> {
    generator_shape::<N>(config, generator)?;
    Ok(N - 1 - config.shortened_by)
}

/// Возвращает число информационных битов.
pub fn message_bits<const N: usize>(
    config: &BchConfig,
    generator: &[u8],
) -> Result<usize, BchError> {
    let degree = generator_shape::<N>(config, generator)?;
    Ok(N - 1 - degree - config.shortened_by)
}
