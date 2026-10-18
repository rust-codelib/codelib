//! Быстрые сквозные проверки: кодирование, восстановление данных и узла.

use cascade_codes::{CascadeCode, CascadeParams, DataDecoder, DecodeStatus, Encoder, NodeRepairer};
use gf2m::Gf256;
use gfpm::Gf13;

fn message<F: cascade_codes::CodeField>(len: usize, seed: u64) -> Vec<F> {
    let q = F::order() as u64;
    (0..len as u64)
        .map(|i| F::from_index((i * 7 + seed * 13 + i * i * 3) % q).unwrap())
        .collect()
}

fn smoke<F: cascade_codes::CodeField>(n: usize, k: usize, d: usize, mu: usize) {
    let params = CascadeParams::try_new(n, k, d, mu).unwrap();
    let code = CascadeCode::<F>::try_new(params).unwrap();
    let msg = message::<F>(params.file_size(), 1);
    let nodes = code.encode(&msg).unwrap();

    // Восстановление данных из последних k узлов.
    let shares = nodes[n - k..].to_vec();
    let result = code.decode(&shares, None).unwrap();
    assert_eq!(result.message(), &msg[..]);
    assert_eq!(result.status(), DecodeStatus::Consistent);

    // Восстановление узла 0 по узлам 1..=d.
    let payloads: Vec<_> = nodes[1..=d]
        .iter()
        .map(|h| code.repair_payload(h, 0).unwrap())
        .collect();
    assert!(payloads.iter().all(|p| p.symbols.len() == params.beta()));
    let repaired = code.repair(0, &payloads, None).unwrap();
    assert_eq!(repaired, nodes[0]);
}

#[test]
fn smoke_running_example_odd_characteristic() {
    smoke::<Gf13>(8, 4, 6, 4);
}

#[test]
fn smoke_running_example_characteristic_two() {
    smoke::<Gf256>(8, 4, 6, 4);
}

#[test]
fn smoke_interior_point() {
    smoke::<Gf13>(7, 4, 6, 2);
}
