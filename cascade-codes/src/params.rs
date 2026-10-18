//! Параметры системы `(n, k, d; mu)` и формулы теоремы 1.

use crate::subsets::binomial;
use crate::CascadeError;

/// Наибольшее число узлов `n`: матрица кодера имеет `n` строк.
pub const MAX_NODES: usize = gf_linalg::MAX_MATRIX_DIM;

/// Наибольший размер узла `alpha`: супер-матрица сообщения имеет `alpha`
/// столбцов. Тот же предел действует для числа столбцов матриц восстановления.
pub const MAX_ALPHA: usize = gf_linalg::MAX_MATRIX_DIM;

/// Проверенные параметры каскадного кода `(n, k, d; mu)`.
///
/// - `n` — число узлов хранения;
/// - `k` — любые `k` узлов позволяют восстановить файл;
/// - `d` — число помощников при восстановлении одного узла;
/// - `mu` — режим (точка компромисса): `1` даёт MBR, `k` даёт MSR.
///
/// ```rust
/// use cascade_codes::CascadeParams;
///
/// let params = CascadeParams::try_new(8, 4, 6, 4)?;
/// assert_eq!((params.alpha(), params.beta(), params.file_size()), (81, 27, 324));
/// # Ok::<(), cascade_codes::CascadeError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CascadeParams {
    n: usize,
    k: usize,
    d: usize,
    mu: usize,
    alpha: usize,
    beta: usize,
    file_size: usize,
}

impl CascadeParams {
    /// Проверяет параметры и вычисляет `(alpha, beta, F)` по формулам (3).
    ///
    /// # Ошибки
    ///
    /// - [`CascadeError::InvalidSystem`], если нарушено `1 <= k <= d < n`;
    /// - [`CascadeError::InvalidMode`], если нарушено `1 <= mu <= k`;
    /// - [`CascadeError::SizeLimitExceeded`], если `n > MAX_NODES` или
    ///   `alpha > MAX_ALPHA`.
    pub fn try_new(n: usize, k: usize, d: usize, mu: usize) -> Result<Self, CascadeError> {
        if k == 0 || k > d || d >= n {
            return Err(CascadeError::InvalidSystem { n, k, d });
        }
        if mu == 0 || mu > k {
            return Err(CascadeError::InvalidMode { mu, k });
        }
        if n > MAX_NODES {
            return Err(CascadeError::SizeLimitExceeded {
                what: "число узлов n",
                actual: n as u128,
                max: MAX_NODES,
            });
        }
        let alpha = alpha_formula(k, d, mu);
        if alpha > MAX_ALPHA as u128 {
            return Err(CascadeError::SizeLimitExceeded {
                what: "размер узла alpha",
                actual: alpha,
                max: MAX_ALPHA,
            });
        }
        // beta <= alpha и F <= k * alpha, поэтому значения помещаются в usize.
        let beta = beta_formula(k, d, mu) as usize;
        let file_size = file_size_formula(k, d, mu) as usize;
        Ok(Self {
            n,
            k,
            d,
            mu,
            alpha: alpha as usize,
            beta,
            file_size,
        })
    }

    /// Число узлов `n`.
    #[must_use]
    pub fn n(&self) -> usize {
        self.n
    }

    /// Число узлов `k` для восстановления данных.
    #[must_use]
    pub fn k(&self) -> usize {
        self.k
    }

    /// Число помощников `d` для восстановления узла.
    #[must_use]
    pub fn d(&self) -> usize {
        self.d
    }

    /// Режим `mu`.
    #[must_use]
    pub fn mu(&self) -> usize {
        self.mu
    }

    /// Число символов поля, хранимых одним узлом.
    #[must_use]
    pub fn alpha(&self) -> usize {
        self.alpha
    }

    /// Число символов, которое каждый помощник передаёт при восстановлении.
    #[must_use]
    pub fn beta(&self) -> usize {
        self.beta
    }

    /// Число информационных символов `F` в одном блоке (страйпе).
    #[must_use]
    pub fn file_size(&self) -> usize {
        self.file_size
    }

    /// `true` для режима минимальной полосы восстановления (`mu = 1`).
    #[must_use]
    pub fn is_mbr(&self) -> bool {
        self.mu == 1
    }

    /// `true` для режима минимального объёма хранения (`mu = k`).
    #[must_use]
    pub fn is_msr(&self) -> bool {
        self.mu == self.k
    }

    /// Все допустимые точки компромисса `mu = 1..=k` для `(n, k, d)`.
    ///
    /// Точки, нарушающие пределы размера, пропускаются.
    ///
    /// # Ошибки
    ///
    /// [`CascadeError::InvalidSystem`] или ошибка предела `n`.
    pub fn tradeoff(n: usize, k: usize, d: usize) -> Result<Vec<Self>, CascadeError> {
        // Режим mu = 1 всегда помещается в предел alpha (alpha = d < n).
        let mut points = vec![Self::try_new(n, k, d, 1)?];
        for mu in 2..=k {
            if alpha_formula(k, d, mu) <= MAX_ALPHA as u128 {
                points.push(Self::try_new(n, k, d, mu)?);
            }
        }
        Ok(points)
    }
}

