//! Проверки мажоритарного декодера Рида.

use rm_codes::{Bit, RmConfig, RmConfigurator, RmDecodeStatus};

fn message_from_index(index: usize, k: usize) -> Vec<Bit> {
    (0..k)
        .map(|bit| Bit::from_bool((index >> bit) & 1 == 1))
        .collect()
}

fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn visit(
        start: usize,
        n: usize,
        k: usize,
        current: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == k {
            result.push(current.clone());
            return;
        }

        let remaining = k - current.len();
        for position in start..=n - remaining {
            current.push(position);
            visit(position + 1, n, k, current, result);
            current.pop();
        }
    }

    let mut result = Vec::new();
    visit(0, n, k, &mut Vec::new(), &mut result);
    result
}

fn apply_errors(word: &[Bit], positions: &[usize]) -> Vec<Bit> {
    let mut received = word.to_vec();
    for &position in positions {
        received[position] = received[position].flip();
    }
    received
}

#[test]
fn corrects_all_error_patterns_up_to_t_for_small_codes() {
    for (r, m) in [(0, 4), (1, 4), (2, 4)] {
        let config = RmConfig::new(r, m).unwrap();
        let code = RmConfigurator::build(config);
        let k = config.dimension();
        let n = config.length();
        let patterns = combinations(n, config.correctable_errors());

        for index in 0..(1usize << k) {
            let message = message_from_index(index, k);
            let encoded = code.encode(&message).unwrap();

            for positions in &patterns {
                let received = apply_errors(&encoded, positions);
                let result = code.decode(&received, None).unwrap();

                assert_eq!(
                    result.message(),
                    message.as_slice(),
                    "r = {r}, m = {m}, message = {index}, errors = {positions:?}"
                );
                assert_eq!(result.word(), encoded.as_slice());
                assert_eq!(result.changed_bits(), positions.len());
                assert_eq!(result.corrected_positions(), positions.as_slice());

                if positions.is_empty() {
                    assert_eq!(result.status(), RmDecodeStatus::Valid);
                } else {
                    assert_eq!(result.status(), RmDecodeStatus::Corrected);
                }
            }
        }
    }
}

#[test]
fn ties_are_explicit_for_repetition_code() {
    let code = RmConfigurator::build(RmConfig::new(0, 2).unwrap());
    let received = [Bit::Zero, Bit::Zero, Bit::One, Bit::One];

    let result = code.decode(&received, None).unwrap();

    assert_eq!(result.status(), RmDecodeStatus::Ambiguous);
    assert_eq!(result.ambiguous_coefficients(), &[0]);
}

#[test]
fn errors_beyond_t_still_return_a_valid_codeword() {
    let code = RmConfigurator::build(RmConfig::new(1, 4).unwrap());
    let message = message_from_index(0b10011, 5);
    let encoded = code.encode(&message).unwrap();
    let received = apply_errors(&encoded, &[0, 2, 4, 6, 8]);

    let result = code.decode(&received, None).unwrap();

    assert_eq!(result.word().len(), encoded.len());
    assert_eq!(result.message().len(), message.len());
    assert_eq!(code.encode(result.message()).unwrap(), result.word());
    assert!(matches!(
        result.status(),
        RmDecodeStatus::Corrected | RmDecodeStatus::Ambiguous
    ));
}

#[test]
fn r_equals_m_recovers_every_word() {
    let code = RmConfigurator::build(RmConfig::new(3, 3).unwrap());

    for index in 0..(1usize << 8) {
        let word = message_from_index(index, 8);
        let result = code.decode(&word, None).unwrap();

        assert_eq!(result.status(), RmDecodeStatus::Valid);
        assert_eq!(result.word(), word.as_slice());
        assert_eq!(code.encode(result.message()).unwrap(), word);
    }
}
