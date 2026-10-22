use super::DecodeInput;
use crate::test_support::prepared_channel;
use crate::{DecoderConfig, LdpcError};

#[test]
fn preparation_clips_extreme_finite_values_and_zeroes_erasures() {
    let llrs = [f64::MAX, -f64::MAX, 1.25, -2.5];
    let erasures = [2];

    let channel = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
        4,
        DecoderConfig::try_new(0, 3.0).expect("valid limit"),
    )
    .expect("finite LLRs and valid erasures should be prepared");

    assert_eq!(channel, [3.0, -3.0, 0.0, -2.5]);
    assert_eq!(llrs, [f64::MAX, -f64::MAX, 1.25, -2.5]);
    assert_eq!(erasures, [2]);
}

#[test]
fn preparation_accepts_no_erasures_and_erasing_every_bit() {
    let llrs = [-30.0, 4.0, 30.0];
    let config = DecoderConfig::default();
    let unerased = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
        llrs.len(),
        config,
    )
    .expect("an empty erasure list is valid");
    let fully_erased = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &[0, 1, 2],
        },
        llrs.len(),
        config,
    )
    .expect("all bits may be erased");

    assert_eq!(unerased, [-20.0, 4.0, 20.0]);
    assert_eq!(fully_erased, [0.0, 0.0, 0.0]);
    assert_eq!(llrs, [-30.0, 4.0, 30.0]);
}

#[test]
fn length_mismatch_is_reported_before_other_input_errors() {
    let llrs = [f64::NAN];
    let erasures = [9, 9];

    let error = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
        2,
        DecoderConfig::default(),
    )
    .expect_err("wrong LLR length must take precedence");

    assert_eq!(
        error,
        LdpcError::LlrLengthMismatch {
            expected: 2,
            actual: 1,
        }
    );
}

#[test]
fn non_finite_erased_llr_is_reported_before_erasure_errors() {
    let llrs = [1.0, f64::INFINITY, f64::NAN];
    let erasures = [3, 1];

    let error = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
        3,
        DecoderConfig::default(),
    )
    .expect_err("every LLR must be finite even when its bit is erased");

    assert_eq!(error, LdpcError::NonFiniteLlr { index: 1 });
    assert_eq!(llrs[0], 1.0);
    assert_eq!(llrs[1], f64::INFINITY);
    assert!(llrs[2].is_nan());
    assert_eq!(erasures, [3, 1]);
}

#[test]
fn erasure_bounds_are_checked_before_duplicate_positions() {
    let llrs = [0.0; 3];
    let erasures = [1, 1, 3];

    let error = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
        3,
        DecoderConfig::default(),
    )
    .expect_err("all bounds errors precede duplicate errors");

    assert_eq!(
        error,
        LdpcError::ErasureIndexOutOfBounds { bit: 3, bits: 3 }
    );
}

#[test]
fn the_first_repeated_erasure_is_reported() {
    let llrs = [0.0; 3];
    let erasures = [2, 1, 2, 1];

    let error = prepared_channel(
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
        3,
        DecoderConfig::default(),
    )
    .expect_err("the first duplicate in input order should be reported");

    assert_eq!(error, LdpcError::DuplicateErasureIndex { bit: 2 });
}
