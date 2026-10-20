use crate::{types::check_symbol_bytes, BlockDescriptor, EncodedSymbol, FountainError, Profile};
use coding_core::MessageBytes;

fn exact(actual: usize, expected: usize) -> Result<(), FountainError> {
    if actual != expected {
        Err(FountainError::WireLength { expected, actual })
    } else {
        Ok(())
    }
}
fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("checked wire length"))
}
fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("checked wire length"))
}
/// Strict standalone descriptor parser. The error order follows wire v1.
/// All representable draw counts are accepted; components apply local limits.
pub fn parse_descriptor(bytes: &[u8]) -> Result<BlockDescriptor, FountainError> {
    exact(bytes.len(), 48)?;
    if &bytes[..4] != b"RFCD" {
        return Err(FountainError::WireMagic);
    }
    let version = u16_at(bytes, 4);
    if version != 1 {
        return Err(FountainError::WireVersion { actual: version });
    }
    let profile = match u16_at(bytes, 6) {
        1 => Profile::Gf65536,
        2 => Profile::Gf256,
        actual => return Err(FountainError::WireProfile { actual }),
    };
    if u32_at(bytes, 44) != 0 {
        return Err(FountainError::WireReserved);
    }
    let polynomial = u32_at(bytes, 40);
    if polynomial != profile.polynomial() {
        return Err(FountainError::WirePolynomial {
            expected: profile.polynomial(),
            actual: polynomial,
        });
    }
    let (n, t, k, d) = (
        u16_at(bytes, 24) as usize,
        u16_at(bytes, 26) as usize,
        u16_at(bytes, 28) as usize,
        u16_at(bytes, 30),
    );
    MessageBytes::try_new(n)?;
    check_symbol_bytes(profile, t)?;
    let expected_k = n / t + usize::from(n % t != 0);
    if k != expected_k {
        return Err(FountainError::InvalidParameter {
            parameter: "source_symbols",
            actual: k,
        });
    }
    BlockDescriptor::try_new(
        profile,
        bytes[8..24].try_into().expect("checked wire length"),
        n,
        t,
        d,
        u64_at(bytes, 32),
    )
}
/// Strict self-contained packet parser. Payload is copied only after all header
/// and exact-length checks. The receiving component checks the full context.
pub fn parse_symbol(bytes: &[u8]) -> Result<EncodedSymbol, FountainError> {
    if bytes.len() < 64 {
        return Err(FountainError::WireLength {
            expected: 64,
            actual: bytes.len(),
        });
    }
    if &bytes[..4] != b"RFCS" {
        return Err(FountainError::WireMagic);
    }
    let descriptor = parse_descriptor(&bytes[4..52])?;
    if u16_at(bytes, 62) != 0 {
        return Err(FountainError::WireReserved);
    }
    descriptor.check_nonempty()?;
    let payload_len = u16_at(bytes, 60) as usize;
    if payload_len != descriptor.symbol_bytes() {
        return Err(FountainError::PayloadLength {
            expected: descriptor.symbol_bytes(),
            actual: payload_len,
        });
    }
    exact(bytes.len(), 64 + descriptor.symbol_bytes())?;
    EncodedSymbol::try_new(descriptor, u64_at(bytes, 52), bytes[64..].to_vec())
}
impl BlockDescriptor {
    /// Exactly 48 bytes, fixed little-endian integers, no Rust memory layout.
    pub fn to_wire(&self) -> Result<Vec<u8>, FountainError> {
        let mut out = Vec::with_capacity(48);
        out.extend_from_slice(b"RFCD");
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&self.profile.id().to_le_bytes());
        out.extend_from_slice(&self.block_id);
        for v in [self.n as u16, self.t as u16, self.k as u16, self.d] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.profile.polynomial().to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        Ok(out)
    }
}
impl EncodedSymbol {
    /// Exactly 64+T bytes, with the complete descriptor and no checksum.
    pub fn to_wire(&self) -> Result<Vec<u8>, FountainError> {
        let mut out = Vec::with_capacity(64 + self.payload().len());
        out.extend_from_slice(b"RFCS");
        out.extend_from_slice(&self.descriptor().to_wire()?);
        out.extend_from_slice(&self.id().to_le_bytes());
        out.extend_from_slice(&(self.payload().len() as u16).to_le_bytes());
        out.extend_from_slice(&[0; 2]);
        out.extend_from_slice(self.payload());
        Ok(out)
    }
}
