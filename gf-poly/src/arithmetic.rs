//! Арифметика с нормализацией результатов и проверкой размеров.

use core::ops::{Add, Div, Mul, Rem, Sub};

use crate::polynomial::check_coefficient_limit;
use crate::{FieldElement, Polynomial, PolynomialError};

impl<F: FieldElement> Polynomial<F> {
    /// Складывает многочлены одного поля, удаляя старшие нули результата.
    ///
    /// ```compile_fail
    /// use gf_poly::Polynomial;
    /// let a = Polynomial::<gf2m::Gf256>::try_new(vec![]).unwrap();
    /// let b = Polynomial::<gfpm::Gf9>::try_new(vec![]).unwrap();
    /// let _ = &a + &b;
    /// ```
    pub fn try_add(&self, rhs: &Self) -> Result<Self, PolynomialError> {
        self.combine(rhs, |left, right| left + right)
    }

    /// Вычитает многочлен того же поля, удаляя старшие нули результата.
    pub fn try_sub(&self, rhs: &Self) -> Result<Self, PolynomialError> {
        self.combine(rhs, |left, right| left - right)
    }

    fn combine(&self, rhs: &Self, operation: impl Fn(F, F) -> F) -> Result<Self, PolynomialError> {
        let left = self.significant_coefficients();
        let right = rhs.significant_coefficients();
        let len = left.len().max(right.len());
        check_coefficient_limit::<F>(len)?;
        let coefficients = (0..len)
            .map(|index| {
                operation(
                    left.get(index).copied().unwrap_or_else(F::zero),
                    right.get(index).copied().unwrap_or_else(F::zero),
                )
            })
            .collect();
        Self::from_computed(coefficients)
    }

    /// Умножает многочлены; предел 128 КиБ проверяется до выделения результата.
    /// Нулевой результат хранится пустым массивом.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, PolynomialError> {
        let left = self.significant_coefficients();
        let right = rhs.significant_coefficients();
        if left.is_empty() || right.is_empty() {
            return Self::try_new(Vec::new());
        }
        let len = left
            .len()
            .checked_add(right.len() - 1)
            .ok_or(PolynomialError::SizeOverflow)?;
        check_coefficient_limit::<F>(len)?;
        let mut coefficients = vec![F::zero(); len];
        for (i, &a) in left.iter().enumerate() {
            for (j, &b) in right.iter().enumerate() {
                coefficients[i + j] = coefficients[i + j] + a * b;
            }
        }
        Self::from_computed(coefficients)
    }

    /// Делит с остатком: `self = divisor * quotient + remainder`.
    /// Степень ненулевого остатка меньше степени делителя.
    /// Нулевой делитель возвращает [`PolynomialError::DivisionByZero`].
    pub fn try_div_rem(&self, divisor: &Self) -> Result<(Self, Self), PolynomialError> {
        let dividend = self.significant_coefficients();
        let divisor = divisor.significant_coefficients();
        let Some(&leading) = divisor.last() else {
            return Err(PolynomialError::DivisionByZero);
        };
        if dividend.len() < divisor.len() {
            return Ok((Self::try_new(Vec::new())?, Self::try_from_slice(dividend)?));
        }

        let quotient_len = dividend.len() - divisor.len() + 1;
        check_coefficient_limit::<F>(quotient_len)?;
        check_coefficient_limit::<F>(dividend.len())?;
        let mut quotient = vec![F::zero(); quotient_len];
        let mut remainder = dividend.to_vec();
        let inverse = leading.inv();
        while remainder.len() >= divisor.len() {
            let shift = remainder.len() - divisor.len();
            let factor = remainder[remainder.len() - 1] * inverse;
            quotient[shift] = factor;
            for (index, &coefficient) in divisor.iter().enumerate() {
                remainder[shift + index] = remainder[shift + index] - factor * coefficient;
            }
            while remainder.last().is_some_and(|value| value.is_zero()) {
                remainder.pop();
            }
        }
        Ok((
            Self::from_computed(quotient)?,
            Self::from_computed(remainder)?,
        ))
    }

    /// Возвращает частное евклидова деления, отбрасывая остаток.
    pub fn try_div(&self, divisor: &Self) -> Result<Self, PolynomialError> {
        self.try_div_rem(divisor).map(|(quotient, _)| quotient)
    }

    /// Возвращает остаток евклидова деления.
    pub fn try_rem(&self, divisor: &Self) -> Result<Self, PolynomialError> {
        self.try_div_rem(divisor).map(|(_, remainder)| remainder)
    }

    /// Вычисляет gcd алгоритмом Евклида со старшим коэффициентом 1.
    /// Для двух нулевых многочленов возвращает нулевой многочлен.
    pub fn try_gcd(&self, rhs: &Self) -> Result<Self, PolynomialError> {
        let mut left = Self::try_from_slice(self.significant_coefficients())?;
        let mut right = Self::try_from_slice(rhs.significant_coefficients())?;
        while !right.is_zero() {
            let remainder = left.try_rem(&right)?;
            left = right;
            right = remainder;
        }
        let Some(&leading) = left.coefficients().last() else {
            return Ok(left);
        };
        let inverse = leading.inv();
        Self::from_computed(
            left.coefficients()
                .iter()
                .map(|&value| value * inverse)
                .collect(),
        )
    }

    /// Подставляет элемент того же поля по схеме Горнера, без выделения памяти.
    /// Для нулевого многочлена возвращает нуль поля.
    ///
    /// ```compile_fail
    /// use gf_poly::Polynomial;
    /// let a = Polynomial::<gf2m::Gf256>::try_new(vec![]).unwrap();
    /// let _ = a.evaluate(gfpm::Gf9::one());
    /// ```
    pub fn evaluate(&self, x: F) -> F {
        self.significant_coefficients()
            .iter()
            .rev()
            .fold(F::zero(), |value, &coefficient| value * x + coefficient)
    }
}

macro_rules! polynomial_operator {
    ($trait:ident, $method:ident, $try_method:ident) => {
        impl<F: FieldElement> $trait for Polynomial<F> {
            type Output = Result<Self, PolynomialError>;

            fn $method(self, rhs: Self) -> Self::Output {
                self.$try_method(&rhs)
            }
        }

        impl<F: FieldElement> $trait<&Polynomial<F>> for &Polynomial<F> {
            type Output = Result<Polynomial<F>, PolynomialError>;

            fn $method(self, rhs: &Polynomial<F>) -> Self::Output {
                self.$try_method(rhs)
            }
        }
    };
}

polynomial_operator!(Add, add, try_add);
polynomial_operator!(Sub, sub, try_sub);
polynomial_operator!(Mul, mul, try_mul);
polynomial_operator!(Div, div, try_div);
polynomial_operator!(Rem, rem, try_rem);
