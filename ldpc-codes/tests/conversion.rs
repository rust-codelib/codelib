use gf_linalg::Vector;
use gfpm::Gf;
use ldpc_codes::{bits_to_vector, vector_to_bits, Bit, Gf2};

#[cfg(not(debug_assertions))]
use ldpc_codes::LdpcError;

#[test]
fn empty_input_round_trips() {
    let bits = [];

    let vector = bits_to_vector(&bits);

    assert!(vector.is_empty());
    assert_eq!(vector_to_bits(&vector), Ok(Vec::new()));
}

#[test]
fn conversion_preserves_bit_order_and_trailing_zeros() {
    let bits = [Bit::One, Bit::Zero, Bit::One, Bit::Zero];

    let vector = bits_to_vector(&bits);

    assert_eq!(
        vector.as_slice(),
        &[Gf2::one(), Gf2::zero(), Gf2::one(), Gf2::zero()]
    );
    assert_eq!(vector.len(), bits.len());
    assert_eq!(vector_to_bits(&vector), Ok(bits.to_vec()));
    assert_eq!(
        vector.as_slice(),
        &[Gf2::one(), Gf2::zero(), Gf2::one(), Gf2::zero()]
    );
    assert_eq!(bits, [Bit::One, Bit::Zero, Bit::One, Bit::Zero]);
}

#[test]
fn all_zero_input_keeps_its_length() {
    let bits = [Bit::Zero, Bit::Zero, Bit::Zero, Bit::Zero, Bit::Zero];

    let vector = bits_to_vector(&bits);

    assert_eq!(vector.as_slice(), &[Gf2::zero(); 5]);
    assert_eq!(vector.len(), bits.len());
    assert_eq!(vector_to_bits(&vector), Ok(bits.to_vec()));
    assert_eq!(vector.as_slice(), &[Gf2::zero(); 5]);
    assert_eq!(bits, [Bit::Zero; 5]);
}

#[test]
fn directly_created_vector_converts_without_changing_source() {
    let vector = Vector::<Gf2>::new(vec![Gf2::zero(), Gf2::one(), Gf2::zero()]);
    let original = vector.as_slice().to_vec();

    assert_eq!(
        vector_to_bits(&vector),
        Ok(vec![Bit::Zero, Bit::One, Bit::Zero])
    );
    assert_eq!(vector.as_slice(), original);
    assert_eq!(vector.len(), original.len());
}

#[test]
fn gf2_addition_is_xor_and_multiplication_is_binary_and() {
    let zero = Gf2::zero();
    let one = Gf2::one();

    assert_eq!(zero + zero, zero);
    assert_eq!(zero + one, one);
    assert_eq!(one + zero, one);
    assert_eq!(one + one, zero);
    assert_eq!(zero * zero, zero);
    assert_eq!(zero * one, zero);
    assert_eq!(one * zero, zero);
    assert_eq!(one * one, one);
}

#[cfg(not(debug_assertions))]
#[test]
fn noncanonical_release_elements_are_rejected_without_becoming_bits() {
    let vector = Vector::<Gf2>::new(vec![Gf2::one(), Gf2::from_coeffs([2])]);

    assert_eq!(
        vector_to_bits(&vector),
        Err(LdpcError::InvalidFieldElement { index: 1, value: 2 })
    );
    assert_eq!(vector.as_slice()[1].coeffs(), [2]);

    let vector = Vector::<Gf2>::new(vec![Gf2::from_coeffs([u32::MAX])]);

    assert_eq!(
        vector_to_bits(&vector),
        Err(LdpcError::InvalidFieldElement {
            index: 0,
            value: u64::from(u32::MAX),
        })
    );
    assert_eq!(vector.as_slice()[0].coeffs(), [u32::MAX]);
}

#[test]
fn public_field_alias_is_the_expected_gf2_type() {
    let _value: Gf2 = Gf::<2, 1, 1>::one();
}
