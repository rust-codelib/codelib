use crate::{
    arithmetic::{inverse, negative_product, Accumulator},
    canonical_support,
    resources::{add, buffer, limit},
    BlockDescriptor, EncodedSymbol, FountainError, ResourceCharge, ResourceLimits,
};
use coding_core::{
    NoopObserver, Observer, RepairExecutor, RepairHelper, RepairPlan, RepairPlanner,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairEvent {
    GroupSelected {
        target: u64,
        parity_id: u64,
        helpers: usize,
    },
    HelperResponded {
        target: u64,
        helper_id: u64,
        payload_bytes: usize,
    },
    HelperRead {
        target: u64,
        requested_id: u64,
        returned_id: u64,
        payload_bytes: usize,
        read_symbols: usize,
        read_payload_bytes: usize,
    },
    Completed {
        target: u64,
        read_symbols: usize,
        read_payload_bytes: usize,
    },
    Unavailable {
        target: u64,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct RepairRequest<'a> {
    pub target: u64,
    pub available_ids: &'a [u64],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairUnavailable {
    pub target: u64,
}

/// Greedy disjoint groups. Only selected IDs are retained; plans are produced
/// one at a time so a large catalog does not duplicate descriptors or supports.
#[derive(Debug)]
pub struct Availability<'a> {
    catalog: &'a RepairCatalog,
    target: u64,
    parity_ids: Vec<u64>,
}
impl Availability<'_> {
    pub fn len(&self) -> usize {
        self.parity_ids.len()
    }
    pub fn is_empty(&self) -> bool {
        self.parity_ids.is_empty()
    }
    pub fn parity_ids(&self) -> &[u64] {
        &self.parity_ids
    }
    pub fn plans(&self) -> impl ExactSizeIterator<Item = LocalRepairPlan> + '_ {
        self.parity_ids
            .iter()
            .map(|&id| self.catalog.make_plan(self.target, id))
    }
}

