use ldpc_codes::{DecodeInput, DecoderConfig, LdpcError};

#[test]
fn default_config_uses_planned_values() {
    let config = DecoderConfig::default();

    assert_eq!(config.max_iterations(), 50);
    assert_eq!(config.llr_limit(), 20.0);
}

#[test]
fn config_accepts_zero_iteration_budget_and_inclusive_limit() {
    let config = DecoderConfig::try_new(0, 20.0).expect("20 is the inclusive upper bound");

    assert_eq!(config.max_iterations(), 0);
    assert_eq!(config.llr_limit(), 20.0);
}

#[test]
fn config_accepts_a_small_positive_finite_limit() {
    let config = DecoderConfig::try_new(7, f64::MIN_POSITIVE)
        .expect("every positive finite limit in range is valid");

    assert_eq!(config.max_iterations(), 7);
    assert_eq!(config.llr_limit(), f64::MIN_POSITIVE);
}

#[test]
fn config_rejects_limits_outside_the_finite_open_closed_range() {
    for llr_limit in [
        0.0,
        -0.0,
        -1.0,
        20.000_000_000_000_004,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert!(
            matches!(
                DecoderConfig::try_new(50, llr_limit),
                Err(LdpcError::InvalidLlrLimit)
            ),
            "expected {llr_limit:?} to be rejected"
        );
    }
}

#[test]
fn invalid_limit_error_has_a_clear_display_message() {
    let error = LdpcError::InvalidLlrLimit;
    let message = error.to_string();

    assert!(message.contains("конечным"));
    assert!(message.contains("(0, 20]"));
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn decode_input_borrows_the_llrs_and_erasure_positions() {
    let llrs = [1.5, -2.0, 0.0];
    let erasures = [2, 0];
    let input = DecodeInput {
        llrs: &llrs,
        erasures: &erasures,
    };
    let copied_input = input;

    assert_eq!(input.llrs, &[1.5, -2.0, 0.0]);
    assert_eq!(input.erasures, &[2, 0]);
    assert_eq!(copied_input.llrs, input.llrs);
    assert_eq!(copied_input.erasures, input.erasures);
    assert_eq!(llrs, [1.5, -2.0, 0.0]);
    assert_eq!(erasures, [2, 0]);
}
