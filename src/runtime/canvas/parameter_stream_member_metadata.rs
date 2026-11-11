#[path = "parameter_stream_member_metadata_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_metadata_return.rs"]
pub mod native_return;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub metadata_rcx: u64, pub incoming_rax: u64, pub rbx: u64, pub rbp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct Store { pub address: u64, pub word: u64 }
pub struct Owners<S: Services> { pub storage: S::Storage }
pub trait Services: Sized {
    type Storage; type Error;

    fn read_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn save_rbx_568a1f(&mut self, o: &mut Owners<Self>, out: &mut Option<Store>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Compare, SetRdi, Branch, ReadCurrent, SaveRbx, NativeReturnRequired, ProtectedRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub first_word: Option<u64>,
    pub receiver_rdi: Option<u64>, pub current_word: Option<u64>, pub store: Option<Store>, pub attempted_read: Option<(u32, u64)> }
pub enum Fault<E> { Service(E), MissingReceipt, StoreBinding, AlreadyStopped }
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
    pub fn required_next_pc(&self) -> u32 { 0x568a6b }
    pub fn raw_rax(&self) -> u64 { self.parts.captures.current_word.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
#[derive(Clone, Copy, Debug)]
pub struct ProtectedRequirement { pub call_pc: u32, pub target: u64, pub work_rsp: u64, pub metadata_rdi: u64, pub incoming_rax: u64 }
pub struct ProtectedRequired<S: Services> { parts: Parts<S> }
impl<S: Services> ProtectedRequired<S> {

    pub fn requirement(&self) -> ProtectedRequirement { ProtectedRequirement { call_pc: 0x568a24,
        target: self.parts.actual.source_image_base.wrapping_add(0xabe764), work_rsp: self.parts.actual.work_rsp,
        metadata_rdi: self.parts.captures.receiver_rdi.unwrap(), incoming_rax: self.parts.actual.incoming_rax } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x568a16 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage }, actual: input.actual,
            captures: Captures::default(), phase: Phase::Compare, attempted: None, stopped: false })
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
            Phase::Compare => {
                let address = self.actual.metadata_rcx; self.captures.attempted_read = Some((0x568a16, address));
                self.captures.first_word = Some(self.services.read_qword(&mut self.owners, 0x568a16, address).map_err(Fault::Service)?);
                self.phase = Phase::SetRdi;
            }
            Phase::SetRdi => { self.captures.receiver_rdi = Some(self.actual.metadata_rcx); self.phase = Phase::Branch; }
            Phase::Branch => { self.phase = if self.captures.first_word.ok_or(Fault::MissingReceipt)? != 0 { Phase::ReadCurrent } else { Phase::SaveRbx }; }
            Phase::ReadCurrent => {

                let address = self.captures.receiver_rdi.ok_or(Fault::MissingReceipt)?;
                self.captures.attempted_read = Some((0x568a68, address));
                self.captures.current_word = Some(self.services.read_qword(&mut self.owners, 0x568a68, address).map_err(Fault::Service)?);
                self.phase = Phase::NativeReturnRequired;
            }
            Phase::SaveRbx => {
                self.services.save_rbx_568a1f(&mut self.owners, &mut self.captures.store).map_err(Fault::Service)?;
                let a = self.captures.store.ok_or(Fault::MissingReceipt)?;
                if a.address != self.actual.work_rsp.wrapping_add(0x38) || a.word != self.actual.rbx { return Err(Fault::StoreBinding); }
                self.phase = Phase::ProtectedRequired;
            }
            Phase::NativeReturnRequired | Phase::ProtectedRequired => {}
        }
        Ok(())
    }
}
