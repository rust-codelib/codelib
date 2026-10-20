/// Unnormalized Walsh transform; the length must be a nonzero power of two.
/// For ±1 inputs of length at most 1024, all intermediate values fit in i32.
pub(crate) fn transform(values: &mut [i32]) {
    debug_assert!(values.len().is_power_of_two());
    let mut half = 1;
    while half < values.len() {
        for block in values.chunks_exact_mut(2 * half) {
            let (left, right) = block.split_at_mut(half);
            for (x, y) in left.iter_mut().zip(right) {
                let (sum, difference) = (*x + *y, *x - *y);
                *x = sum;
                *y = difference;
            }
        }
        half *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::transform;

    // Catch wrong butterfly signs, ordering, or skipped stages using direct sums.
    #[test]
    fn transform_matches_direct_walsh_sums() {
        for n in [1, 2, 4, 8, 16] {
            let input: Vec<i32> = (0..n).map(|j| (j * 7 % 11) as i32 - 5).collect();
            let expected: Vec<i32> = (0..n)
                .map(|b: usize| {
                    input
                        .iter()
                        .enumerate()
                        .map(|(j, &value)| {
                            if (b & j).count_ones() % 2 == 0 {
                                value
                            } else {
                                -value
                            }
                        })
                        .sum()
                })
                .collect();
            let mut actual = input;
            transform(&mut actual);
            assert_eq!(actual, expected, "n={n}");
        }
    }

    #[test]
    fn constant_and_linear_signals_have_expected_spectral_peaks() {
        for (mut input, expected) in [([1, 1, 1, 1], [4, 0, 0, 0]), ([1, -1, 1, -1], [0, 4, 0, 0])]
        {
            transform(&mut input);
            assert_eq!(input, expected);
        }
    }

    // Catch accidental normalization or a noninvertible butterfly.
    #[test]
    fn transforming_twice_scales_by_length() {
        for n in [1, 2, 4, 8, 16] {
            let original: Vec<i32> = (0..n).map(|j| (j * 3 % 7) - 3).collect();
            let mut values = original.clone();
            transform(&mut values);
            transform(&mut values);
            assert_eq!(
                values,
                original.iter().map(|value| n * value).collect::<Vec<_>>()
            );
        }
    }

    // Catch narrow accumulators at the largest supported block length.
    #[test]
    fn maximum_length_constant_reaches_full_coefficient() {
        let mut values = vec![-1; 1024];
        transform(&mut values);
        assert_eq!(values[0], -1024);
        assert!(values[1..].iter().all(|&value| value == 0));
    }
}
