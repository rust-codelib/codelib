use gf2m::Gf256;
use gf_linalg::{LinalgError, Vector};

#[test]
fn constructor_preserves_order_length_and_trailing_zeros() {
    let data = vec![Gf256::new(7), Gf256::new(3), Gf256::zero(), Gf256::zero()];
    let expected = data.clone();
    let vector = Vector::new(data);

    assert_eq!(vector.len(), 4);
    assert!(!vector.is_empty());
    assert_eq!(vector.as_slice(), expected.as_slice());
}

#[test]
fn empty_vector_has_no_readable_or_mutable_elements() {
    let mut vector = Vector::new(Vec::new());

    assert_eq!(vector.len(), 0);
    assert!(vector.is_empty());
    assert!(vector.as_slice().is_empty());
    assert_eq!(vector.get(0), None);
    assert_eq!(vector.get(usize::MAX), None);
    assert!(vector.get_mut(0).is_none());
    assert!(vector.get_mut(usize::MAX).is_none());
}

#[test]
fn get_returns_values_in_order_and_none_outside_the_vector() {
    let vector = Vector::new(vec![Gf256::new(8), Gf256::new(5), Gf256::zero()]);

    assert_eq!(vector.get(0), Some(Gf256::new(8)));
    assert_eq!(vector.get(1), Some(Gf256::new(5)));
    assert_eq!(vector.get(2), Some(Gf256::zero()));
    assert_eq!(vector.get(3), None);
    assert_eq!(vector.get(usize::MAX), None);
}

#[test]
fn get_mut_changes_only_the_selected_element_and_preserves_length() {
    let mut vector = Vector::new(vec![Gf256::new(8), Gf256::new(5), Gf256::zero()]);

    *vector.get_mut(0).unwrap() = Gf256::new(9);
    assert_eq!(
        vector.as_slice(),
        &[Gf256::new(9), Gf256::new(5), Gf256::zero()]
    );
    *vector.get_mut(1).unwrap() = Gf256::new(6);
    assert_eq!(
        vector.as_slice(),
        &[Gf256::new(9), Gf256::new(6), Gf256::zero()]
    );
    *vector.get_mut(2).unwrap() = Gf256::new(7);
    assert_eq!(
        vector.as_slice(),
        &[Gf256::new(9), Gf256::new(6), Gf256::new(7)]
    );
    assert_eq!(vector.len(), 3);
    assert!(!vector.is_empty());
}

#[test]
fn invalid_mutable_indices_return_none_without_changing_the_vector() {
    let data = vec![Gf256::new(8), Gf256::zero()];
    let mut vector = Vector::new(data.clone());

    assert!(vector.get_mut(2).is_none());
    assert!(vector.get_mut(usize::MAX).is_none());
    assert_eq!(vector.len(), 2);
    assert_eq!(vector.as_slice(), data.as_slice());
}

#[test]
fn constructor_accepts_data_larger_than_matrix_and_polynomial_limits() {
    let mut data = vec![Gf256::new(4); 65_537];
    data[65_536] = Gf256::zero();
    assert!(data.len() > 4096);
    assert!(std::mem::size_of_val(data.as_slice()) > 128 * 1024);

    let vector = Vector::new(data);

    assert_eq!(vector.len(), 65_537);
    assert!(!vector.is_empty());
    assert!(vector.as_slice()[..65_536]
        .iter()
        .all(|&value| value == Gf256::new(4)));
    assert_eq!(vector.get(65_536), Some(Gf256::zero()));
    assert_eq!(vector.get(65_537), None);
}

#[test]
fn try_add_computes_field_sum_and_preserves_length_and_trailing_zeros() {
    let left_data = vec![Gf256::new(0x57), Gf256::zero(), Gf256::zero()];
    let right_data = vec![Gf256::new(0x83), Gf256::zero(), Gf256::zero()];
    let left = Vector::new(left_data.clone());
    let right = Vector::new(right_data.clone());

    let sum = left.try_add(&right).unwrap();

    assert_eq!(sum.len(), 3);
    assert_eq!(
        sum.as_slice(),
        &[Gf256::new(0xD4), Gf256::zero(), Gf256::zero()]
    );
    assert_eq!(left.as_slice(), left_data.as_slice());
    assert_eq!(right.as_slice(), right_data.as_slice());
}

#[test]
fn try_add_accepts_two_empty_vectors() {
    let left = Vector::new(Vec::new());
    let right = Vector::new(Vec::new());

    let sum = left.try_add(&right).unwrap();

    assert!(sum.is_empty());
}

#[test]
fn try_add_reports_both_length_orders_without_changing_operands() {
    let empty = Vector::new(Vec::new());
    let one = Vector::new(vec![Gf256::new(1)]);
    let two = Vector::new(vec![Gf256::new(1), Gf256::new(2)]);
    let three = Vector::new(vec![Gf256::new(1), Gf256::new(2), Gf256::new(3)]);

    assert!(matches!(
        empty.try_add(&one),
        Err(LinalgError::VectorLengthMismatch { left: 0, right: 1 })
    ));
    assert!(matches!(
        two.try_add(&three),
        Err(LinalgError::VectorLengthMismatch { left: 2, right: 3 })
    ));
    assert_eq!(empty.len(), 0);
    assert_eq!(one.len(), 1);
    assert_eq!(two.len(), 2);
    assert_eq!(three.len(), 3);
}

