use std::collections::HashSet;

use gf2m::Gf256;
use gf_linalg::{FieldElement, Matrix, MAX_MATRIX_DIM};
use gfpm::Gf3;

type Gf2 = gfpm::Gf<2, 1, 1>;

fn gf2_matrix(rows: usize, cols: usize, values: &[u8]) -> Matrix<Gf2> {
    Matrix::try_new(
        rows,
        cols,
        values
            .iter()
            .copied()
            .map(|value| Gf2::new(u64::from(value)))
            .collect(),
    )
    .unwrap()
}

fn assert_rref_invariants<F: FieldElement>(
    source: &Matrix<F>,
    reduced: &Matrix<F>,
    pivot_columns: &[usize],
    rank: usize,
) {
    assert_eq!(
        (reduced.rows(), reduced.cols()),
        (source.rows(), source.cols())
    );
    assert_eq!(reduced.as_slice().len(), source.rows() * source.cols());
    assert_eq!(rank, pivot_columns.len());
    assert!(rank <= source.rows().min(source.cols()));
    assert!(pivot_columns.iter().all(|&col| col < source.cols()));
    assert!(pivot_columns.windows(2).all(|pair| pair[0] < pair[1]));

    let mut zero_row_seen = false;
    let mut nonzero_rows = 0;
    for row_index in 0..reduced.rows() {
        let row = reduced.row(row_index).unwrap();
        if let Some((leading_col, leading_value)) =
            row.iter().enumerate().find(|(_, value)| !value.is_zero())
        {
            assert!(!zero_row_seen, "nonzero rows must precede zero rows");
            assert_eq!(*leading_value, F::one());
            assert_eq!(pivot_columns.get(nonzero_rows), Some(&leading_col));
            nonzero_rows += 1;
        } else {
            zero_row_seen = true;
        }
    }
    assert_eq!(nonzero_rows, rank);

    for (pivot_row, &pivot_col) in pivot_columns.iter().enumerate() {
        assert_eq!(reduced.get(pivot_row, pivot_col), Some(F::one()));
        for row in 0..reduced.rows() {
            if row != pivot_row {
                assert_eq!(reduced.get(row, pivot_col), Some(F::zero()));
            }
        }
    }
}

#[test]
fn gf2_rref_handles_zero_and_single_element_matrices() {
    let zero = gf2_matrix(2, 3, &[0, 0, 0, 0, 0, 0]);
    let zero_before = zero.as_slice().to_vec();
    let zero_result = zero.rref();

    assert_eq!(zero_result.matrix(), &zero);
    assert_eq!(zero_result.pivot_columns(), &[]);
    assert_eq!(zero_result.rank(), 0);
    assert_eq!(zero.rank(), 0);
    assert_rref_invariants(
        &zero,
        zero_result.matrix(),
        zero_result.pivot_columns(),
        zero_result.rank(),
    );
    assert_eq!(zero.as_slice(), zero_before);

    let scalar = gf2_matrix(1, 1, &[1]);
    let scalar_before = scalar.as_slice().to_vec();
    let scalar_result = scalar.rref();
    assert_eq!(scalar_result.matrix(), &scalar);
    assert_eq!(scalar_result.pivot_columns(), &[0]);
    assert_eq!(scalar_result.rank(), 1);
    assert_eq!(scalar.rank(), 1);
    assert_eq!(scalar.as_slice(), scalar_before);
}

#[test]
fn gf2_rref_skips_zero_columns_and_swaps_rows() {
    let matrix = gf2_matrix(3, 4, &[0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1]);
    let before = matrix.as_slice().to_vec();
    let result = matrix.rref();
    let expected = gf2_matrix(3, 4, &[0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0]);

    assert_eq!(result.matrix(), &expected);
    assert_eq!(result.pivot_columns(), &[1, 2]);
    assert_eq!(result.rank(), 2);
    assert_eq!(matrix.rank(), 2);
    assert_rref_invariants(
        &matrix,
        result.matrix(),
        result.pivot_columns(),
        result.rank(),
    );
    assert_eq!(matrix.as_slice(), before);
}

