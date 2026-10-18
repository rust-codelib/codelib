//! Восстановление данных и узлов на более крупных кодах (alpha до ~700).

mod common;

use cascade_codes::{CascadeCode, CascadeParams, DataDecoder, Encoder, NodeRepairer};
use common::{message, pick};
use gfpm::Gf251;

const CASES: [(usize, usize, usize, usize); 6] = [
    (10, 5, 8, 3),
    (12, 6, 9, 4),
    (10, 6, 8, 6),
    (12, 3, 10, 3),
    (12, 7, 9, 5),
    (14, 2, 12, 2),
];

#[test]
fn larger_codes_recover_data_and_nodes() {
    for (case, &(n, k, d, mu)) in CASES.iter().enumerate() {
        let params = CascadeParams::try_new(n, k, d, mu).unwrap();
        let code = CascadeCode::<Gf251>::try_new(params).unwrap();
        let msg = message::<Gf251>(params.file_size(), case as u64 + 1);
        let nodes = code.encode(&msg).unwrap();

        // Последние k узлов в обратном порядке и равномерно разбросанные узлы.
        let last: Vec<usize> = (n - k..n).rev().collect();
        let spread: Vec<usize> = (0..k).map(|i| i * n / k).collect();
        for set in [last, spread] {
            let result = code.decode(&pick(&nodes, &set), None).unwrap();
            assert_eq!(result.message(), &msg[..], "{params:?}, узлы {set:?}");
        }

        for failed in [0, n / 2, n - 1] {
            let helpers: Vec<usize> = (1..n).map(|i| (failed + i) % n).take(d).collect();
            let payloads: Vec<_> = helpers
                .iter()
                .map(|&h| code.repair_payload(&nodes[h], failed).unwrap())
                .collect();
            assert!(payloads.iter().all(|p| p.symbols.len() == params.beta()));
            let repaired = code.repair(failed, &payloads, None).unwrap();
            assert_eq!(repaired, nodes[failed], "{params:?}, f = {failed}");
        }
    }
}
