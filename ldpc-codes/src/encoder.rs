use core::mem::size_of;

use gf_linalg::Matrix;
use gf_linalg::MAX_MATRIX_DIM;

use crate::{Gf2, LdpcError, ParityCheckMatrix};

fn dense_from_checks(checks: &ParityCheckMatrix) -> Result<Matrix<Gf2>, LdpcError> {
    let rows = checks.rows();
    let cols = checks.cols();
    if !(1..=MAX_MATRIX_DIM).contains(&rows) || !(1..=MAX_MATRIX_DIM).contains(&cols) {
        return Err(LdpcError::InvalidDimensions { rows, cols });
    }

    let element_count = rows.checked_mul(cols).ok_or(LdpcError::SizeOverflow)?;
    element_count
        .checked_mul(size_of::<Gf2>())
        .ok_or(LdpcError::SizeOverflow)?;

    let mut data = vec![Gf2::zero(); element_count];
    for row in 0..rows {
        for &bit in checks
            .check_bits(row)
            .expect("row index is within checked matrix dimensions")
        {
            data[row * cols + bit] = Gf2::one();
        }
    }

    Ok(Matrix::try_new(rows, cols, data)?)
}

#[cfg(test)]
mod tests {
    use gf_linalg::Matrix;

    use crate::{encoder::dense_from_checks, Gf2, ParityCheckMatrix};

    #[test]
    fn dense_conversion_preserves_shape_entries_and_sparse_source() {
        let checks =
            ParityCheckMatrix::try_from_rows(4, vec![vec![2, 0], vec![], vec![0, 2], vec![3]])
                .unwrap();

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
}
