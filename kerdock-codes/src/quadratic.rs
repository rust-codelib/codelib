use gf2m::GfRuntime;

/// Precomputed quadratic-form bits, packed into `u64` words.
/// Each row represents one form; each bit represents a codeword position.
pub(crate) struct QuadraticTable {
    words_per_row: usize,
    words: Vec<u64>,
}

impl QuadraticTable {
    /// Accepts only a supported exponent selected by Kerdock::new.
    pub(crate) fn new(m: u32) -> Self {
        debug_assert!((4..=10).contains(&m) && m % 2 == 0);
        let s = m - 1;
        let n: usize = 1 << m;
        let words_per_row = n.div_ceil(64);
        let mut table = Self {
            words_per_row,
            words: vec![0; (n / 2) * words_per_row],
        };
        let basis: Vec<_> = (0..m)
            .map(|i| {
                let x = if i < s { 1 << i } else { 0 };
                (GfRuntime::new(s, x).expect("validated field basis"), i == s)
            })
            .collect();

        for a in 0..n / 2 {
            let field_a = GfRuntime::new(s, a as u16).expect("validated field element");
            let a_squared = field_a.sqr();
            let traces: Vec<_> = basis
                .iter()
                .map(|&(x, _)| field_a.mul(x).trace().value() == 1)
                .collect();
            let mut monomials = Vec::new();
            for (i, &(x, u)) in basis.iter().enumerate() {
                for (j, &(y, v)) in basis.iter().enumerate().skip(i + 1) {
                    // B_a((x,u),(y,v)); keep only the strict upper triangle.
                    let coefficient = (a_squared.mul(x).mul(y).trace().value() == 1)
                        ^ (traces[i] & traces[j])
                        ^ (u & traces[j])
                        ^ (v & traces[i]);
                    if coefficient {
                        monomials.push((1usize << i) | (1usize << j));
                    }
                }
            }
            for coordinate in 0..n {
                let bit = monomials
                    .iter()
                    .fold(false, |sum, &mask| sum ^ (coordinate & mask == mask));
                if bit {
                    table.words[a * words_per_row + coordinate / 64] |= 1u64 << (coordinate % 64);
                }
            }
        }
        table
    }

    pub(crate) fn bit(&self, a: usize, coordinate: usize) -> bool {
        self.words[a * self.words_per_row + coordinate / 64] & (1u64 << (coordinate % 64)) != 0
    }
}
