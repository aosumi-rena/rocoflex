#[path = "parameter_stream_member_diagnostic_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_diagnostic_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_dispatch.rs"]
pub mod dispatch;
#[path = "parameter_stream_member_dispatch_call.rs"]
pub mod dispatch_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64, pub incoming_rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct AndReturned { pub address: u64, pub before: u64, pub after: u64 }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub pc: u32, pub target: u64, pub resume_pc: u32, pub work_rsp: u64,
    pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64, pub stack_arg5: u64, pub required_rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub struct Owners<S: Services> { pub storage: S::Storage, pub call: Option<CallReturned<S::CallOwner>> }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn and_stack_qword_zero(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64,
        returned: &mut Option<AndReturned>) -> Result<(), Self::Error>;

    fn call_dispatch(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ClearFifth, Dispatch, NativeReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub and_returned: Option<AndReturned>,
    pub dispatch_call: Option<dispatch_call::History>,
    pub request: Option<CallRequest>, pub completed: bool, pub result: Option<u64> }
pub enum Fault<E> { Service(E), MissingReceipt, RmwBinding, OccupiedCall, CallBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57a01d }
    pub fn result(&self) -> u64 { self.parts.captures.result.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    fn dispatch_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != Phase::Dispatch { return Err(Fault::MissingReceipt); }
        Ok(CallRequest { pc: 0x57a018, target: self.actual.source_image_base.wrapping_add(0x57a384),
            resume_pc: 0x57a01d, work_rsp: self.actual.work_rsp, rcx: 0, rdx: 0, r8: 0, r9: 0,
            stack_arg5: 0, required_rax: self.actual.incoming_rax })
    }
    fn record_dispatch_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let a = self.owners.call.as_ref().ok_or(Fault::MissingReceipt)?.actual;
        if a.resume_pc != request.resume_pc || a.work_rsp != request.work_rsp { return Err(Fault::CallBinding); }
        Ok(a.rax)
    }
    fn record_dispatch_value(&mut self, rax: u64) {
        self.captures.result = Some(rax); self.captures.completed = true; self.phase = Phase::NativeReturnRequired;
    }
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a008 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, call: None }, actual: input.actual,
            captures: Captures::default(), phase: Phase::ClearFifth, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::ClearFifth => {
                let address = self.actual.work_rsp.wrapping_add(0x20);
                self.services.and_stack_qword_zero(&mut self.owners, 0x57a008, address, &mut self.captures.and_returned).map_err(Fault::Service)?;
                let a = self.captures.and_returned.ok_or(Fault::MissingReceipt)?;
                if a.address != address || a.after != 0 { return Err(Fault::RmwBinding); }
                self.phase = Phase::Dispatch;
            }
            Phase::Dispatch => {
                if self.owners.call.is_some() { return Err(Fault::OccupiedCall); }
                let request = self.dispatch_request()?;
                self.captures.request = Some(request);
                self.services.call_dispatch(&mut self.owners, request).map_err(Fault::Service)?;
                let value = self.record_dispatch_return(request)?; self.record_dispatch_value(value);
            }
            Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
