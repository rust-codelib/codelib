//! Параметры двоичного кода Рида–Маллера.

use crate::RmError;

/// Максимальное значение параметра `m`.
///
/// Ограничение выбрано так, чтобы длина кода `2^m` не превышала
/// [`MAX_CODE_LENGTH`] = 4096, согласованный с пределом оси матрицы
/// в остальном workspace.
pub const MAX_M: usize = 12;

/// Максимальная длина кодового слова `2^MAX_M`.
pub const MAX_CODE_LENGTH: usize = 1 << MAX_M;

/// Параметры кода `RM(r, m)`.
///
/// Поля закрыты: значение создаётся только через [`RmConfig::new`],
/// который проверяет ограничения.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RmConfig {
    r: usize,
    m: usize,
}

impl RmConfig {
    /// Создаёт параметры кода `RM(r, m)`.
    ///
    /// # Ошибки
    ///
    /// Возвращает [`RmError::InvalidParameters`], если `m` не находится в
    /// диапазоне `1..=12` или если `r > m`.
    pub fn new(r: usize, m: usize) -> Result<Self, RmError> {
        if !(1..=MAX_M).contains(&m) || r > m {
            return Err(RmError::InvalidParameters { r, m });
        }

        Ok(Self { r, m })
    }

    /// Возвращает порядок кода `r`.
    #[must_use]
    pub const fn r(&self) -> usize {
        self.r
    }

    /// Возвращает параметр длины `m`.
    #[must_use]
    pub const fn m(&self) -> usize {
        self.m
    }

    /// Возвращает длину кодового слова `n = 2^m`.
    #[must_use]
    pub const fn length(&self) -> usize {
        1usize << self.m
    }

    /// Возвращает размерность кода `k = Σ_{i=0}^{r} C(m, i)`.
    #[must_use]
    pub fn dimension(&self) -> usize {
        (0..=self.r).map(|i| binomial(self.m, i)).sum()
    }

    /// Возвращает минимальное расстояние `d = 2^(m-r)`.
    #[must_use]
    pub const fn distance(&self) -> usize {
        1usize << (self.m - self.r)
    }

    /// Возвращает гарантированно исправляемое число ошибок
    /// `t = (d - 1) / 2`.
    #[must_use]
    pub const fn correctable_errors(&self) -> usize {
        (self.distance() - 1) / 2
    }

    /// Возвращает число обнаруживаемых ошибок `d - 1`.
    #[must_use]
    pub const fn detectable_errors(&self) -> usize {
        self.distance() - 1
    }
}

/// Биномиальный коэффициент `C(n, k)` для небольших `n`.
pub(crate) fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }

    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{binomial, RmConfig, MAX_CODE_LENGTH, MAX_M};
    use crate::RmError;

    #[test]
    fn rejects_invalid_parameters() {
        assert_eq!(
            RmConfig::new(0, 0),
            Err(RmError::InvalidParameters { r: 0, m: 0 })
        );
        assert_eq!(
            RmConfig::new(0, MAX_M + 1),
            Err(RmError::InvalidParameters { r: 0, m: MAX_M + 1 })
        );
        assert_eq!(
            RmConfig::new(3, 2),
            Err(RmError::InvalidParameters { r: 3, m: 2 })
        );
    }

    #[test]
    fn accepts_boundary_parameters() {
        let repetition = RmConfig::new(0, 1).unwrap();
        assert_eq!(repetition.length(), 2);
        assert_eq!(repetition.dimension(), 1);
        assert_eq!(repetition.distance(), 2);
        assert_eq!(repetition.correctable_errors(), 0);
        assert_eq!(repetition.detectable_errors(), 1);

        let full = RmConfig::new(MAX_M, MAX_M).unwrap();
        assert_eq!(full.length(), MAX_CODE_LENGTH);
        assert_eq!(full.dimension(), MAX_CODE_LENGTH);
        assert_eq!(full.distance(), 1);
        assert_eq!(full.correctable_errors(), 0);
    }

    #[test]
    fn dimensions_match_known_values() {
        let rm14 = RmConfig::new(1, 4).unwrap();
        assert_eq!(rm14.length(), 16);
        assert_eq!(rm14.dimension(), 5);
        assert_eq!(rm14.distance(), 8);
        assert_eq!(rm14.correctable_errors(), 3);
        assert_eq!(rm14.detectable_errors(), 7);

        let rm24 = RmConfig::new(2, 4).unwrap();
        assert_eq!(rm24.dimension(), 11);
        assert_eq!(rm24.distance(), 4);
        assert_eq!(rm24.correctable_errors(), 1);
    }

    #[test]
    fn binomial_handles_edges_and_symmetry() {
        assert_eq!(binomial(0, 0), 1);
        assert_eq!(binomial(5, 0), 1);
        assert_eq!(binomial(5, 5), 1);
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(5, 3), 10);
        assert_eq!(binomial(5, 6), 0);
    }
}
