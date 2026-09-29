use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM, MAX_POLYNOMIAL_BYTES};
use gfpm::Gf;
use gfpm::Gf9;
use std::mem::size_of;

#[test]
fn polynomial_coefficients_follow_increasing_powers_and_preserve_the_source() {
    let values = [Gf256::new(0x01), Gf256::new(0xAF), Gf256::zero()];
    let vector = Vector::new(values.to_vec());

    let coefficients = vector.try_to_polynomial_coefficients().unwrap();

    assert_eq!(coefficients.as_ref(), &values);
    assert_eq!(vector.as_slice(), &values);
}

#[test]
fn empty_zero_and_trailing_zero_vectors_round_trip_without_normalization() {
    let cases = [
        Vec::new(),
        vec![Gf256::zero()],
        vec![Gf256::zero(); 4],
        vec![Gf256::new(0x2A), Gf256::zero(), Gf256::zero()],
    ];

    for values in cases {
        let vector = Vector::new(values.clone());
        let coefficients = vector.try_to_polynomial_coefficients().unwrap();

        assert_eq!(coefficients.as_ref(), values.as_slice());
        assert_eq!(Vector::from_polynomial_coefficients(coefficients), vector);
    }
}

#[test]
fn polynomial_byte_limit_is_public_and_the_exact_coefficient_bound_is_allowed() {
    assert_eq!(MAX_POLYNOMIAL_BYTES, 131_072);

    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    let vector = Vector::new(vec![Gf256::zero(); max_coefficients]);

    let coefficients = vector.try_to_polynomial_coefficients().unwrap();

    assert_eq!(coefficients.len(), max_coefficients);
    assert_eq!(vector.len(), max_coefficients);
}

#[test]
fn one_coefficient_over_limit_returns_error_and_keeps_vector_unchanged() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    let mut values = vec![Gf256::zero(); max_coefficients + 1];
    values[0] = Gf256::new(0xA5);
    let vector = Vector::new(values.clone());

    assert_eq!(
        vector.try_to_polynomial_coefficients(),
        Err(LinalgError::PolynomialCoefficientLimitExceeded {
            row: None,
            coefficients: max_coefficients + 1,
            max_coefficients,
        })
    );
    assert_eq!(vector.as_slice(), values.as_slice());
}

#[test]
fn spare_vector_capacity_above_limit_does_not_reject_a_short_output() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    let mut values = Vec::with_capacity(max_coefficients + 1);
    values.push(Gf256::new(0x7E));
    let vector = Vector::new(values);

    let coefficients = vector.try_to_polynomial_coefficients().unwrap();

    assert_eq!(coefficients.as_ref(), &[Gf256::new(0x7E)]);
    assert_eq!(vector.as_slice(), &[Gf256::new(0x7E)]);
}

#[test]
fn reverse_conversion_accepts_a_box_larger_than_the_polynomial_limit() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf256>();
    let mut values = vec![Gf256::zero(); max_coefficients + 1].into_boxed_slice();
    values[0] = Gf256::new(0x19);

    let vector = Vector::from_polynomial_coefficients(values);

    assert_eq!(vector.len(), max_coefficients + 1);
    assert_eq!(vector.get(0), Some(Gf256::new(0x19)));
    assert_eq!(vector.get(max_coefficients), Some(Gf256::zero()));
}

#[test]
fn matrix_rows_round_trip_in_order_and_preserve_trailing_zeros_and_source() {
    let values = [
        Gf256::new(0x01),
        Gf256::zero(),
        Gf256::new(0xFF),
        Gf256::new(0x02),
        Gf256::new(0x03),
        Gf256::zero(),
    ];
    let matrix = Matrix::try_new(2, 3, values.to_vec()).unwrap();

    let rows = matrix.try_to_polynomial_rows().unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].as_ref(), &values[..3]);
    assert_eq!(rows[1].as_ref(), &values[3..]);
    assert_eq!(rows[0].len(), 3);
    assert_eq!(rows[1].len(), 3);
    assert_eq!(matrix.as_slice(), &values);
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(rows).unwrap(),
        matrix
    );
}

#[test]
fn matrix_row_conversion_accepts_one_by_n_and_n_by_one_at_dimension_limit() {
    let wide = Matrix::try_new(
        1,
        MAX_MATRIX_DIM,
        (0..MAX_MATRIX_DIM)
            .map(|index| Gf256::new((index % 256) as u16))
            .collect(),
    )
    .unwrap();
    let wide_rows = wide.try_to_polynomial_rows().unwrap();
    assert_eq!(wide_rows.len(), 1);
    assert_eq!(wide_rows[0].len(), MAX_MATRIX_DIM);
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(wide_rows).unwrap(),
        wide
    );

    let tall = Matrix::try_new(
        MAX_MATRIX_DIM,
        1,
        (0..MAX_MATRIX_DIM)
            .map(|index| Gf256::new((index % 256) as u16))
            .collect(),
    )
    .unwrap();
    let tall_rows = tall.try_to_polynomial_rows().unwrap();
    assert_eq!(tall_rows.len(), MAX_MATRIX_DIM);
    assert!(tall_rows.iter().all(|row| row.len() == 1));
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(tall_rows).unwrap(),
        tall
    );
}

