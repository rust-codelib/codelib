//! Тестовый декодер XOR-уравнений для двух байтов.

use coding_core::{
    Candidate, DecodeOutcome, Decoder, Encoder, IncrementalDecoder, Message, MessageBytes,
    Observer, SymbolEncoder,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    block: u32,
    bytes: MessageBytes,
    salt: u8,
}
impl Context {
    pub fn try_new(block: u32, bytes: usize, salt: u8) -> Result<Self, Error> {
        let bytes = MessageBytes::try_new(bytes).map_err(|_| Error::Shape)?;
        if bytes.get() > 2 {
            return Err(Error::Shape);
        }
        Ok(Self { block, bytes, salt })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub context: Context,
    pub id: u64,
    pub value: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Shape,
    Context,
    Empty,
    Limit,
    Conflict,
    Inconsistent,
    Padding,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    NeedMore { rank: usize, unique: usize },
    Complete,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Independent,
    Dependent,
    Duplicate,
    Verified,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Update {
    pub kind: Kind,
    pub rank: usize,
    pub unique: usize,
    pub progress: Progress,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Accepted(u64),
    RankIncreased(usize),
    Dependent(u64),
    Duplicate(u64),
    Verified(u64),
    Completed,
    Invalidated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recovered {
    bytes: [u8; 2],
    len: usize,
}
impl Recovered {
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsistentEquations;
pub type Output = Candidate<Message<Recovered>, ConsistentEquations>;

pub struct Prepare(pub Context);
pub struct Session {
    context: Context,
    source: [u8; 2],
}
impl Encoder<&[u8]> for Prepare {
    type Output = Session;
    type Error = Error;
    type Event = ();
    fn encode(&mut self, input: &[u8], _: &mut dyn Observer<()>) -> Result<Session, Error> {
        if input.len() != self.0.bytes.get() {
            return Err(Error::Shape);
        }
        let mut source = [0; 2];
        source[..input.len()].copy_from_slice(input);
        Ok(Session {
            context: self.0,
            source,
        })
    }
}
fn mask(id: u64) -> u8 {
    [1, 2, 3, 0][(id % 4) as usize]
}
fn evaluate(mask: u8, source: [u8; 2]) -> u8 {
    (if mask & 1 != 0 { source[0] } else { 0 }) ^ (if mask & 2 != 0 { source[1] } else { 0 })
}
impl SymbolEncoder<u64> for Session {
    type Symbol = Symbol;
    type Error = Error;
    type Event = ();
    fn symbol(&self, id: u64, _: &mut dyn Observer<()>) -> Result<Symbol, Error> {
        if self.context.bytes.get() == 0 {
            return Err(Error::Empty);
        }
        Ok(Symbol {
            context: self.context,
            id,
            value: evaluate(mask(id), self.source),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    mask: u8,
    rhs: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    context: Context,
    rows: [Option<Row>; 2],
    accepted: [Option<Symbol>; 8],
    limit: usize,
    rank: usize,
    unique: usize,
    invalid: bool,
    result: Option<Output>,
}
impl Stream {
    pub fn try_new(context: Context, limit: usize) -> Result<Self, Error> {
        if limit == 0 || limit > 8 {
            return Err(Error::Limit);
        }
        let result = (context.bytes.get() == 0).then_some(Candidate {
            value: Message(Recovered {
                bytes: [0; 2],
                len: 0,
            }),
            checks: ConsistentEquations,
        });
        Ok(Self {
            context,
            rows: [None; 2],
            accepted: [None; 8],
            limit,
            rank: 0,
            unique: 0,
            invalid: false,
            result,
        })
    }
    fn update(&self, kind: Kind) -> Update {
        Update {
            kind,
            rank: self.rank,
            unique: self.unique,
            progress: self.progress(),
        }
    }
    fn invalidate(&mut self, observer: &mut dyn Observer<Event>, error: Error) -> Error {
        self.invalid = true;
        self.result = None;
        observer.on_event(&Event::Invalidated);
        error
    }
}
impl IncrementalDecoder<&Symbol> for Stream {
    type Progress = Progress;
    type Update = Update;
    type Output = Output;
    type Error = Error;
    type Event = Event;
    fn push(
        &mut self,
        symbol: &Symbol,
        observer: &mut dyn Observer<Event>,
    ) -> Result<Update, Error> {
        if self.invalid {
            return Err(Error::Invalid);
        }
        if symbol.context != self.context {
            return Err(Error::Context);
        }
        if self.context.bytes.get() == 0 {
            return Err(Error::Empty);
        }
        if let Some(previous) = self.accepted.iter().flatten().find(|p| p.id == symbol.id) {
            if previous != symbol {
                return Err(Error::Conflict);
            }
            observer.on_event(&Event::Duplicate(symbol.id));
            return Ok(self.update(Kind::Duplicate));
        }
        // Неверный входной символ дополнения отклоняется с сохранением состояния.
        if self.context.bytes.get() == 1 && symbol.id == 1 && symbol.value != 0 {
            return Err(Error::Padding);
        }
        if self.unique == self.limit {
            return Err(Error::Limit);
        }
        if let Some(result) = self.result {
            if evaluate(mask(symbol.id), result.value.0.bytes) != symbol.value {
                return Err(self.invalidate(observer, Error::Inconsistent));
            }
            self.accepted[self.unique] = Some(*symbol);
            self.unique += 1;
            observer.on_event(&Event::Accepted(symbol.id));
            observer.on_event(&Event::Verified(symbol.id));
            return Ok(self.update(Kind::Verified));
        }
        let mut row = Row {
            mask: mask(symbol.id),
            rhs: symbol.value,
        };
        for (pivot, basis) in self.rows.iter().enumerate() {
            if row.mask & (1 << pivot) != 0 {
                if let Some(basis) = basis {
                    row.mask ^= basis.mask;
                    row.rhs ^= basis.rhs;
                }
            }
        }
        if row.mask == 0 && row.rhs != 0 {
            return Err(self.invalidate(observer, Error::Inconsistent));
        }
        self.accepted[self.unique] = Some(*symbol);
        self.unique += 1;
        observer.on_event(&Event::Accepted(symbol.id));
        if row.mask == 0 {
            observer.on_event(&Event::Dependent(symbol.id));
            return Ok(self.update(Kind::Dependent));
        }
        let pivot = row.mask.trailing_zeros() as usize;
        self.rows[pivot] = Some(row);
        self.rank += 1;
        observer.on_event(&Event::RankIncreased(self.rank));
        if self.rank == 2 {
            let second = self.rows[1].unwrap().rhs;
            let first = self.rows[0].unwrap();
            let source = [
                first.rhs ^ if first.mask & 2 != 0 { second } else { 0 },
                second,
            ];
            if self.context.bytes.get() == 1 && source[1] != 0 {
                return Err(self.invalidate(observer, Error::Padding));
            }
            self.result = Some(Candidate {
                value: Message(Recovered {
                    bytes: source,
                    len: self.context.bytes.get(),
                }),
                checks: ConsistentEquations,
            });
            observer.on_event(&Event::Completed);
        }
        Ok(self.update(Kind::Independent))
    }
    fn progress(&self) -> Progress {
        if self.invalid {
            Progress::Invalid
        } else if self.result.is_some() {
            Progress::Complete
        } else {
            Progress::NeedMore {
                rank: self.rank,
                unique: self.unique,
            }
        }
    }
    fn result(&self) -> Option<&Output> {
        self.result.as_ref()
    }
}

/// Каждый вызов обрабатывает новый набор частей без сохранения предыдущего входа.
pub struct Batch(pub Context);
impl Decoder<&[Symbol]> for Batch {
    type Value = Message<Recovered>;
    type Checks = ConsistentEquations;
    type NeedMore = Progress;
    type StopReason = ();
    type Error = Error;
    type Event = Event;
    fn decode(
        &mut self,
        input: &[Symbol],
        observer: &mut dyn Observer<Event>,
    ) -> Result<DecodeOutcome<Self::Value, ConsistentEquations, Progress, ()>, Error> {
        let mut stream = Stream::try_new(self.0, 8)?;
        for symbol in input {
            stream.push(symbol, observer)?;
        }
        Ok(match stream.result() {
            Some(result) => DecodeOutcome::Candidate(*result),
            None => DecodeOutcome::NeedMore(stream.progress()),
        })
    }
}
