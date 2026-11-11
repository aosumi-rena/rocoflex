#[path = "parameter_stream_member_handler_context_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_handler_context_return.rs"]
pub mod native_return;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub metadata_rcx: u64, pub incoming_rax: u64, pub rbp: u64 }
pub struct Owners<S: Services> { pub storage: S::Storage }
pub trait Services: Sized {
    type Storage; type Error;

    fn read_context(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadContext, ClearRdi, BindRbx, Test, Branch, NativeReturnRequired, ProtectedRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub context_rsi: Option<u64>,
    pub zero_rdi: Option<u64>, pub metadata_rbx: Option<u64>, pub nonzero: Option<bool>, pub attempted_read: Option<(u32, u64)> }
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Protected(ProtectedRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57a323 }

    pub fn incoming_rax(&self) -> u64 { self.parts.actual.incoming_rax }
    pub fn context_word(&self) -> u64 { self.parts.captures.context_rsi.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
#[derive(Clone, Copy, Debug)]
pub struct ProtectedRequirement { pub call_pc: u32, pub target: u64, pub work_rsp: u64,
    pub rax: u64, pub rbx: u64, pub rsi: u64, pub rdi: u64 }
pub struct ProtectedRequired<S: Services> { parts: Parts<S> }
impl<S: Services> ProtectedRequired<S> {
    pub fn requirement(&self) -> ProtectedRequirement { ProtectedRequirement { call_pc: 0x57a2e8,
        target: self.parts.actual.source_image_base.wrapping_add(0xc27e1c), work_rsp: self.parts.actual.work_rsp,
        rax: self.parts.actual.incoming_rax, rbx: self.parts.captures.metadata_rbx.unwrap(),
        rsi: self.parts.captures.context_rsi.unwrap(), rdi: self.parts.captures.zero_rdi.unwrap() } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a2db { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage }, actual: input.actual,
            captures: Captures::default(), phase: Phase::ReadContext, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && self.phase != Phase::ProtectedRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase { Phase::NativeReturnRequired => Poll::Ready(NativeReturnRequired { parts: self.into_parts() }),
            Phase::ProtectedRequired => Poll::Protected(ProtectedRequired { parts: self.into_parts() }), _ => Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::ReadContext => {
                let address = self.actual.metadata_rcx; self.captures.attempted_read = Some((0x57a2db, address));
                self.captures.context_rsi = Some(self.services.read_context(&mut self.owners, 0x57a2db, address).map_err(Fault::Service)?);
                self.phase = Phase::ClearRdi;
            }
            Phase::ClearRdi => { self.captures.zero_rdi = Some(0); self.phase = Phase::BindRbx; }
            Phase::BindRbx => { self.captures.metadata_rbx = Some(self.actual.metadata_rcx); self.phase = Phase::Test; }
            Phase::Test => { self.captures.nonzero = Some(self.captures.context_rsi.ok_or(Fault::MissingState)? != 0); self.phase = Phase::Branch; }
            Phase::Branch => { self.phase = if self.captures.nonzero.ok_or(Fault::MissingState)? { Phase::NativeReturnRequired } else { Phase::ProtectedRequired }; }
            Phase::NativeReturnRequired | Phase::ProtectedRequired => {}
        }
        Ok(())
    }
}
