use ldpc_codes::{spa_step, Bit, DecodeInput, DecoderConfig, LdpcError, ParityCheckMatrix};

#[test]
fn zero_iteration_budget_still_runs_one_complete_step_and_keeps_inputs() {
    let checks =
        ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]]).expect("matrix is valid");
    let llrs = [2.0, -1.0, 3.0];
    let erasures: [usize; 0] = [];
    let config = DecoderConfig::try_new(0, 20.0).expect("limit is valid");

    let result = spa_step(
        &checks,
        config,
        DecodeInput {
            llrs: &llrs,
            erasures: &erasures,
        },
    )
    .expect("valid input runs one SPA step");

    assert_eq!(result.word(), [Bit::Zero, Bit::Zero, Bit::Zero]);
    assert_eq!(result.posterior_llrs(), [1.0, 4.0, 2.0]);
    assert_eq!(
        checks.syndrome(result.word()),
        Ok(vec![Bit::Zero, Bit::Zero])
    );
    assert_eq!(llrs, [2.0, -1.0, 3.0]);
    assert_eq!(erasures, []);
    assert_eq!(result, result.clone());

    let saved_result = result.clone();
    let next_llrs = [-8.0, 9.0, 7.0];
    let next_erasures = [1];
    let _next_result = spa_step(
        &checks,
        DecoderConfig::default(),
        DecodeInput {
            llrs: &next_llrs,
            erasures: &next_erasures,
        },
    )
    .expect("a separate block is valid");

    assert_eq!(result, saved_result);
    assert_eq!(llrs, [2.0, -1.0, 3.0]);
    assert_eq!(next_llrs, [-8.0, 9.0, 7.0]);
    assert_eq!(erasures, []);
    assert_eq!(next_erasures, [1]);
}

#[test]
fn a_zero_initial_syndrome_does_not_skip_the_step() {
    let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![0, 1]]).expect("matrix is valid");
    let llrs = [2.0, 2.0];

    let result = spa_step(
        &checks,
        DecoderConfig::default(),
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
    )
    .expect("valid codeword can be stepped");

    assert_eq!(checks.is_codeword(&[Bit::Zero, Bit::Zero]), Ok(true));
    assert_eq!(result.posterior_llrs(), [4.0, 4.0]);
    assert_eq!(result.word(), [Bit::Zero, Bit::Zero]);
}

#[test]
fn isolated_bits_and_both_signed_zeros_have_zero_hard_decisions() {
    let checks = ParityCheckMatrix::try_from_rows(4, vec![vec![]]).expect("matrix is valid");
    let llrs = [-0.0, 0.0, -1.0, 1.0];

    let result = spa_step(
        &checks,
        DecoderConfig::default(),
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
    )
    .expect("isolated bits are valid");

    assert_eq!(result.word(), [Bit::Zero, Bit::Zero, Bit::One, Bit::Zero]);
    assert_eq!(result.posterior_llrs()[0].to_bits(), (-0.0_f64).to_bits());
    assert_eq!(result.posterior_llrs()[1].to_bits(), 0.0_f64.to_bits());
    assert_eq!(result.posterior_llrs()[2..], [-1.0, 1.0]);
}

#[test]
fn invalid_input_is_returned_as_an_error() {
    let checks = ParityCheckMatrix::try_from_rows(2, vec![vec![0, 1]]).expect("matrix is valid");

    let error = spa_step(
        &checks,
        DecoderConfig::default(),
        DecodeInput {
            llrs: &[f64::NAN, 0.0],
            erasures: &[],
        },
    )
    .expect_err("non-finite LLR must be rejected");

    assert_eq!(error, LdpcError::NonFiniteLlr { index: 0 });
}
