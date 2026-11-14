//! Восстановление данных из любых k узлов (предложение 5).

mod common;

use cascade_codes::{CascadeCode, CodeField, DataDecoder, DecodeStatus, Encoder};
use common::{code, message, parameter_grid, pick, subsets};
use gf2m::Gf256;
use gfpm::{Gf13, Gf9};

fn all_subsets_recover<F: CodeField>(code: &CascadeCode<F>, seed: u64) {
    let p = *code.params();
    let msg = message::<F>(p.file_size(), seed);
    let nodes = code.encode(&msg).unwrap();
    for set in subsets(p.n(), p.k()) {
        let result = code.decode(&pick(&nodes, &set), None).unwrap();
        assert_eq!(result.message(), &msg[..], "{p:?}, узлы {set:?}");
        assert_eq!(result.status(), DecodeStatus::Consistent);
        assert_eq!(result.used_nodes(), &set[..]);
        assert!(result.suspect_nodes().is_empty());
        assert_eq!(result.subsets_tried(), 1);
    }
}

#[test]
fn running_example_every_k_subset_gf13() {
    all_subsets_recover(&code::<Gf13>(8, 4, 6, 4), 1);
}

#[test]
fn running_example_every_k_subset_gf256() {
    all_subsets_recover(&code::<Gf256>(8, 4, 6, 4), 2);
}

#[test]
fn extension_field_of_odd_characteristic() {
    // GF(9) = GF(3^2): знаки и нетривиальное расширение одновременно.
    all_subsets_recover(&code::<Gf9>(7, 3, 5, 2), 3);
    all_subsets_recover(&code::<Gf9>(9, 3, 6, 3), 4);
}

#[test]
fn whole_parameter_grid_gf13() {
    for p in parameter_grid(8, 120) {
        let code = CascadeCode::<Gf13>::try_new(p).unwrap();
        let msg = message::<Gf13>(p.file_size(), p.alpha() as u64);
        let nodes = code.encode(&msg).unwrap();
        // Первые, последние и «разбросанные» k узлов.
        let first: Vec<usize> = (0..p.k()).collect();
        let last: Vec<usize> = (p.n() - p.k()..p.n()).collect();
        let spread: Vec<usize> = (0..p.k()).map(|i| i * p.n() / p.k()).collect();
        for set in [first, last, spread] {
            let result = code.decode(&pick(&nodes, &set), None).unwrap();
            assert_eq!(result.message(), &msg[..], "{p:?}, узлы {set:?}");
        }
    }
}

#[test]
fn whole_parameter_grid_gf256() {
    for p in parameter_grid(8, 120) {
        let code = CascadeCode::<Gf256>::try_new(p).unwrap();
        let msg = message::<Gf256>(p.file_size(), 17);
        let nodes = code.encode(&msg).unwrap();
        let set: Vec<usize> = (p.n() - p.k()..p.n()).rev().collect();
        let result = code.decode(&pick(&nodes, &set), None).unwrap();
        assert_eq!(result.message(), &msg[..], "{p:?}");
    }
}

#[test]
fn order_of_shares_does_not_matter() {
    let code = code::<Gf13>(8, 4, 6, 3);
    let msg = message::<Gf13>(code.message_len(), 5);
    let nodes = code.encode(&msg).unwrap();
    for set in [[7, 0, 3, 5], [5, 3, 0, 7], [2, 6, 1, 4]] {
        let result = code.decode(&pick(&nodes, &set), None).unwrap();
        assert_eq!(result.message(), &msg[..]);
    }
}

#[test]
fn more_than_k_shares_are_verified() {
    let code = code::<Gf13>(8, 4, 6, 2);
    let msg = message::<Gf13>(code.message_len(), 6);
    let nodes = code.encode(&msg).unwrap();
    let result = code.decode(&nodes, None).unwrap();
    assert_eq!(result.message(), &msg[..]);
    assert_eq!(result.status(), DecodeStatus::Consistent);
    assert_eq!(result.used_nodes(), &[0, 1, 2, 3]);
}

#[test]
fn zero_message_encodes_to_zero_nodes() {
    let code = code::<Gf13>(6, 3, 4, 2);
    let zero = vec![Gf13::zero(); code.message_len()];
    for node in code.encode(&zero).unwrap() {
        assert!(node.symbols.iter().all(|s| *s == Gf13::zero()));
    }
}

#[test]
fn encoding_is_linear() {
    let code = code::<Gf13>(8, 4, 6, 3);
    let a = message::<Gf13>(code.message_len(), 7);
    let b = message::<Gf13>(code.message_len(), 8);
    let sum: Vec<Gf13> = a.iter().zip(&b).map(|(&x, &y)| x + y).collect();
    let (ea, eb, es) = (
        code.encode(&a).unwrap(),
        code.encode(&b).unwrap(),
        code.encode(&sum).unwrap(),
    );
    for i in 0..8 {
        for c in 0..code.node_len() {
            assert_eq!(es[i].symbols[c], ea[i].symbols[c] + eb[i].symbols[c]);
        }
    }
}

#[test]
fn single_node_encoding_matches_full_encoding() {
    let code = code::<Gf256>(8, 4, 6, 4);
    let msg = message::<Gf256>(code.message_len(), 9);
    let nodes = code.encode(&msg).unwrap();
    for (node, expected) in nodes.iter().enumerate() {
        assert_eq!(&code.encode_node(&msg, node).unwrap(), expected);
    }
}

#[test]
fn every_message_symbol_affects_the_codeword() {
    // Ни один информационный символ не теряется при построении шаблона.
    let code = code::<Gf13>(8, 4, 6, 4);
    let zero = vec![Gf13::zero(); code.message_len()];
    for i in 0..code.message_len() {
        let mut unit = zero.clone();
        unit[i] = Gf13::one();
        let nodes = code.encode(&unit).unwrap();
        assert!(
            nodes
                .iter()
                .any(|n| n.symbols.iter().any(|s| *s != Gf13::zero())),
            "символ {i}"
        );
    }
}
