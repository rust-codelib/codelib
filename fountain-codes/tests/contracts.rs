//! All seven implementations are called through coding-core generic bounds.
use coding_core::*;
use fountain_codes::*;
use std::fmt::Debug;
fn prepare<'a, E>(e: &mut E, input: PrepareBlock<'a>) -> RepairableEncoder
where
    E: Encoder<
        PrepareBlock<'a>,
        Output = RepairableEncoder,
        Error = FountainError,
        Event = EncodeEvent,
    >,
{
    e.encode(input, &mut NoopObserver).unwrap()
}
fn symbol<E: SymbolEncoder<u64, Symbol = EncodedSymbol>>(e: &E, id: u64) -> EncodedSymbol
where
    E::Error: Debug,
{
    e.symbol(id, &mut NoopObserver).unwrap()
}
fn push<'a, D>(d: &mut D, s: &'a EncodedSymbol) -> PushOutcome
where
    D: IncrementalDecoder<
        &'a EncodedSymbol,
        Update = PushOutcome,
        Progress = DecodeProgress,
        Output = RecoveredMessage,
        Error = FountainError,
        Event = DecodeEvent,
    >,
{
    d.push(s, &mut NoopObserver).unwrap()
}
fn decode<'a, D>(d: &mut D, s: &'a [EncodedSymbol]) -> Candidate<D::Value, D::Checks>
where
    D: Decoder<&'a [EncodedSymbol]>,
    D::Error: Debug,
{
    match d.decode(s, &mut NoopObserver).unwrap() {
        DecodeOutcome::Candidate(c) => c,
        _ => panic!("candidate expected"),
    }
}
fn plan<'a, P>(p: &P, r: RepairRequest<'a>) -> LocalRepairPlan
where
    P: RepairPlanner<RepairRequest<'a>, Plan = LocalRepairPlan, Unavailable = RepairUnavailable>,
    P::Error: Debug,
{
    match p.plan(r, &mut NoopObserver).unwrap() {
        RepairPlan::Ready(p) => p,
        _ => panic!("plan expected"),
    }
}
fn respond<'a, H>(h: &H, r: &'a HelperRequest) -> HelperResponse
where
    H: RepairHelper<&'a HelperRequest, Response = HelperResponse, Error = FountainError>,
{
    h.respond(r, &mut NoopObserver).unwrap()
}
fn repair<'a, E>(e: &E, input: ExecuteRepair<'a>) -> EncodedSymbol
where
    E: RepairExecutor<ExecuteRepair<'a>, Output = EncodedSymbol>,
    E::Error: Debug,
{
    e.repair(input, &mut NoopObserver).unwrap()
}
#[test]
fn all_seven_roles_two_profiles() {
    for profile in [Profile::Gf65536, Profile::Gf256] {
        let config =
            RepairableConfig::try_new(profile, 2, 5, 0x0123456789abcdef, ResourceLimits::default())
                .unwrap();
        let encoder = prepare(
            &mut FountainConfigurator::new(config),
            PrepareBlock {
                message: Message(&[0x80, 0xff, 0x34, 0x12, 0]),
                block_id: [0; 16],
            },
        );
        let parts: Vec<_> = [3, 4, 5]
            .into_iter()
            .map(|id| symbol(&encoder, id))
            .collect();
        let desc = encoder.descriptor().clone();
        let mut incremental =
            RepairableDecoder::try_new(desc.clone(), ResourceLimits::default()).unwrap();
        for s in &parts {
            push(&mut incremental, s);
        }
        assert_eq!(incremental.message().unwrap(), [0x80, 0xff, 0x34, 0x12, 0]);
        let batch = decode(
            &mut RepairableBatchDecoder::try_new(desc.clone(), ResourceLimits::default()).unwrap(),
            &parts,
        );
        assert_eq!(batch.value, Message(vec![0x80, 0xff, 0x34, 0x12, 0]));
        let mut catalog = RepairCatalog::try_new(desc, ResourceLimits::default()).unwrap();
        catalog.register(4).unwrap();
        let plan = plan(
            &catalog,
            RepairRequest {
                target: 0,
                available_ids: &[1, 2, 4],
            },
        );
        let store: std::collections::BTreeMap<_, _> = [1, 2, 4]
            .into_iter()
            .map(|id| (id, StoredSymbol::new(symbol(&encoder, id))))
            .collect();
        let restored = repair(
            &catalog,
            ExecuteRepair {
                plan: &plan,
                read_helper: &mut |r| Ok(respond(&store[&r.helper_id()], r)),
            },
        );
        assert_eq!(restored, symbol(&encoder, 0));
    }
}
