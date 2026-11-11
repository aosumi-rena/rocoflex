#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct LoadRequest { pub pc: u32, pub next_pc: u32, pub address: u64, pub rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct LoadReturned { pub pc: u32, pub next_pc: u32, pub address: u64, pub rsp: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ReturnRequest { pub pc: u32, pub rsp: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64 }
pub trait Services {
    type Error;

    fn load_current(&mut self, request: LoadRequest, returned: &mut Option<LoadReturned>) -> Result<(), Self::Error>;

    fn actual_return(&mut self, request: ReturnRequest, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Load, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub load_request: Option<LoadRequest>, pub loaded: Option<LoadReturned>,
    pub return_request: Option<ReturnRequest>, pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, LoadBinding, ReturnBinding, AlreadyStopped }

pub struct Cursor { actual: ActualEntry, captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Parts { pub actual: ActualEntry, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl Cursor {
    pub fn from_actual_entry(actual: ActualEntry) -> Result<Self, ActualEntry> {
        if actual.pc != 0x55e7ac { return Err(actual); }
        Ok(Self { actual, captures: Captures::default(), phase: Phase::Load, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn history(&self) -> History { History { actual: self.actual, captures: self.captures } }
    pub fn into_parts(self) -> Parts { Parts { actual: self.actual, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll<S: Services + ?Sized>(&mut self, services: &mut S, budget: usize) -> Result<bool, Fault<S::Error>> {
        if self.stopped { return Err(Fault::AlreadyStopped); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step(services) { self.stopped = true; return Err(fault); }
            self.attempted = None;
        }
        Ok(self.phase == Phase::Done)
    }
    fn step<S: Services + ?Sized>(&mut self, services: &mut S) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Load => {
                let request = LoadRequest { pc: 0x55e7ac, next_pc: 0x55e7b3,
                    address: self.actual.source_image_base.wrapping_add(0x85f698), rsp: self.actual.rsp };
                self.captures.load_request = Some(request);
                services.load_current(request, &mut self.captures.loaded).map_err(Fault::Service)?;
                let a = self.captures.loaded.ok_or(Fault::MissingReceipt)?;
                if a.pc != request.pc || a.next_pc != request.next_pc || a.address != request.address || a.rsp != request.rsp {
                    return Err(Fault::LoadBinding);
                }
                self.phase = Phase::Return;
            }
            Phase::Return => {
                let request = ReturnRequest { pc: 0x55e7b3, rsp: self.actual.rsp,
                    rax: self.captures.loaded.ok_or(Fault::MissingReceipt)?.rax };
                self.captures.return_request = Some(request);
                services.actual_return(request, &mut self.captures.actual_return).map_err(Fault::Service)?;
                let a = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != request.rsp || a.after_rsp != request.rsp.wrapping_add(8) || a.rax != request.rax {
                    return Err(Fault::ReturnBinding);
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
