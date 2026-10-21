//! Хранение и равенство формальных многочленов.

use core::mem::size_of;

use gf2m::Gf256;

use crate::{FieldElement, PolynomialError, MAX_POLYNOMIAL_BYTES};

pub(crate) fn check_coefficient_limit<F: FieldElement>(
    coefficients: usize,
) -> Result<(), PolynomialError> {
    let max_bytes = MAX_POLYNOMIAL_BYTES.min(isize::MAX as usize);
    let max_coefficients = max_bytes.checked_div(size_of::<F>()).unwrap_or(usize::MAX);
    if coefficients > max_coefficients {
        return Err(PolynomialError::CoefficientLimitExceeded {
            coefficients,
            max_coefficients,
        });
    }
    Ok(())
}

/// Формальный многочлен над одним статическим конечным полем `F`.
///
/// Коэффициент `i` относится к `x^i`. Хвостовые нули хранятся, но при
/// сравнении игнорируются: `[a] == [a, 0]`. Пустой массив задаёт нулевой
/// многочлен. Сравниваются формальные многочлены, а не функции на поле.
///
/// Конструкторы проверяют [`MAX_POLYNOMIAL_BYTES`] до копирования массива.
/// Допустимость элементов и неприводимость модуля задаёт контракт
/// [`FieldElement`]; повторной проверки поля здесь нет.
///
/// Поле входит в тип многочлена. Сравнение разных полей не компилируется:
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gfpm::Gf9;
/// use gf_poly::Polynomial;
///
/// let binary = Polynomial::try_new(vec![Gf256::one()]).unwrap();
/// let ternary = Polynomial::try_new(vec![Gf9::one()]).unwrap();
/// let _ = binary == ternary;
/// ```
///
/// Разные модули при одинаковом порядке поля также задают разные типы:
///
/// ```compile_fail
/// use gf2m::Gf;
/// use gf_poly::Polynomial;
///
/// let first = Polynomial::try_new(vec![Gf::<16, 0x13>::one()]).unwrap();
/// let second = Polynomial::try_new(vec![Gf::<16, 0x19>::one()]).unwrap();
/// let _ = first == second;
/// ```
///
/// Нельзя подменить поле при создании массива:
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gfpm::Gf9;
/// use gf_poly::Polynomial;
///
/// let _ = Polynomial::<Gf256>::try_new(vec![Gf9::one()]);
/// ```
#[derive(Clone, Debug)]
pub struct Polynomial<F: FieldElement = Gf256> {
    coefficients: Box<[F]>,
}

impl<F: FieldElement> Polynomial<F> {
    /// Принимает массив, сохраняя длину и хвостовые нули, и убирает запас ёмкости.
    ///
    /// Возвращает [`PolynomialError::CoefficientLimitExceeded`] при превышении
    /// предела по длине массива (не ёмкости `Vec`), в том числе в release.
    pub fn try_new(coefficients: Vec<F>) -> Result<Self, PolynomialError> {
        check_coefficient_limit::<F>(coefficients.len())?;
        Ok(Self {
            coefficients: coefficients.into_boxed_slice(),
        })
    }

    /// Копирует срез целиком; [`PolynomialError::CoefficientLimitExceeded`]
    /// возвращается до выделения выходного массива при превышении предела.
    pub fn try_from_slice(coefficients: &[F]) -> Result<Self, PolynomialError> {
        check_coefficient_limit::<F>(coefficients.len())?;
        Ok(Self {
            coefficients: coefficients.into(),
        })
    }

    /// Возвращает все сохранённые коэффициенты, включая хвостовые нули.
    pub fn coefficients(&self) -> &[F] {
        &self.coefficients
    }

    /// Передаёт массив коэффициентов вызывающему коду без копирования.
    pub fn into_coefficients(self) -> Box<[F]> {
        self.coefficients
    }

    /// Возвращает индекс старшего ненулевого коэффициента.
    ///
    /// Для пустого массива и любого массива из одних нулей возвращает `None`.
    pub fn degree(&self) -> Option<usize> {
        self.coefficients
            .iter()
            .rposition(|coefficient| !coefficient.is_zero())
    }

    /// Проверяет, что все коэффициенты равны нулю; пустой массив тоже нулевой.
    pub fn is_zero(&self) -> bool {
        self.degree().is_none()
    }

    fn significant_coefficients(&self) -> &[F] {
        self.degree()
            .map_or(&[], |degree| &self.coefficients[..=degree])
    }
}

/// Проверяет размер и принимает массив без копирования элементов.
impl<F: FieldElement> TryFrom<Box<[F]>> for Polynomial<F> {
    type Error = PolynomialError;

    fn try_from(coefficients: Box<[F]>) -> Result<Self, Self::Error> {
        check_coefficient_limit::<F>(coefficients.len())?;
        Ok(Self { coefficients })
    }
}

/// Принимает коэффициенты во владение с теми же проверками, что и `try_new`.
impl<F: FieldElement> TryFrom<Vec<F>> for Polynomial<F> {
    type Error = PolynomialError;

    fn try_from(coefficients: Vec<F>) -> Result<Self, Self::Error> {
        Self::try_new(coefficients)
    }
}

impl<F: FieldElement> PartialEq for Polynomial<F> {
    fn eq(&self, other: &Self) -> bool {
        self.significant_coefficients() == other.significant_coefficients()
    }
}

impl<F: FieldElement> Eq for Polynomial<F> {}
