use polar_codes::{Bit, Combine, PolarCode, PolarError};

fn bits(values: &[u8]) -> Vec<Bit> {
    values
        .iter()
        .copied()
        .map(|value| Bit::try_from(value).expect("test vector contains only bits"))
        .collect()
}

fn channel_llrs(word: &[Bit]) -> Vec<f64> {
    word.iter()
        .map(|bit| match bit {
            Bit::Zero => 8.0,
            Bit::One => -8.0,
        })
        .collect()
}

fn assert_noiseless_round_trip(code: &PolarCode, message: &[Bit], rule: Combine) {
    let word = code.encode(message).expect("message matches K");
    let decision = code
        .decode(&channel_llrs(&word), rule)
        .expect("LLRs match N");
    assert_eq!(decision.message(), message);
    assert_eq!(decision.word(), word.as_slice());
    assert_eq!(code.extract_message(decision.word()).unwrap(), message);
    for &index in &code.frozen_positions() {
        assert_eq!(decision.source()[index], Bit::Zero);
    }
}

#[test]
fn successive_cancellation_recovers_the_length_four_hand_example() {
    // u = (1, 0, 1, 0), x = (0, 0, 1, 0). Оба правила f дают тот же жёсткий ответ.
    let code = PolarCode::try_new(4, &[0, 1, 2, 3]).unwrap();
    let message = bits(&[1, 0, 1, 0]);
    let word = code.encode(&message).unwrap();
    assert_eq!(word, bits(&[0, 0, 1, 0]));

    for rule in [Combine::Exact, Combine::MinSum] {
        let decision = code.decode(&[10.0, 10.0, -10.0, 10.0], rule).unwrap();
        assert_eq!(decision.source(), message.as_slice());
        assert_eq!(decision.word(), word.as_slice());
    }
}

#[test]
fn frozen_bit_corrects_the_weaker_half_of_a_length_two_code() {
    // u0 заморожен, u1 = 1, x = (1, 1). Знак первого LLR перепутан, второй надёжен.
    let code = PolarCode::try_new(2, &[1]).unwrap();
    let decision = code.decode(&[2.0, -10.0], Combine::MinSum).unwrap();
    assert_eq!(decision.source(), bits(&[0, 1]).as_slice());
    assert_eq!(decision.message(), bits(&[1]).as_slice());
    assert_eq!(decision.word(), bits(&[1, 1]).as_slice());
}

#[test]
fn every_length_four_code_round_trips_without_noise() {
    for mask in 0..16_u8 {
        let information: Vec<usize> = (0..4).filter(|index| (mask >> index) & 1 == 1).collect();
        let code = PolarCode::try_new(4, &information).unwrap();
        let patterns = 1_usize << code.message_len();
        for pattern in 0..patterns {
            let message = (0..code.message_len())
                .map(|shift| {
                    if (pattern >> shift) & 1 == 1 {
                        Bit::One
                    } else {
                        Bit::Zero
                    }
                })
                .collect::<Vec<_>>();
            assert_noiseless_round_trip(&code, &message, Combine::Exact);
            assert_noiseless_round_trip(&code, &message, Combine::MinSum);
        }
    }
}

#[test]
fn signed_zero_llr_decides_a_zero_bit() {
    let code = PolarCode::try_new(1, &[0]).unwrap();
    assert_eq!(
        code.decode(&[-0.0], Combine::Exact).unwrap().message(),
        bits(&[0]).as_slice()
    );
    assert_eq!(
        code.decode(&[0.0], Combine::MinSum).unwrap().message(),
        bits(&[0]).as_slice()
    );
}

#[test]
fn decoder_rejects_a_bad_observation() {
    let code = PolarCode::try_new(2, &[0]).unwrap();
    assert_eq!(
        code.decode(&[1.0], Combine::Exact),
        Err(PolarError::LlrLengthMismatch {
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        code.decode(&[f64::NAN, 1.0], Combine::Exact),
        Err(PolarError::NonFiniteLlr { index: 0 })
    );
    assert_eq!(
        code.decode(&[0.0, f64::INFINITY], Combine::MinSum),
        Err(PolarError::NonFiniteLlr { index: 1 })
    );
}

#[test]
fn longer_bec_codes_round_trip_without_noise() {
    for length in [1_usize, 2, 8, 16, 32] {
        let code = PolarCode::from_bec(length, length / 2, 0.4).unwrap();
        let zeros = vec![Bit::Zero; code.message_len()];
        let ones = vec![Bit::One; code.message_len()];
        let mixed = (0..code.message_len())
            .map(|index| if index % 2 == 0 { Bit::One } else { Bit::Zero })
            .collect::<Vec<_>>();
        for message in [&zeros, &ones, &mixed] {
            assert_noiseless_round_trip(&code, message, Combine::Exact);
            assert_noiseless_round_trip(&code, message, Combine::MinSum);
        }
    }
}
