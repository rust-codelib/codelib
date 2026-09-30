use gf_linalg::{LinalgError, Vector};
use gfpm::Gf9;

#[test]
fn gf9_vector_construction_access_scaling_and_length_are_generic() {
    let values = vec![Gf9::new(1), Gf9::new(2), Gf9::new(3), Gf9::zero()];
    let mut vector = Vector::<Gf9>::new(values.clone());

    assert_eq!(vector.len(), 4);
    assert_eq!(vector.as_slice(), values);
    assert_eq!(vector.get(2), Some(Gf9::new(3)));
    assert_eq!(vector.get(4), None);
    assert_eq!(vector.get(usize::MAX), None);
    *vector.get_mut(1).unwrap() = Gf9::new(1);
    assert_eq!(
        vector.as_slice(),
        &[Gf9::new(1), Gf9::new(1), Gf9::new(3), Gf9::zero()]
    );
    assert!(vector.get_mut(usize::MAX).is_none());

    let source = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2), Gf9::new(3), Gf9::zero()]);
    let scaled = source.scale(Gf9::new(2));
    assert_eq!(
        scaled.as_slice(),
        &[Gf9::new(2), Gf9::new(1), Gf9::new(6), Gf9::zero()]
    );
    assert_eq!(
        source.as_slice(),
        &[Gf9::new(1), Gf9::new(2), Gf9::new(3), Gf9::zero()]
    );

    let long = Vector::<Gf9>::new(vec![Gf9::new(4); 4097]);
    assert_eq!(long.len(), 4097);
    assert_eq!(long.get(4096), Some(Gf9::new(4)));
    assert!(Vector::<Gf9>::new(Vec::new()).is_empty());
}

#[test]
fn gf9_addition_and_equality_preserve_length_trailing_zeros_and_sources() {
    let left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2), Gf9::zero(), Gf9::zero()]);
    let right = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::new(1), Gf9::zero(), Gf9::zero()]);

    let sum = left.try_add(&right).unwrap();
    assert_eq!(sum.as_slice(), &[Gf9::zero(); 4]);
    assert_eq!(sum.len(), 4);
    assert_eq!(
        left.as_slice(),
        &[Gf9::new(1), Gf9::new(2), Gf9::zero(), Gf9::zero()]
    );
    assert_eq!(
        right.as_slice(),
        &[Gf9::new(2), Gf9::new(1), Gf9::zero(), Gf9::zero()]
    );
    assert_eq!(left, left);
    assert_ne!(left, Vector::<Gf9>::new(left.as_slice()[..3].to_vec()));
}

#[test]
fn gf9_operators_match_methods_and_dot_keeps_its_stub_contract() {
    let left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2)]);
    let right = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::new(1)]);

    assert_eq!((&left + &right).unwrap().as_slice(), &[Gf9::zero(); 2]);
    let owned_left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2)]);
    let owned_right = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::new(1)]);
    assert_eq!(
        owned_left + owned_right,
        Ok(Vector::<Gf9>::new(vec![Gf9::zero(); 2]))
    );
    assert_eq!(
        (&left * Gf9::new(2)).as_slice(),
        &[Gf9::new(2), Gf9::new(1)]
    );
    assert_eq!(
        (Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2)]) * Gf9::new(2)).as_slice(),
        &[Gf9::new(2), Gf9::new(1)]
    );
    assert_eq!(
        left.try_dot(&Vector::<Gf9>::new(Vec::new())),
        Err(LinalgError::NotImplemented {
            operation: "скалярное произведение векторов",
        })
    );
    assert_eq!(
        Vector::<Gf9>::new(Vec::new()).try_dot(&right),
        Err(LinalgError::NotImplemented {
            operation: "скалярное произведение векторов",
        })
    );
}

