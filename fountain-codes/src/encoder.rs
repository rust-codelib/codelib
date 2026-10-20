use crate::{
    arithmetic::FieldData,
    canonical_support,
    resources::{buffer, mul},
    BlockDescriptor, EncodedSymbol, FountainError, RepairableConfig, ResourceCharge,
};
use coding_core::{Encoder, Message, NoopObserver, Observer, SymbolEncoder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeEvent {
    Prepared {
        message_bytes: usize,
        source_symbols: usize,
    },
    SymbolGenerated {
        id: u64,
        payload_bytes: usize,
    },
}
pub struct PrepareBlock<'a> {
    pub message: Message<&'a [u8]>,
    pub block_id: [u8; 16],
}
#[derive(Clone, Debug)]
pub struct FountainConfigurator {
    config: RepairableConfig,
}
impl FountainConfigurator {
    pub fn new(config: RepairableConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &RepairableConfig {
        &self.config
    }
}
impl<'a> Encoder<PrepareBlock<'a>> for FountainConfigurator {
    type Output = RepairableEncoder;
    type Error = FountainError;
    type Event = EncodeEvent;
    fn encode(
        &mut self,
        input: PrepareBlock<'a>,
        observer: &mut dyn Observer<EncodeEvent>,
    ) -> Result<Self::Output, FountainError> {
        let result =
            RepairableEncoder::prepare(input.message.0, input.block_id, self.config.clone())?;
        observer.on_event(&EncodeEvent::Prepared {
            message_bytes: result.descriptor.n,
            source_symbols: result.descriptor.k,
        });
        Ok(result)
    }
}

/// Owns the padded source. Generating by ID has no mutable counter.
#[derive(Debug)]
pub struct RepairableEncoder {
    descriptor: BlockDescriptor,
    source: FieldData,
}
impl RepairableEncoder {
    fn prepare(
        message: &[u8],
        block_id: [u8; 16],
        config: RepairableConfig,
    ) -> Result<Self, FountainError> {
        let descriptor = config.descriptor(message.len(), block_id)?;
        config
            .limits()
            .check_bytes(ResourceCharge::calculate(&descriptor, 0, 0, 0)?.encoder_bytes)?;
        buffer::<u16>(mul(descriptor.k, descriptor.elements_per_symbol())?)?;
        let source = FieldData::source(&descriptor, message);
        Ok(Self { descriptor, source })
    }
    pub fn try_new(
        message: &[u8],
        block_id: [u8; 16],
        config: RepairableConfig,
    ) -> Result<Self, FountainError> {
        FountainConfigurator::new(config).encode(
            PrepareBlock {
                message: Message(message),
                block_id,
            },
            &mut NoopObserver,
        )
    }
    pub fn descriptor(&self) -> &BlockDescriptor {
        &self.descriptor
    }
    pub fn symbol_unobserved(&self, id: u64) -> Result<EncodedSymbol, FountainError> {
        self.symbol(id, &mut NoopObserver)
    }
    /// Inclusive traversal ends at u64::MAX without wrapping. An empty block
    /// yields one EmptyBlock error and ends rather than repeating errors.
    pub fn symbols_from(&self, first: u64) -> Symbols<'_> {
        Symbols {
            encoder: self,
            next: Some(first),
        }
    }
}
impl SymbolEncoder<u64> for RepairableEncoder {
    type Symbol = EncodedSymbol;
    type Error = FountainError;
    type Event = EncodeEvent;
    fn symbol(
        &self,
        id: u64,
        observer: &mut dyn Observer<EncodeEvent>,
    ) -> Result<EncodedSymbol, FountainError> {
        let support = canonical_support(&self.descriptor, id)?;
        let payload = self
            .source
            .weighted(self.descriptor.elements_per_symbol(), &support);
        let result = EncodedSymbol::try_new(self.descriptor.clone(), id, payload)?;
        observer.on_event(&EncodeEvent::SymbolGenerated {
            id,
            payload_bytes: result.payload().len(),
        });
        Ok(result)
    }
}
pub struct Symbols<'a> {
    encoder: &'a RepairableEncoder,
    next: Option<u64>,
}
impl Iterator for Symbols<'_> {
    type Item = Result<EncodedSymbol, FountainError>;
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        let symbol = self.encoder.symbol_unobserved(id);
        self.next = if symbol.is_ok() {
            id.checked_add(1)
        } else {
            None
        };
        Some(symbol)
    }
}
