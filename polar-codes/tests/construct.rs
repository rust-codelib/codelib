use polar_codes::{bhattacharyya_bec, PolarCode, PolarError};

#[test]
fn bec_scores_follow_the_minus_and_plus_recursion() {
    let scores = bhattacharyya_bec(4, 0.5).unwrap();
    assert_eq!(scores, vec![0.9375, 0.5625, 0.4375, 0.0625]);

    let code = PolarCode::from_bec(4, 2, 0.5).unwrap();
    assert_eq!(code.information_positions(), &[2, 3]);
    assert_eq!(
        PolarCode::from_bec(4, 1, 0.5)
            .unwrap()
            .information_positions(),
        &[3]
    );
    assert_eq!(
        PolarCode::from_bec(4, 3, 0.5)
            .unwrap()
            .information_positions(),
        &[1, 2, 3]
    );
}

#[test]
fn equal_scores_prefer_the_smaller_index() {
    let code = PolarCode::from_bec(4, 2, 0.0).unwrap();
    assert_eq!(code.information_positions(), &[0, 1]);
    assert!(bhattacharyya_bec(4, 0.0)
        .unwrap()
        .iter()
        .all(|score| *score == 0.0));
}

#[test]
fn construction_rejects_a_probability_or_dimension_outside_the_code() {
    assert_eq!(
        bhattacharyya_bec(4, 1.1),
        Err(PolarError::InvalidErasureProbability)
    );
    assert_eq!(
        bhattacharyya_bec(4, f64::NAN),
        Err(PolarError::InvalidErasureProbability)
    );
    assert_eq!(
        PolarCode::from_bec(4, 5, 0.5),
        Err(PolarError::InvalidMessageLength {
            message_len: 5,
            length: 4
        })
    );
    assert!(PolarCode::from_bec(4, 0, 0.5)
        .unwrap()
        .information_positions()
        .is_empty());
    assert_eq!(
        PolarCode::from_bec(1, 1, 1.0)
            .unwrap()
            .information_positions(),
        &[0]
    );
}
