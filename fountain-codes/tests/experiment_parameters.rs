// Compile the same experiment-only helper into the normal Cargo test suite.
// The production library and its public API are unaffected.
#[path = "../examples/support/sample_size.rs"]
mod sample_size;

use sample_size::requested_sample_size;

#[test]
fn regression_and_paper_grid_sample_counts() {
    for (k, expected) in [(100, 110), (12, 14), (300, 330), (500, 550)] {
        assert_eq!(requested_sample_size(k, "0.1"), Ok(expected));
    }
}

#[test]
fn endpoints_and_equivalent_decimal_notations() {
    for k in [0, 1, 12, 100, 300, 500, 4096] {
        for zero in [
            "0",
            "0.0",
            "-0",
            "+0.00e99999999999999999999999999999999999999999",
        ] {
            assert_eq!(requested_sample_size(k, zero), Ok(k));
        }
        for one in ["1", "1.0", "0.1e1", "1000e-3"] {
            assert_eq!(requested_sample_size(k, one), Ok(2 * k));
        }
    }
    for decimal in ["0.1", "+0.10", ".1", "1e-1", "1E-1", "10.e-2", "000.1000"] {
        assert_eq!(requested_sample_size(100, decimal), Ok(110));
    }
}

#[test]
fn exact_rounding_on_both_sides_of_integer_boundaries() {
    for (epsilon, expected) in [
        ("0.09999999999999999", 110),
        ("0.10000000000000000", 110),
        ("0.10000000000000001", 111),
        (
            "0.09999999999999999999999999999999999999999999999999999999",
            110,
        ),
        (
            "0.10000000000000000000000000000000000000000000000000000001",
            111,
        ),
        ("0.009999999999999999", 101),
        ("0.01", 101),
        ("0.010000000000000001", 102),
        ("0.99999999999999999", 200),
        ("1e-400", 101),
        (
            "1e-9999999999999999999999999999999999999999999999999999999",
            101,
        ),
    ] {
        assert_eq!(
            requested_sample_size(100, epsilon),
            Ok(expected),
            "{epsilon}"
        );
    }
    assert_eq!(requested_sample_size(0, "1e-400"), Ok(0));
}

#[test]
fn invalid_decimals_ranges_and_size_overflow() {
    for epsilon in [
        "",
        ".",
        "+",
        "--0.1",
        "0.1.0",
        "NaN",
        "inf",
        "-inf",
        "1e",
        "1e+",
        "1e--1",
        "1e-1e-1",
        " 0.1",
        "0.1 ",
        "0,1",
        "0_1",
        "０.１",
        "-0.1",
        "-1e-400",
        "1.00000000000000001",
        "1.1",
        "10",
        "1e99999999999999999999999999999999999999999",
    ] {
        assert!(requested_sample_size(100, epsilon).is_err(), "{epsilon:?}");
    }
    assert_eq!(requested_sample_size(usize::MAX, "0"), Ok(usize::MAX));
    assert!(requested_sample_size(usize::MAX, "1").is_err());
    assert!(requested_sample_size(usize::MAX, "0.1").is_err());
}

#[test]
fn bounded_decimal_sweep_matches_independent_integer_fraction_formula() {
    // Expected values use small exact fractions, not the tested parser or f64.
    for k in [1usize, 12, 100, 300, 500, 4096] {
        for numerator in 0..1000 {
            let decimal = format!("0.{numerator:03}");
            let product = k * numerator;
            let expected = k + product / 1000 + usize::from(product % 1000 != 0);
            assert_eq!(
                requested_sample_size(k, &decimal),
                Ok(expected),
                "K={k}, {decimal}"
            );
        }
    }
}
