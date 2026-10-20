use fountain_codes::*;
use serde_json::Value;

fn fixtures() -> Value {
    serde_json::from_str(include_str!("../../docs/fixtures/fountain-v1.json")).unwrap()
}
fn descriptor(v: &Value) -> BlockDescriptor {
    BlockDescriptor::try_new(
        if v["profile"] == 1 {
            Profile::Gf65536
        } else {
            Profile::Gf256
        },
        std::array::from_fn(|i| i as u8),
        v["n"].as_u64().unwrap() as usize,
        v["t"].as_u64().unwrap() as usize,
        v["d"].as_u64().unwrap() as u16,
        0x0123456789abcdef,
    )
    .unwrap()
}
#[test]
fn saved_supports_and_budgets() {
    let json = fixtures();
    for v in json["messages"].as_array().unwrap() {
        let desc = descriptor(v);
        for s in v["symbols"].as_array().unwrap() {
            let expected: Vec<_> = s["support"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    (
                        p[0].as_u64().unwrap() as usize,
                        p[1].as_u64().unwrap() as u16,
                    )
                })
                .collect();
            assert_eq!(
                canonical_support(&desc, s["id"].as_str().unwrap().parse().unwrap()).unwrap(),
                expected
            );
        }
    }
    for v in json["resource_cases"].as_array().unwrap() {
        let d = BlockDescriptor::try_new(
            if v["width"] == 2 {
                Profile::Gf65536
            } else {
                Profile::Gf256
            },
            [0; 16],
            v["n"].as_u64().unwrap() as usize,
            v["t"].as_u64().unwrap() as usize,
            v["d"].as_u64().unwrap() as u16,
            0,
        )
        .unwrap();
        let c = ResourceCharge::calculate(
            &d,
            v["unique_symbols"].as_u64().unwrap() as usize,
            v["catalog_entries"].as_u64().unwrap() as usize,
            v["catalog_edges"].as_u64().unwrap() as usize,
        )
        .unwrap();
        assert_eq!(
            [
                c.scratch_bytes,
                c.encoder_bytes,
                c.decoder_bytes,
                c.catalog_bytes
            ],
            [
                "scratch_bytes",
                "encoder_bytes",
                "decoder_bytes",
                "catalog_bytes"
            ]
            .map(|k| v[k].as_u64().unwrap() as usize)
        );
    }
}
#[test]
fn boundaries_and_zero_support() {
    let d = BlockDescriptor::try_new(Profile::Gf256, [0; 16], 1, 1, 1, 0x224).unwrap();
    assert!(canonical_support(&d, 1).unwrap().is_empty());
    assert!(BlockDescriptor::try_new(Profile::Gf256, [0; 16], 4097, 1, 1, 0).is_err());
    assert!(BlockDescriptor::try_new(Profile::Gf65536, [0; 16], 1, 1, 1, 0).is_err());
    let empty = BlockDescriptor::try_new(Profile::Gf65536, [0; 16], 0, 2, 0, 0).unwrap();
    assert_eq!(canonical_support(&empty, 0), Err(FountainError::EmptyBlock));
    assert_eq!(logarithmic_draws(0, 1.0).unwrap(), 0);
    assert_eq!(logarithmic_draws(1, 1.0).unwrap(), 1);
    assert!(logarithmic_draws(2, f64::INFINITY).is_err());
    assert!(ResourceLimits::try_new(0, 1, 1, 1, 1).is_err());
    assert!(ResourceCharge::calculate(&d, usize::MAX, 0, 0).is_err());
}

fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn saved_encoder_payloads() {
    for v in fixtures()["messages"].as_array().unwrap() {
        let d = descriptor(v);
        let message = hex(v["message_hex"].as_str().unwrap());
        let original = message.clone();
        let config = RepairableConfig::try_new(
            d.profile(),
            d.symbol_bytes(),
            d.parity_draws(),
            d.seed(),
            ResourceLimits::default(),
        )
        .unwrap();
        let enc = RepairableEncoder::try_new(&message, *d.block_id(), config).unwrap();
        assert_eq!(message, original);
        for s in v["symbols"].as_array().unwrap().iter().rev() {
            let id = s["id"].as_str().unwrap().parse().unwrap();
            assert_eq!(
                enc.symbol_unobserved(id).unwrap().payload(),
                hex(s["payload_hex"].as_str().unwrap())
            );
        }
        assert_eq!(enc.symbols_from(u64::MAX).count(), 1);
    }
}

