use gf_linalg::Matrix;

use crate::{encoder::dense_from_checks, Gf2, ParityCheckMatrix, SystematicEncoder};

#[test]
fn dense_conversion_preserves_shape_entries_and_sparse_source() {
    let checks =
        ParityCheckMatrix::try_from_rows(4, vec![vec![2, 0], vec![], vec![0, 2], vec![3]]).unwrap();

    let dense = dense_from_checks(&checks).unwrap();

    assert_eq!(dense.rows(), 4);
    assert_eq!(dense.cols(), 4);
    assert_eq!(
        dense.as_slice(),
        &[
            Gf2::one(),
            Gf2::zero(),
            Gf2::one(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::one(),
            Gf2::zero(),
            Gf2::one(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::zero(),
            Gf2::one(),
        ]
    );

    assert_eq!(checks.check_bits(0), Some(&[0, 2][..]));
    assert_eq!(checks.check_bits(1), Some(&[][..]));
    assert_eq!(checks.check_bits(2), Some(&[0, 2][..]));
    assert_eq!(checks.bit_checks(0), Some(&[0, 2][..]));
    assert_eq!(checks.bit_checks(1), Some(&[][..]));
    assert_eq!(checks.bit_checks(2), Some(&[0, 2][..]));
    assert_eq!(checks.bit_checks(3), Some(&[3][..]));
}

#[test]
fn dense_conversion_keeps_rectangular_dimensions_and_isolated_columns() {
    let checks = ParityCheckMatrix::try_from_rows(5, vec![vec![4, 1], vec![2]]).unwrap();

    let dense = dense_from_checks(&checks).unwrap();

    assert_eq!(
        dense,
        Matrix::try_new(
            2,
            5,
            vec![
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::zero(),
                Gf2::one(),
                Gf2::zero(),
                Gf2::zero(),
            ]
        )
        .unwrap()
    );
    assert_eq!(checks.bit_checks(0), Some(&[][..]));
    assert_eq!(checks.bit_checks(3), Some(&[][..]));
}

#[test]
fn preparation_builds_exact_coefficients_and_mapping_for_main_example() {
    let checks = ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )
    .unwrap();

    let encoder = SystematicEncoder::try_new(checks).unwrap();

    assert_eq!(
        encoder.parity_coefficients,
        Matrix::try_new(3, 3, [0, 1, 1, 1, 1, 1, 1, 0, 1].map(Gf2::new).to_vec()).unwrap()
    );
    assert_eq!(encoder.systematic_to_original, [3, 4, 5, 0, 1, 2]);
    assert_mapping_is_permutation(&encoder);
}

#[test]
fn preparation_builds_exact_coefficients_and_mapping_for_cycle_three() {
    let checks = ParityCheckMatrix::try_from_rows(5, vec![vec![0, 1, 4], vec![2, 3, 4]]).unwrap();

    let encoder = SystematicEncoder::try_new(checks).unwrap();

    assert_eq!(
        encoder.parity_coefficients,
        Matrix::try_new(2, 3, [1, 0, 1, 0, 1, 1].map(Gf2::new).to_vec()).unwrap()
    );
    assert_eq!(encoder.systematic_to_original, [1, 3, 4, 0, 2]);
    assert_mapping_is_permutation(&encoder);
}

#[test]
fn preparation_distinguishes_swap_from_identity_permutation() {
    let swap =
        SystematicEncoder::try_new(ParityCheckMatrix::try_from_rows(3, vec![vec![1, 2]]).unwrap())
            .unwrap();
    assert_eq!(swap.systematic_to_original, [0, 2, 1]);
    assert_mapping_is_permutation(&swap);

    let identity =
        SystematicEncoder::try_new(ParityCheckMatrix::try_from_rows(3, vec![vec![2]]).unwrap())
            .unwrap();
    assert_eq!(identity.systematic_to_original, [0, 1, 2]);
    assert_mapping_is_permutation(&identity);
}

fn assert_mapping_is_permutation(encoder: &SystematicEncoder) {
    let mut sorted_positions = encoder.systematic_to_original.clone();
    sorted_positions.sort_unstable();
    assert_eq!(
        sorted_positions,
        (0..encoder.codeword_len()).collect::<Vec<_>>()
    );
}
