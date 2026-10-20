use crate::{
    arithmetic::{Solution, System},
    canonical_support,
    resources::{add, buffer, limit},
    BlockDescriptor, EncodedSymbol, FountainError, ResourceCharge, ResourceLimits,
};
use coding_core::{
    Candidate, DecodeOutcome, Decoder, IncrementalDecoder, Message, NoopObserver, Observer,
};
use gf_linalg::EquationInsertion;
use std::{collections::BTreeMap, convert::Infallible};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeProgress {
    NeedMoreSymbols,
    Complete,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolOutcome {
    Independent,
    Dependent,
    Duplicate,
    Verified,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PushOutcome {
    pub kind: SymbolOutcome,
    pub progress: DecodeProgress,
    pub rank: usize,
    pub unique_symbols: usize,
}
/// These checks describe the accepted equations, not authenticity or a comparison
/// with the sender's original. Integrity remains the transport's responsibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryChecks {
    pub full_rank: bool,
    pub equations_consistent: bool,
    pub padding_valid: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidReason {
    Equation { rhs_column: usize },
    Padding { symbol_id: u64, byte_offset: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeEvent {
    Accepted {
        id: u64,
        unique_symbols: usize,
        rank: usize,
    },
    RankIncreased {
        id: u64,
        previous_rank: usize,
        rank: usize,
        pivot_column: usize,
    },
    Dependent {
        id: u64,
        rank: usize,
    },
    Duplicate {
        id: u64,
        rank: usize,
    },
    Verified {
        id: u64,
        rank: usize,
    },
    Completed {
        rank: usize,
        unique_symbols: usize,
        message_bytes: usize,
    },
    Invalidated {
        id: u64,
        rank: usize,
        reason: InvalidReason,
    },
}
pub type RecoveredMessage = Candidate<Message<Vec<u8>>, RecoveryChecks>;

/// Stores owned copies of accepted payloads; rank and received count are distinct.
/// Ordinary errors preserve the basis and events. A new inconsistent equation
/// invalidates the stream; no candidate is then accessible.
#[derive(Debug)]
pub struct RepairableDecoder {
    descriptor: BlockDescriptor,
    limits: ResourceLimits,
    system: Option<System>,
    accepted: BTreeMap<u64, Vec<u8>>,
    solution: Option<Solution>,
    candidate: Option<RecoveredMessage>,
    progress: DecodeProgress,
}
impl RepairableDecoder {
    pub fn try_new(
        descriptor: BlockDescriptor,
        limits: ResourceLimits,
    ) -> Result<Self, FountainError> {
        limits.check_draws(descriptor.d)?;
        limits.check_bytes(ResourceCharge::calculate(&descriptor, 0, 0, 0)?.decoder_bytes)?;
        let empty = descriptor.k == 0;
        let system = if empty {
            None
        } else {
            Some(System::new(&descriptor)?)
        };
        Ok(Self {
            descriptor,
            limits,
            system,
            accepted: BTreeMap::new(),
            solution: None,
            candidate: empty.then(|| recovered(Vec::new())),
            progress: if empty {
                DecodeProgress::Complete
            } else {
                DecodeProgress::NeedMoreSymbols
            },
        })
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn rank(&self) -> usize {
        self.system.as_ref().map_or(0, System::rank)
    }
    pub fn unique_symbols(&self) -> usize {
        self.accepted.len()
    }
    pub fn message(&self) -> Option<&[u8]> {
        self.result().map(|c| c.value.0.as_slice())
    }
    pub fn push_unobserved(
        &mut self,
        symbol: &EncodedSymbol,
    ) -> Result<PushOutcome, FountainError> {
        self.push(symbol, &mut NoopObserver)
    }
    fn outcome(&self, kind: SymbolOutcome) -> PushOutcome {
        PushOutcome {
            kind,
            progress: self.progress,
            rank: self.rank(),
            unique_symbols: self.accepted.len(),
        }
    }
    fn invalidate(
        &mut self,
        id: u64,
        reason: InvalidReason,
        observer: &mut dyn Observer<DecodeEvent>,
    ) {
        self.progress = DecodeProgress::Invalid;
        observer.on_event(&DecodeEvent::Invalidated {
            id,
            rank: self.rank(),
            reason,
        });
    }
}
fn recovered(bytes: Vec<u8>) -> RecoveredMessage {
    Candidate {
        value: Message(bytes),
        checks: RecoveryChecks {
            full_rank: true,
            equations_consistent: true,
            padding_valid: true,
        },
    }
}
impl IncrementalDecoder<&EncodedSymbol> for RepairableDecoder {
    type Progress = DecodeProgress;
    type Update = PushOutcome;
    type Output = RecoveredMessage;
    type Error = FountainError;
    type Event = DecodeEvent;
    fn progress(&self) -> DecodeProgress {
        self.progress
    }
    fn result(&self) -> Option<&RecoveredMessage> {
        if self.progress == DecodeProgress::Invalid {
            None
        } else {
            self.candidate.as_ref()
        }
    }
    fn push(
        &mut self,
        symbol: &EncodedSymbol,
        observer: &mut dyn Observer<DecodeEvent>,
    ) -> Result<PushOutcome, FountainError> {
        if self.progress == DecodeProgress::Invalid {
            return Err(FountainError::InvalidDecoder);
        }
        if symbol.descriptor() != &self.descriptor {
            return Err(FountainError::ContextMismatch);
        }
        self.descriptor.check_nonempty()?;
        let (id, payload) = (symbol.id(), symbol.payload());
        if payload.len() != self.descriptor.t {
            return Err(FountainError::PayloadLength {
                expected: self.descriptor.t,
                actual: payload.len(),
            });
        }
        if let Some(old) = self.accepted.get(&id) {
            if old != payload {
                return Err(FountainError::ConflictingSymbol { id });
            }
            observer.on_event(&DecodeEvent::Duplicate {
                id,
                rank: self.rank(),
            });
            return Ok(self.outcome(SymbolOutcome::Duplicate));
        }
        self.descriptor.check_padding(id, payload)?;
        let new_m = add(self.accepted.len(), 1)?;
        limit("unique_symbols", new_m, self.limits.max_unique_symbols())?;
        self.limits
            .check_bytes(ResourceCharge::calculate(&self.descriptor, new_m, 0, 0)?.decoder_bytes)?;
        buffer::<(u64, Vec<u8>)>(new_m)?;
        let support = canonical_support(&self.descriptor, id)?;
        let previous_rank = self.rank();
        let insertion = if let Some(solution) = &self.solution {
            let expected = solution.weighted(self.descriptor.elements_per_symbol(), &support);
            if let Some(byte) = expected.iter().zip(payload).position(|(a, b)| a != b) {
                EquationInsertion::Inconsistent {
                    rhs_column: byte / self.descriptor.profile.width(),
                }
            } else {
                EquationInsertion::Dependent
            }
        } else {
            self.system
                .as_mut()
                .ok_or(FountainError::EmptyBlock)?
                .insert(self.descriptor.k, &support, payload)?
        };
        if let EquationInsertion::Inconsistent { rhs_column } = insertion {
            self.invalidate(id, InvalidReason::Equation { rhs_column }, observer);
            return Err(FountainError::InconsistentEquation { id, rhs_column });
        }
        self.accepted.insert(id, payload.to_vec());
        observer.on_event(&DecodeEvent::Accepted {
            id,
            unique_symbols: new_m,
            rank: self.rank(),
        });
        if self.progress == DecodeProgress::Complete {
            observer.on_event(&DecodeEvent::Verified {
                id,
                rank: self.rank(),
            });
            return Ok(self.outcome(SymbolOutcome::Verified));
        }
        match insertion {
            EquationInsertion::Independent { pivot_column, rank } => {
                observer.on_event(&DecodeEvent::RankIncreased {
                    id,
                    previous_rank,
                    rank,
                    pivot_column,
                });
                if rank == self.descriptor.k {
                    let solution = self
                        .system
                        .as_ref()
                        .expect("nonempty system")
                        .solution()?
                        .expect("full rank");
                    let last =
                        solution.weighted(self.descriptor.elements_per_symbol(), &[(rank - 1, 1)]);
                    if let Err(FountainError::NonzeroPadding {
                        symbol_id,
                        byte_offset,
                    }) = self.descriptor.check_padding((rank - 1) as u64, &last)
                    {
                        self.invalidate(
                            id,
                            InvalidReason::Padding {
                                symbol_id,
                                byte_offset,
                            },
                            observer,
                        );
                        return Err(FountainError::NonzeroPadding {
                            symbol_id,
                            byte_offset,
                        });
                    }
                    self.candidate = Some(recovered(solution.message(self.descriptor.n)));
                    self.solution = Some(solution);
                    self.progress = DecodeProgress::Complete;
                    observer.on_event(&DecodeEvent::Completed {
                        rank,
                        unique_symbols: new_m,
                        message_bytes: self.descriptor.n,
                    });
                }
                Ok(self.outcome(SymbolOutcome::Independent))
            }
            EquationInsertion::Dependent => {
                observer.on_event(&DecodeEvent::Dependent {
                    id,
                    rank: self.rank(),
                });
                Ok(self.outcome(SymbolOutcome::Dependent))
            }
            EquationInsertion::Inconsistent { .. } => unreachable!("handled before accepting"),
        }
    }
}

/// Each decode creates a fresh stream and processes every input, even after a
/// candidate first becomes available. No data accumulate across batch calls.
#[derive(Clone, Debug)]
pub struct RepairableBatchDecoder {
    descriptor: BlockDescriptor,
    limits: ResourceLimits,
}
impl RepairableBatchDecoder {
    pub fn try_new(
        descriptor: BlockDescriptor,
        limits: ResourceLimits,
    ) -> Result<Self, FountainError> {
        // Check feasibility without keeping a second basis or changing limits.
        RepairableDecoder::try_new(descriptor.clone(), limits.clone())?;
        Ok(Self { descriptor, limits })
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
}
impl Decoder<&[EncodedSymbol]> for RepairableBatchDecoder {
    type Value = Message<Vec<u8>>;
    type Checks = RecoveryChecks;
    type NeedMore = DecodeProgress;
    type StopReason = Infallible;
    type Error = FountainError;
    type Event = DecodeEvent;
    fn decode(
        &mut self,
        input: &[EncodedSymbol],
        observer: &mut dyn Observer<DecodeEvent>,
    ) -> Result<
        DecodeOutcome<Self::Value, Self::Checks, Self::NeedMore, Self::StopReason>,
        FountainError,
    > {
        let mut decoder = RepairableDecoder::try_new(self.descriptor.clone(), self.limits.clone())?;
        for symbol in input {
            decoder.push(symbol, observer)?;
        }
        Ok(match decoder.candidate {
            Some(candidate) => DecodeOutcome::Candidate(candidate),
            None => DecodeOutcome::NeedMore(decoder.progress),
        })
    }
}
