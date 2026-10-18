//! Свойства на случайных данных: восстановление данных и узла.

mod common;

use cascade_codes::{CascadeCode, CascadeParams, DataDecoder, Encoder, NodeRepairer};
use common::pick;
use gfpm::Gf251;
use proptest::prelude::*;

fn params() -> impl Strategy<Value = CascadeParams> {
    (2usize..=10, 1usize..=5, 0usize..=4, 1usize..=5).prop_filter_map(
        "допустимые параметры",
        |(extra, k, gap, mu)| {
            let d = k + gap;
            let n = d + extra.min(4);
            CascadeParams::try_new(n, k, d, mu.min(k))
                .ok()
                .filter(|p| p.alpha() <= 150)
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_k_nodes_recover_any_message(
        p in params(),
        seed in any::<u64>(),
        raw in proptest::collection::vec(0u64..251, 0..2000),
    ) {
        let code = CascadeCode::<Gf251>::try_new(p).unwrap();
        let msg: Vec<Gf251> = (0..p.file_size())
            .map(|i| Gf251::new(raw.get(i).copied().unwrap_or(i as u64 % 251)))
            .collect();
        let nodes = code.encode(&msg).unwrap();
        // Случайная перестановка узлов по зерну.
        let mut order: Vec<usize> = (0..p.n()).collect();
        let mut s = seed | 1;
        for i in (1..order.len()).rev() {
            s ^= s << 13; s ^= s >> 7; s ^= s << 17;
            order.swap(i, (s % (i as u64 + 1)) as usize);
        }
        let chosen = &order[..p.k()];
        let result = code.decode(&pick(&nodes, chosen), None).unwrap();
        prop_assert_eq!(result.message(), &msg[..]);
        // Восстановление узла, не попавшего в выборку помощников.
        let failed = order[0];
        let helpers: Vec<usize> = order[1..=p.d()].to_vec();
        let payloads: Vec<_> = helpers
            .iter()
            .map(|&h| code.repair_payload(&nodes[h], failed).unwrap())
            .collect();
        prop_assert_eq!(code.repair(failed, &payloads, None).unwrap(), nodes[failed].clone());
    }
}
