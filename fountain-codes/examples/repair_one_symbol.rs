mod support;
use fountain_codes::*;
use std::collections::BTreeMap;
fn main() -> Result<(), FountainError> {
    let encoder = support::encoder()?;
    let expected = encoder.symbol_unobserved(0)?; // Retained only by checking code.
    let mut catalog =
        RepairCatalog::try_new(encoder.descriptor().clone(), ResourceLimits::default())?;
    catalog.register(4)?;
    let store: BTreeMap<_, _> = [1, 2, 4]
        .into_iter()
        .map(|id| Ok((id, StoredSymbol::new(encoder.symbol_unobserved(id)?))))
        .collect::<Result<_, FountainError>>()?;
    drop(encoder); // The repair callback has no original, encoder, or target.
    let mut metrics = (0, 0);
    let restored = repair_local(
        &catalog,
        RepairRequest {
            target: 0,
            available_ids: &[1, 2, 4],
        },
        |request| store[&request.helper_id()].respond_unobserved(request),
        &mut |event: &RepairEvent| {
            if let RepairEvent::Completed {
                read_symbols,
                read_payload_bytes,
                ..
            } = event
            {
                metrics = (*read_symbols, *read_payload_bytes);
            }
        },
    )?
    .expect("fixture has a repair group");
    assert_eq!(restored, expected); // Checking code only.
    assert_eq!(metrics, (3, 12));
    println!("lost=0, available=[1,2,4], helpers_read={}, payload_bytes={}, restored={:02x?}, original comparison=OK",metrics.0,metrics.1,restored.payload());
    Ok(())
}