#[test]
fn add_operators_match_try_add_and_reference_form_keeps_operands_available() {
    let left = Vector::new(vec![Gf256::new(0x57), Gf256::zero()]);
    let right = Vector::new(vec![Gf256::new(0x83), Gf256::zero()]);

    let borrowed_sum = (&left + &right).unwrap();
    assert_eq!(borrowed_sum.as_slice(), &[Gf256::new(0xD4), Gf256::zero()]);
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 2);

    let owned_sum =
        (Vector::new(vec![Gf256::new(0x57)]) + Vector::new(vec![Gf256::new(0x83)])).unwrap();
    assert_eq!(owned_sum.as_slice(), &[Gf256::new(0xD4)]);
}

#[test]
fn add_operators_return_length_mismatch_errors() {
    let left = Vector::new(vec![Gf256::zero()]);
    let right = Vector::new(Vec::new());

    assert!(matches!(
        &left + &right,
        Err(LinalgError::VectorLengthMismatch { left: 1, right: 0 })
    ));
    assert!(matches!(
        Vector::new(vec![Gf256::zero()]) + Vector::new(Vec::new()),
        Err(LinalgError::VectorLengthMismatch { left: 1, right: 0 })
    ));
}

#[test]
fn scale_multiplies_each_element_and_preserves_the_source_vector() {
    let data = vec![Gf256::new(0x57), Gf256::zero(), Gf256::zero()];
    let vector = Vector::new(data.clone());

    let scaled = vector.scale(Gf256::new(0x02));

    assert_eq!(scaled.len(), 3);
    assert_eq!(
        scaled.as_slice(),
        &[Gf256::new(0xAE), Gf256::zero(), Gf256::zero()]
    );
    assert_eq!(vector.as_slice(), data.as_slice());
}

#[test]
fn scaling_by_zero_and_one_preserves_vector_length() {
    let vector = Vector::new(vec![Gf256::new(7), Gf256::zero(), Gf256::new(9)]);

    let zeroed = vector.scale(Gf256::zero());
    let unchanged = vector.scale(Gf256::new(1));

    assert_eq!(zeroed.len(), 3);
    assert_eq!(zeroed.as_slice(), &[Gf256::zero(); 3]);
    assert_eq!(unchanged.len(), 3);
    assert_eq!(unchanged.as_slice(), vector.as_slice());
    assert!(Vector::new(Vec::new()).scale(Gf256::new(2)).is_empty());
}

#[test]
fn multiplication_operators_match_scale_for_owned_and_borrowed_vectors() {
    let vector = Vector::new(vec![Gf256::new(0x57), Gf256::zero()]);

    let borrowed_product = &vector * Gf256::new(0x02);
    assert_eq!(
        borrowed_product.as_slice(),
        &[Gf256::new(0xAE), Gf256::zero()]
    );
    assert_eq!(vector.as_slice(), &[Gf256::new(0x57), Gf256::zero()]);

    let owned_product = Vector::new(vec![Gf256::new(0x57)]) * Gf256::new(0x02);
    assert_eq!(owned_product.as_slice(), &[Gf256::new(0xAE)]);
}

#[test]
fn scaling_has_no_matrix_dimension_limit() {
    let vector = Vector::new(vec![Gf256::new(4); 4097]);

    let scaled = &vector * Gf256::new(1);

    assert_eq!(scaled.len(), 4097);
    assert!(scaled
        .as_slice()
        .iter()
        .all(|&value| value == Gf256::new(4)));
}

#[test]
fn vector_equality_compares_exact_length_and_all_elements() {
    let empty = Vector::new(Vec::new());
    let another_empty = Vector::new(Vec::new());
    let with_trailing_zero = Vector::new(vec![Gf256::new(7), Gf256::zero()]);
    let same_values_and_length = Vector::new(vec![Gf256::new(7), Gf256::zero()]);
    let shorter = Vector::new(vec![Gf256::new(7)]);

    assert_eq!(empty, another_empty);
    assert_eq!(with_trailing_zero, same_values_and_length);
    assert_ne!(with_trailing_zero, shorter);
    assert!(with_trailing_zero != shorter);
}

#[test]
fn changing_an_element_changes_vector_equality() {
    let original = Vector::new(vec![Gf256::new(7), Gf256::zero()]);
    let mut changed = Vector::new(vec![Gf256::new(7), Gf256::zero()]);
    assert_eq!(original, changed);

    *changed.get_mut(1).unwrap() = Gf256::new(1);

    assert_ne!(original, changed);
}

#[test]
fn try_dot_returns_not_implemented_for_equal_different_and_empty_vectors() {
    let equal_left = Vector::new(vec![Gf256::new(2), Gf256::new(3)]);
    let equal_right = Vector::new(vec![Gf256::new(4), Gf256::new(5)]);
    let different = Vector::new(vec![Gf256::new(6)]);
    let empty = Vector::new(Vec::new());
    let expected = LinalgError::NotImplemented {
        operation: "скалярное произведение векторов",
    };

    assert_eq!(equal_left.try_dot(&equal_right), Err(expected));
    assert_eq!(equal_left.try_dot(&different), Err(expected));
    assert_eq!(empty.try_dot(&empty), Err(expected));
}
