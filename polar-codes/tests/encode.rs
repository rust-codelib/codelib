use polar_codes::{bit_reverse, polar_transform, Bit, PolarCode, PolarError};

fn bits(values: &[u8]) -> Vec<Bit> {
    values
        .iter()
        .copied()
        .map(|value| Bit::try_from(value).expect("test vector contains only bits"))
        .collect()
}

fn reference_transform(bits: &[Bit]) -> Vec<Bit> {
    if bits.len() <= 1 {
        return bits.to_vec();
    }
    let half = bits.len() / 2;
    let mut left = Vec::with_capacity(half);
    let mut right = Vec::with_capacity(half);
    for index in 0..half {
        left.push(bits[index].xor(bits[index + half]));
        right.push(bits[index + half]);
    }
    let mut encoded = reference_transform(&left);
    encoded.extend(reference_transform(&right));
    encoded
}

trait XorBit {
    fn xor(self, other: Self) -> Self;
}

impl XorBit for Bit {
    fn xor(self, other: Self) -> Self {
        match (self, other) {
            (Bit::Zero, Bit::One) | (Bit::One, Bit::Zero) => Bit::One,
            _ => Bit::Zero,
        }
    }
}

#[test]
fn transform_matches_the_recursive_kernel_for_every_block_up_to_eight() {
    for length in [1_usize, 2, 4, 8] {
        let patterns = 1_usize << length;
        for pattern in 0..patterns {
            let mut word = (0..length)
                .map(|shift| {
                    if (pattern >> shift) & 1 == 1 {
                        Bit::One
                    } else {
                        Bit::Zero
                    }
                })
                .collect::<Vec<_>>();
            let expected = reference_transform(&word);
            polar_transform(&mut word).expect("length is a power of two");
            assert_eq!(word, expected);
            polar_transform(&mut word).expect("inverse uses the same map");
            let original = (0..length)
                .map(|shift| {
                    if (pattern >> shift) & 1 == 1 {
                        Bit::One
                    } else {
                        Bit::Zero
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(word, original);
        }
    }
}

#[test]
fn length_four_matrix_matches_the_lecture_kernel() {
    // u = (1, 0, 1, 0), x = u F^{\otimes 2} = (0, 0, 1, 0).
    let mut word = bits(&[1, 0, 1, 0]);
    polar_transform(&mut word).unwrap();
    assert_eq!(word, bits(&[0, 0, 1, 0]));
}

#[test]
fn bit_reverse_is_an_involution_on_the_block() {
    let image: Vec<usize> = (0..8).map(|index| bit_reverse(index, 8).unwrap()).collect();
    assert_eq!(image, [0, 4, 2, 6, 1, 5, 3, 7]);
    for (index, &reversed) in image.iter().enumerate() {
        assert_eq!(bit_reverse(reversed, 8).unwrap(), index);
    }
    assert_eq!(bit_reverse(0, 1).unwrap(), 0);
}

#[test]
fn encode_places_message_bits_and_freezes_the_rest() {
    let code = PolarCode::try_new(4, &[3, 1]).unwrap();
    assert_eq!(code.information_positions(), &[1, 3]);
    assert_eq!(code.frozen_positions(), vec![0, 2]);
    assert_eq!(code.message_len(), 2);
    assert_eq!(code.length(), 4);

    // message (1, 0) -> u = (0, 1, 0, 0) -> x = (1, 1, 0, 0).
    let word = code.encode(&bits(&[1, 0])).unwrap();
    assert_eq!(word, bits(&[1, 1, 0, 0]));
    assert_eq!(code.extract_message(&word).unwrap(), bits(&[1, 0]));
}

#[test]
fn all_frozen_code_encodes_only_the_zero_word() {
    let code = PolarCode::try_new(4, &[]).unwrap();
    assert_eq!(code.encode(&[]).unwrap(), bits(&[0, 0, 0, 0]));
    assert_eq!(
        code.extract_message(&bits(&[1, 0, 0, 0])),
        Err(PolarError::InvalidCodeword { frozen_ones: 1 })
    );
}

#[test]
fn extract_rejects_a_word_outside_the_code() {
    let code = PolarCode::try_new(4, &[2, 3]).unwrap();
    let word = code.encode(&bits(&[1, 0])).unwrap();
    let mut damaged = word.clone();
    damaged[0] = match damaged[0] {
        Bit::Zero => Bit::One,
        Bit::One => Bit::Zero,
    };
    assert!(matches!(
        code.extract_message(&damaged),
        Err(PolarError::InvalidCodeword { .. })
    ));
}

#[test]
fn constructor_rejects_bad_lengths_and_indexes() {
    assert!(matches!(
        PolarCode::try_new(3, &[0]),
        Err(PolarError::InvalidLength { length: 3 })
    ));
    assert!(matches!(
        PolarCode::try_new(8192, &[0]),
        Err(PolarError::InvalidLength { length: 8192 })
    ));
    assert!(matches!(
        polar_transform(&mut [Bit::Zero, Bit::Zero, Bit::Zero]),
        Err(PolarError::InvalidLength { length: 3 })
    ));
    assert_eq!(
        PolarCode::try_new(4, &[4]),
        Err(PolarError::IndexOutOfBounds {
            index: 4,
            length: 4
        })
    );
    assert_eq!(
        PolarCode::try_new(4, &[1, 1]),
        Err(PolarError::DuplicateInformationIndex { index: 1 })
    );
    assert_eq!(
        bit_reverse(8, 8),
        Err(PolarError::IndexOutOfBounds {
            index: 8,
            length: 8
        })
    );
    assert_eq!(Bit::try_from(2), Err(PolarError::InvalidBit { value: 2 }));
}

#[test]
fn length_mismatches_name_the_expected_size() {
    let code = PolarCode::try_new(2, &[1]).unwrap();
    assert_eq!(
        code.encode(&bits(&[0, 1])),
        Err(PolarError::MessageLengthMismatch {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        code.extract_message(&bits(&[0])),
        Err(PolarError::WordLengthMismatch {
            expected: 2,
            actual: 1
        })
    );
}
