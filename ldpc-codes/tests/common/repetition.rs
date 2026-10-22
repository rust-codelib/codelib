use ldpc_codes::ParityCheckMatrix;

pub fn repetition_checks() -> ParityCheckMatrix {
    ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])
        .expect("the repetition-code matrix is valid")
}
