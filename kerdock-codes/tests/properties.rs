mod common;

use common::{message, reference_word};
use kerdock_codes::{Kerdock, KerdockSize};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use std::collections::BTreeSet;

fn check_recovery(code: &Kerdock, input: &[bool], positions: &BTreeSet<usize>) {
    let expected_word = reference_word(code.exponent(), input);
    assert_eq!(code.encode(input).unwrap(), expected_word);
    let mut received = expected_word.clone();
    for &position in positions {
        received[position] ^= true;
    }
    let result = code.decode(&received).unwrap();
    assert_eq!(result.message(), input);
    assert_eq!(result.word(), expected_word);
    assert_eq!(result.distance(), positions.len());
}

fn check_size(m: u32) {
    // One prepared codec per exponent, reused by deterministic and random cases.
    let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
    let n = code.codeword_len();
    let t = code.correction_radius();
    let input = message(m, (1 << (m - 1)) - 1, (1 << m) - 1, true);
    for weight in [0, 1, t] {
        let positions = (0..weight).map(|i| (n - 1 + 37 * i) % n).collect();
        check_recovery(&code, &input, &positions);
    }
    let strategy = (
        proptest::collection::vec(any::<bool>(), 2 * m as usize),
        proptest::collection::btree_set(0..n, 0..=t),
    );
    let mut runner = TestRunner::new(Config {
        cases: 12,
        // This manual runner has no source-file macro context; failures include
        // the minimized message and distinct positions in the returned error.
        failure_persistence: None,
        ..Config::default()
    });
    runner
        .run(&strategy, |(input, positions)| {
            check_recovery(&code, &input, &positions);
            Ok(())
        })
        .unwrap();
}

#[test]
fn length64_random_messages_and_distinct_errors_within_radius() {
    check_size(6);
}

#[test]
fn length256_random_messages_and_distinct_errors_within_radius() {
    check_size(8);
}

#[test]
fn length1024_random_messages_and_distinct_errors_within_radius() {
    check_size(10);
}
