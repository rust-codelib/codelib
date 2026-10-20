use crate::{FountainError, ResourceLimits};
use coding_core::MessageBytes;

pub type SymbolId = u64;

/// Fixed byte/field profiles; the polynomial is part of the wire contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Gf65536,
    Gf256,
}
impl Profile {
    pub fn id(self) -> u16 {
        match self {
            Self::Gf65536 => 1,
            Self::Gf256 => 2,
        }
    }
    pub fn width(self) -> usize {
        match self {
            Self::Gf65536 => 2,
            Self::Gf256 => 1,
        }
    }
    pub fn polynomial(self) -> u32 {
        match self {
            Self::Gf65536 => 0x1002d,
            Self::Gf256 => 0x11d,
        }
    }
}

pub(crate) fn check_symbol_bytes(profile: Profile, t: usize) -> Result<(), FountainError> {
    if t == 0 || t > coding_core::MAX_MESSAGE_BYTES || t % profile.width() != 0 {
        return Err(FountainError::InvalidParameter {
            parameter: "symbol_bytes",
            actual: t,
        });
    }
    Ok(())
}

/// Immutable, validated mathematical block context. Local limits are separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDescriptor {
    pub(crate) profile: Profile,
    pub(crate) block_id: [u8; 16],
    pub(crate) n: usize,
    pub(crate) t: usize,
    pub(crate) k: usize,
    pub(crate) d: u16,
    pub(crate) seed: u64,
}
impl BlockDescriptor {
    pub fn try_new(
        profile: Profile,
        block_id: [u8; 16],
        message_bytes: usize,
        symbol_bytes: usize,
        draws: u16,
        seed: u64,
    ) -> Result<Self, FountainError> {
        MessageBytes::try_new(message_bytes)?;
        check_symbol_bytes(profile, symbol_bytes)?;
        let k = message_bytes / symbol_bytes + usize::from(message_bytes % symbol_bytes != 0);
        if (k == 0) != (draws == 0) {
            return Err(FountainError::InvalidParameter {
                parameter: "parity_draws",
                actual: draws as usize,
            });
        }
        Ok(Self {
            profile,
            block_id,
            n: message_bytes,
            t: symbol_bytes,
            k,
            d: draws,
            seed,
        })
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn block_id(&self) -> &[u8; 16] {
        &self.block_id
    }
    pub fn message_bytes(&self) -> usize {
        self.n
    }
    pub fn symbol_bytes(&self) -> usize {
        self.t
    }
    pub fn source_symbols(&self) -> usize {
        self.k
    }
    pub fn elements_per_symbol(&self) -> usize {
        self.t / self.profile.width()
    }
    pub fn parity_draws(&self) -> u16 {
        self.d
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn wire_version(&self) -> u16 {
        1
    }
    pub(crate) fn check_nonempty(&self) -> Result<(), FountainError> {
        if self.k == 0 {
            Err(FountainError::EmptyBlock)
        } else {
            Ok(())
        }
    }
    pub(crate) fn check_padding(&self, id: u64, bytes: &[u8]) -> Result<(), FountainError> {
        if self.k != 0 && id == (self.k - 1) as u64 {
            let start = self.n - (self.k - 1) * self.t;
            if let Some(offset) = bytes
                .iter()
                .enumerate()
                .skip(start)
                .find_map(|(i, &b)| (b != 0).then_some(i))
            {
                return Err(FountainError::NonzeroPadding {
                    symbol_id: id,
                    byte_offset: offset,
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairableConfig {
    profile: Profile,
    symbol_bytes: usize,
    draws: u16,
    seed: u64,
    limits: ResourceLimits,
}
impl RepairableConfig {
    /// `draws` is explicit and positive; empty preparation stores d=0 instead.
    pub fn try_new(
        profile: Profile,
        symbol_bytes: usize,
        draws: u16,
        seed: u64,
        limits: ResourceLimits,
    ) -> Result<Self, FountainError> {
        check_symbol_bytes(profile, symbol_bytes)?;
        if draws == 0 {
            return Err(FountainError::InvalidParameter {
                parameter: "parity_draws",
                actual: 0,
            });
        }
        limits.check_draws(draws)?;
        Ok(Self {
            profile,
            symbol_bytes,
            draws,
            seed,
            limits,
        })
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn symbol_bytes(&self) -> usize {
        self.symbol_bytes
    }
    pub fn parity_draws(&self) -> u16 {
        self.draws
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }
    pub fn descriptor(
        &self,
        n: usize,
        block_id: [u8; 16],
    ) -> Result<BlockDescriptor, FountainError> {
        BlockDescriptor::try_new(
            self.profile,
            block_id,
            n,
            self.symbol_bytes,
            if n == 0 { 0 } else { self.draws },
            self.seed,
        )
    }
}

/// Owned payload. Construction checks shape; equation and padding validation
/// belong to the receiving decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedSymbol {
    descriptor: BlockDescriptor,
    id: u64,
    payload: Vec<u8>,
}
impl EncodedSymbol {
    pub fn try_new(
        descriptor: BlockDescriptor,
        id: u64,
        payload: Vec<u8>,
    ) -> Result<Self, FountainError> {
        descriptor.check_nonempty()?;
        if payload.len() != descriptor.t {
            return Err(FountainError::PayloadLength {
                expected: descriptor.t,
                actual: payload.len(),
            });
        }
        Ok(Self {
            descriptor,
            id,
            payload,
        })
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}