#[test]
fn gf2_rref_reduces_dependent_rows_and_preserves_the_kernel() {
    let matrix = gf2_matrix(3, 3, &[1, 0, 1, 0, 1, 1, 1, 1, 0]);
    let before = matrix.as_slice().to_vec();
    let result = matrix.rref();
    let expected = gf2_matrix(3, 3, &[1, 0, 1, 0, 1, 1, 0, 0, 0]);

    assert_eq!(result.matrix(), &expected);
    assert_eq!(result.pivot_columns(), &[0, 1]);
    assert_eq!(result.rank(), 2);
    assert_rref_invariants(
        &matrix,
        result.matrix(),
        result.pivot_columns(),
        result.rank(),
    );
    assert_eq!(matrix.as_slice(), before);

    for vector in 0..(1_u16 << matrix.cols()) {
        assert_eq!(
            binary_vector_is_in_kernel(&matrix, vector),
            binary_vector_is_in_kernel(result.matrix(), vector),
            "kernel changed for vector mask {vector:#b}"
        );
    }
}

#[test]
fn gf2_rref_handles_tall_wide_and_maximum_axis_shapes() {
    let tall = gf2_matrix(4, 2, &[1, 0, 0, 1, 1, 1, 0, 0]);
    let tall_before = tall.as_slice().to_vec();
    let tall_result = tall.rref();
    assert_eq!(
        tall_result.matrix(),
        &gf2_matrix(4, 2, &[1, 0, 0, 1, 0, 0, 0, 0])
    );
    assert_eq!(tall_result.pivot_columns(), &[0, 1]);
    assert_eq!(tall_result.rank(), 2);
    assert_eq!(tall.rank(), 2);
    assert_rref_invariants(
        &tall,
        tall_result.matrix(),
        tall_result.pivot_columns(),
        tall_result.rank(),
    );
    assert_eq!(tall.as_slice(), tall_before);

    let wide = gf2_matrix(2, 4, &[1, 0, 1, 0, 0, 1, 0, 1]);
    let wide_before = wide.as_slice().to_vec();
    let wide_result = wide.rref();
    assert_eq!(wide_result.matrix(), &wide);
    assert_eq!(wide_result.pivot_columns(), &[0, 1]);
    assert_eq!(wide_result.rank(), 2);
    assert_rref_invariants(
        &wide,
        wide_result.matrix(),
        wide_result.pivot_columns(),
        wide_result.rank(),
    );
    assert_eq!(wide.as_slice(), wide_before);

    let mut tall_values = vec![Gf2::zero(); MAX_MATRIX_DIM];
    tall_values[MAX_MATRIX_DIM - 1] = Gf2::one();
    let maximum_rows = Matrix::try_new(MAX_MATRIX_DIM, 1, tall_values).unwrap();
    let maximum_rows_before = maximum_rows.as_slice().to_vec();
    let maximum_rows_result = maximum_rows.rref();
    assert_eq!(maximum_rows_result.matrix().rows(), MAX_MATRIX_DIM);
    assert_eq!(maximum_rows_result.matrix().cols(), 1);
    assert_eq!(maximum_rows_result.pivot_columns(), &[0]);
    assert_eq!(maximum_rows_result.rank(), 1);
    assert_eq!(maximum_rows.rank(), 1);
    assert_eq!(maximum_rows_result.matrix().get(0, 0), Some(Gf2::one()));
    assert!(maximum_rows_result.matrix().as_slice()[1..]
        .iter()
        .all(|value| value.is_zero()));
    assert_rref_invariants(
        &maximum_rows,
        maximum_rows_result.matrix(),
        maximum_rows_result.pivot_columns(),
        maximum_rows_result.rank(),
    );
    assert_eq!(maximum_rows.as_slice(), maximum_rows_before);

    let mut wide_values = vec![Gf2::zero(); MAX_MATRIX_DIM];
    wide_values[MAX_MATRIX_DIM - 1] = Gf2::one();
    let maximum_cols = Matrix::try_new(1, MAX_MATRIX_DIM, wide_values).unwrap();
    let maximum_cols_before = maximum_cols.as_slice().to_vec();
    let maximum_cols_result = maximum_cols.rref();
    assert_eq!(maximum_cols_result.matrix().rows(), 1);
    assert_eq!(maximum_cols_result.matrix().cols(), MAX_MATRIX_DIM);
    assert_eq!(maximum_cols_result.pivot_columns(), &[MAX_MATRIX_DIM - 1]);
    assert_eq!(maximum_cols_result.rank(), 1);
    assert_eq!(
        maximum_cols_result.matrix().get(0, MAX_MATRIX_DIM - 1),
        Some(Gf2::one())
    );
    assert_eq!(maximum_cols.rank(), 1);
    assert_rref_invariants(
        &maximum_cols,
        maximum_cols_result.matrix(),
        maximum_cols_result.pivot_columns(),
        maximum_cols_result.rank(),
    );
    assert_eq!(maximum_cols.as_slice(), maximum_cols_before);
}

