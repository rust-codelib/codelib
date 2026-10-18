//! Тестовые схемы локального XOR-ремонта и ремонта по ответам, зависящим от цели.
//! Схема с фрагментами проверяет интерфейс; алгоритм Cascade здесь не реализован.

use coding_core::{Observer, RepairExecutor, RepairHelper, RepairPlan, RepairPlanner};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Local,
    Projection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub block: u32,
    pub mode: Mode,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Target,
    Context,
    Helper,
    Missing,
    Duplicate,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Selected(u8),
    Responded(u8, u8),
    Completed(u8),
}
pub struct Request<'a> {
    pub target: u8,
    pub available: &'a [u8],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelperRequest {
    pub context: Context,
    pub target: u8,
    pub helper: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Response {
    pub request: HelperRequest,
    pub value: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    context: Context,
    target: u8,
    requests: [HelperRequest; 2],
}
impl Plan {
    pub fn requests(&self) -> &[HelperRequest; 2] {
        &self.requests
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingHelpers(pub u8);
pub struct Protocol(pub Context);

impl RepairPlanner<Request<'_>> for Protocol {
    type Plan = Plan;
    type Unavailable = MissingHelpers;
    type Error = Error;
    type Event = Event;
    fn plan(
        &self,
        input: Request<'_>,
        observer: &mut dyn Observer<Event>,
    ) -> Result<RepairPlan<Plan, MissingHelpers>, Error> {
        if input.target >= 3 {
            return Err(Error::Target);
        }
        if input.available.iter().any(|&id| id >= 3) {
            return Err(Error::Helper);
        }
        let helpers = match input.target {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let mut missing = 0;
        for helper in helpers {
            if !input.available.contains(&helper) {
                missing |= 1 << helper;
            }
        }
        if missing != 0 {
            return Ok(RepairPlan::Unavailable(MissingHelpers(missing)));
        }
        let requests = helpers.map(|helper| HelperRequest {
            context: self.0,
            target: input.target,
            helper,
        });
        observer.on_event(&Event::Selected(input.target));
        Ok(RepairPlan::Ready(Plan {
            context: self.0,
            target: input.target,
            requests,
        }))
    }
}

/// Данные отдельного помощника без доступа к потерянной цели и оригиналу.
pub struct Helper {
    pub context: Context,
    pub id: u8,
    pub stored: [u8; 2],
    pub online: bool,
}
impl RepairHelper<&HelperRequest> for Helper {
    type Response = Response;
    type Error = Error;
    type Event = Event;
    fn respond(
        &self,
        request: &HelperRequest,
        observer: &mut dyn Observer<Event>,
    ) -> Result<Response, Error> {
        if request.context != self.context {
            return Err(Error::Context);
        }
        if request.target >= 3 || request.target == self.id {
            return Err(Error::Target);
        }
        if request.helper != self.id || self.id >= 3 {
            return Err(Error::Helper);
        }
        if !self.online {
            return Err(Error::Unavailable);
        }
        let value = match self.context.mode {
            Mode::Local => self.stored[0],
            Mode::Projection if request.target < 2 => self.stored[0] ^ self.stored[1],
            Mode::Projection => self.stored[0],
        };
        observer.on_event(&Event::Responded(request.target, self.id));
        Ok(Response {
            request: *request,
            value,
        })
    }
}

pub struct Execute<'a> {
    pub plan: &'a Plan,
    pub responses: &'a [Response],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repaired {
    Symbol(u8),
    Node([u8; 2]),
}
impl RepairExecutor<Execute<'_>> for Protocol {
    type Output = Repaired;
    type Error = Error;
    type Event = Event;
    fn repair(
        &self,
        input: Execute<'_>,
        observer: &mut dyn Observer<Event>,
    ) -> Result<Repaired, Error> {
        if input.plan.context != self.0 {
            return Err(Error::Context);
        }
        let mut values = [None; 2];
        for response in input.responses {
            if response.request.context != self.0 {
                return Err(Error::Context);
            }
            if response.request.target != input.plan.target {
                return Err(Error::Target);
            }
            let index = input
                .plan
                .requests
                .iter()
                .position(|r| r == &response.request)
                .ok_or(Error::Helper)?;
            if values[index].replace(response.value).is_some() {
                return Err(Error::Duplicate);
            }
        }
        let [Some(a), Some(b)] = values else {
            return Err(Error::Missing);
        };
        let output = match self.0.mode {
            Mode::Local => Repaired::Symbol(a ^ b),
            Mode::Projection => Repaired::Node(match input.plan.target {
                0 => [a, b],
                1 => [b, a],
                _ => [a ^ b, a],
            }),
        };
        observer.on_event(&Event::Completed(input.plan.target));
        Ok(output)
    }
}
