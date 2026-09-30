//! Вектор и основные операции над элементами конечного поля.

use gf2m::Gf256;
use std::ops::{Add, Mul, Sub};

use crate::FieldElement;

/// Вектор элементов поля `F`, сохраняющий их порядок и хвостовые нули.
///
/// Пустой вектор допустим. Отдельных ограничений длины или объёма данных нет.
/// Данные закрыты: через публичные методы можно менять элементы, сохраняя длину.
/// Сложение и вычитание, а также умножение на скаляр создают новые векторы;
/// точная длина и нулевые элементы в конце сохраняются. Равенство сравнивает
/// также длину.
/// Параметр поля по умолчанию — `gf2m::Gf256`.
///
/// ```
/// use gf_linalg::Vector;
/// use gf2m::Gf256;
///
/// let data = vec![Gf256::new(7), Gf256::zero(), Gf256::zero()];
/// let mut vector = Vector::new(data); // Владение data переходит к vector.
/// assert_eq!(vector.len(), 3);
/// assert_eq!(vector.get(0), Some(Gf256::new(7)));
/// if let Some(element) = vector.get_mut(1) {
///     *element = Gf256::new(5); // Изменяем элемент через ссылку.
/// }
/// assert_eq!(vector.as_slice(), &[Gf256::new(7), Gf256::new(5), Gf256::zero()]);
/// assert_eq!(vector.get(3), None);
/// ```
///
/// Операции требуют одного и того же типа поля. Следующие выражения не
/// компилируются, хотя сложение, вычитание и умножение на скаляр доступны для
/// векторов одного поля:
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::Vector;
/// use gfpm::Gf9;
///
/// let binary = Vector::<Gf256>::new(vec![Gf256::one()]);
/// let ternary = Vector::<Gf9>::new(vec![Gf9::one()]);
/// let _ = &binary + &binary;
/// let _ = binary.try_add(&ternary);
/// let _ = &binary + &ternary;
/// ```
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::Vector;
/// use gfpm::Gf9;
///
/// let binary = Vector::<Gf256>::new(vec![Gf256::one()]);
/// let ternary = Vector::<Gf9>::new(vec![Gf9::one()]);
/// let _ = &binary - &binary;
/// let _ = binary.try_sub(&ternary);
/// let _ = &binary - &ternary;
/// ```
///
/// ```compile_fail
/// use gf2m::Gf256;
/// use gf_linalg::Vector;
/// use gfpm::Gf9;
///
/// let binary = Vector::<Gf256>::new(vec![Gf256::one()]);
/// let _ = &binary * Gf256::one();
/// let _ = &binary * Gf9::one();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Vector<F: FieldElement = Gf256> {
    data: Vec<F>,
}

impl<F: FieldElement> Vector<F> {
    /// Принимает `Vec` во владение без копирования элементов.
    ///
    /// После вызова переданный `Vec` принадлежит вектору; исходная переменная
    /// больше недоступна. Порядок, длина и нулевые элементы сохраняются.
    pub fn new(data: Vec<F>) -> Self {
        Self { data }
    }

    /// Возвращает число элементов, включая нулевые элементы в конце.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Возвращает `true`, если элементов нет.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Возвращает срез — ссылку на все элементы для чтения без копирования.
    ///
    /// Срез не передаёт владение данными и не позволяет менять длину вектора.
    pub fn as_slice(&self) -> &[F] {
        &self.data
    }

    /// Возвращает копию элемента по индексу, начиная с нуля.
    ///
    /// `Option` обозначает наличие значения: `Some(элемент)` для верного
    /// индекса и `None` при выходе за границы, без паники.
    pub fn get(&self, index: usize) -> Option<F> {
        self.data.get(index).copied()
    }

