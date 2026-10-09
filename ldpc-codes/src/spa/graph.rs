use crate::size::check_buffer_bytes;
use crate::{Bit, LdpcError, ParityCheckMatrix};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SparseGraph {
    pub(super) check_edges: Vec<Vec<usize>>,
    pub(super) bit_edges: Vec<Vec<usize>>,
    pub(super) edge_bits: Vec<usize>,
}

impl SparseGraph {
    #[cfg(test)]
    pub(super) fn try_new(checks: &ParityCheckMatrix) -> Result<Self, LdpcError> {
        let sizes = checked_workspace_sizes(checks)?;
        Ok(Self::from_checked_sizes(checks, sizes))
    }

    pub(super) fn from_checked_sizes(checks: &ParityCheckMatrix, sizes: WorkspaceSizes) -> Self {
        let mut check_edges = Vec::with_capacity(sizes.checks);
        let mut bit_edges = Vec::with_capacity(sizes.bits);
        for bit in 0..sizes.bits {
            let degree = checks
                .bit_checks(bit)
                .expect("bit index is within the matrix shape")
                .len();
            bit_edges.push(Vec::with_capacity(degree));
        }

        let mut edge_bits = Vec::with_capacity(sizes.edges);
        for check in 0..sizes.checks {
            let bits = checks
                .check_bits(check)
                .expect("check index is within the matrix shape");
            let mut edges = Vec::with_capacity(bits.len());
            for &bit in bits {
                let edge = edge_bits.len();
                edge_bits.push(bit);
                edges.push(edge);
                bit_edges[bit].push(edge);
            }
            check_edges.push(edges);
        }

        debug_assert_eq!(edge_bits.len(), sizes.edges);
        Self {
            check_edges,
            bit_edges,
            edge_bits,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceSizes {
    pub(super) checks: usize,
    pub(super) bits: usize,
    pub(super) edges: usize,
    pub(super) scratch_len: usize,
}

pub(super) fn scratch_len_for_degree(max_degree: usize) -> Result<usize, LdpcError> {
    max_degree.checked_add(1).ok_or(LdpcError::SizeOverflow)
}

pub(super) fn checked_workspace_sizes(
    checks: &ParityCheckMatrix,
) -> Result<WorkspaceSizes, LdpcError> {
    let check_count = checks.rows();
    let bit_count = checks.cols();
    let edge_count = checks.edge_count();

    check_buffer_bytes::<Vec<usize>>(check_count)?;
    check_buffer_bytes::<Vec<usize>>(bit_count)?;
    check_buffer_bytes::<usize>(edge_count)?;
    // Каждый массив указанного типа и длины имеет один и тот же размер в байтах.
    check_buffer_bytes::<Bit>(check_count)?;
    check_buffer_bytes::<f64>(bit_count)?;
    check_buffer_bytes::<bool>(bit_count)?;
    check_buffer_bytes::<Bit>(bit_count)?;
    check_buffer_bytes::<f64>(edge_count)?;

    let mut max_check_degree = 0;
    for check in 0..check_count {
        let degree = checks
            .check_bits(check)
            .expect("check index is within the matrix shape")
            .len();
        check_buffer_bytes::<usize>(degree)?;
        max_check_degree = max_check_degree.max(degree);
    }
    for bit in 0..bit_count {
        let degree = checks
            .bit_checks(bit)
            .expect("bit index is within the matrix shape")
            .len();
        check_buffer_bytes::<usize>(degree)?;
    }

    let scratch_len = scratch_len_for_degree(max_check_degree)?;
    check_buffer_bytes::<f64>(scratch_len)?;

    Ok(WorkspaceSizes {
        checks: check_count,
        bits: bit_count,
        edges: edge_count,
        scratch_len,
    })
}
