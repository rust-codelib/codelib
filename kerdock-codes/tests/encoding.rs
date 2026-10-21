mod common;

use common::{message, reference_word};
use kerdock_codes::{Kerdock, KerdockError, KerdockSize};

#[test]
fn zero_message_encodes_to_exactly_one_zero_block() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        assert_eq!(
            code.encode(&vec![false; 2 * m as usize]).unwrap(),
            vec![false; 1 << m]
        );
    }
}

#[test]
fn changing_only_constant_complements_every_coordinate() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        let word = code.encode(&message(m, 3, 5, false)).unwrap();
        let complement = code.encode(&message(m, 3, 5, true)).unwrap();
        assert_eq!(complement, word.iter().map(|bit| !bit).collect::<Vec<_>>());
    }
}

#[test]
fn encoding_rejects_empty_short_and_long_messages() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        let expected = 2 * m as usize;
        for actual in [0, expected - 1, expected + 1] {
            assert_eq!(
                code.encode(&vec![false; actual]),
                Err(KerdockError::MessageLengthMismatch { expected, actual })
            );
        }
    }
}

#[test]
fn nonzero_messages_match_independent_defining_form_without_mutating_input() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::new(KerdockSize::try_from(1usize << m).unwrap());
        for (a, b, c) in [
            (1, 3, false),
            (2, 5, true),
            (3, 1 << (m - 1), true),
            (1 << (m - 2), 1, false),
            ((1 << (m - 1)) - 1, (1 << m) - 1, true),
        ] {
            let input = message(m, a, b, c);
            let original = input.clone();
            let expected = reference_word(m, &input);
            let actual = code.encode(&input).unwrap();
            assert_eq!(actual.len(), 1 << m);
            assert_eq!(actual, expected, "m={m}, a={a}, b={b}, c={c}");
            // Includes coordinate 1023, the final bit of the final packed word.
            assert_eq!(actual[(1 << m) - 1], expected[(1 << m) - 1]);
            assert_eq!(input, original);
        }
    }
}

#[test]
fn length_16_matches_manually_derived_coordinate_fixtures() {
    // GF(8) modulo 0xB: Q_1(j) = j_0 j_3 + j_1 j_2;
    // Q_2(j) = j_0 j_1 + j_1 j_2 + j_2 j_3.
    // Coordinate j is bit j of the u16; parity(3 & j) has mask 0x6666.
    // These constants are derived from those monomials, not from the encoder.
    let code = Kerdock::new(KerdockSize::Bits16);
    for (a, b, c, expected) in [
        (1, 0, false, 0x6ac0),
        (1, 3, false, 0x0ca6),
        (1, 3, true, 0xf359),
        (2, 0, false, 0xb848),
    ] {
        let input = message(4, a, b, c);
        let pack = |word: &[bool]| {
            word.iter()
                .enumerate()
                .fold(0u16, |mask, (j, &bit)| mask | (u16::from(bit) << j))
        };
        assert_eq!(pack(&code.encode(&input).unwrap()), expected);
        assert_eq!(pack(&reference_word(4, &input)), expected);
    }
}
