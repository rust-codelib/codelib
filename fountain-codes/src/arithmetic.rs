use crate::{BlockDescriptor, FountainError, Profile};
use gf2m::{Gf256, Gf65536};
use gf_linalg::{EquationInsertion, FieldElement, LinearSystem, Matrix};

pub(crate) trait WireField: FieldElement {
    const WIDTH: usize;
    fn from_word(value: u16) -> Self;
    fn word(self) -> u16;
}
impl WireField for Gf65536 {
    const WIDTH: usize = 2;
    fn from_word(value: u16) -> Self {
        Self::new(value)
    }
    fn word(self) -> u16 {
        self.value()
    }
}
impl WireField for Gf256 {
    const WIDTH: usize = 1;
    fn from_word(value: u16) -> Self {
        Self::new(value)
    }
    fn word(self) -> u16 {
        self.value()
    }
}
pub(crate) fn unpack<F: WireField>(bytes: &[u8]) -> Vec<F> {
    bytes
        .chunks(F::WIDTH)
        .map(|c| {
            F::from_word(if F::WIDTH == 1 {
                c[0] as u16
            } else {
                u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)])
            })
        })
        .collect()
}
pub(crate) fn pack<F: WireField>(values: &[F]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * F::WIDTH);
    for value in values {
        bytes.push(value.word() as u8);
        if F::WIDTH == 2 {
            bytes.push((value.word() >> 8) as u8);
        }
    }
    bytes
}
fn source<F: WireField>(d: &BlockDescriptor, message: &[u8]) -> Vec<F> {
    (0..d.k * d.elements_per_symbol())
        .map(|i| {
            let at = i * F::WIDTH;
            let lo = message.get(at).copied().unwrap_or(0) as u16;
            let hi = if F::WIDTH == 2 {
                message.get(at + 1).copied().unwrap_or(0) as u16
            } else {
                0
            };
            F::from_word(lo | (hi << 8))
        })
        .collect()
}
fn weighted<F: WireField>(x: &[F], l: usize, support: &[(usize, u16)]) -> Vec<u8> {
    let mut out = vec![F::zero(); l];
    for &(i, a) in support {
        for (y, &v) in out.iter_mut().zip(&x[i * l..(i + 1) * l]) {
            *y = *y + F::from_word(a) * v;
        }
    }
    pack(&out)
}

#[derive(Debug)]
pub(crate) enum FieldData {
    F16(Vec<Gf65536>),
    F8(Vec<Gf256>),
}
impl FieldData {
    pub(crate) fn source(d: &BlockDescriptor, message: &[u8]) -> Self {
        match d.profile {
            Profile::Gf65536 => Self::F16(source(d, message)),
            Profile::Gf256 => Self::F8(source(d, message)),
        }
    }
    pub(crate) fn weighted(&self, l: usize, support: &[(usize, u16)]) -> Vec<u8> {
        match self {
            Self::F16(x) => weighted(x, l, support),
            Self::F8(x) => weighted(x, l, support),
        }
    }
}

#[derive(Debug)]
pub(crate) enum System {
    F16(LinearSystem<Gf65536>),
    F8(LinearSystem<Gf256>),
}
#[derive(Debug)]
pub(crate) enum Solution {
    F16(Matrix<Gf65536>),
    F8(Matrix<Gf256>),
}
impl System {
    pub(crate) fn new(d: &BlockDescriptor) -> Result<Self, FountainError> {
        Ok(match d.profile {
            Profile::Gf65536 => Self::F16(LinearSystem::try_new(d.k, d.elements_per_symbol())?),
            Profile::Gf256 => Self::F8(LinearSystem::try_new(d.k, d.elements_per_symbol())?),
        })
    }
    pub(crate) fn rank(&self) -> usize {
        match self {
            Self::F16(s) => s.rank(),
            Self::F8(s) => s.rank(),
        }
    }
    pub(crate) fn insert(
        &mut self,
        k: usize,
        support: &[(usize, u16)],
        bytes: &[u8],
    ) -> Result<EquationInsertion, FountainError> {
        fn insert<F: WireField>(
            s: &mut LinearSystem<F>,
            k: usize,
            support: &[(usize, u16)],
            bytes: &[u8],
        ) -> Result<EquationInsertion, FountainError> {
            let mut a = vec![F::zero(); k];
            for &(i, c) in support {
                a[i] = F::from_word(c);
            }
            Ok(s.insert_equation(&a, &unpack::<F>(bytes))?)
        }
        match self {
            Self::F16(s) => insert(s, k, support, bytes),
            Self::F8(s) => insert(s, k, support, bytes),
        }
    }
    pub(crate) fn solution(&self) -> Result<Option<Solution>, FountainError> {
        Ok(match self {
            Self::F16(s) => s.try_solution()?.map(Solution::F16),
            Self::F8(s) => s.try_solution()?.map(Solution::F8),
        })
    }
}
impl Solution {
    pub(crate) fn weighted(&self, l: usize, support: &[(usize, u16)]) -> Vec<u8> {
        match self {
            Self::F16(m) => weighted(m.as_slice(), l, support),
            Self::F8(m) => weighted(m.as_slice(), l, support),
        }
    }
    pub(crate) fn message(&self, n: usize) -> Vec<u8> {
        fn message<F: WireField>(x: &[F], n: usize) -> Vec<u8> {
            // Allocate exactly N; padding remains in the solution, not this output.
            (0..n)
                .map(|i| (x[i / F::WIDTH].word() >> (8 * (i % F::WIDTH))) as u8)
                .collect()
        }
        match self {
            Self::F16(m) => message(m.as_slice(), n),
            Self::F8(m) => message(m.as_slice(), n),
        }
    }
}

pub(crate) fn inverse(profile: Profile, a: u16) -> u16 {
    match profile {
        Profile::Gf65536 => Gf65536::new(a).inv().value(),
        Profile::Gf256 => Gf256::new(a).inv().value(),
    }
}
pub(crate) fn negative_product(profile: Profile, a: u16, b: u16) -> u16 {
    match profile {
        Profile::Gf65536 => (-Gf65536::new(a) * Gf65536::new(b)).value(),
        Profile::Gf256 => (-Gf256::new(a) * Gf256::new(b)).value(),
    }
}
/// Accumulator stores field values and only one helper payload at a time.
pub(crate) enum Accumulator {
    F16(Vec<Gf65536>),
    F8(Vec<Gf256>),
}
impl Accumulator {
    pub(crate) fn new(d: &BlockDescriptor) -> Self {
        match d.profile {
            Profile::Gf65536 => Self::F16(vec![Gf65536::zero(); d.elements_per_symbol()]),
            Profile::Gf256 => Self::F8(vec![Gf256::zero(); d.elements_per_symbol()]),
        }
    }
    pub(crate) fn add(&mut self, weight: u16, bytes: &[u8]) {
        fn add<F: WireField>(out: &mut [F], weight: u16, bytes: &[u8]) {
            for (y, v) in out.iter_mut().zip(unpack::<F>(bytes)) {
                *y = *y + F::from_word(weight) * v;
            }
        }
        match self {
            Self::F16(x) => add(x, weight, bytes),
            Self::F8(x) => add(x, weight, bytes),
        }
    }
    pub(crate) fn payload(&self) -> Vec<u8> {
        match self {
            Self::F16(x) => pack(x),
            Self::F8(x) => pack(x),
        }
    }
}
