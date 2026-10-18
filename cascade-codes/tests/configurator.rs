//! Конфигуратор и структура супер-матрицы через публичный API.

use cascade_codes::{CascadeConfigurator, CascadeParams, DecoderConfig, Encoder, NodeRepairer};
use gf2m::Gf256;
use gfpm::Gf13;

#[test]
fn default_mode_is_msr() {
    let code = CascadeConfigurator::new(8, 4, 6).build::<Gf13>().unwrap();
    assert!(code.params().is_msr());
    assert_eq!(code.params().mu(), 4);
    assert_eq!(code.node_len(), 81);
    assert_eq!(code.repair_payload_len(), 27);
}

#[test]
fn mbr_is_a_single_mode_one_segment() {
    let code = CascadeConfigurator::new(9, 3, 7)
        .mbr()
        .build::<Gf256>()
        .unwrap();
    assert_eq!(code.segment_modes(), vec![1]);
    assert_eq!(code.params().alpha(), 7);
    assert_eq!(code.params().beta(), 1);
}

#[test]
fn running_example_segment_modes() {
    let code = CascadeConfigurator::new(8, 4, 6).build::<Gf13>().unwrap();
    let modes = code.segment_modes();
    assert_eq!(modes[0], 4);
    let count = |m| modes.iter().filter(|&&x| x == m).count();
    assert_eq!((count(4), count(2), count(1), count(0)), (1, 3, 2, 9));
}

#[test]
fn decoder_config_is_forwarded() {
    let config = DecoderConfig {
        max_subset_trials: 3,
    };
    let code = CascadeConfigurator::new(5, 2, 3)
        .decoder_config(config)
        .build::<Gf13>()
        .unwrap();
    assert_eq!(code.decoder_config(), config);
}

#[test]
fn configurator_reports_parameter_errors() {
    assert!(CascadeConfigurator::new(4, 3, 2).build::<Gf13>().is_err());
    assert!(CascadeConfigurator::new(5, 2, 3).mode(3).params().is_err());
    assert_eq!(
        CascadeConfigurator::new(7, 4, 6).mode(2).params().unwrap(),
        CascadeParams::try_new(7, 4, 6, 2).unwrap()
    );
}

#[test]
fn message_matrix_and_encoder_shapes() {
    let code = CascadeConfigurator::new(8, 4, 6)
        .mode(2)
        .build::<Gf13>()
        .unwrap();
    let psi = code.encoder_matrix().unwrap();
    assert_eq!((psi.rows(), psi.cols()), (8, 6));
    let msg = vec![Gf13::one(); code.message_len()];
    let m = code.message_matrix(&msg).unwrap();
    assert_eq!((m.rows(), m.cols()), (6, code.node_len()));
    // Узлы — строки Ψ · M.
    let product = psi.try_mul(&m).unwrap();
    for (i, node) in code.encode(&msg).unwrap().iter().enumerate() {
        assert_eq!(product.row(i).unwrap(), &node.symbols[..]);
    }
}
