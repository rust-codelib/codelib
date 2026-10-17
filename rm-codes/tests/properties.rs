//! Свойства кодирования и декодирования.

use proptest::prelude::*;

use rm_codes::{Bit, RmConfig, RmConfigurator, RmDecodeStatus};

fn bits_from_seed(seed: u64, len: usize) -> Vec<Bit> {
    (0..len)
        .map(|index| Bit::from_bool((seed >> (index % 64)) & 1 == 1))
        .collect()
}

fn xor_bits(left: &[Bit], right: &[Bit]) -> Vec<Bit> {
    left.iter().zip(right).map(|(&a, &b)| a ^ b).collect()
}

proptest! {
    #[test]
    fn random_round_trip(
        m in 1usize..=6,
        r_seed in 0usize..=6,
        seed in any::<u64>(),
    ) {
        let r = r_seed.min(m);
        let config = RmConfig::new(r, m).unwrap();
        let code = RmConfigurator::build(config);
        let message = bits_from_seed(seed, config.dimension());

        let word = code.encode(&message).unwrap();
        prop_assert_eq!(word.len(), config.length());

        let result = code.decode(&word, None).unwrap();
        prop_assert_eq!(result.status(), RmDecodeStatus::Valid);
        prop_assert_eq!(result.message(), message.as_slice());
        prop_assert_eq!(result.word(), word.as_slice());
        prop_assert_eq!(code.extract_message(&word).unwrap(), message);
    }

    #[test]
    fn random_word_decode_is_consistent(
        m in 1usize..=6,
        r_seed in 0usize..=6,
        seed in any::<u64>(),
    ) {
        let r = r_seed.min(m);
        let config = RmConfig::new(r, m).unwrap();
        let code = RmConfigurator::build(config);
        let word = bits_from_seed(seed, config.length());

        let result = code.decode(&word, None).unwrap();

        prop_assert_eq!(result.word().len(), config.length());
        prop_assert_eq!(result.message().len(), config.dimension());
        prop_assert_eq!(code.encode(result.message()).unwrap(), result.word());

        let changed: Vec<_> = word
            .iter()
            .zip(result.word())
            .enumerate()
            .filter(|(_, (before, after))| before != after)
            .map(|(index, _)| index)
            .collect();
        prop_assert_eq!(result.changed_bits(), changed.len());
        prop_assert_eq!(result.corrected_positions(), changed.as_slice());

        if result.status() == RmDecodeStatus::Valid {
            prop_assert_eq!(result.word(), word.as_slice());
        }
        if result.status() == RmDecodeStatus::Corrected {
            prop_assert_ne!(result.word(), word.as_slice());
        }
    }

    #[test]
    fn encoding_is_linear(
        m in 1usize..=6,
        r_seed in 0usize..=6,
        left_seed in any::<u64>(),
        right_seed in any::<u64>(),
    ) {
        let r = r_seed.min(m);
        let config = RmConfig::new(r, m).unwrap();
        let code = RmConfigurator::build(config);
        let left = bits_from_seed(left_seed, config.dimension());
        let right = bits_from_seed(right_seed, config.dimension());

        let encoded_sum = code.encode(&xor_bits(&left, &right)).unwrap();
        let sum_of_encoded = xor_bits(
            &code.encode(&left).unwrap(),
            &code.encode(&right).unwrap(),
        );

        prop_assert_eq!(encoded_sum, sum_of_encoded);
    }
}

#[test]
fn minimum_distance_matches_formula_for_small_codes() {
    for m in 1..=3 {
        for r in 0..=m {
            assert_minimum_distance(r, m);
        }
    }

    for r in 0..=2 {
        assert_minimum_distance(r, 4);
    }
}

fn assert_minimum_distance(r: usize, m: usize) {
    let config = RmConfig::new(r, m).unwrap();
    let code = RmConfigurator::build(config);
    let k = config.dimension();
    let mut minimum = config.length();

    for index in 1..(1usize << k) {
        let message: Vec<_> = (0..k)
            .map(|bit| Bit::from_bool((index >> bit) & 1 == 1))
            .collect();
        let weight = code
            .encode(&message)
            .unwrap()
            .iter()
            .filter(|bit| bit.is_one())
            .count();
        minimum = minimum.min(weight);
    }

    assert_eq!(minimum, config.distance(), "r = {r}, m = {m}");
}