/// Constructed only by the catalog; helpers are distinct, sorted, and never the
/// target. Weights are field representations, not integer multipliers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalRepairPlan {
    descriptor: BlockDescriptor,
    target: u64,
    parity_id: u64,
    helpers: Vec<(u64, u16)>,
}
impl LocalRepairPlan {
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn target(&self) -> u64 {
        self.target
    }
    pub fn parity_id(&self) -> u64 {
        self.parity_id
    }
    pub fn helpers(&self) -> &[(u64, u16)] {
        &self.helpers
    }
    pub fn requests(&self) -> impl Iterator<Item = HelperRequest> + '_ {
        self.helpers.iter().map(|&(helper_id, _)| HelperRequest {
            descriptor: self.descriptor.clone(),
            target: self.target,
            helper_id,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperRequest {
    descriptor: BlockDescriptor,
    target: u64,
    helper_id: u64,
}
impl HelperRequest {
    pub fn try_new(
        descriptor: BlockDescriptor,
        target: u64,
        helper_id: u64,
    ) -> Result<Self, FountainError> {
        descriptor.check_nonempty()?;
        if target == helper_id {
            return Err(FountainError::HelperMismatch {
                expected: target,
                actual: helper_id,
            });
        }
        Ok(Self {
            descriptor,
            target,
            helper_id,
        })
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn target(&self) -> u64 {
        self.target
    }
    pub fn helper_id(&self) -> u64 {
        self.helper_id
    }
}
/// Transport-facing data. The executor validates every field, including payload
/// lengths in malformed responses; successfully returned bytes count as reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperResponse {
    pub descriptor: BlockDescriptor,
    pub target: u64,
    pub helper_id: u64,
    pub payload: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct StoredSymbol {
    symbol: EncodedSymbol,
}
impl StoredSymbol {
    pub fn new(symbol: EncodedSymbol) -> Self {
        Self { symbol }
    }
    pub fn symbol(&self) -> &EncodedSymbol {
        &self.symbol
    }
    pub fn respond_unobserved(
        &self,
        request: &HelperRequest,
    ) -> Result<HelperResponse, FountainError> {
        self.respond(request, &mut NoopObserver)
    }
}
impl RepairHelper<&HelperRequest> for StoredSymbol {
    type Response = HelperResponse;
    type Error = FountainError;
    type Event = RepairEvent;
    fn respond(
        &self,
        request: &HelperRequest,
        observer: &mut dyn Observer<RepairEvent>,
    ) -> Result<HelperResponse, FountainError> {
        if request.descriptor != *self.symbol.descriptor() {
            return Err(FountainError::ContextMismatch);
        }
        if request.helper_id != self.symbol.id() {
            return Err(FountainError::HelperMismatch {
                expected: request.helper_id,
                actual: self.symbol.id(),
            });
        }
        let response = HelperResponse {
            descriptor: request.descriptor.clone(),
            target: request.target,
            helper_id: request.helper_id,
            payload: self.symbol.payload().to_vec(),
        };
        observer.on_event(&RepairEvent::HelperResponded {
            target: request.target,
            helper_id: request.helper_id,
            payload_bytes: response.payload.len(),
        });
        Ok(response)
    }
}
pub type ReadHelper<'a> = dyn FnMut(&HelperRequest) -> Result<HelperResponse, FountainError> + 'a;
pub struct ExecuteRepair<'a> {
    pub plan: &'a LocalRepairPlan,
    pub read_helper: &'a mut ReadHelper<'a>,
}

/// Finite parity catalog, storing only actual nonzero edges and a reverse index.
/// No original message, target data, or full decoder is stored or consulted.
#[derive(Debug)]
pub struct RepairCatalog {
    descriptor: BlockDescriptor,
    limits: ResourceLimits,
    parities: BTreeMap<u64, Vec<(usize, u16)>>,
    by_source: Vec<Vec<u64>>,
    edges: usize,
}
impl RepairCatalog {
    pub fn try_new(
        descriptor: BlockDescriptor,
        limits: ResourceLimits,
    ) -> Result<Self, FountainError> {
        limits.check_draws(descriptor.d)?;
        limits.check_bytes(ResourceCharge::calculate(&descriptor, 0, 0, 0)?.catalog_bytes)?;
        buffer::<Vec<u64>>(descriptor.k)?;
        let by_source = vec![Vec::new(); descriptor.k];
        Ok(Self {
            descriptor,
            limits,
            parities: BTreeMap::new(),
            by_source,
            edges: 0,
        })
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn entries(&self) -> usize {
        self.parities.len()
    }
    pub fn edges(&self) -> usize {
        self.edges
    }
    pub fn register(&mut self, parity_id: u64) -> Result<(), FountainError> {
        self.descriptor.check_nonempty()?;
        if parity_id < self.descriptor.k as u64 {
            return Err(FountainError::InvalidParityId { id: parity_id });
        }
        if self.parities.contains_key(&parity_id) {
            return Ok(());
        }
        let c = add(self.parities.len(), 1)?;
        limit("catalog_entries", c, self.limits.max_catalog_entries())?;
        let support = canonical_support(&self.descriptor, parity_id)?;
        let e = add(self.edges, support.len())?;
        limit("catalog_edges", e, self.limits.max_catalog_edges())?;
        self.limits
            .check_bytes(ResourceCharge::calculate(&self.descriptor, 0, c, e)?.catalog_bytes)?;
        buffer::<(u64, Vec<(usize, u16)>)>(c)?;
        buffer::<(usize, u16)>(e)?;
        for &(i, _) in &support {
            buffer::<u64>(add(self.by_source[i].len(), 1)?)?;
        }
        for &(i, _) in &support {
            self.by_source[i].push(parity_id);
        }
        self.parities.insert(parity_id, support);
        self.edges = e;
        Ok(())
    }
    fn candidates(&self, request: RepairRequest<'_>) -> Result<Vec<(usize, u64)>, FountainError> {
        self.descriptor.check_nonempty()?;
        let k = self.descriptor.k as u64;
        let mut out = Vec::new();
        let mut consider = |id: u64| {
            let support = &self.parities[&id];
            let available = if request.target < k {
                request.available_ids.contains(&id)
                    && support.iter().all(|&(i, _)| {
                        i as u64 == request.target || request.available_ids.contains(&(i as u64))
                    })
            } else {
                support
                    .iter()
                    .all(|&(i, _)| request.available_ids.contains(&(i as u64)))
            };
            if available {
                out.push((support.len(), id));
            }
        };
        if request.target < k {
            for &id in &self.by_source[request.target as usize] {
                consider(id);
            }
        } else if self.parities.contains_key(&request.target) {
            consider(request.target);
        }
        out.sort_unstable();
        Ok(out)
    }
    fn make_plan(&self, target: u64, parity_id: u64) -> LocalRepairPlan {
        let support = &self.parities[&parity_id];
        let mut helpers: Vec<(u64, u16)> = if target < self.descriptor.k as u64 {
            let a = support
                .iter()
                .find(|&&(i, _)| i as u64 == target)
                .expect("candidate covers target")
                .1;
            let inv = inverse(self.descriptor.profile, a);
            support
                .iter()
                .filter(|&&(i, _)| i as u64 != target)
                .map(|&(i, b)| (i as u64, negative_product(self.descriptor.profile, inv, b)))
                .chain(std::iter::once((parity_id, inv)))
                .collect()
        } else {
            support.iter().map(|&(i, a)| (i as u64, a)).collect()
        };
        helpers.sort_unstable_by_key(|h| h.0);
        LocalRepairPlan {
            descriptor: self.descriptor.clone(),
            target,
            parity_id,
            helpers,
        }
    }
    pub fn plan_unobserved(
        &self,
        request: RepairRequest<'_>,
    ) -> Result<RepairPlan<LocalRepairPlan, RepairUnavailable>, FountainError> {
        self.plan(request, &mut NoopObserver)
    }
    /// Deterministic greedy lower bound on disjoint groups, not a maximum.
    pub fn availability(
        &self,
        request: RepairRequest<'_>,
    ) -> Result<Availability<'_>, FountainError> {
        let candidates = self.candidates(request)?;
        // Different groups have distinct parity helpers. Thus their only
        // possible intersections are among the K source helpers.
        let mut used = vec![false; self.descriptor.k];
        let mut result = Vec::new();
        for (_, id) in candidates {
            let plan = self.make_plan(request.target, id);
            if plan
                .helpers
                .iter()
                .all(|h| h.0 >= self.descriptor.k as u64 || !used[h.0 as usize])
            {
                for &(id, _) in &plan.helpers {
                    if id < self.descriptor.k as u64 {
                        used[id as usize] = true;
                    }
                }
                result.push(id);
            }
        }
        Ok(Availability {
            catalog: self,
            target: request.target,
            parity_ids: result,
        })
    }
}
impl<'a> RepairPlanner<RepairRequest<'a>> for RepairCatalog {
    type Plan = LocalRepairPlan;
    type Unavailable = RepairUnavailable;
    type Error = FountainError;
    type Event = RepairEvent;
    fn plan(
        &self,
        request: RepairRequest<'a>,
        observer: &mut dyn Observer<RepairEvent>,
    ) -> Result<RepairPlan<LocalRepairPlan, RepairUnavailable>, FountainError> {
        if let Some(&(_, id)) = self.candidates(request)?.first() {
            Ok(RepairPlan::Ready(self.make_plan(request.target, id)))
        } else {
            observer.on_event(&RepairEvent::Unavailable {
                target: request.target,
            });
            Ok(RepairPlan::Unavailable(RepairUnavailable {
                target: request.target,
            }))
        }
    }
}
impl<'a> RepairExecutor<ExecuteRepair<'a>> for RepairCatalog {
    type Output = EncodedSymbol;
    type Error = FountainError;
    type Event = RepairEvent;
    fn repair(
        &self,
        input: ExecuteRepair<'a>,
        observer: &mut dyn Observer<RepairEvent>,
    ) -> Result<EncodedSymbol, FountainError> {
        let plan = input.plan;
        if plan.descriptor != self.descriptor {
            return Err(FountainError::ContextMismatch);
        }
        observer.on_event(&RepairEvent::GroupSelected {
            target: plan.target,
            parity_id: plan.parity_id,
            helpers: plan.helpers.len(),
        });
        let mut out = Accumulator::new(&self.descriptor);
        let (mut read_symbols, mut read_payload_bytes) = (0, 0);
        for (request, &(_, weight)) in plan.requests().zip(&plan.helpers) {
            let response = (input.read_helper)(&request)?;
            read_symbols = add(read_symbols, 1)?;
            read_payload_bytes = add(read_payload_bytes, response.payload.len())?;
            observer.on_event(&RepairEvent::HelperRead {
                target: plan.target,
                requested_id: request.helper_id,
                returned_id: response.helper_id,
                payload_bytes: response.payload.len(),
                read_symbols,
                read_payload_bytes,
            });
            if response.descriptor != plan.descriptor {
                return Err(FountainError::ContextMismatch);
            }
            if response.target != plan.target {
                return Err(FountainError::TargetMismatch {
                    expected: plan.target,
                    actual: response.target,
                });
            }
            if response.helper_id != request.helper_id {
                return Err(FountainError::HelperMismatch {
                    expected: request.helper_id,
                    actual: response.helper_id,
                });
            }
            if response.payload.len() != self.descriptor.t {
                return Err(FountainError::PayloadLength {
                    expected: self.descriptor.t,
                    actual: response.payload.len(),
                });
            }
            out.add(weight, &response.payload);
        }
        let result = EncodedSymbol::try_new(self.descriptor.clone(), plan.target, out.payload())?;
        observer.on_event(&RepairEvent::Completed {
            target: plan.target,
            read_symbols,
            read_payload_bytes,
        });
        Ok(result)
    }
}
/// Compose the common planner and executor; unavailable groups return None.
pub fn repair_local(
    catalog: &RepairCatalog,
    request: RepairRequest<'_>,
    mut read_helper: impl FnMut(&HelperRequest) -> Result<HelperResponse, FountainError>,
    observer: &mut dyn Observer<RepairEvent>,
) -> Result<Option<EncodedSymbol>, FountainError> {
    match catalog.plan(request, observer)? {
        RepairPlan::Ready(plan) => catalog
            .repair(
                ExecuteRepair {
                    plan: &plan,
                    read_helper: &mut read_helper,
                },
                observer,
            )
            .map(Some),
        RepairPlan::Unavailable(_) => Ok(None),
    }
}