#[test]
fn gf9_subtraction_uses_field_subtraction_and_preserves_trailing_zeros() {
    let left = Vector::<Gf9>::new([1, 2, 3, 4, 0, 0].map(Gf9::new).to_vec());
    let right = Vector::<Gf9>::new([2, 1, 4, 3, 0, 0].map(Gf9::new).to_vec());
    let expected = Vector::<Gf9>::new([2, 1, 2, 1, 0, 0].map(Gf9::new).to_vec());

    let difference = left.try_sub(&right).unwrap();

    assert_eq!(difference, expected);
    assert_eq!(difference.len(), 6);
    assert_ne!(difference.as_slice()[..2], [Gf9::zero(), Gf9::zero()]);
    assert_eq!(left.as_slice(), &[1, 2, 3, 4, 0, 0].map(Gf9::new));
    assert_eq!(right.as_slice(), &[2, 1, 4, 3, 0, 0].map(Gf9::new));
    assert_eq!(
        Vector::<Gf9>::new(vec![Gf9::new(3), Gf9::new(4)])
            .try_sub(&Vector::<Gf9>::new(vec![Gf9::new(4), Gf9::new(3)]))
            .unwrap()
            .as_slice(),
        &[Gf9::new(2), Gf9::one()]
    );
}

#[test]
fn gf9_subtraction_operators_match_method_and_cover_empty_and_long_vectors() {
    let left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2), Gf9::zero()]);
    let right = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::new(1), Gf9::zero()]);
    let expected = Vector::<Gf9>::new(vec![Gf9::new(2), Gf9::one(), Gf9::zero()]);

    assert_eq!((&left - &right).unwrap(), expected);
    assert_eq!(
        (Vector::<Gf9>::new(left.as_slice().to_vec())
            - Vector::<Gf9>::new(right.as_slice().to_vec()))
        .unwrap(),
        expected
    );
    assert_eq!(left.as_slice(), &[Gf9::new(1), Gf9::new(2), Gf9::zero()]);
    assert_eq!(right.as_slice(), &[Gf9::new(2), Gf9::one(), Gf9::zero()]);

    let empty = Vector::<Gf9>::new(Vec::new());
    assert_eq!(empty.try_sub(&empty).unwrap(), empty);

    let long = Vector::<Gf9>::new(vec![Gf9::new(4); 4097]);
    let long_difference = long.try_sub(&long).unwrap();
    assert_eq!(long_difference.len(), 4097);
    assert!(long_difference
        .as_slice()
        .iter()
        .all(|value| value.is_zero()));
}

#[test]
fn vector_subtraction_matches_addition_in_characteristic_two_and_reports_lengths() {
    use gf2m::Gf256;

    let left_256 = Vector::<Gf256>::new(vec![Gf256::new(1), Gf256::new(17), Gf256::zero()]);
    let right_256 = Vector::<Gf256>::new(vec![Gf256::new(2), Gf256::new(17), Gf256::zero()]);
    assert_eq!(
        left_256.try_sub(&right_256).unwrap(),
        left_256.try_add(&right_256).unwrap()
    );

    let left = Vector::<Gf9>::new(vec![Gf9::new(1), Gf9::new(2)]);
    let short = Vector::<Gf9>::new(vec![Gf9::new(2)]);
    assert_eq!(
        left.try_sub(&short),
        Err(LinalgError::VectorLengthMismatch { left: 2, right: 1 })
    );
    assert_eq!(
        short.try_sub(&left),
        Err(LinalgError::VectorLengthMismatch { left: 1, right: 2 })
    );
    assert_eq!(
        (&left - &short),
        Err(LinalgError::VectorLengthMismatch { left: 2, right: 1 })
    );
    assert_eq!(
        (Vector::<Gf9>::new(left.as_slice().to_vec())
            - Vector::<Gf9>::new(short.as_slice().to_vec())),
        Err(LinalgError::VectorLengthMismatch { left: 2, right: 1 })
    );
    assert_eq!(left.as_slice(), &[Gf9::new(1), Gf9::new(2)]);
    assert_eq!(short.as_slice(), &[Gf9::new(2)]);
}