#[test]
fn matrix_rows_may_exceed_the_polynomial_limit_in_total_when_each_row_fits() {
    let rows = MAX_POLYNOMIAL_BYTES / (MAX_MATRIX_DIM * size_of::<Gf256>()) + 1;
    let matrix = Matrix::try_new(
        rows,
        MAX_MATRIX_DIM,
        vec![Gf256::zero(); rows * MAX_MATRIX_DIM],
    )
    .unwrap();

    let coefficients = matrix.try_to_polynomial_rows().unwrap();
    let total_bytes: usize = coefficients
        .iter()
        .map(|row| row.len() * size_of::<Gf256>())
        .sum();

    assert!(total_bytes > MAX_POLYNOMIAL_BYTES);
    assert!(coefficients
        .iter()
        .all(|row| row.len() * size_of::<Gf256>() <= MAX_POLYNOMIAL_BYTES));
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(coefficients).unwrap(),
        matrix
    );
}

#[test]
fn polynomial_rows_reject_empty_and_oversized_dimensions() {
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(Vec::new()).unwrap_err(),
        LinalgError::InvalidDimensions { rows: 0, cols: 0 }
    );
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(vec![Box::new([])]).unwrap_err(),
        LinalgError::InvalidDimensions { rows: 1, cols: 0 }
    );
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(vec![
            vec![Gf256::zero()].into_boxed_slice(),
            Box::new([])
        ])
        .unwrap_err(),
        LinalgError::InvalidDimensions { rows: 2, cols: 0 }
    );

    let oversized_cols = vec![Gf256::zero(); MAX_MATRIX_DIM + 1].into_boxed_slice();
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(vec![
            vec![Gf256::zero()].into_boxed_slice(),
            oversized_cols
        ])
        .unwrap_err(),
        LinalgError::InvalidDimensions {
            rows: 2,
            cols: MAX_MATRIX_DIM + 1,
        }
    );

    let oversized_rows = (0..=MAX_MATRIX_DIM)
        .map(|_| vec![Gf256::zero()].into_boxed_slice())
        .collect();
    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(oversized_rows).unwrap_err(),
        LinalgError::InvalidDimensions {
            rows: MAX_MATRIX_DIM + 1,
            cols: 1,
        }
    );
}

#[test]
fn polynomial_rows_report_ragged_row_index_and_widths() {
    let rows = vec![
        vec![Gf256::zero(); 3].into_boxed_slice(),
        vec![Gf256::zero(); 2].into_boxed_slice(),
    ];

    assert_eq!(
        Matrix::<Gf256>::try_from_polynomial_rows(rows).unwrap_err(),
        LinalgError::PolynomialRowLengthMismatch {
            row: 1,
            expected: 3,
            actual: 2,
        }
    );
}

#[test]
fn gf9_vector_coefficients_use_the_field_element_size_and_keep_vector_unbounded() {
    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<Gf9>();

    for values in [Vec::new(), vec![Gf9::new(2), Gf9::zero(), Gf9::zero()]] {
        let vector = Vector::<Gf9>::new(values.clone());
        let coefficients = vector.try_to_polynomial_coefficients().unwrap();
        assert_eq!(coefficients.as_ref(), values.as_slice());
        assert_eq!(vector.as_slice(), values.as_slice());
        assert_eq!(Vector::from_polynomial_coefficients(coefficients), vector);
    }

    let exact_limit = Vector::<Gf9>::new(vec![Gf9::zero(); max_coefficients]);
    let coefficients = exact_limit.try_to_polynomial_coefficients().unwrap();
    assert_eq!(coefficients.len(), max_coefficients);
    assert_eq!(exact_limit.len(), max_coefficients);

    let over_limit = Vector::<Gf9>::new(vec![Gf9::zero(); max_coefficients + 1]);
    assert_eq!(
        over_limit.try_to_polynomial_coefficients(),
        Err(LinalgError::PolynomialCoefficientLimitExceeded {
            row: None,
            coefficients: max_coefficients + 1,
            max_coefficients,
        })
    );
    assert_eq!(over_limit.len(), max_coefficients + 1);

    let mut spare_capacity = Vec::with_capacity(max_coefficients + 1);
    spare_capacity.push(Gf9::new(3));
    let short_vector = Vector::<Gf9>::new(spare_capacity);
    assert_eq!(
        short_vector
            .try_to_polynomial_coefficients()
            .unwrap()
            .as_ref(),
        &[Gf9::new(3)]
    );

    let mut beyond_limit = vec![Gf9::zero(); max_coefficients + 1].into_boxed_slice();
    beyond_limit[0] = Gf9::new(4);
    let restored = Vector::<Gf9>::from_polynomial_coefficients(beyond_limit);
    assert_eq!(restored.len(), max_coefficients + 1);
    assert_eq!(restored.get(0), Some(Gf9::new(4)));
    assert_eq!(restored.get(max_coefficients), Some(Gf9::zero()));
}

