use std::fmt;

/// External input errors leave state unchanged, except equation contradictions
/// and padding discovered in a complete solution, which invalidate a decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FountainError {
    MessageSize(coding_core::SizeError),
    InvalidParameter {
        parameter: &'static str,
        actual: usize,
    },
    InvalidLogarithmicDraws,
    SizeOverflow,
    ResourceLimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    EmptyBlock,
    ContextMismatch,
    PayloadLength {
        expected: usize,
        actual: usize,
    },
    ConflictingSymbol {
        id: u64,
    },
    InconsistentEquation {
        id: u64,
        rhs_column: usize,
    },
    NonzeroPadding {
        symbol_id: u64,
        byte_offset: usize,
    },
    InvalidDecoder,
    InvalidParityId {
        id: u64,
    },
    HelperMismatch {
        expected: u64,
        actual: u64,
    },
    TargetMismatch {
        expected: u64,
        actual: u64,
    },
    HelperUnavailable {
        id: u64,
    },
    WireLength {
        expected: usize,
        actual: usize,
    },
    WireMagic,
    WireVersion {
        actual: u16,
    },
    WireProfile {
        actual: u16,
    },
    WireReserved,
    WirePolynomial {
        expected: u32,
        actual: u32,
    },
    Linalg(gf_linalg::LinalgError),
}

impl fmt::Display for FountainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fountain code error: {self:?}")
    }
}
impl std::error::Error for FountainError {}
impl From<gf_linalg::LinalgError> for FountainError {
    fn from(value: gf_linalg::LinalgError) -> Self {
        Self::Linalg(value)
    }
}
impl From<coding_core::SizeError> for FountainError {
    fn from(value: coding_core::SizeError) -> Self {
        Self::MessageSize(value)
    }
}
