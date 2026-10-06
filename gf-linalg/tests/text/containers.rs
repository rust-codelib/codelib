use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix, Vector, MAX_MATRIX_DIM};
use std::mem::size_of;

#[test]
fn empty_vector_has_a_canonical_single_line_representation() {
    let vector = Vector::new(Vec::new());

    assert_eq!(vector.to_string(), "vector 0");
    assert_eq!(vector.to_string().parse::<Vector>(), Ok(vector));
}

#[test]
fn display_preserves_element_order_and_trailing_zeroes() {
    let vector = Vector::new(vec![
        Gf256::new(0x01),
        Gf256::new(0xAF),
        Gf256::zero(),
        Gf256::zero(),
    ]);

    assert_eq!(vector.to_string(), "vector 4 0x01 0xaf 0x00 0x00");
    assert_eq!(vector.to_string().parse::<Vector>(), Ok(vector));
}

#[test]
fn parser_accepts_ascii_whitespace_leading_zeroes_and_uppercase_hex() {
    let input = "\x0bvector\x0c0003\t0x0A\r\n0xFf 0x00\x0c";
    let parsed = input.parse::<Vector>().unwrap();

    assert_eq!(
        parsed,
        Vector::new(vec![Gf256::new(0x0A), Gf256::new(0xFF), Gf256::zero()])
    );
}

#[test]
fn parser_requires_ascii_whitespace_and_ascii_digits() {
    assert_eq!(
        "vector\u{00a0}0".parse::<Vector>(),
        Err(LinalgError::InvalidTextHeader { expected: "vector" })
    );
    assert_eq!(
        "vector ٠".parse::<Vector>(),
        Err(LinalgError::InvalidTextDimension { token_index: 1 })
    );
}

