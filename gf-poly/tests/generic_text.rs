use core::fmt;
use core::ops::{Add, Mul, Neg, Sub};

use gf_poly::{FieldElement, FieldText, Polynomial};
use gfpm::Gf5;

// Произвольный пользовательский формат с тегом "-v". Начальный минус здесь
// часть имени токена, а не отрицание элемента. Такой формат разрешён FieldText.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TaggedField(Gf5);

macro_rules! binary_operator {
    ($trait:ident, $method:ident, $operator:tt) => {
        impl $trait for TaggedField {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self {
                Self(self.0 $operator rhs.0)
            }
        }
    };
}

binary_operator!(Add, add, +);
binary_operator!(Sub, sub, -);
binary_operator!(Mul, mul, *);

impl Neg for TaggedField {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl FieldElement for TaggedField {
    fn zero() -> Self {
        Self(Gf5::zero())
    }

    fn one() -> Self {
        Self(Gf5::one())
    }

    fn inv(self) -> Self {
        Self(self.0.inv())
    }
}

impl FieldText for TaggedField {
    fn fmt_element(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "-v{}", self.0.value())
    }

    fn parse_element(token: &str) -> Option<Self> {
        let value = token.strip_prefix("-v")?.parse::<u64>().ok()?;
        (value < 5).then(|| Self(Gf5::new(value)))
    }
}

#[test]
fn canonical_custom_tokens_round_trip_even_when_they_start_with_a_sign() {
    let values = [TaggedField(Gf5::new(3)), TaggedField::zero()];
    let polynomial = Polynomial::try_from_slice(&values).unwrap();
    let text = polynomial.to_string();
    assert_eq!(text, "polynomial 2 -v3 -v0");
    let restored: Polynomial<TaggedField> = text.parse().unwrap();
    assert_eq!(restored.coefficients(), &values);

    let signed: Polynomial<TaggedField> = "polynomial 2 --v3 +-v3".parse().unwrap();
    assert_eq!(signed.coefficients(), &[-values[0], values[0]]);
}
