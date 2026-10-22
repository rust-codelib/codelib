//! Тип двоичного бита, его преобразования и диагностика различий.

use gf_linalg::Vector;
use gfpm::Gf;

use crate::LdpcError;

/// Один бит двоичного значения.
///
/// При преобразовании в `u8` вариант [`Zero`](Self::Zero) становится `0`, а
/// [`One`](Self::One) — `1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bit {
    /// Нулевой бит.
    Zero,
    /// Единичный бит.
    One,
}

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

/// Считает число различающихся позиций в слове и эталоне.
///
/// Функция не зависит от проверочной матрицы и подходит для сравнения как
/// кодовых слов, так и информационных сообщений. Пустые последовательности
/// дают `0`; входные данные остаются неизменными.
///
/// Это сравнение с известным эталоном, которого декодер обычно не знает:
/// [`crate::DecodeStatus::ParitySatisfied`] означает только допустимость слова,
/// а [`crate::DecodeResult::changed_bits`] считает изменения от начального
/// решения. Ни один из этих показателей не сообщает, сколько битов совпадает с
/// переданным словом.
///
/// # Ошибки
///
/// Возвращает [`LdpcError::ReferenceLengthMismatch`], если длины различаются.
/// В таком случае `expected` равен `word.len()`, а `actual` равен
/// `reference.len()`.
pub fn count_bit_errors(word: &[Bit], reference: &[Bit]) -> Result<usize, LdpcError> {
    if word.len() != reference.len() {
        return Err(LdpcError::ReferenceLengthMismatch {
            expected: word.len(),
            actual: reference.len(),
        });
    }

    Ok(bit_distance(word, reference))
}

pub(crate) fn bit_parity(bits: impl IntoIterator<Item = Bit>) -> Bit {
    let is_odd = bits
        .into_iter()
        .fold(false, |parity, bit| parity ^ (bit == Bit::One));

    if is_odd {
        Bit::One
    } else {
        Bit::Zero
    }
}

pub(crate) fn count_ones(bits: &[Bit]) -> usize {
    bits.iter().filter(|&&bit| bit == Bit::One).count()
}

pub(crate) fn bit_distance(left: &[Bit], right: &[Bit]) -> usize {
    debug_assert_eq!(left.len(), right.len());
    left.iter()
        .zip(right)
        .filter(|(left, right)| left != right)
        .count()
}

impl TryFrom<u8> for Bit {
    type Error = LdpcError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            value => Err(LdpcError::InvalidBit { value }),
        }
    }
}

impl From<Bit> for u8 {
    fn from(bit: Bit) -> Self {
        match bit {
            Bit::Zero => 0,
            Bit::One => 1,
        }
    }
}
