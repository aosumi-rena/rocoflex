#[path = "parameter_stream_object_register_return.rs"]
pub mod native_return;
#[path = "parameter_stream_object_register_entry.rs"]
pub mod native_entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64, pub receiver_rcx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct AllocationRequest { pub pc: u32, pub target_rva: u32, pub target: u64, pub work_rsp: u64,
    pub rcx: u64, pub required_rbx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualAllocationReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub rbx: u64 }

pub struct AllocationReturned<R> { pub actual: ActualAllocationReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub struct StoreRequest { pub pc: u32, pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ReadRequest { pub pc: u32, pub address: u64 }
pub trait Services: Sized {
    type Storage; type AllocationOwner; type Error;

    fn allocate(&mut self, owners: &mut Owners<Self>, request: AllocationRequest) -> Result<(), Self::Error>;

    fn read_word(&mut self, owners: &mut Owners<Self>, request: ReadRequest) -> Result<u64, Self::Error>;
    fn store_word(&mut self, owners: &mut Owners<Self>, request: StoreRequest) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub allocation: Option<AllocationReturned<S::AllocationOwner>> }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Allocate, SaveAllocation, BranchAllocation, ReadHead, StoreNext, StoreMember, Publish, NativeReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub request: Option<AllocationRequest>, pub allocation_return: Option<ActualAllocationReturn>,
    pub node: Option<u64>, pub head_request: Option<ReadRequest>, pub head: Option<u64>,

    pub stores: [Option<StoreRequest>; 4], pub completed_stores: [bool; 4] }
pub enum Fault<E> { Service(E), OccupiedAllocation, MissingState, AllocationBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &Owners<S> { &self.owners }
}

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x55e406 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x55e3da { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, allocation: None }, actual: input.actual,
            captures: Captures::default(), phase: Phase::Allocate, attempted: None, stopped: false })
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
    fn store(&mut self, index: usize, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> {
        let request = StoreRequest { pc, address, word }; self.captures.stores[index] = Some(request);
        self.services.store_word(&mut self.owners, request).map_err(Fault::Service)?;
        self.captures.completed_stores[index] = true; Ok(())
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let base = self.actual.source_image_base;
        match self.phase {
            Phase::Allocate => {
                if self.owners.allocation.is_some() { return Err(Fault::OccupiedAllocation); }

                let request = AllocationRequest { pc: 0x55e3e2, target_rva: 0x55d9a0,
                    target: base.wrapping_add(0x55d9a0), work_rsp: self.actual.work_rsp, rcx: 16, required_rbx: self.actual.receiver_rcx };
                self.captures.request = Some(request);
                self.services.allocate(&mut self.owners, request).map_err(Fault::Service)?;
                let actual = self.owners.allocation.as_ref().ok_or(Fault::MissingState)?.actual;
                self.captures.allocation_return = Some(actual);
                if actual.resume_pc != 0x55e3e7 || actual.work_rsp != request.work_rsp || actual.rbx != request.required_rbx {
                    return Err(Fault::AllocationBinding);
                }
                self.captures.node = Some(actual.rax); self.phase = Phase::SaveAllocation;
            }
            Phase::SaveAllocation => {
                self.store(0, 0x55e3e7, self.actual.work_rsp.wrapping_add(0x38), self.captures.node.ok_or(Fault::MissingState)?)?;
                self.phase = Phase::BranchAllocation;
            }
            Phase::BranchAllocation => {
                self.phase = if self.captures.node.ok_or(Fault::MissingState)? == 0 { Phase::Publish } else { Phase::ReadHead };
            }
            Phase::ReadHead => {
                let request = ReadRequest { pc: 0x55e3f1, address: base.wrapping_add(0x85f688) };
                self.captures.head_request = Some(request);
                self.captures.head = Some(self.services.read_word(&mut self.owners, request).map_err(Fault::Service)?);
                self.phase = Phase::StoreNext;
            }
            Phase::StoreNext => {
                self.store(1, 0x55e3f8, self.captures.node.ok_or(Fault::MissingState)?, self.captures.head.ok_or(Fault::MissingState)?)?;
                self.phase = Phase::StoreMember;
            }
            Phase::StoreMember => {
                self.store(2, 0x55e3fb, self.captures.node.ok_or(Fault::MissingState)?.wrapping_add(8), self.actual.receiver_rcx)?;
                self.phase = Phase::Publish;
            }
            Phase::Publish => {
                self.store(3, 0x55e3ff, base.wrapping_add(0x85f688), self.captures.node.ok_or(Fault::MissingState)?)?;
                self.phase = Phase::NativeReturnRequired;
            }
            Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
