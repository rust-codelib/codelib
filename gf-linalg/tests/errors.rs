use gf2m::Gf256;
use gf_linalg::{LinalgError, Matrix};

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

#[test]
fn matrix_shape_mismatch_preserves_both_shapes_and_names_the_operation() {
    let error = LinalgError::MatrixShapeMismatch {
        left_rows: 2,
        left_cols: 3,
        right_rows: 1,
        right_cols: 2,
    };

    assert_eq!(
        error,
        LinalgError::MatrixShapeMismatch {
            left_rows: 2,
            left_cols: 3,
            right_rows: 1,
            right_cols: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "размеры матриц для сложения не совпадают: слева 2 × 3, справа 1 × 2"
    );
}

#[test]
fn matrix_product_mismatch_preserves_operand_dimensions_and_has_a_readable_message() {
    let error = LinalgError::MatrixProductMismatch {
        left_cols: 3,
        right_rows: 4,
    };

    assert_eq!(
        error,
        LinalgError::MatrixProductMismatch {
            left_cols: 3,
            right_rows: 4,
        }
    );
    assert_eq!(
        error.to_string(),
        "размеры матриц для умножения не совпадают: число столбцов слева 3, число строк справа 4"
    );
}

#[test]
fn matrix_vector_length_mismatch_preserves_both_lengths_and_names_the_operation() {
    let error = LinalgError::MatrixVectorLengthMismatch {
        matrix_cols: 3,
        vector_len: 2,
    };

    assert_eq!(
        error,
        LinalgError::MatrixVectorLengthMismatch {
            matrix_cols: 3,
            vector_len: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "умножение матрицы на вектор невозможно: число столбцов матрицы 3, длина вектора 2"
    );
}

#[test]
fn non_square_matrix_preserves_dimensions_and_has_a_readable_message() {
    let error = LinalgError::NonSquareMatrix { rows: 2, cols: 3 };

    assert_eq!(error, LinalgError::NonSquareMatrix { rows: 2, cols: 3 });
    assert_eq!(error.to_string(), "матрица не квадратная: 2 × 3");
}

#[test]
fn singular_matrix_has_a_readable_message() {
    let error = LinalgError::SingularMatrix;

    assert_eq!(error, LinalgError::SingularMatrix);
    assert_eq!(
        error.to_string(),
        "матрица вырождена: обратная матрица не существует"
    );
}

#[test]
fn text_errors_preserve_context_and_have_readable_messages() {
    assert_eq!(
        LinalgError::InvalidTextHeader { expected: "vector" }.to_string(),
        "неверный заголовок текстового формата: ожидалось `vector`"
    );
    assert_eq!(
        LinalgError::InvalidTextDimension { token_index: 1 }.to_string(),
        "неверный размер в токене 1"
    );
    assert_eq!(
        LinalgError::TextElementCountMismatch {
            expected: 3,
            actual: 2,
        }
        .to_string(),
        "неверное число элементов текста: ожидалось 3, получено 2"
    );
    assert_eq!(
        LinalgError::InvalidTextElement { index: 4 }.to_string(),
        "неверный текстовый элемент с индексом 4"
    );
}

#[test]
fn linalg_error_remains_copy_after_adding_text_errors() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<LinalgError>();
}

#[test]
fn polynomial_coefficient_limit_error_preserves_context_and_has_a_readable_message() {
    let vector_error = LinalgError::PolynomialCoefficientLimitExceeded {
        row: None,
        coefficients: 65_537,
        max_coefficients: 65_536,
    };
    assert_eq!(
        vector_error.to_string(),
        "превышен предел коэффициентов многочлена вектора: 65537 > 65536"
    );

    let row_error = LinalgError::PolynomialCoefficientLimitExceeded {
        row: Some(0),
        coefficients: 65_537,
        max_coefficients: 65_536,
    };
    assert_eq!(
        row_error,
        LinalgError::PolynomialCoefficientLimitExceeded {
            row: Some(0),
            coefficients: 65_537,
            max_coefficients: 65_536,
        }
    );
    assert_eq!(
        row_error.to_string(),
        "превышен предел коэффициентов многочлена в строке матрицы с индексом 0: 65537 > 65536"
    );
}

#[test]
fn polynomial_row_length_mismatch_preserves_row_and_widths() {
    let error = LinalgError::PolynomialRowLengthMismatch {
        row: 2,
        expected: 5,
        actual: 3,
    };

    assert_eq!(
        error,
        LinalgError::PolynomialRowLengthMismatch {
            row: 2,
            expected: 5,
            actual: 3,
        }
    );
    assert_eq!(
        error.to_string(),
        "длина коэффициентов строки матрицы с индексом 2 не совпадает: ожидалось 5, получено 3"
    );

    fn assert_copy<T: Copy>() {}
    assert_copy::<LinalgError>();
}

#[test]
fn matrix_addition_rejects_each_shape_mismatch_with_ordered_dimensions() {
    for (left_rows, left_cols, right_rows, right_cols) in [
        (2, 3, 1, 3), // строки отличаются
        (2, 3, 2, 2), // столбцы отличаются
        (2, 3, 1, 2), // отличаются обе оси
        (2, 3, 3, 2), // число элементов одинаковое, форма различается
    ] {
        let expected = LinalgError::MatrixShapeMismatch {
            left_rows,
            left_cols,
            right_rows,
            right_cols,
        };

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!(left.try_add(&right).unwrap_err(), expected);

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!((&left + &right).unwrap_err(), expected);
        assert_eq!(left.rows(), left_rows);
        assert_eq!(left.cols(), left_cols);
        assert_eq!(right.rows(), right_rows);
        assert_eq!(right.cols(), right_cols);

        let left = zero_matrix(left_rows, left_cols);
        let right = zero_matrix(right_rows, right_cols);
        assert_eq!((left + right).unwrap_err(), expected);
    }
}

fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    Matrix::try_new(rows, cols, vec![Gf256::zero(); rows * cols]).unwrap()
}
