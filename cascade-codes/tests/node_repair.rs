//! Точное восстановление узлов (предложение 4) и полоса восстановления.

mod common;

use cascade_codes::{CascadeCode, CodeField, Encoder, NodeRepairer, RepairPayload};
use common::{code, message, parameter_grid, subsets};
use gf2m::Gf256;
use gfpm::{Gf13, Gf9};

fn repair_from<F: CodeField>(
    code: &CascadeCode<F>,
    nodes: &[cascade_codes::NodeShare<F>],
    failed: usize,
    helpers: &[usize],
) -> cascade_codes::NodeShare<F> {
    let payloads: Vec<RepairPayload<F>> = helpers
        .iter()
        .map(|&h| code.repair_payload(&nodes[h], failed).unwrap())
        .collect();
    for p in &payloads {
        assert_eq!(p.symbols.len(), code.params().beta());
    }
    code.repair(failed, &payloads, None).unwrap()
}

fn every_failure_every_helper_set<F: CodeField>(code: &CascadeCode<F>, seed: u64) {
    let p = *code.params();
    let msg = message::<F>(p.file_size(), seed);
    let nodes = code.encode(&msg).unwrap();
    for failed in 0..p.n() {
        let others: Vec<usize> = (0..p.n()).filter(|&i| i != failed).collect();
        for set in subsets(others.len(), p.d()) {
            let helpers: Vec<usize> = set.iter().map(|&i| others[i]).collect();
            let repaired = repair_from(code, &nodes, failed, &helpers);
            assert_eq!(
                repaired, nodes[failed],
                "{p:?}, f = {failed}, H = {helpers:?}"
            );
        }
    }
}

#[test]
fn running_example_all_failures_gf13() {
    every_failure_every_helper_set(&code::<Gf13>(8, 4, 6, 4), 1);
}

#[test]
fn running_example_all_failures_gf256() {
    every_failure_every_helper_set(&code::<Gf256>(8, 4, 6, 4), 2);
}

#[test]
fn interior_and_mbr_points() {
    for mu in 1..=4 {
        every_failure_every_helper_set(&code::<Gf13>(8, 4, 6, mu), mu as u64);
    }
}

#[test]
fn extension_field_of_odd_characteristic() {
    every_failure_every_helper_set(&code::<Gf9>(7, 3, 5, 3), 3);
    every_failure_every_helper_set(&code::<Gf9>(8, 3, 6, 2), 4);
}

#[test]
fn whole_parameter_grid() {
    for p in parameter_grid(8, 120) {
        let code = CascadeCode::<Gf13>::try_new(p).unwrap();
        let msg = message::<Gf13>(p.file_size(), 11);
        let nodes = code.encode(&msg).unwrap();
        for failed in [0, p.n() - 1] {
            let helpers: Vec<usize> = (0..p.n()).filter(|&i| i != failed).take(p.d()).collect();
            assert_eq!(
                repair_from(&code, &nodes, failed, &helpers),
                nodes[failed],
                "{p:?}"
            );
        }
    }
}

#[test]
fn helper_order_does_not_matter() {
    let code = code::<Gf13>(9, 3, 6, 3);
    let msg = message::<Gf13>(code.message_len(), 5);
    let nodes = code.encode(&msg).unwrap();
    let helpers = [8, 2, 6, 1, 7, 3];
    assert_eq!(repair_from(&code, &nodes, 4, &helpers), nodes[4]);
}

#[test]
fn payload_is_helper_independent() {
    // Пакет помощника зависит только от него и от отказавшего узла.
    let code = code::<Gf13>(8, 4, 6, 3);
    let msg = message::<Gf13>(code.message_len(), 6);
    let nodes = code.encode(&msg).unwrap();
    let payload = code.repair_payload(&nodes[5], 0).unwrap();
    for helpers in [[1, 2, 3, 4, 5, 6], [5, 7, 6, 4, 3, 2]] {
        let payloads: Vec<RepairPayload<Gf13>> = helpers
            .iter()
            .map(|&h| code.repair_payload(&nodes[h], 0).unwrap())
            .collect();
        let from_five = payloads.iter().find(|p| p.helper == 5).unwrap();
        assert_eq!(from_five, &payload);
        assert_eq!(code.repair(0, &payloads, None).unwrap(), nodes[0]);
    }
}

#[test]
fn repair_bandwidth_matches_theorem_1() {
    // Сумма рангов Ξ по сегментам равна beta из (3) — иначе repair вернул бы
    // внутреннюю ошибку; здесь дополнительно проверяем длину пакета.
    for p in parameter_grid(8, 200) {
        let code = CascadeCode::<Gf13>::try_new(p).unwrap();
        let nodes = code.encode(&message::<Gf13>(p.file_size(), 1)).unwrap();
        let payload = code.repair_payload(&nodes[1], 0).unwrap();
        assert_eq!(payload.symbols.len(), p.beta(), "{p:?}");
        assert_eq!(code.repair_payload_len(), p.beta());
        assert!(
            p.beta() * p.d() >= p.alpha(),
            "{p:?}: скачано не меньше alpha"
        );
    }
}

#[test]
fn repaired_node_supports_further_recovery() {
    // Цепочка: восстанавливаем узел 0, затем используем его как помощника.
    let code = code::<Gf13>(8, 4, 6, 4);
    let msg = message::<Gf13>(code.message_len(), 12);
    let mut nodes = code.encode(&msg).unwrap();
    let original = nodes[0].clone();
    nodes[0].symbols.iter_mut().for_each(|s| *s = Gf13::zero());
    nodes[0] = repair_from(&code, &nodes, 0, &[1, 2, 3, 4, 5, 6]);
    assert_eq!(nodes[0], original);
    assert_eq!(
        repair_from(&code, &nodes, 7, &[0, 1, 2, 3, 4, 5]),
        code.encode_node(&msg, 7).unwrap()
    );
}
