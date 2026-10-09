//! Преобразования между битами и векторами над GF(2).

use gf_linalg::Vector;
use gfpm::Gf;

use crate::{Bit, LdpcError};

/// Тип элемента простого поля GF(2).
pub type Gf2 = Gf<2, 1, 1>;

/// Преобразует биты в вектор над GF(2), сохраняя порядок и длину.
///
/// Каждый [`Bit::Zero`] становится нулём поля, а каждый [`Bit::One`] —
/// единицей. Пустой вход и нулевые элементы в конце сохраняются.
///
/// ```rust
/// use ldpc_codes::{bits_to_vector, vector_to_bits, Bit};
///
/// let bits = [Bit::One, Bit::Zero, Bit::One, Bit::Zero];
/// let vector = bits_to_vector(&bits);
/// assert_eq!(vector_to_bits(&vector), Ok(bits.to_vec()));
/// assert_eq!(vector.len(), 4);
/// ```
#[must_use]
pub fn bits_to_vector(bits: &[Bit]) -> Vector<Gf2> {
    let elements = bits
        .iter()
        .map(|bit| match bit {
            Bit::Zero => Gf2::zero(),
            Bit::One => Gf2::one(),
        })
        .collect();

    Vector::new(elements)
}

/// Преобразует вектор над GF(2) в биты, проверяя каждый элемент.
///
/// Возвращает `InvalidFieldElement` с индексом и упакованным значением, если
/// элемент не равен настоящему нулю или единице поля. Такая проверка также
/// ловит неканонические значения, созданные в release-сборке конструктором
/// `Gf::from_coeffs`. Исходный вектор не меняется, его длина и порядок
/// сохраняются.
///
/// ```rust
/// use gf_linalg::Vector;
/// use ldpc_codes::{vector_to_bits, Bit, Gf2};
///
/// let vector = Vector::<Gf2>::new(vec![Gf2::one(), Gf2::zero()]);
/// assert_eq!(vector_to_bits(&vector), Ok(vec![Bit::One, Bit::Zero]));
/// ```
pub fn vector_to_bits(vector: &Vector<Gf2>) -> Result<Vec<Bit>, LdpcError> {
    let zero = Gf2::zero();
    let one = Gf2::one();

    vector
        .as_slice()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, element)| {
            if element == zero {
                Ok(Bit::Zero)
            } else if element == one {
                Ok(Bit::One)
            } else {
                Err(LdpcError::InvalidFieldElement {
                    index,
                    value: element.value(),
                })
            }
        })
        .collect()
}
