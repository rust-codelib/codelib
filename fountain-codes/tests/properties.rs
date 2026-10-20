//! Bounded deterministic property sweeps; no assumption that K random parts
//! necessarily have full rank. Known vectors are tested separately.
use coding_core::{IncrementalDecoder, NoopObserver};
use fountain_codes::*;
use std::collections::BTreeMap;
#[test]
fn linearity_systematicity_order_and_local_repair() {
    for profile in [Profile::Gf65536, Profile::Gf256] {
        for seed in 0..24u64 {
            let a: Vec<u8> = (0..17).map(|i| (i * 31 + seed) as u8).collect();
            let b: Vec<u8> = (0..17).map(|i| (i * 7 + seed * 13) as u8).collect();
            let sum: Vec<u8> = a.iter().zip(&b).map(|(a, b)| a ^ b).collect();
            let config =
                RepairableConfig::try_new(profile, 4, 5, seed, ResourceLimits::default()).unwrap();
            let enc = |message: &[u8]| {
                RepairableEncoder::try_new(message, [0; 16], config.clone()).unwrap()
            };
            let (ea, eb, es) = (enc(&a), enc(&b), enc(&sum));
            let k = ea.descriptor().source_symbols() as u64;
            for id in 0..3 * k {
                let sa = ea.symbol_unobserved(id).unwrap();
                let sb = eb.symbol_unobserved(id).unwrap();
                let sc = es.symbol_unobserved(id).unwrap();
                assert_eq!(
                    sa.payload()
                        .iter()
                        .zip(sb.payload())
                        .map(|(a, b)| a ^ b)
                        .collect::<Vec<_>>(),
                    sc.payload()
                );
                if id < k {
                    let mut expected =
                        a[id as usize * 4..a.len().min((id as usize + 1) * 4)].to_vec();
                    expected.resize(4, 0);
                    assert_eq!(sa.payload(), expected);
                }
            }
            let mut decoder =
                RepairableDecoder::try_new(ea.descriptor().clone(), ResourceLimits::default())
                    .unwrap();
            let mut events = vec![];
            for id in (k..3 * k).rev() {
                let symbol = ea.symbol_unobserved(id).unwrap();
                decoder
                    .push(&symbol, &mut |e: &DecodeEvent| events.push(e.clone()))
                    .unwrap();
                decoder.push_unobserved(&symbol).unwrap();
                if decoder.rank() < k as usize {
                    assert!(decoder.message().is_none());
                } else {
                    assert_eq!(decoder.message(), Some(a.as_slice()));
                }
            }
            // Guarantee completion by supplying source equations, independently
            // of whether this particular set of parities reached full rank.
            for id in 0..k {
                decoder
                    .push_unobserved(&ea.symbol_unobserved(id).unwrap())
                    .unwrap();
            }
            assert_eq!(decoder.message(), Some(a.as_slice()));
            let mut catalog =
                RepairCatalog::try_new(ea.descriptor().clone(), ResourceLimits::default()).unwrap();
            for id in k..3 * k {
                catalog.register(id).unwrap();
            }
            for target in 0..2 * k {
                let store: BTreeMap<_, _> = (0..3 * k)
                    .filter(|&id| id != target)
                    .map(|id| (id, StoredSymbol::new(ea.symbol_unobserved(id).unwrap())))
                    .collect();
                let available: Vec<_> = store.keys().copied().collect();
                let repaired = repair_local(
                    &catalog,
                    RepairRequest {
                        target,
                        available_ids: &available,
                    },
                    |r| store[&r.helper_id()].respond_unobserved(r),
                    &mut NoopObserver,
                )
                .unwrap();
                if let Some(repaired) = repaired {
                    assert_eq!(repaired, ea.symbol_unobserved(target).unwrap());
                }
            }
        }
    }
}
#[test]
fn exact_event_order_and_contradiction_before_completion() {
    let config =
        RepairableConfig::try_new(Profile::Gf256, 1, 1, 0x224, ResourceLimits::default()).unwrap();
    let mut configurator = FountainConfigurator::new(config);
    let mut encode_events = vec![];
    use coding_core::{Encoder, Message, SymbolEncoder};
    let enc = configurator
        .encode(
            PrepareBlock {
                message: Message(&[5]),
                block_id: [0; 16],
            },
            &mut |e: &EncodeEvent| encode_events.push(e.clone()),
        )
        .unwrap();
    enc.symbol(1, &mut |e: &EncodeEvent| encode_events.push(e.clone()))
        .unwrap();
    assert_eq!(
        encode_events,
        [
            EncodeEvent::Prepared {
                message_bytes: 1,
                source_symbols: 1
            },
            EncodeEvent::SymbolGenerated {
                id: 1,
                payload_bytes: 1
            }
        ]
    );
    let mut dec =
        RepairableDecoder::try_new(enc.descriptor().clone(), ResourceLimits::default()).unwrap();
    let bad = EncodedSymbol::try_new(enc.descriptor().clone(), 1, vec![1]).unwrap();
    let mut events = vec![];
    assert_eq!(
        dec.push(&bad, &mut |e: &DecodeEvent| events.push(e.clone())),
        Err(FountainError::InconsistentEquation {
            id: 1,
            rhs_column: 0
        })
    );
    assert_eq!(
        (dec.rank(), dec.unique_symbols(), dec.progress()),
        (0, 0, DecodeProgress::Invalid)
    );
    assert_eq!(
        events,
        [DecodeEvent::Invalidated {
            id: 1,
            rank: 0,
            reason: InvalidReason::Equation { rhs_column: 0 }
        }]
    );
}
