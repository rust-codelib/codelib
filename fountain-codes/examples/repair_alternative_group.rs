mod support;
use coding_core::{NoopObserver, RepairExecutor, RepairPlan};
use fountain_codes::*;
use std::collections::BTreeMap;
fn main() -> Result<(), FountainError> {
    let encoder = support::encoder()?;
    let expected = encoder.symbol_unobserved(0)?;
    let mut catalog =
        RepairCatalog::try_new(encoder.descriptor().clone(), ResourceLimits::default())?;
    for id in 3..20 {
        catalog.register(id)?;
    }
    let mut store: BTreeMap<_, _> = (1..20)
        .map(|id| Ok((id, StoredSymbol::new(encoder.symbol_unobserved(id)?))))
        .collect::<Result<_, FountainError>>()?;
    drop(encoder);
    let ids: Vec<_> = store.keys().copied().collect();
    let RepairPlan::Ready(first) = catalog.plan_unobserved(RepairRequest {
        target: 0,
        available_ids: &ids,
    })?
    else {
        panic!("fixture group")
    };
    store.remove(&first.parity_id()); // The previously selected parity is now lost too.
    let ids: Vec<_> = store.keys().copied().collect();
    let RepairPlan::Ready(second) = catalog.plan_unobserved(RepairRequest {
        target: 0,
        available_ids: &ids,
    })?
    else {
        panic!("alternative group")
    };
    assert_ne!(first.parity_id(), second.parity_id());
    let restored = catalog.repair(
        ExecuteRepair {
            plan: &second,
            read_helper: &mut |r| store[&r.helper_id()].respond_unobserved(r),
        },
        &mut NoopObserver,
    )?;
    assert_eq!(restored, expected);
    println!(
        "unavailable parity={}, alternative parity={}, helpers={:?}, comparison=OK",
        first.parity_id(),
        second.parity_id(),
        second.helpers()
    );
    println!(
        "greedy disjoint groups (lower bound)={}",
        catalog
            .availability(RepairRequest {
                target: 0,
                available_ids: &ids
            })?
            .len()
    );
    Ok(())
}
