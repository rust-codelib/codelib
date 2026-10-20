use gf2m::{Gf256, Gf65536};
use gf_linalg::{
    EquationInsertion, FieldElement, LinalgError, LinearSystem, Matrix, MAX_MATRIX_DIM,
};
use gfpm::{Gf5, Gf9};

fn check_fountain_vector<F: FieldElement>(a: &[F], b: &[F], x: &[F]) {
    let coefficients = Matrix::try_new(3, 3, a.to_vec()).unwrap();
    let rhs = Matrix::try_new(3, 2, b.to_vec()).unwrap();
    let expected = Matrix::try_new(3, 2, x.to_vec()).unwrap();
    let mut system = LinearSystem::<F>::try_new(3, 2).unwrap();
    assert_eq!(
        (system.unknowns(), system.rhs_columns(), system.rank()),
        (3, 2, 0)
    );
    for (row, pivot_column) in [1, 0, 2].into_iter().enumerate() {
        assert_eq!(
            system.insert_equation(coefficients.row(row).unwrap(), rhs.row(row).unwrap()),
            Ok(EquationInsertion::Independent {
                pivot_column,
                rank: row + 1
            })
        );
        if row < 2 {
            assert_eq!(system.try_solution(), Ok(None));
        }
    }
    assert_eq!(system.try_solution().unwrap(), Some(expected));
    assert_eq!(
        coefficients
            .try_mul(&system.try_solution().unwrap().unwrap())
            .unwrap(),
        rhs
    );
    // A second query cannot consume or modify the saved solution/basis.
    assert_eq!(system.try_solution().unwrap().unwrap().as_slice(), x);
    assert_eq!(coefficients.as_slice(), a);
    assert_eq!(rhs.as_slice(), b);
}

// Fixed reference values from fountain-v1.json, computed by independent Python
// and JavaScript arithmetic. Expected values never use the implementation under test.
#[test]
fn fountain_gf256_reference_with_out_of_order_pivots() {
    check_fountain_vector(
        &[0, 182, 101, 242, 9, 128, 215, 78, 126].map(Gf256::new),
        &[14, 222, 37, 239, 95, 215].map(Gf256::new),
        &[128, 255, 52, 18, 0, 0].map(Gf256::new),
    );
}

#[test]
fn fountain_gf65536_reference_with_multiple_rhs() {
    check_fountain_vector(
        &[0, 12470, 2661, 30194, 35337, 9856, 4055, 10830, 25726].map(Gf65536::new),
        &[10825, 24903, 25934, 37814, 47720, 37504].map(Gf65536::new),
        &[384, 255, 4660, 32768, 21930, 204].map(Gf65536::new),
    );
}

#[test]
fn odd_characteristic_dependence_and_contradictions_preserve_partial_basis() {
    let mut system = LinearSystem::<Gf5>::try_new(2, 2).unwrap();
    let first_a = [1, 1].map(Gf5::new);
    let first_b = [1, 3].map(Gf5::new);
    system.insert_equation(&first_a, &first_b).unwrap();
    assert_eq!(
        system.insert_equation(&[3, 3].map(Gf5::new), &[3, 4].map(Gf5::new)),
        Ok(EquationInsertion::Dependent)
    );
    assert_eq!(
        system.insert_equation(&[3, 3].map(Gf5::new), &[3, 0].map(Gf5::new)),
        Ok(EquationInsertion::Inconsistent { rhs_column: 1 })
    );
    assert_eq!(system.rank(), 1);
    assert_eq!(system.try_solution(), Ok(None));
    system
        .insert_equation(&[2, 1].map(Gf5::new), &[3, 3].map(Gf5::new))
        .unwrap();
    let expected = Matrix::try_new(2, 2, [2, 0, 4, 3].map(Gf5::new).to_vec()).unwrap();
    assert_eq!(system.try_solution().unwrap(), Some(expected));
    assert_eq!(first_a, [1, 1].map(Gf5::new));
    assert_eq!(first_b, [1, 3].map(Gf5::new));
}

