use coding_core::{NoopObserver, RepairExecutor, RepairHelper, RepairPlan, RepairPlanner};
use fountain_codes::*;
use std::collections::{BTreeMap, BTreeSet};
fn setup() -> (
    RepairableEncoder,
    RepairCatalog,
    BTreeMap<u64, StoredSymbol>,
) {
    let cfg = RepairableConfig::try_new(
        Profile::Gf256,
        2,
        5,
        0x0123456789abcdef,
        ResourceLimits::default(),
    )
    .unwrap();
    let enc = RepairableEncoder::try_new(&[0x80, 0xff, 0x34, 0x12, 0], [0; 16], cfg).unwrap();
    let mut catalog =
        RepairCatalog::try_new(enc.descriptor().clone(), ResourceLimits::default()).unwrap();
    for id in 3..20 {
        catalog.register(id).unwrap();
    }
    let store = (1..20)
        .map(|id| (id, StoredSymbol::new(enc.symbol_unobserved(id).unwrap())))
        .collect();
    (enc, catalog, store)
}
#[test]
fn callbacks_validate_all_fields_and_count_rejected_payloads() {
    let (_, catalog, store) = setup();
    let mut events = vec![];
    let RepairPlan::Ready(plan) = catalog
        .plan(
            RepairRequest {
                target: 0,
                available_ids: &[1, 2, 4],
            },
            &mut |e: &RepairEvent| events.push(e.clone()),
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(events.is_empty());
    for corruption in 0..4 {
        events.clear();
        let mut called = vec![];
        let result = catalog.repair(
            ExecuteRepair {
                plan: &plan,
                read_helper: &mut |r| {
                    called.push(r.helper_id());
                    let mut response = store[&r.helper_id()].respond_unobserved(r)?;
                    match corruption {
                        0 => {
                            response.descriptor =
                                BlockDescriptor::try_new(Profile::Gf256, [0; 16], 5, 2, 5, 999)
                                    .unwrap();
                        }
                        1 => {
                            response.target = 99;
                        }
                        2 => {
                            response.helper_id = 2;
                        }
                        _ => {
                            response.payload.push(77);
                        }
                    }
                    Ok(response)
                },
            },
            &mut |e: &RepairEvent| events.push(e.clone()),
        );
        assert!(result.is_err());
        assert_eq!(called, [1]);
        assert!(
            matches!(events.as_slice(),[RepairEvent::GroupSelected {..},RepairEvent::HelperRead {requested_id:1,read_symbols:1,read_payload_bytes,..}] if *read_payload_bytes==if corruption==3 {3} else {2})
        );
    }
    events.clear();
    assert!(catalog
        .repair(
            ExecuteRepair {
                plan: &plan,
                read_helper: &mut |_| Err(FountainError::HelperUnavailable { id: 1 })
            },
            &mut |e: &RepairEvent| events.push(e.clone())
        )
        .is_err());
    assert!(matches!(
        events.as_slice(),
        [RepairEvent::GroupSelected { .. }]
    ));
}
#[test]
fn order_one_call_per_helper_and_real_helper_events() {
    let (enc, catalog, store) = setup();
    let mut events = vec![];
    let mut called = vec![];
    let result = repair_local(
        &catalog,
        RepairRequest {
            target: 0,
            available_ids: &[4, 2, 1, 0, 1],
        },
        |r| {
            called.push(r.helper_id());
            store[&r.helper_id()].respond_unobserved(r)
        },
        &mut |e: &RepairEvent| events.push(e.clone()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(called, [1, 2, 4]);
    assert_eq!(result, enc.symbol_unobserved(0).unwrap());
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, RepairEvent::GroupSelected { .. }))
            .count(),
        1
    );
    assert!(matches!(
        events.last().unwrap(),
        RepairEvent::Completed {
            read_symbols: 3,
            read_payload_bytes: 6,
            ..
        }
    ));
    let request = HelperRequest::try_new(enc.descriptor().clone(), 0, 1).unwrap();
    let mut helper_events = vec![];
    store[&1]
        .respond(&request, &mut |e: &RepairEvent| {
            helper_events.push(e.clone())
        })
        .unwrap();
    assert!(matches!(
        helper_events.as_slice(),
        [RepairEvent::HelperResponded { .. }]
    ));
    assert!(store[&2].respond_unobserved(&request).is_err());
    let other = HelperRequest::try_new(
        BlockDescriptor::try_new(Profile::Gf256, [0; 16], 5, 2, 5, 7).unwrap(),
        0,
        1,
    )
    .unwrap();
    assert_eq!(
        store[&1].respond_unobserved(&other),
        Err(FountainError::ContextMismatch)
    );
}
#[test]
fn parity_target_and_zero_parity_need_no_lost_data() {
    let (enc, catalog, _) = setup();
    let store: BTreeMap<_, _> = (0..3)
        .map(|id| (id, StoredSymbol::new(enc.symbol_unobserved(id).unwrap())))
        .collect();
    let result = repair_local(
        &catalog,
        RepairRequest {
            target: 4,
            available_ids: &[0, 1, 2, 4],
        },
        |r| store[&r.helper_id()].respond_unobserved(r),
        &mut NoopObserver,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, enc.symbol_unobserved(4).unwrap());
    let d = BlockDescriptor::try_new(Profile::Gf256, [0; 16], 1, 1, 1, 0x224).unwrap();
    let mut catalog = RepairCatalog::try_new(d, ResourceLimits::default()).unwrap();
    catalog.register(1).unwrap();
    let mut events = vec![];
    let zero = repair_local(
        &catalog,
        RepairRequest {
            target: 1,
            available_ids: &[],
        },
        |_| panic!("zero parity reads no helpers"),
        &mut |e: &RepairEvent| events.push(e.clone()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(zero.payload(), [0]);
    assert!(matches!(
        events.as_slice(),
        [
            RepairEvent::GroupSelected { helpers: 0, .. },
            RepairEvent::Completed {
                read_symbols: 0,
                read_payload_bytes: 0,
                ..
            }
        ]
    ));
}
#[test]
fn unavailable_alternatives_and_greedy_disjoint_lower_bound() {
    let (_, catalog, _) = setup();
    let mut events = vec![];
    assert!(matches!(
        catalog
            .plan(
                RepairRequest {
                    target: 0,
                    available_ids: &[0]
                },
                &mut |e: &RepairEvent| events.push(e.clone())
            )
            .unwrap(),
        RepairPlan::Unavailable(_)
    ));
    assert_eq!(events, [RepairEvent::Unavailable { target: 0 }]);
    let available: Vec<_> = (1..20).collect();
    let RepairPlan::Ready(first) = catalog
        .plan_unobserved(RepairRequest {
            target: 0,
            available_ids: &available,
        })
        .unwrap()
    else {
        panic!()
    };
    let remaining: Vec<_> = available
        .iter()
        .copied()
        .filter(|&id| id != first.parity_id())
        .collect();
    let RepairPlan::Ready(second) = catalog
        .plan_unobserved(RepairRequest {
            target: 0,
            available_ids: &remaining,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_ne!(first.parity_id(), second.parity_id());
    let groups = catalog
        .availability(RepairRequest {
            target: 0,
            available_ids: &available,
        })
        .unwrap();
    let mut used = BTreeSet::new();
    for group in groups.plans() {
        for &(id, _) in group.helpers() {
            assert!(used.insert(id));
            assert_ne!(id, 0);
        }
    }
    assert!(!groups.is_empty());
    // Reordering, duplicates and a claimed-available target do not change choice.
    let mut reordered = available.clone();
    reordered.reverse();
    reordered.extend([0, 1, 2]);
    assert_eq!(
        catalog
            .plan_unobserved(RepairRequest {
                target: 0,
                available_ids: &reordered
            })
            .unwrap(),
        RepairPlan::Ready(first)
    );
}
#[test]
fn catalog_limits_before_commit_and_context_before_reads() {
    let (enc, _, _) = setup();
    let d = enc.descriptor().clone();
    let mut catalog = RepairCatalog::try_new(
        d.clone(),
        ResourceLimits::try_new(256, 10, 1, 100, 100000).unwrap(),
    )
    .unwrap();
    catalog.register(4).unwrap();
    let edges = catalog.edges();
    catalog.register(4).unwrap();
    assert!(matches!(
        catalog.register(5),
        Err(FountainError::ResourceLimitExceeded {
            resource: "catalog_entries",
            ..
        })
    ));
    assert_eq!((catalog.entries(), catalog.edges()), (1, edges));
    assert!(catalog.register(0).is_err());
    let mut edge_limited = RepairCatalog::try_new(
        d.clone(),
        ResourceLimits::try_new(256, 10, 10, 1, 100000).unwrap(),
    )
    .unwrap();
    assert!(edge_limited.register(4).is_err());
    assert_eq!((edge_limited.entries(), edge_limited.edges()), (0, 0));
    let bytes = ResourceCharge::calculate(&d, 0, 1, 3)
        .unwrap()
        .catalog_bytes
        - 1;
    let mut limited =
        RepairCatalog::try_new(d, ResourceLimits::try_new(256, 10, 10, 100, bytes).unwrap())
            .unwrap();
    assert!(limited.register(4).is_err());
    assert_eq!(limited.entries(), 0);
    let RepairPlan::Ready(plan) = catalog
        .plan_unobserved(RepairRequest {
            target: 0,
            available_ids: &[1, 2, 4],
        })
        .unwrap()
    else {
        panic!()
    };
    let other = RepairCatalog::try_new(
        BlockDescriptor::try_new(Profile::Gf256, [0; 16], 5, 2, 5, 7).unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        other.repair(
            ExecuteRepair {
                plan: &plan,
                read_helper: &mut |_| panic!("context checked first")
            },
            &mut NoopObserver
        ),
        Err(FountainError::ContextMismatch)
    );
}

#[test]
fn zero_target_coefficient_and_large_set_of_disjoint_groups() {
    let d = BlockDescriptor::try_new(Profile::Gf256, [0; 16], 1, 1, 1, 0x224).unwrap();
    let mut catalog = RepairCatalog::try_new(d, ResourceLimits::default()).unwrap();
    catalog.register(1).unwrap();
    assert!(matches!(
        catalog
            .plan_unobserved(RepairRequest {
                target: 0,
                available_ids: &[1]
            })
            .unwrap(),
        RepairPlan::Unavailable(_)
    ));
    for id in 2..1000 {
        catalog.register(id).unwrap();
    }
    let available: Vec<_> = (1..1000).collect();
    let groups = catalog
        .availability(RepairRequest {
            target: 0,
            available_ids: &available,
        })
        .unwrap();
    assert!(groups.len() > 980);
    let mut used = BTreeSet::new();
    for plan in groups.plans() {
        assert_eq!(plan.helpers().len(), 1);
        assert!(used.insert(plan.helpers()[0].0));
    }
}