    /// Возвращает изменяемую ссылку на элемент по индексу, начиная с нуля.
    ///
    /// `Some(ссылка)` позволяет заменить элемент через `*ссылка`, сохраняя
    /// длину вектора. Ссылка временно заимствует элемент, не передавая владение.
    /// При выходе за границы возвращает `None` без паники.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut F> {
        self.data.get_mut(index)
    }

    /// Складывает вектор с вектором той же длины по правилам поля `F`.
    ///
    /// Возвращает ошибку с обеими длинами, если они различаются. При успехе
    /// создаётся новый вектор той же длины; исходные векторы не изменяются,
    /// включая нулевые элементы в конце.
    pub fn try_add(&self, rhs: &Self) -> Result<Self, crate::LinalgError> {
        if self.len() != rhs.len() {
            return Err(crate::LinalgError::VectorLengthMismatch {
                left: self.len(),
                right: rhs.len(),
            });
        }

        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(&left, &right)| left + right)
            .collect();

        Ok(Self::new(data))
    }

    /// Вычитает вектор той же длины по правилам поля `F`.
    ///
    /// При несовпадении длин возвращает ошибку с длинами в порядке операндов.
    /// При успехе создаёт новый вектор той же длины; исходные векторы не
    /// изменяются, включая нулевые элементы в конце.
    pub fn try_sub(&self, rhs: &Self) -> Result<Self, crate::LinalgError> {
        if self.len() != rhs.len() {
            return Err(crate::LinalgError::VectorLengthMismatch {
                left: self.len(),
                right: rhs.len(),
            });
        }

        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(&left, &right)| left - right)
            .collect();

        Ok(Self::new(data))
    }

    /// Умножает каждый элемент вектора на скаляр того же поля.
    ///
    /// Создаёт новый вектор той же длины, не меняя исходный. Нули в конце,
    /// пустой вектор и длины больше предела матрицы сохраняются.
    pub fn scale(&self, scalar: F) -> Self {
        let data = self.data.iter().map(|&value| value * scalar).collect();
        Self::new(data)
    }

    /// Объявляет контракт скалярного произведения, которое пока не реализовано.
    ///
    /// На этом этапе каждый вызов возвращает [`crate::LinalgError::NotImplemented`],
    /// в том числе для пустых векторов и векторов разной длины. Ни фиктивное
    /// значение, ни частичный результат не вычисляются.
    pub fn try_dot(&self, _rhs: &Self) -> Result<F, crate::LinalgError> {
        Err(crate::LinalgError::NotImplemented {
            operation: "скалярное произведение векторов",
        })
    }
}

/// Сложение двух принадлежащих операндам векторов.
///
/// Оба вектора передаются оператору во владение; результат имеет тип
/// `Result<Vector, LinalgError>`, поскольку длины могут различаться.
impl<F: FieldElement> Add for Vector<F> {
    type Output = Result<Vector<F>, crate::LinalgError>;

    fn add(self, rhs: Self) -> Self::Output {
        self.try_add(&rhs)
    }
}

/// Сложение заимствованных векторов без передачи владения операндами.
///
/// Как и для owned-формы, результатом является `Result`, так как длины могут
/// различаться.
impl<F: FieldElement> Add<&Vector<F>> for &Vector<F> {
    type Output = Result<Vector<F>, crate::LinalgError>;

    fn add(self, rhs: &Vector<F>) -> Self::Output {
        self.try_add(rhs)
    }
}

/// Вычитание двух принадлежащих операндам векторов.
///
/// Оба вектора передаются оператору во владение; результатом является
/// `Result<Vector<F>, LinalgError>`, поскольку длины могут различаться.
impl<F: FieldElement> Sub for Vector<F> {
    type Output = Result<Vector<F>, crate::LinalgError>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.try_sub(&rhs)
    }
}

/// Вычитание заимствованных векторов без передачи владения операндами.
///
/// Результатом `&left - &right` является `Result<Vector<F>, LinalgError>`;
/// исходные векторы остаются доступными.
impl<F: FieldElement> Sub<&Vector<F>> for &Vector<F> {
    type Output = Result<Vector<F>, crate::LinalgError>;

    fn sub(self, rhs: &Vector<F>) -> Self::Output {
        self.try_sub(rhs)
    }
}

/// Умножение принадлежащего операнду вектора на скаляр того же поля.
impl<F: FieldElement> Mul<F> for Vector<F> {
    type Output = Vector<F>;

    fn mul(self, rhs: F) -> Self::Output {
        self.scale(rhs)
    }
}

/// Умножение заимствованного вектора на скаляр того же поля.
impl<F: FieldElement> Mul<F> for &Vector<F> {
    type Output = Vector<F>;

    fn mul(self, rhs: F) -> Self::Output {
        self.scale(rhs)
    }
}