#[test]
fn rref_result_is_idempotent_and_can_transfer_matrix_ownership() {
    let matrix = gf2_matrix(2, 3, &[1, 1, 0, 1, 0, 1]);
    let expected = gf2_matrix(2, 3, &[1, 0, 1, 0, 1, 1]);
    let once = matrix.rref();
    let twice = once.matrix().rref();

    assert_eq!(once.matrix(), &expected);
    assert_eq!(twice.matrix(), once.matrix());
    assert_eq!(twice.pivot_columns(), once.pivot_columns());
    assert_eq!(twice.rank(), once.rank());
    assert_eq!(matrix.rref().into_matrix(), expected);
    assert_eq!(matrix.as_slice(), &[1, 1, 0, 1, 0, 1].map(Gf2::new));
}

#[test]
fn rref_normalizes_nonunit_pivots_over_gf256_and_gf3() {
    let gf256 = Matrix::<Gf256>::try_new(2, 2, [2, 2, 0, 2].map(Gf256::new).to_vec()).unwrap();
    let gf256_before = gf256.as_slice().to_vec();
    let gf256_result = gf256.rref();
    assert_eq!(
        gf256_result.matrix(),
        &Matrix::try_new(2, 2, [1, 0, 0, 1].map(Gf256::new).to_vec()).unwrap()
    );
    assert_eq!(gf256_result.pivot_columns(), &[0, 1]);
    assert_eq!(gf256_result.rank(), 2);
    assert_eq!(gf256.rank(), 2);
    assert_rref_invariants(
        &gf256,
        gf256_result.matrix(),
        gf256_result.pivot_columns(),
        gf256_result.rank(),
    );
    assert_eq!(gf256.as_slice(), gf256_before);

    let gf3_row = Matrix::<Gf3>::try_new(1, 2, [2, 1].map(Gf3::new).to_vec()).unwrap();
    let gf3_row_before = gf3_row.as_slice().to_vec();
    let gf3_row_result = gf3_row.rref();
    assert_eq!(
        gf3_row_result.matrix(),
        &Matrix::try_new(1, 2, [1, 2].map(Gf3::new).to_vec()).unwrap()
    );
    assert_eq!(gf3_row_result.pivot_columns(), &[0]);
    assert_eq!(gf3_row_result.rank(), 1);
    assert_eq!(gf3_row.rank(), 1);
    assert_rref_invariants(
        &gf3_row,
        gf3_row_result.matrix(),
        gf3_row_result.pivot_columns(),
        gf3_row_result.rank(),
    );
    assert_eq!(gf3_row.as_slice(), gf3_row_before);

    let gf3_dependent = Matrix::<Gf3>::try_new(2, 2, [1, 2, 2, 1].map(Gf3::new).to_vec()).unwrap();
    let gf3_dependent_before = gf3_dependent.as_slice().to_vec();
    let gf3_dependent_result = gf3_dependent.rref();
    assert_eq!(
        gf3_dependent_result.matrix(),
        &Matrix::try_new(2, 2, [1, 2, 0, 0].map(Gf3::new).to_vec()).unwrap()
    );
    assert_eq!(gf3_dependent_result.pivot_columns(), &[0]);
    assert_eq!(gf3_dependent_result.rank(), 1);
    assert_eq!(gf3_dependent.rank(), 1);
    assert_rref_invariants(
        &gf3_dependent,
        gf3_dependent_result.matrix(),
        gf3_dependent_result.pivot_columns(),
        gf3_dependent_result.rank(),
    );
    assert_eq!(gf3_dependent.as_slice(), gf3_dependent_before);
}

