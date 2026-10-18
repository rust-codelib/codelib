//! Подсчёт расхождений и указание предполагаемых мест ошибок (испорченных узлов).

mod common;

use cascade_codes::{DataDecoder, DecodeStatus, DecoderConfig, Encoder};
use common::{code, message, pick};
use gfpm::Gf13;

fn corrupt(share: &mut cascade_codes::NodeShare<Gf13>, positions: &[usize]) {
    for &p in positions {
        share.symbols[p] += Gf13::one();
    }
}

#[test]
fn corrupted_node_outside_first_k_is_reported() {
    let code = code::<Gf13>(8, 4, 6, 3);
    let msg = message::<Gf13>(code.message_len(), 1);
    let mut nodes = code.encode(&msg).unwrap();
    corrupt(&mut nodes[6], &[0, 5, 9]);

    let result = code.decode(&nodes, None).unwrap();
    assert_eq!(result.message(), &msg[..]);
    assert_eq!(result.status(), DecodeStatus::Corrected);
    assert_eq!(result.suspect_nodes(), &[6]);
    assert_eq!(result.mismatched_symbols(), 3);
    assert_eq!(result.subsets_tried(), 1);
}

#[test]
fn corrupted_node_inside_first_k_is_located_by_search() {
    let code = code::<Gf13>(8, 4, 6, 4);
    let msg = message::<Gf13>(code.message_len(), 2);
    let mut nodes = code.encode(&msg).unwrap();
    corrupt(&mut nodes[1], &[3]);

    let result = code.decode(&nodes, None).unwrap();
    assert_eq!(result.message(), &msg[..]);
    assert_eq!(result.status(), DecodeStatus::Corrected);
    assert_eq!(result.suspect_nodes(), &[1]);
    assert!(!result.used_nodes().contains(&1));
    assert!(result.subsets_tried() > 1);
}

#[test]
fn two_corrupted_nodes_with_enough_redundancy() {
    // s = 8 >= k + 2e = 4 + 4.
    let code = code::<Gf13>(8, 4, 6, 2);
    let msg = message::<Gf13>(code.message_len(), 3);
    let mut nodes = code.encode(&msg).unwrap();
    corrupt(&mut nodes[0], &[0]);
    corrupt(&mut nodes[5], &[1, 2]);

    let result = code.decode(&nodes, None).unwrap();
    assert_eq!(result.message(), &msg[..]);
    assert_eq!(result.status(), DecodeStatus::Corrected);
    assert_eq!(result.suspect_nodes(), &[0, 5]);
    assert_eq!(result.mismatched_symbols(), 3);
}

#[test]
fn ambiguous_when_redundancy_is_too_small() {
    // s = 5 < k + 2e = 6: однозначность не гарантируется.
    let code = code::<Gf13>(8, 4, 6, 4);
    let msg = message::<Gf13>(code.message_len(), 4);
    let mut nodes = code.encode(&msg).unwrap();
    corrupt(&mut nodes[2], &[7]);
    let shares = pick(&nodes, &[0, 1, 2, 3, 4]);

    let result = code.decode(&shares, None).unwrap();
    assert_eq!(result.status(), DecodeStatus::Inconsistent);
    // В точке MSR любые k узлов согласованы сами с собой, поэтому неверный
    // кандидат по {0, 1, 2, 3} расходится только с узлом 4 — так же, как
    // верный расходится только с узлом 2. Выбрать между ними нельзя.
    assert_eq!(result.suspect_nodes().len(), 1);
    assert_eq!(result.subsets_tried(), 5);
}

#[test]
fn search_limit_is_respected() {
    let code = code::<Gf13>(8, 4, 6, 4).with_decoder_config(DecoderConfig {
        max_subset_trials: 1,
    });
    let msg = message::<Gf13>(code.message_len(), 5);
    let mut nodes = code.encode(&msg).unwrap();
    corrupt(&mut nodes[0], &[0]);

    let result = code.decode(&nodes, None).unwrap();
    assert_eq!(result.subsets_tried(), 1);
    // Первая попытка использовала испорченный узел: результат неверен,
    // и это видно по статусу и числу расхождений.
    assert_eq!(result.status(), DecodeStatus::Inconsistent);
    assert!(result.suspect_nodes().len() >= 2);
}

#[test]
fn redundant_parity_detects_corruption_with_exactly_k_nodes() {
    // При mu < k данных меньше, чем k·alpha: часть повреждений видна даже по k узлам.
    let code = code::<Gf13>(8, 4, 6, 2);
    let p = *code.params();
    assert!(p.file_size() < p.k() * p.alpha());
    let msg = message::<Gf13>(p.file_size(), 6);
    let nodes = code.encode(&msg).unwrap();
    let mut detected = 0;
    for position in 0..p.alpha() {
        let mut shares = pick(&nodes, &[0, 1, 2, 3]);
        corrupt(&mut shares[2], &[position]);
        let result = code.decode(&shares, None).unwrap();
        if result.status() != DecodeStatus::Consistent {
            detected += 1;
        }
    }
    assert!(detected > 0);
}

#[test]
fn msr_point_has_no_redundancy_with_exactly_k_nodes() {
    // При mu = k имеем F = k·alpha: повреждение по k узлам не обнаружить.
    let code = code::<Gf13>(8, 4, 6, 4);
    let p = *code.params();
    assert_eq!(p.file_size(), p.k() * p.alpha());
    let msg = message::<Gf13>(p.file_size(), 7);
    let mut shares = pick(&code.encode(&msg).unwrap(), &[0, 1, 2, 3]);
    corrupt(&mut shares[0], &[0]);
    let result = code.decode(&shares, None).unwrap();
    assert_eq!(result.status(), DecodeStatus::Consistent);
    assert_ne!(result.message(), &msg[..]);
}

#[test]
fn many_shares_do_not_enumerate_all_subsets() {
    // 60 узлов при k = 30: C(60, 30) ≈ 1.2e17 подмножеств — перебор обязан быть ленивым.
    let code = code::<gf2m::Gf256>(60, 30, 31, 1);
    let msg = message::<gf2m::Gf256>(code.message_len(), 8);
    let mut nodes = code.encode(&msg).unwrap();
    nodes[0].symbols[0] += gf2m::Gf256::new(1);
    let result = code
        .decode_with(
            &nodes,
            DecoderConfig {
                max_subset_trials: 3,
            },
            None,
        )
        .unwrap();
    assert_eq!(result.subsets_tried(), 3);
    assert_eq!(result.status(), DecodeStatus::Inconsistent);
}
