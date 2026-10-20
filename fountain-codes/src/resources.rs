use crate::{BlockDescriptor, FountainError};
use std::mem::size_of;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    max_parity_draws: usize,
    max_unique_symbols: usize,
    max_catalog_entries: usize,
    max_catalog_edges: usize,
    max_logical_buffer_bytes: usize,
}
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_parity_draws: 256,
            max_unique_symbols: 16384,
            max_catalog_entries: 16384,
            max_catalog_edges: 1048576,
            max_logical_buffer_bytes: 67108864,
        }
    }
}
impl ResourceLimits {
    pub fn try_new(
        max_parity_draws: usize,
        max_unique_symbols: usize,
        max_catalog_entries: usize,
        max_catalog_edges: usize,
        max_logical_buffer_bytes: usize,
    ) -> Result<Self, FountainError> {
        for (parameter, actual) in [
            ("max_parity_draws", max_parity_draws),
            ("max_unique_symbols", max_unique_symbols),
            ("max_catalog_entries", max_catalog_entries),
            ("max_catalog_edges", max_catalog_edges),
            ("max_logical_buffer_bytes", max_logical_buffer_bytes),
        ] {
            if actual == 0 || (parameter == "max_parity_draws" && actual > u16::MAX as usize) {
                return Err(FountainError::InvalidParameter { parameter, actual });
            }
        }
        Ok(Self {
            max_parity_draws,
            max_unique_symbols,
            max_catalog_entries,
            max_catalog_edges,
            max_logical_buffer_bytes,
        })
    }
    pub fn max_parity_draws(&self) -> usize {
        self.max_parity_draws
    }
    pub fn max_unique_symbols(&self) -> usize {
        self.max_unique_symbols
    }
    pub fn max_catalog_entries(&self) -> usize {
        self.max_catalog_entries
    }
    pub fn max_catalog_edges(&self) -> usize {
        self.max_catalog_edges
    }
    pub fn max_logical_buffer_bytes(&self) -> usize {
        self.max_logical_buffer_bytes
    }
    pub(crate) fn check_draws(&self, d: u16) -> Result<(), FountainError> {
        limit("parity_draws", d as usize, self.max_parity_draws)
    }
    pub(crate) fn check_bytes(&self, bytes: usize) -> Result<(), FountainError> {
        limit("logical_buffer_bytes", bytes, self.max_logical_buffer_bytes)
    }
}
pub(crate) fn add(a: usize, b: usize) -> Result<usize, FountainError> {
    a.checked_add(b).ok_or(FountainError::SizeOverflow)
}
pub(crate) fn mul(a: usize, b: usize) -> Result<usize, FountainError> {
    a.checked_mul(b).ok_or(FountainError::SizeOverflow)
}
pub(crate) fn sum(values: &[usize]) -> Result<usize, FountainError> {
    values.iter().try_fold(0, |a, &b| add(a, b))
}
pub(crate) fn buffer<T>(len: usize) -> Result<(), FountainError> {
    if mul(len, size_of::<T>())? > isize::MAX as usize {
        Err(FountainError::SizeOverflow)
    } else {
        Ok(())
    }
}
pub(crate) fn limit(
    resource: &'static str,
    actual: usize,
    bound: usize,
) -> Result<(), FountainError> {
    if actual > bound {
        Err(FountainError::ResourceLimitExceeded {
            resource,
            actual,
            limit: bound,
        })
    } else {
        Ok(())
    }
}

/// Exact logical charges from v1, not RSS or an allocator quota.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceCharge {
    pub scratch_bytes: usize,
    pub encoder_bytes: usize,
    pub decoder_bytes: usize,
    pub catalog_bytes: usize,
}
impl ResourceCharge {
    pub fn calculate(
        desc: &BlockDescriptor,
        unique_symbols: usize,
        catalog_entries: usize,
        catalog_edges: usize,
    ) -> Result<Self, FountainError> {
        let (k, l, d, t, n) = (
            desc.k,
            desc.elements_per_symbol(),
            desc.d as usize,
            desc.t,
            desc.n,
        );
        let r = if k == 0 {
            0
        } else {
            sum(&[
                128,
                mul(16, d)?,
                mul(16, d.min(k))?,
                mul(2, k)?,
                mul(4, l)?,
                64,
                t,
            ])?
        };
        let kl2 = mul(2, mul(k, l)?)?;
        let encoder_bytes = sum(&[48, kl2, r])?;
        let decoder_bytes = sum(&[
            48,
            mul(64, k)?,
            kl2,
            n,
            r,
            mul(mul(2, k)?, add(k, l)?)?,
            mul(unique_symbols, add(32, t)?)?,
        ])?;
        let catalog_bytes = sum(&[
            48,
            mul(64, k)?,
            mul(64, catalog_entries)?,
            mul(48, catalog_edges)?,
            r,
        ])?;
        Ok(Self {
            scratch_bytes: r,
            encoder_bytes,
            decoder_bytes,
            catalog_bytes,
        })
    }
}
