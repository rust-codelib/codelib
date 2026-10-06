use std::error::Error;

use gf_linalg::LinalgError;
use ldpc_codes::LdpcError;

#[test]
fn linear_algebra_conversion_preserves_the_source() {
    let source = LinalgError::MatrixProductMismatch {
        left_cols: 2,
        right_rows: 3,
    };

    let error = LdpcError::from(source);

    assert_eq!(error, LdpcError::LinearAlgebra { source });
    let cause = error
        .source()
        .expect("linear algebra cause should be exposed");
    assert_eq!(cause.downcast_ref::<LinalgError>(), Some(&source));
    assert!(error.to_string().contains(&source.to_string()));
}

#[test]
fn errors_without_wrapped_causes_have_no_source() {
    let error = LdpcError::SizeOverflow;

    assert!(error.source().is_none());
}

#[test]
fn invalid_encoder_rank_has_diagnostic_and_no_wrapped_cause() {
    let error = LdpcError::InvalidEncoderRank { rank: 2, cols: 2 };

    assert_eq!(
        error.to_string(),
        "недопустимый ранг 2 для систематического кодера с 2 столбцами"
    );
    assert!(error.source().is_none());
}
