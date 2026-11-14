use ldpc_codes::ParityCheckMatrix;

pub fn general_plan_checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(
        6,
        vec![vec![0, 1, 3], vec![1, 2, 4], vec![0, 4, 5], vec![2, 3, 5]],
    )
    .expect("the general-plan matrix is valid")
}
