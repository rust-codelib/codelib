use gf2m::Gf256;
use gf_linalg::Vector;

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
