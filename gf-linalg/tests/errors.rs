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
fn vector_length_mismatch_preserves_operand_lengths_and_has_a_readable_message() {
    let error = LinalgError::VectorLengthMismatch { left: 2, right: 3 };

    assert_eq!(
        error,
        LinalgError::VectorLengthMismatch { left: 2, right: 3 }
    );
    assert_eq!(
        error.to_string(),
        "длины векторов не совпадают: слева 2, справа 3"
    );
}

#[test]
fn not_implemented_error_names_the_operation_and_has_a_readable_message() {
    let error = LinalgError::NotImplemented {
        operation: "скалярное произведение векторов",
    };

    assert_eq!(
        error,
        LinalgError::NotImplemented {
            operation: "скалярное произведение векторов",
        }
    );
    assert_eq!(
        error.to_string(),
        "операция не реализована: скалярное произведение векторов"
    );
}

#[test]
fn linalg_error_implements_standard_error_and_equality_traits() {
    fn assert_error<T: std::error::Error>() {}
    fn assert_equality<T: PartialEq + Eq>() {}

    assert_error::<LinalgError>();
    assert_equality::<LinalgError>();
}
