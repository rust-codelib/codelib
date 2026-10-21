mod common;

use common::{message, reference_word};
use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

fn pack(word: &[bool]) -> u64 {
    assert!(word.len() <= 64);
    word.iter()
        .enumerate()
        .fold(0, |packed, (i, &bit)| packed | (u64::from(bit) << i))
}

#[test]
fn every_length16_received_word_matches_independent_nearest_search() {
    let code = Kerdock::new(KerdockSize::Bits16);
    let mut candidates = Vec::new();
    for a in 0..8 {
        for b in 0..16 {
            for c in [false, true] {
                let input = message(4, a, b, c);
                let word = pack(&reference_word(4, &input)) as u16;
                candidates.push((input, word));
            }
        }
    }
    assert_eq!(candidates.len(), 256);
    let mut accepted = 0;
    let mut rejected_ties = 0;
    for received in 0..=u16::MAX {
        let mut nearest_distance = 17;
        let mut nearest_index = 0;
        let mut ties = 0;
        for (index, (_, word)) in candidates.iter().enumerate() {
            let distance = (received ^ word).count_ones() as usize;
            if distance < nearest_distance {
                nearest_distance = distance;
                nearest_index = index;
                ties = 1;
            } else if distance == nearest_distance {
                ties += 1;
            }
        }
        let bits: [bool; 16] = std::array::from_fn(|i| received & (1 << i) != 0);
        let result = code.decode(&bits);
        if nearest_distance <= 2 {
            assert_eq!(ties, 1, "received={received:#06x}");
            let result = result.unwrap();
            assert_eq!(result.message(), candidates[nearest_index].0);
            assert_eq!(pack(result.word()), u64::from(candidates[nearest_index].1));
            assert_eq!(result.distance(), nearest_distance);
            accepted += 1;
        } else {
            assert_eq!(
                result,
                Err(KerdockError::NoCodewordWithinRadius {
                    nearest_distance,
                    radius: 2
                }),
                "received={received:#06x}"
            );
            rejected_ties += usize::from(ties > 1);
        }
    }
    assert_eq!(accepted, 256 * (1 + 16 + 120));
    assert_eq!(accepted, 35072);
    assert!(rejected_ties > 0);
}

#[test]
fn length64_selected_inputs_match_independent_exhaustive_oracle() {
    let code = Kerdock::new(KerdockSize::Bits64);
    let mut candidates = Vec::new();
    for a in 0..32 {
        for b in 0..64 {
            for c in [false, true] {
                let input = message(6, a, b, c);
                let word = pack(&reference_word(6, &input));
                candidates.push((input, word));
            }
        }
    }
    assert_eq!(candidates.len(), 4096);
    let boundary = (1u64 << 13) - 1;
    let mut received_words = vec![boundary];
    for index in [0, 1, 127, 128, 2049, 4095] {
        for weight in [0, 1, 13, 14, 20, 32] {
            let errors = (0..weight).fold(0, |mask, i| mask | (1u64 << ((63 + 17 * i) % 64)));
            received_words.push(candidates[index].1 ^ errors);
        }
    }
    // Fixed-seed xorshift: reproducible, unstructured received words, not an
    // assumption that a particular channel-error weight implies rejection.
    let mut noisy = 0x91e1_0da5_c79e_7b1du64;
    for _ in 0..16 {
        noisy ^= noisy << 13;
        noisy ^= noisy >> 7;
        noisy ^= noisy << 17;
        received_words.push(noisy);
    }
    let mut accepted = 0;
    let mut rejected = 0;
    for received in received_words {
        let distances: Vec<_> = candidates
            .iter()
            .map(|(_, word)| (received ^ word).count_ones() as usize)
            .collect();
        let (index, &nearest_distance) = distances
            .iter()
            .enumerate()
            .min_by_key(|(_, d)| *d)
            .unwrap();
        let bits: [bool; 64] = std::array::from_fn(|i| received & (1 << i) != 0);
        let result = code.decode(&bits);
        if received == boundary {
            assert_eq!(nearest_distance, 13);
            assert_eq!(candidates[index].1, 0);
        }
        if nearest_distance <= 13 {
            assert_eq!(
                distances.iter().filter(|&&d| d == nearest_distance).count(),
                1
            );
            let result = result.unwrap();
            assert_eq!(result.message(), candidates[index].0);
            assert_eq!(pack(result.word()), candidates[index].1);
            assert_eq!(result.distance(), nearest_distance);
            accepted += 1;
        } else {
            assert_eq!(
                result,
                Err(KerdockError::NoCodewordWithinRadius {
                    nearest_distance,
                    radius: 13
                })
            );
            rejected += 1;
        }
    }
    assert!(accepted > 0);
    assert!(rejected > 0);
}