#[test]
fn saved_global_recovery() {
    use coding_core::{IncrementalDecoder, NoopObserver};
    for v in fixtures()["messages"].as_array().unwrap() {
        let d = descriptor(v);
        let mut decoder = RepairableDecoder::try_new(d.clone(), ResourceLimits::default()).unwrap();
        let mut events = Vec::new();
        for id in [3, 4, 5] {
            let s = &v["symbols"][id];
            let symbol = EncodedSymbol::try_new(
                d.clone(),
                id as u64,
                hex(s["payload_hex"].as_str().unwrap()),
            )
            .unwrap();
            decoder
                .push(&symbol, &mut |e: &DecodeEvent| events.push(e.clone()))
                .unwrap();
        }
        assert_eq!(
            decoder.message().unwrap(),
            hex(v["message_hex"].as_str().unwrap())
        );
        let pivots: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let DecodeEvent::RankIncreased { pivot_column, .. } = e {
                    Some(*pivot_column)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(pivots, [1, 0, 2]);
        let bad =
            EncodedSymbol::try_new(d, 6, vec![0; decoder.descriptor().symbol_bytes()]).unwrap();
        assert!(matches!(
            decoder.push(&bad, &mut NoopObserver),
            Err(FountainError::InconsistentEquation { .. })
        ));
        assert_eq!(decoder.progress(), DecodeProgress::Invalid);
        assert!(decoder.message().is_none());
    }
}

#[test]
fn saved_local_repair_without_target_or_source() {
    use coding_core::{RepairExecutor, RepairPlan};
    use std::collections::BTreeMap;
    for v in fixtures()["messages"].as_array().unwrap() {
        let d = descriptor(v);
        let mut catalog = RepairCatalog::try_new(d.clone(), ResourceLimits::default()).unwrap();
        for id in [5, 3, 4] {
            catalog.register(id).unwrap();
        }
        let mut store = BTreeMap::new();
        for id in [1, 2, 3, 4, 5] {
            let s = &v["symbols"][id];
            store.insert(
                id as u64,
                StoredSymbol::new(
                    EncodedSymbol::try_new(
                        d.clone(),
                        id as u64,
                        hex(s["payload_hex"].as_str().unwrap()),
                    )
                    .unwrap(),
                ),
            );
        }
        let RepairPlan::Ready(plan) = catalog
            .plan_unobserved(RepairRequest {
                target: 0,
                available_ids: &[5, 4, 3, 2, 1, 0, 1],
            })
            .unwrap()
        else {
            panic!("expected plan")
        };
        assert_eq!(plan.parity_id(), 4);
        let groups = catalog
            .availability(RepairRequest {
                target: 0,
                available_ids: &[1, 2, 3, 4, 5],
            })
            .unwrap();
        assert_eq!(groups.parity_ids(), [4]);
        assert_eq!(groups.plans().next().unwrap(), plan);
        let expected: Vec<_> = v["local_repair"]["helpers_and_weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| {
                (
                    a[0].as_str().unwrap().parse::<u64>().unwrap(),
                    a[1].as_u64().unwrap() as u16,
                )
            })
            .collect();
        assert_eq!(plan.helpers(), expected);
        let mut events = vec![];
        let restored = catalog
            .repair(
                ExecuteRepair {
                    plan: &plan,
                    read_helper: &mut |r| store[&r.helper_id()].respond_unobserved(r),
                },
                &mut |e: &RepairEvent| events.push(e.clone()),
            )
            .unwrap();
        assert_eq!(
            restored.payload(),
            hex(v["local_repair"]["payload_hex"].as_str().unwrap())
        );
        assert!(
            matches!(events.last().unwrap(),RepairEvent::Completed {read_symbols:3,read_payload_bytes,..} if *read_payload_bytes==3*d.symbol_bytes())
        );
    }
}

#[test]
fn saved_wire_packets_and_round_trips() {
    for v in fixtures()["messages"].as_array().unwrap() {
        let d = descriptor(v);
        let bytes = hex(v["descriptor_hex"].as_str().unwrap());
        assert_eq!(d.to_wire().unwrap(), bytes);
        assert_eq!(parse_descriptor(&bytes).unwrap(), d);
        for s in v["symbols"].as_array().unwrap() {
            let bytes = hex(s["packet_hex"].as_str().unwrap());
            let symbol = parse_symbol(&bytes).unwrap();
            assert_eq!(symbol.to_wire().unwrap(), bytes);
            assert_eq!(symbol.descriptor(), &d);
            assert_eq!(symbol.payload(), hex(s["payload_hex"].as_str().unwrap()));
        }
    }
    let bytes = hex(fixtures()["empty_descriptor_hex"].as_str().unwrap());
    let d = parse_descriptor(&bytes).unwrap();
    assert_eq!(d.source_symbols(), 0);
    assert_eq!(d.to_wire().unwrap(), bytes);
}