fn pow(base: usize, exp: usize) -> u128 {
    let mut acc: u128 = 1;
    for _ in 0..exp {
        acc = acc.saturating_mul(base as u128);
    }
    acc
}

/// `alpha = Σ_{m=0}^{mu} (d-k)^{mu-m} C(k, m)`.
pub(crate) fn alpha_formula(k: usize, d: usize, mu: usize) -> u128 {
    (0..=mu).fold(0u128, |acc, m| {
        acc.saturating_add(pow(d - k, mu - m).saturating_mul(binomial(k, m)))
    })
}

/// `beta = Σ_{m=0}^{mu} (d-k)^{mu-m} C(k-1, m-1)`, где `C(·, -1) = 0`.
pub(crate) fn beta_formula(k: usize, d: usize, mu: usize) -> u128 {
    (1..=mu).fold(0u128, |acc, m| {
        acc.saturating_add(pow(d - k, mu - m).saturating_mul(binomial(k - 1, m - 1)))
    })
}

/// `F = Σ_{m=0}^{mu} k (d-k)^{mu-m} C(k, m) - C(k, mu+1)`.
pub(crate) fn file_size_formula(k: usize, d: usize, mu: usize) -> u128 {
    alpha_formula(k, d, mu)
        .saturating_mul(k as u128)
        .saturating_sub(binomial(k, mu + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_example_of_paper() {
        let p = CascadeParams::try_new(8, 4, 6, 4).unwrap();
        assert_eq!((p.alpha(), p.beta(), p.file_size()), (81, 27, 324));
        assert!(p.is_msr());
    }

    #[test]
    fn corollary_1_mbr_and_msr() {
        for (n, k, d) in [(5, 2, 3), (8, 4, 6), (9, 3, 7), (6, 5, 5)] {
            let mbr = CascadeParams::try_new(n, k, d, 1).unwrap();
            assert_eq!(mbr.alpha(), d);
            assert_eq!(mbr.beta(), 1);
            assert_eq!(mbr.file_size(), k * (2 * d - k + 1) / 2);

            let msr = CascadeParams::try_new(n, k, d, k).unwrap();
            let s = d - k + 1;
            assert_eq!(msr.alpha(), s.pow(k as u32));
            assert_eq!(msr.beta(), s.pow(k as u32 - 1));
            assert_eq!(msr.file_size(), k * s.pow(k as u32));
        }
    }

    #[test]
    fn fig_2_parameters() {
        // (n >= 7, k = 4, d = 6), mu = 2: (alpha, beta) = (18, 5), F = 68.
        let p = CascadeParams::try_new(7, 4, 6, 2).unwrap();
        assert_eq!((p.alpha(), p.beta(), p.file_size()), (18, 5, 68));
    }

    #[test]
    fn rejects_bad_parameters() {
        assert_eq!(
            CascadeParams::try_new(5, 0, 3, 1),
            Err(CascadeError::InvalidSystem { n: 5, k: 0, d: 3 })
        );
        assert_eq!(
            CascadeParams::try_new(5, 4, 3, 1),
            Err(CascadeError::InvalidSystem { n: 5, k: 4, d: 3 })
        );
        assert_eq!(
            CascadeParams::try_new(5, 2, 5, 1),
            Err(CascadeError::InvalidSystem { n: 5, k: 2, d: 5 })
        );
        assert_eq!(
            CascadeParams::try_new(5, 2, 3, 3),
            Err(CascadeError::InvalidMode { mu: 3, k: 2 })
        );
        assert_eq!(
            CascadeParams::try_new(5, 2, 3, 0),
            Err(CascadeError::InvalidMode { mu: 0, k: 2 })
        );
        assert!(matches!(
            CascadeParams::try_new(40, 12, 30, 12),
            Err(CascadeError::SizeLimitExceeded { .. })
        ));
    }

    #[test]
    fn tradeoff_lists_all_modes() {
        let points = CascadeParams::tradeoff(8, 4, 6).unwrap();
        let modes: Vec<usize> = points.iter().map(CascadeParams::mu).collect();
        assert_eq!(modes, vec![1, 2, 3, 4]);
        // Чем выше режим, тем меньше хранится на единицу данных.
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.alpha() * b.file_size() >= b.alpha() * a.file_size());
        }
    }

    #[test]
    fn corollary_2_cut_set_bound_at_mu_k_minus_1() {
        // F = Σ_{i=1}^{k} min(alpha, (d - i + 1) beta) для mu = k - 1.
        for (k, d) in [(3, 4), (4, 6), (4, 5), (5, 7)] {
            let p = CascadeParams::try_new(d + 1, k, d, k - 1).unwrap();
            let bound: usize = (1..=k).map(|i| p.alpha().min((d - i + 1) * p.beta())).sum();
            assert_eq!(p.file_size(), bound, "k = {k}, d = {d}");
        }
    }
}
