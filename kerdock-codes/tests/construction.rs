mod common;

use common::{message, reference_word};
use gf2m::GfRuntime;
use kerdock_codes::Kerdock;
use std::collections::{BTreeMap, HashSet};

#[test]
fn configured_fields_have_pinned_moduli_and_binary_traces() {
    for (degree, modulus) in [(3, 0xB), (5, 0x25), (7, 0x83), (9, 0x211)] {
        assert_eq!(GfRuntime::standard_poly(degree).unwrap(), modulus);
        for value in 0..1 << degree {
            assert!(GfRuntime::new(degree, value).unwrap().trace().value() <= 1);
        }
    }
}

// Row elimination over GF(2), independent of production field/form arithmetic.
fn binary_rank(mut rows: Vec<u16>, columns: u32) -> usize {
    let mut rank = 0;
    for column in 0..columns {
        if let Some(pivot) = (rank..rows.len()).find(|&r| rows[r] & (1 << column) != 0) {
            rows.swap(rank, pivot);
            for r in rank + 1..rows.len() {
                if rows[r] & (1 << column) != 0 {
                    rows[r] ^= rows[rank];
                }
            }
            rank += 1;
        }
    }
    rank
}

#[test]
fn every_encoder_polar_difference_has_full_rank_at_every_size() {
    for m in [4, 6, 8, 10] {
        let code = Kerdock::try_new(m).unwrap();
        let matrices: Vec<Vec<u16>> = (0..1 << (m - 1))
            .map(|a| {
                let word = code.encode(&message(m, a, 0, false)).unwrap();
                assert!(!word[0]);
                (0..m)
                    .map(|i| {
                        (0..m).fold(0, |row, j| {
                            let entry =
                                word[(1 << i) ^ (1 << j)] ^ word[1 << i] ^ word[1 << j] ^ word[0];
                            if i == j {
                                assert!(!entry);
                            }
                            row | (u16::from(entry) << j)
                        })
                    })
                    .collect()
            })
            .collect();
        for a in 0..matrices.len() {
            for b in a + 1..matrices.len() {
                let difference = matrices[a]
                    .iter()
                    .zip(&matrices[b])
                    .map(|(left, right)| left ^ right)
                    .collect();
                assert_eq!(
                    binary_rank(difference, m),
                    m as usize,
                    "m={m}, a={a}, b={b}"
                );
            }
        }
    }
}

fn pack16(word: &[bool]) -> u16 {
    assert_eq!(word.len(), 16);
    word.iter()
        .enumerate()
        .fold(0, |packed, (i, &bit)| packed | (u16::from(bit) << i))
}

fn assert_nordstrom_robinson_parameters(words: &[u16]) {
    assert_eq!(words.len(), 256);
    assert_eq!(words.iter().copied().collect::<HashSet<_>>().len(), 256);
    let mut weights = BTreeMap::new();
    let mut minimum_distance = 16;
    for (i, &word) in words.iter().enumerate() {
        *weights.entry(word.count_ones()).or_insert(0) += 1;
        for &other in &words[i + 1..] {
            minimum_distance = minimum_distance.min((word ^ other).count_ones());
        }
    }
    assert_eq!(
        weights,
        BTreeMap::from([(0, 1), (6, 112), (8, 30), (10, 112), (16, 1)])
    );
    assert_eq!(minimum_distance, 6);
}

#[test]
fn length16_codebook_has_exact_parameters_affine_subcode_and_is_nonlinear() {
    let code = Kerdock::try_new(4).unwrap();
    let mut words = Vec::new();
    for a in 0..8 {
        for b in 0..16 {
            for c in [false, true] {
                let input = message(4, a, b, c);
                let word = code.encode(&input).unwrap();
                assert_eq!(word, reference_word(4, &input));
                words.push(pack16(&word));
            }
        }
    }
    assert_nordstrom_robinson_parameters(&words);
    let affine: HashSet<_> = (0usize..16)
        .flat_map(|b| {
            [false, true].map(move |c| {
                (0..16).fold(0u16, |word, j| {
                    word | (u16::from(((b & j).count_ones() % 2 != 0) ^ c) << j)
                })
            })
        })
        .collect();
    assert_eq!(affine.len(), 32);
    assert_eq!(words[..32].iter().copied().collect::<HashSet<_>>(), affine);
    let codebook: HashSet<_> = words.iter().copied().collect();
    assert!(words.iter().any(|left| words
        .iter()
        .any(|right| !codebook.contains(&(left ^ right)))));
}

#[test]
fn published_octacode_gray_image_has_same_parameters() {
    // Archived Magma handbook, octacode generator and Gray map:
    // https://www.math.uzh.ch/sepp/magma-2.20.4-cr/html/text1810.htm
    // This is a separate Z/4Z construction, not an ordering/label fixture.
    let rows = [*b"10003121", *b"01001231", *b"00103332", *b"00012311"];
    let gray = [0b00u16, 0b01, 0b11, 0b10];
    let words: Vec<_> = (0..256)
        .map(|coefficients| {
            (0..8).fold(0u16, |word, coordinate| {
                let symbol = rows.iter().enumerate().fold(0, |sum, (i, row)| {
                    sum + ((coefficients >> (2 * i)) & 3) * usize::from(row[coordinate] - b'0')
                }) % 4;
                word | (gray[symbol] << (2 * coordinate))
            })
        })
        .collect();
    assert_nordstrom_robinson_parameters(&words);
}
