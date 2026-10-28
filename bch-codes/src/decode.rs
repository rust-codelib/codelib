//! Полный конвейер декодирования и исправления.

use crate::berlekamp_massey::locator;
use crate::bits::{bytes_for, toggle, valid_padding};
use crate::chien::error_positions;
use crate::config::decode_workspace_len;
use crate::generator::validate_generator;
use crate::syndrome::syndromes;
use crate::{BchChecks, BchConfig, BchCorrection, BchError, BchStopReason, Gf};

/// Рабочая память декодера.
pub struct BchDecodeWorkspace<'a, const N: usize, const POLY: u32> {
    /// Элементы поля для синдромов и полиномов Берлекампа—Мэсси.
    pub field: &'a mut [Gf<N, POLY>],
    /// Позиции найденных ошибок; требуется не менее `t` элементов.
    pub positions: &'a mut [usize],
}

/// Исправляет принятое слово на месте. При `Stopped` восстанавливает вход.
pub fn correct<const N: usize, const POLY: u32>(
    config: &BchConfig,
    generator: &[u8],
    received: &mut [u8],
    workspace: &mut BchDecodeWorkspace<'_, N, POLY>,
) -> Result<BchCorrection, BchError> {
    validate_generator::<N, POLY>(config, generator)?;
    let n = N - 1 - config.shortened_by;
    let expected = bytes_for(n);
    if received.len() != expected {
        return Err(BchError::BufferLength {
            expected,
            actual: received.len(),
        });
    }
    if !valid_padding(received, n) {
        return Err(BchError::NonZeroPaddingBits);
    }
    let t = config.correctable_errors;
    let required = decode_workspace_len(config);
    if workspace.field.len() < required {
        return Err(BchError::WorkspaceTooSmall {
            required,
            actual: workspace.field.len(),
        });
    }
    if workspace.positions.len() < t {
        return Err(BchError::WorkspaceTooSmall {
            required: t,
            actual: workspace.positions.len(),
        });
    }

    let (syndrome, rest) = workspace.field.split_at_mut(2 * t);
    let (current, rest) = rest.split_at_mut(t + 1);
    let (previous, temporary) = rest.split_at_mut(t + 1);
    if syndromes::<N, POLY>(received, n, syndrome) {
        return Ok(BchCorrection::Candidate(BchChecks {
            initial_syndromes_zero: true,
            final_syndromes_zero: true,
            corrected_bits: 0,
        }));
    }

    let degree = locator(syndrome, current, previous, &mut temporary[..t + 1]);
    if degree == 0 || degree > t {
        return Ok(BchCorrection::Stopped(BchStopReason::InvalidLocator {
            degree,
            roots: 0,
        }));
    }
    let roots = match error_positions(current, degree, n, &mut workspace.positions[..t]) {
        Ok(roots) => roots,
        Err(reason) => return Ok(BchCorrection::Stopped(reason)),
    };

    for &position in &workspace.positions[..roots] {
        toggle(received, position);
    }
    if !syndromes::<N, POLY>(received, n, syndrome) {
        for &position in &workspace.positions[..roots] {
            toggle(received, position);
        }
        return Ok(BchCorrection::Stopped(BchStopReason::VerificationFailed));
    }
    Ok(BchCorrection::Candidate(BchChecks {
        initial_syndromes_zero: false,
        final_syndromes_zero: true,
        corrected_bits: roots,
    }))
}
