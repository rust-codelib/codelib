//! Общие помощники интеграционных тестов.
#![allow(dead_code)]

use cascade_codes::{CascadeCode, CascadeParams, CodeField, NodeShare};

/// Детерминированный псевдослучайный блок длины `len`.
pub fn message<F: CodeField>(len: usize, seed: u64) -> Vec<F> {
    let q = F::order() as u64;
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            F::from_index(state % q).unwrap()
        })
        .collect()
}

pub fn code<F: CodeField>(n: usize, k: usize, d: usize, mu: usize) -> CascadeCode<F> {
    CascadeCode::try_new(CascadeParams::try_new(n, k, d, mu).unwrap()).unwrap()
}

/// Все `size`-элементные подмножества `0..n` в лексикографическом порядке.
pub fn subsets(n: usize, size: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    fn rec(start: usize, n: usize, size: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == size {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            rec(i + 1, n, size, cur, out);
            cur.pop();
        }
    }
    rec(0, n, size, &mut current, &mut out);
    out
}

pub fn pick<F: Clone>(nodes: &[NodeShare<F>], indices: &[usize]) -> Vec<NodeShare<F>> {
    indices.iter().map(|&i| nodes[i].clone()).collect()
}

/// Все допустимые (n, k, d, mu) с n <= max_n и alpha <= max_alpha.
pub fn parameter_grid(max_n: usize, max_alpha: usize) -> Vec<CascadeParams> {
    let mut out = Vec::new();
    for n in 2..=max_n {
        for d in 1..n {
            for k in 1..=d {
                for mu in 1..=k {
                    if let Ok(p) = CascadeParams::try_new(n, k, d, mu) {
                        if p.alpha() <= max_alpha {
                            out.push(p);
                        }
                    }
                }
            }
        }
    }
    out
}