#[test]
fn contradiction_after_full_rank_does_not_invalidate_the_mathematical_system() {
    let mut system = LinearSystem::<Gf9>::try_new(1, 2).unwrap();
    let coefficient = Gf9::new(3);
    let answer = [Gf9::new(4), Gf9::zero()];
    system
        .insert_equation(&[coefficient], &answer.map(|x| coefficient * x))
        .unwrap();
    let solution = system.try_solution().unwrap();
    assert_eq!(solution.as_ref().unwrap().as_slice(), &answer);
    assert_eq!(
        system.insert_equation(&[Gf9::one()], &[Gf9::new(4), Gf9::one()]),
        Ok(EquationInsertion::Inconsistent { rhs_column: 1 })
    );
    assert_eq!(system.try_solution().unwrap(), solution);
    assert_eq!(system.rank(), 1);
}

#[test]
fn zero_rows_report_first_nonzero_rhs_and_never_increase_rank() {
    let mut system: LinearSystem = LinearSystem::try_new(3, 3).unwrap();
    assert_eq!(
        system.insert_equation(&[Gf256::zero(); 3], &[Gf256::zero(); 3]),
        Ok(EquationInsertion::Dependent)
    );
    assert_eq!(
        system.insert_equation(&[Gf256::zero(); 3], &[0, 2, 3].map(Gf256::new)),
        Ok(EquationInsertion::Inconsistent { rhs_column: 1 })
    );
    assert_eq!(system.rank(), 0);
    assert_eq!(system.try_solution(), Ok(None));
}

#[test]
fn malformed_rows_check_coefficients_first_and_leave_the_basis_usable() {
    let mut system = LinearSystem::<Gf5>::try_new(2, 1).unwrap();
    system
        .insert_equation(&[1, 0].map(Gf5::new), &[Gf5::new(2)])
        .unwrap();
    for count in [0, 1, 3] {
        assert_eq!(
            system.insert_equation(&vec![Gf5::one(); count], &[]),
            Err(LinalgError::EquationCoefficientCountMismatch {
                expected: 2,
                actual: count
            })
        );
    }
    for count in [0, 2] {
        assert_eq!(
            system.insert_equation(&[Gf5::one(); 2], &vec![Gf5::one(); count]),
            Err(LinalgError::EquationRhsCountMismatch {
                expected: 1,
                actual: count
            })
        );
    }
    assert_eq!(system.rank(), 1);
    system
        .insert_equation(&[0, 1].map(Gf5::new), &[Gf5::new(4)])
        .unwrap();
    assert_eq!(
        system.try_solution().unwrap().unwrap().as_slice(),
        &[2, 4].map(Gf5::new)
    );
}

#[test]
fn validates_both_axes_before_allocating_and_accepts_their_independent_limits() {
    for (unknowns, rhs_columns) in [
        (0, 1),
        (1, 0),
        (0, 0),
        (MAX_MATRIX_DIM + 1, 1),
        (1, MAX_MATRIX_DIM + 1),
        (usize::MAX, usize::MAX),
    ] {
        assert!(matches!(
            LinearSystem::<Gf256>::try_new(unknowns, rhs_columns),
            Err(LinalgError::InvalidLinearSystemDimensions { unknowns: u, rhs_columns: r })
                if u == unknowns && r == rhs_columns
        ));
    }
    // Each buffer has a legal axis. A concatenated [A | B] would be too wide.
    let mut maximum = LinearSystem::<Gf256>::try_new(MAX_MATRIX_DIM, MAX_MATRIX_DIM).unwrap();
    let mut row = vec![Gf256::zero(); MAX_MATRIX_DIM];
    row[MAX_MATRIX_DIM - 1] = Gf256::one();
    assert_eq!(
        maximum.insert_equation(&row, &row),
        Ok(EquationInsertion::Independent {
            pivot_column: MAX_MATRIX_DIM - 1,
            rank: 1
        })
    );
    assert_eq!(maximum.try_solution(), Ok(None));
    let mut wide_rhs = LinearSystem::<Gf256>::try_new(1, MAX_MATRIX_DIM).unwrap();
    wide_rhs.insert_equation(&[Gf256::one()], &row).unwrap();
    let solution = wide_rhs.try_solution().unwrap().unwrap();
    assert_eq!((solution.rows(), solution.cols()), (1, MAX_MATRIX_DIM));
    assert_eq!(solution.as_slice(), row);
}

