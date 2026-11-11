#[path = "parameter_stream_member_encoded_slot_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_encoded_slot_return.rs"]
pub mod native_return;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub rax: u64, pub rcx: u64, pub rdx: u64, pub rbp: u64 }
pub struct Owners<S: Services> { pub storage: S::Storage }
pub trait Services: Sized {
    type Storage; type Error;

    fn read_byte(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn read_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ClearRbx, BindRdi, BindRsi, CompareFlag, Branch, ReadIndex, Address,
    ProtectedRequired, NativeReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub index_rbx: Option<u64>,
    pub metadata_rdi: Option<u64>, pub table_rsi: Option<u64>, pub flag: Option<u8>,
    pub result_rax: Option<u64>, pub attempted_read: Option<(u32, u64)> }
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
    pub fn required_next_pc(&self) -> u32 { 0x57a374 }
    pub fn raw_rax(&self) -> u64 { self.parts.captures.result_rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
#[derive(Clone, Copy, Debug)]
pub struct ProtectedRequirement { pub call_pc: u32, pub target: u64, pub work_rsp: u64,
    pub rax: u64, pub rbx: u64, pub rsi: u64, pub rdi: u64 }
pub struct ProtectedRequired<S: Services> { parts: Parts<S> }
impl<S: Services> ProtectedRequired<S> {

    pub fn requirement(&self) -> ProtectedRequirement { ProtectedRequirement { call_pc: 0x57a354,
        target: self.parts.actual.source_image_base.wrapping_add(0x9ebf22), work_rsp: self.parts.actual.work_rsp,
        rax: self.parts.actual.rax, rbx: self.parts.captures.index_rbx.unwrap(),
        rsi: self.parts.captures.table_rsi.unwrap(), rdi: self.parts.captures.metadata_rdi.unwrap() } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a347 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage }, actual: input.actual,
            captures: Captures::default(), phase: Phase::ClearRbx, attempted: None, stopped: false })
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
            Phase::ClearRbx => { self.captures.index_rbx = Some(0); self.phase = Phase::BindRdi; }
            Phase::BindRdi => { self.captures.metadata_rdi = Some(self.actual.rdx); self.phase = Phase::BindRsi; }
            Phase::BindRsi => { self.captures.table_rsi = Some(self.actual.rcx); self.phase = Phase::CompareFlag; }
            Phase::CompareFlag => {
                let address = self.actual.rdx.wrapping_add(0x10); self.captures.attempted_read = Some((0x57a34f, address));
                self.captures.flag = Some(self.services.read_byte(&mut self.owners, 0x57a34f, address).map_err(Fault::Service)?);
                self.phase = Phase::Branch;
            }
            Phase::Branch => { self.phase = if self.captures.flag.ok_or(Fault::MissingState)? == 0 { Phase::ProtectedRequired } else { Phase::ReadIndex }; }
            Phase::ReadIndex => {
                let address = self.actual.rdx.wrapping_add(8); self.captures.attempted_read = Some((0x57a36c, address));
                self.captures.index_rbx = Some(self.services.read_qword(&mut self.owners, 0x57a36c, address).map_err(Fault::Service)?);
                self.phase = Phase::Address;
            }
            Phase::Address => {
                self.captures.result_rax = Some(self.captures.table_rsi.ok_or(Fault::MissingState)?
                    .wrapping_add(self.captures.index_rbx.ok_or(Fault::MissingState)?.wrapping_mul(8)));
                self.phase = Phase::NativeReturnRequired;
            }
            Phase::ProtectedRequired | Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