#[test]
fn exhaustive_small_gf2_matrices_preserve_row_space_kernel_and_rref_rules() {
    for rows in 1..=3 {
        for cols in 1..=3 {
            let element_count = rows * cols;
            for encoding in 0..(1_u16 << element_count) {
                let values: Vec<u8> = (0..element_count)
                    .map(|bit| ((encoding >> bit) & 1) as u8)
                    .collect();
                let matrix = gf2_matrix(rows, cols, &values);
                let source_before = matrix.as_slice().to_vec();
                let result = matrix.rref();

                let source_rows = binary_rows(&matrix);
                let reduced_rows = binary_rows(result.matrix());
                let row_span_size = binary_row_span_size(&source_rows, rows);
                assert!(row_span_size.is_power_of_two());
                let independent_rank = row_span_size.trailing_zeros() as usize;

                assert_eq!(1_usize << result.rank(), row_span_size);
                assert_eq!(result.rank(), independent_rank);
                assert_eq!(matrix.rank(), independent_rank);
                assert_rref_invariants(
                    &matrix,
                    result.matrix(),
                    result.pivot_columns(),
                    result.rank(),
                );
                assert_eq!(matrix.as_slice(), source_before);

                for vector in 0..(1_u16 << cols) {
                    assert_eq!(
                        binary_vector_is_in_kernel_rows(&source_rows, vector),
                        binary_vector_is_in_kernel_rows(&reduced_rows, vector),
                        "kernel changed for {rows}x{cols} matrix {encoding:#b}, vector {vector:#b}"
                    );
                }
            }
        }
    }
}

fn binary_rows(matrix: &Matrix<Gf2>) -> Vec<u16> {
    (0..matrix.rows())
        .map(|row| {
            matrix
                .row(row)
                .unwrap()
                .iter()
                .enumerate()
                .fold(0, |mask, (col, value)| {
                    if *value == Gf2::one() {
                        mask | (1 << col)
                    } else {
                        mask
                    }
                })
        })
        .collect()
}

fn binary_row_span_size(rows: &[u16], row_count: usize) -> usize {
    let mut span = HashSet::with_capacity(1 << row_count);
    for combination in 0..(1_usize << row_count) {
        let vector = rows.iter().enumerate().fold(0, |sum, (row, value)| {
            if combination & (1 << row) != 0 {
                sum ^ value
            } else {
                sum
            }
        });
        span.insert(vector);
    }
    span.len()
}

fn binary_vector_is_in_kernel(matrix: &Matrix<Gf2>, vector: u16) -> bool {
    binary_vector_is_in_kernel_rows(&binary_rows(matrix), vector)
}

fn binary_vector_is_in_kernel_rows(rows: &[u16], vector: u16) -> bool {
    rows.iter().all(|row| (row & vector).count_ones() % 2 == 0)
}