#[test]
fn number_of_received_equations_is_not_a_matrix_axis() {
    let mut system = LinearSystem::<Gf5>::try_new(1, 1).unwrap();
    system
        .insert_equation(&[Gf5::one()], &[Gf5::new(3)])
        .unwrap();
    for _ in 0..MAX_MATRIX_DIM + 1 {
        assert_eq!(
            system.insert_equation(&[Gf5::new(2)], &[Gf5::one()]),
            Ok(EquationInsertion::Dependent)
        );
    }
    assert_eq!(system.rank(), 1);
    assert_eq!(
        system.try_solution().unwrap().unwrap().as_slice(),
        &[Gf5::new(3)]
    );
}

#[test]
fn every_two_by_two_matrix_over_gf5_agrees_with_rank_and_inverse_oracles() {
    let expected = Matrix::try_new(2, 2, [2, 0, 4, 3].map(Gf5::new).to_vec()).unwrap();
    for encoded in 0..625 {
        let mut rest = encoded;
        let coefficients: Vec<_> = (0..4)
            .map(|_| {
                let value = Gf5::new(rest % 5);
                rest /= 5;
                value
            })
            .collect();
        let a = Matrix::try_new(2, 2, coefficients).unwrap();
        let b = a.try_mul(&expected).unwrap();
        for order in [[0, 1], [1, 0]] {
            let mut system = LinearSystem::<Gf5>::try_new(2, 2).unwrap();
            for row in order {
                system
                    .insert_equation(a.row(row).unwrap(), b.row(row).unwrap())
                    .unwrap();
            }
            assert_eq!(system.rank(), a.rank(), "matrix {encoded}, order {order:?}");
            let solution = system.try_solution().unwrap();
            if a.rank() == 2 {
                assert_eq!(solution.as_ref(), Some(&expected));
                let inverse_answer = a.try_inverse().unwrap().try_mul(&b).unwrap();
                assert_eq!(solution, Some(inverse_answer));
            } else {
                assert!(solution.is_none());
            }
        }
    }
}

#[test]
fn overdetermined_system_preserves_rank_and_solution_for_every_row_order() {
    let a = Matrix::try_new(
        4,
        3,
        [0, 2, 3, 2, 1, 4, 1, 0, 1, 4, 3, 1].map(Gf5::new).to_vec(),
    )
    .unwrap();
    let expected = Matrix::try_new(3, 2, [2, 0, 4, 3, 1, 2].map(Gf5::new).to_vec()).unwrap();
    let b = a.try_mul(&expected).unwrap();
    assert_eq!(a.rank(), 3);
    for first in 0..4 {
        for second in 0..4 {
            for third in 0..4 {
                if first == second || first == third || second == third {
                    continue;
                }
                let fourth = (0..4)
                    .find(|id| ![first, second, third].contains(id))
                    .unwrap();
                let mut system = LinearSystem::<Gf5>::try_new(3, 2).unwrap();
                let mut prefix = Vec::new();
                for row in [first, second, third, fourth] {
                    prefix.extend_from_slice(a.row(row).unwrap());
                    system
                        .insert_equation(a.row(row).unwrap(), b.row(row).unwrap())
                        .unwrap();
                    let rank = Matrix::try_new(prefix.len() / 3, 3, prefix.clone())
                        .unwrap()
                        .rank();
                    assert_eq!(system.rank(), rank);
                    if let Some(solution) = system.try_solution().unwrap() {
                        assert_eq!(solution, expected);
                        assert_eq!(a.try_mul(&solution).unwrap(), b);
                    }
                }
                assert_eq!(system.try_solution().unwrap().as_ref(), Some(&expected));
            }
        }
    }
}