#[test]
fn gf9_matrix_rows_round_trip_and_allow_total_storage_above_the_limit() {
    let values = [
        Gf9::new(1),
        Gf9::zero(),
        Gf9::zero(),
        Gf9::new(2),
        Gf9::new(3),
        Gf9::zero(),
    ];
    let matrix = Matrix::<Gf9>::try_new(2, 3, values.to_vec()).unwrap();
    let rows = matrix.try_to_polynomial_rows().unwrap();

    assert_eq!(rows[0].as_ref(), &values[..3]);
    assert_eq!(rows[1].as_ref(), &values[3..]);
    assert_eq!(matrix.as_slice(), &values);
    assert_eq!(
        Matrix::<Gf9>::try_from_polynomial_rows(rows).unwrap(),
        matrix
    );

    let row_count = MAX_POLYNOMIAL_BYTES / (MAX_MATRIX_DIM * size_of::<Gf9>()) + 1;
    let large_matrix = Matrix::<Gf9>::try_new(
        row_count,
        MAX_MATRIX_DIM,
        vec![Gf9::zero(); row_count * MAX_MATRIX_DIM],
    )
    .unwrap();
    let coefficient_rows = large_matrix.try_to_polynomial_rows().unwrap();
    let total_bytes: usize = coefficient_rows
        .iter()
        .map(|row| row.len() * size_of::<Gf9>())
        .sum();

    assert!(total_bytes > MAX_POLYNOMIAL_BYTES);
    assert!(coefficient_rows
        .iter()
        .all(|row| row.len() * size_of::<Gf9>() <= MAX_POLYNOMIAL_BYTES));
    assert_eq!(
        Matrix::<Gf9>::try_from_polynomial_rows(coefficient_rows).unwrap(),
        large_matrix
    );
}

#[test]
fn wide_gfpm_matrix_row_limit_uses_element_size_but_reverse_checks_only_shape() {
    type WideField = Gf<2, 64, 0x1B>;

    let max_coefficients = MAX_POLYNOMIAL_BYTES / size_of::<WideField>();
    let exact_limit = Matrix::<WideField>::try_new(
        1,
        max_coefficients,
        vec![WideField::zero(); max_coefficients],
    )
    .unwrap();
    let exact_rows = exact_limit.try_to_polynomial_rows().unwrap();
    assert_eq!(exact_rows[0].len(), max_coefficients);
    assert_eq!(
        exact_rows[0].len() * size_of::<WideField>(),
        MAX_POLYNOMIAL_BYTES
    );

    let oversized = Matrix::<WideField>::try_new(
        1,
        max_coefficients + 1,
        vec![WideField::zero(); max_coefficients + 1],
    )
    .unwrap();
    assert_eq!(
        oversized.try_to_polynomial_rows(),
        Err(LinalgError::PolynomialCoefficientLimitExceeded {
            row: Some(0),
            coefficients: max_coefficients + 1,
            max_coefficients,
        })
    );

    let raw_row = vec![WideField::new(u64::MAX); max_coefficients + 1].into_boxed_slice();
    let restored = Matrix::<WideField>::try_from_polynomial_rows(vec![raw_row]).unwrap();
    assert_eq!(restored.rows(), 1);
    assert_eq!(restored.cols(), max_coefficients + 1);
    assert!(restored
        .as_slice()
        .iter()
        .all(|value| *value == WideField::new(u64::MAX)));
}

#[test]
fn row_shape_checks_return_the_first_error_in_input_order() {
    let rows = vec![
        vec![Gf9::zero(); 3].into_boxed_slice(),
        vec![Gf9::zero(); 2].into_boxed_slice(),
        vec![Gf9::zero(); MAX_MATRIX_DIM + 1].into_boxed_slice(),
    ];

    assert_eq!(
        Matrix::<Gf9>::try_from_polynomial_rows(rows).unwrap_err(),
        LinalgError::PolynomialRowLengthMismatch {
            row: 1,
            expected: 3,
            actual: 2,
        }
    );
}
