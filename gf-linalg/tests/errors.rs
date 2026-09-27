use gf_linalg::LinalgError;

#[test]
fn invalid_dimensions_preserve_rows_and_cols_and_have_a_readable_message() {
    let error = LinalgError::InvalidDimensions { rows: 0, cols: 3 };

    assert_eq!(error, LinalgError::InvalidDimensions { rows: 0, cols: 3 });
    assert_eq!(error.to_string(), "недопустимые размеры матрицы: 0 × 3");
}

#[test]
fn element_count_mismatch_preserves_lengths_and_has_a_readable_message() {
    let error = LinalgError::ElementCountMismatch {
        expected: 6,
        actual: 5,
    };

    assert_eq!(
        error,
        LinalgError::ElementCountMismatch {
            expected: 6,
            actual: 5,
        }
    );
    assert_eq!(
        error.to_string(),
        "неверное число элементов матрицы: ожидалось 6, получено 5"
    );
}

#[test]
fn linalg_error_implements_standard_error_and_equality_traits() {
    fn assert_error<T: std::error::Error>() {}
    fn assert_equality<T: PartialEq + Eq>() {}

    assert_error::<LinalgError>();
    assert_equality::<LinalgError>();
}
