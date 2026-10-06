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
    let error = LdpcError::InvalidEncoderRank { rank: 2, cols: 3 };

    let diagnostic = error.to_string();
    assert!(diagnostic.contains("ранг 2"));
    assert!(diagnostic.contains("3 столбцами"));
    assert!(error.source().is_none());
}

#[test]
fn message_length_mismatch_has_diagnostic_and_no_wrapped_cause() {
    let error = LdpcError::MessageLengthMismatch {
        expected: 3,
        actual: 2,
    };

    let diagnostic = error.to_string();
    assert!(diagnostic.contains("сообщения 2"));
    assert!(diagnostic.contains("длиной 3"));
    assert!(error.source().is_none());
}

#[test]
fn invalid_codeword_reports_unsatisfied_checks_without_a_wrapped_cause() {
    let error = LdpcError::InvalidCodeword {
        unsatisfied_checks: 2,
    };

    assert!(error.to_string().contains("2 проверки"));
    assert!(error.source().is_none());
}
