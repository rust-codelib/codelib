//! Конфигурация и известные порождающие многочлены.

#[path = "support/bch15_generator.rs"]
mod bch15;
#[path = "support/bch7_generator.rs"]
mod bch7;

use bch15::bch15_generator;
use bch7::bch7_generator;
use bch_codes::BchConfig;

#[test]
fn config_rejects_zero_error_capability() {
    assert!(BchConfig::new(0, 0).is_err());
}

#[test]
fn known_generator_bch_7_4_3() {
    let config = BchConfig::new(1, 0).unwrap();
    // g(x) = x^3 + x + 1; coefficient of x^j is bit j.
    assert_eq!(bch7_generator(&config), [0b0000_1011]);
}

#[test]
fn known_generator_bch_15_7_5() {
    let config = BchConfig::new(2, 0).unwrap();
    // g(x) = x^8 + x^7 + x^6 + x^4 + 1.
    assert_eq!(bch15_generator(&config), [0b1101_0001, 0b0000_0001]);
}