#[test]
fn parser_reports_header_then_dimension_then_element_count_then_element_errors() {
    assert_eq!(
        "matrix not-a-size".parse::<Vector>(),
        Err(LinalgError::InvalidTextHeader { expected: "vector" })
    );
    assert_eq!(
        "vector -1".parse::<Vector>(),
        Err(LinalgError::InvalidTextDimension { token_index: 1 })
    );
    assert_eq!(
        "vector +1".parse::<Vector>(),
        Err(LinalgError::InvalidTextDimension { token_index: 1 })
    );
    assert_eq!(
        "vector 2 0xGG".parse::<Vector>(),
        Err(LinalgError::TextElementCountMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        "vector 1 0xGG 0x00".parse::<Vector>(),
        Err(LinalgError::TextElementCountMismatch {
            expected: 1,
            actual: 2,
        })
    );
    assert_eq!(
        "vector 1 0xGG".parse::<Vector>(),
        Err(LinalgError::InvalidTextElement { index: 0 })
    );
}

#[test]
fn parser_rejects_missing_and_overflowing_dimensions() {
    assert_eq!(
        "vector".parse::<Vector>(),
        Err(LinalgError::InvalidTextDimension { token_index: 1 })
    );

    let overflow = format!("vector {}0", usize::MAX);
    assert_eq!(
        overflow.parse::<Vector>(),
        Err(LinalgError::InvalidTextDimension { token_index: 1 })
    );
}

#[test]
fn usize_max_dimension_is_counted_before_any_element_buffer_is_needed() {
    let input = format!("vector {}", usize::MAX);

    assert_eq!(
        input.parse::<Vector>(),
        Err(LinalgError::TextElementCountMismatch {
            expected: usize::MAX,
            actual: 0,
        })
    );
}

#[test]
fn parser_rejects_noncanonical_or_non_ascii_element_tokens() {
    for element in ["0X01", "0x1", "0x001", "0xg0", "0xＡ１", "0x100"] {
        assert_eq!(
            format!("vector 1 {element}").parse::<Vector>(),
            Err(LinalgError::InvalidTextElement { index: 0 }),
            "accepted malformed element token {element:?}"
        );
    }
}

#[test]
fn vector_round_trip_has_no_matrix_or_polynomial_length_limit() {
    let longer_than_matrix_axis = 4097;
    let values = (0..longer_than_matrix_axis)
        .map(|index| Gf256::new((index % 256) as u16))
        .collect::<Vec<_>>();
    let vector = Vector::new(values);
    let parsed = vector.to_string().parse::<Vector>().unwrap();

    assert_eq!(parsed, vector);

    let beyond_polynomial_limit = 128 * 1024 / size_of::<Gf256>() + 1;
    let zeros = std::iter::repeat("0x00")
        .take(beyond_polynomial_limit)
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!("vector {beyond_polynomial_limit} {zeros}");
    let parsed = input.parse::<Vector>().unwrap();

    assert_eq!(parsed.len(), beyond_polynomial_limit);
    assert!(parsed
        .as_slice()
        .iter()
        .all(|element| *element == Gf256::zero()));
}

#[test]
fn matrix_display_is_row_major_canonical_and_preserves_trailing_zeroes() {
    let matrix = Matrix::try_new(
        2,
        3,
        vec![
            Gf256::new(0x01),
            Gf256::new(0xAF),
            Gf256::zero(),
            Gf256::new(0x02),
            Gf256::zero(),
            Gf256::zero(),
        ],
    )
    .unwrap();

    let text = "matrix 2 3\n0x01 0xaf 0x00\n0x02 0x00 0x00";
    assert_eq!(matrix.to_string(), text);
    assert_eq!(text.parse::<Matrix>(), Ok(matrix));
}

#[test]
fn single_row_and_single_column_matrices_round_trip_canonically() {
    let row = Matrix::try_new(1, 3, vec![Gf256::new(0x10), Gf256::zero(), Gf256::zero()]).unwrap();
    let column =
        Matrix::try_new(3, 1, vec![Gf256::new(0x10), Gf256::zero(), Gf256::zero()]).unwrap();

    let row_text = "matrix 1 3\n0x10 0x00 0x00";
    let column_text = "matrix 3 1\n0x10\n0x00\n0x00";
    assert_eq!(row.to_string(), row_text);
    assert_eq!(row_text.parse::<Matrix>(), Ok(row));
    assert_eq!(column.to_string(), column_text);
    assert_eq!(column_text.parse::<Matrix>(), Ok(column));
}

#[test]
fn matrix_parser_accepts_ascii_whitespace_leading_zeroes_and_uppercase_hex() {
    let input = "\x0bmatrix\x0c0002\t0003\x0b0x0A\r\n0xFf\x0c0x00 0x01 0x02 0x03";
    let parsed = input.parse::<Matrix>().unwrap();

    assert_eq!(parsed.rows(), 2);
    assert_eq!(parsed.cols(), 3);
    assert_eq!(
        parsed.as_slice(),
        &[
            Gf256::new(0x0A),
            Gf256::new(0xFF),
            Gf256::zero(),
            Gf256::new(0x01),
            Gf256::new(0x02),
            Gf256::new(0x03),
        ]
    );
}

#[test]
fn matrix_parser_rejects_missing_or_invalid_headers_and_dimensions() {
    for input in ["", "vector 1 0x00"] {
        assert_eq!(
            input.parse::<Matrix>(),
            Err(LinalgError::InvalidTextHeader { expected: "matrix" })
        );
    }

    for (input, token_index) in [("matrix", 1), ("matrix 2", 2), ("matrix 2 no", 2)] {
        assert_eq!(
            input.parse::<Matrix>(),
            Err(LinalgError::InvalidTextDimension { token_index })
        );
    }

    let overflow_rows = format!("matrix {}0 1", usize::MAX);
    let overflow_cols = format!("matrix 1 {}0", usize::MAX);
    for (input, token_index) in [(overflow_rows, 1), (overflow_cols, 2)] {
        assert_eq!(
            input.parse::<Matrix>(),
            Err(LinalgError::InvalidTextDimension { token_index })
        );
    }
}

#[test]
fn matrix_parser_rejects_zero_and_oversized_axes_before_counting_elements() {
    for (input, rows, cols) in [
        ("matrix 0 2 0xGG", 0, 2),
        ("matrix 2 0", 2, 0),
        ("matrix 0 0", 0, 0),
        ("matrix 4097 1", MAX_MATRIX_DIM + 1, 1),
        ("matrix 1 4097", 1, MAX_MATRIX_DIM + 1),
    ] {
        assert_eq!(
            input.parse::<Matrix>(),
            Err(LinalgError::InvalidDimensions { rows, cols })
        );
    }
}

#[test]
fn matrix_parser_checks_exact_element_count_before_element_values() {
    assert_eq!(
        "matrix 2 3 0x00 0xGG 0x02 0x03 0x04".parse::<Matrix>(),
        Err(LinalgError::TextElementCountMismatch {
            expected: 6,
            actual: 5,
        })
    );
    assert_eq!(
        "matrix 2 3 0x00 0x01 0x02 0x03 0x04 0x05 0x06".parse::<Matrix>(),
        Err(LinalgError::TextElementCountMismatch {
            expected: 6,
            actual: 7,
        })
    );
}

#[test]
fn matrix_parser_reports_malformed_element_at_flat_row_major_index() {
    for token in ["0X05", "0x5", "0x005", "0xGG", "0x100"] {
        assert_eq!(
            format!("matrix 2 3 0x00 0x01 0x02 0x03 {token} 0x05").parse::<Matrix>(),
            Err(LinalgError::InvalidTextElement { index: 4 }),
            "accepted malformed element token {token:?}"
        );
    }
}

#[test]
fn matrix_with_maximum_axis_is_accepted() {
    let elements = std::iter::repeat("0x00")
        .take(MAX_MATRIX_DIM)
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!("matrix 1 {MAX_MATRIX_DIM} {elements}");

    let parsed = input.parse::<Matrix>().unwrap();

    assert_eq!(parsed.rows(), 1);
    assert_eq!(parsed.cols(), MAX_MATRIX_DIM);
    assert_eq!(parsed.as_slice().len(), MAX_MATRIX_DIM);
}

#[test]
fn existing_matrix_operations_work_after_text_parsing() {
    let matrix = "matrix 2 3 0x01 0x02 0x03 0x04 0x05 0x06"
        .parse::<Matrix>()
        .unwrap();

    let transposed = matrix.transpose();
    assert_eq!(transposed.rows(), 3);
    assert_eq!(transposed.cols(), 2);
    assert_eq!(
        transposed.as_slice(),
        &[
            Gf256::new(0x01),
            Gf256::new(0x04),
            Gf256::new(0x02),
            Gf256::new(0x05),
            Gf256::new(0x03),
            Gf256::new(0x06),
        ]
    );

    let result = matrix
        .try_mul_vector(&Vector::new(vec![
            Gf256::one(),
            Gf256::zero(),
            Gf256::one(),
        ]))
        .unwrap();
    assert_eq!(result.as_slice(), &[Gf256::new(0x02), Gf256::new(0x02)]);
}
