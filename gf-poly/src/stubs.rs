//! Контракты интерполяции и гомоморфизмов для последующей реализации.

use crate::{FieldElement, Polynomial, PolynomialError};

impl<F: FieldElement> Polynomial<F> {
    /// Заглушка интерполяции по точкам `(x, y)` одного поля.
    /// Всегда возвращает [`PolynomialError::NotImplemented`].
    pub fn try_interpolate(_points: &[(F, F)]) -> Result<Self, PolynomialError> {
        Err(PolynomialError::NotImplemented {
            operation: "интерполяция многочлена",
        })
    }

    /// Заглушка гомоморфизма `F[x] → G[x]` с отображлением коэффициентов и образом `x`.
    /// Вызывающий код отвечает за то, что `map_coefficient` — гомоморфизм полей.
    /// Всегда возвращает [`PolynomialError::NotImplemented`], не вызывая отображление.
    pub fn try_homomorphism<G: FieldElement>(
        &self,
        _map_coefficient: impl Fn(F) -> G,
        _image_of_x: &Polynomial<G>,
    ) -> Result<Polynomial<G>, PolynomialError> {
        Err(PolynomialError::NotImplemented {
            operation: "гомоморфизм многочленов",
        })
    }
}
