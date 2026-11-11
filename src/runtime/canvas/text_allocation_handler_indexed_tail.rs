use super::{IndexedTailEntry, IndexedTailRequirement};
#[path = "text_allocation_handler_indexed_borrowed.rs"]
pub mod borrowed;
#[derive(Clone, Copy, Debug)]
pub struct ActualTail { pub entry: IndexedTailEntry, pub pc: u32, pub source_image_base: u64,
    pub original_index_ecx: u32, pub rsp: u64, pub rax: u64, pub rcx: u64 }
pub struct Input<S: Services> { pub services: S, pub owners: S::Owners, pub actual: ActualTail }
#[derive(Clone, Copy, Debug)]
pub struct Request { pub pc: u32, pub slot: u64, pub loaded_target: u64, pub entry_rsp: u64, pub rax: u64, pub rcx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub entry_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64 }
pub struct ReturnedTail<R> { pub actual: ActualReturn, pub owner: R }
pub trait Services: super::Services + Sized {
    type IndexedTailOwner;

    fn read_indexed_target(&mut self, owners: &mut Self::Owners, pc: u32, slot: u64) -> Result<u64, Self::Error>;

    fn indexed_whole_tail(&mut self, owners: &mut Self::Owners, request: Request,
        held: &mut Option<ReturnedTail<Self::IndexedTailOwner>>) -> Result<(), Self::Error>;
}

pub trait Primitives {
    type Owners; type Error; type TailOwner;

    fn read_target(&mut self, owners: &mut Self::Owners, pc: u32, slot: u64) -> Result<u64, Self::Error>;

    fn whole_tail(&mut self, owners: &mut Self::Owners, request: Request,
        held: &mut Option<ReturnedTail<Self::TailOwner>>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadTarget, Tail, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub loaded_target: Option<u64>, pub request: Option<Request> }
pub enum Fault<E> { Service(E), MissingReceipt, OccupiedTail, ReturnBinding, AlreadyStopped }
pub struct Rejected<S: Services> { pub input: Input<S>, pub requirement: IndexedTailRequirement }
pub struct Prepared<S: Services> { input: Input<S>, cursor: Cursor<S::IndexedTailOwner> }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: S::Owners, pub actual: ActualTail,
    pub requirement: IndexedTailRequirement, pub captures: Captures, pub held: Option<ReturnedTail<S::IndexedTailOwner>>,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &S::Owners { &self.owners } }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Returned<S: Services> { parts: Parts<S> }

pub struct Cursor<R> { parts: CursorParts<R> }
pub struct CursorParts<R> { pub actual: ActualTail, pub requirement: IndexedTailRequirement,
    pub captures: Captures, pub held: Option<ReturnedTail<R>>, pub phase: Phase,
    pub attempted: Option<Phase>, pub stopped: bool }
pub struct CursorReturned<R> { parts: CursorParts<R> }
pub enum CursorPoll<R,E> { Pending(Cursor<R>), Ready(CursorReturned<R>),
    Failed { parts: CursorParts<R>, fault: Fault<E> } }
impl<R> CursorReturned<R> {
    pub fn actual_return(&self) -> &ActualReturn { &self.parts.held.as_ref().unwrap().actual }
    pub fn into_parts(self) -> CursorParts<R> { self.parts }
}
impl<R> Cursor<R> {
    pub fn from_actual_indirect_tail(actual: ActualTail) -> Result<Self, IndexedTailRequirement> {
        let requirement = super::indexed_tail_requirement(actual.entry, actual.original_index_ecx, actual.source_image_base);
        if actual.pc != requirement.indirect_rva || actual.rax != requirement.required_rax || actual.rcx != requirement.required_rcx {
            return Err(requirement);
        }
        Ok(Self { parts: CursorParts { actual, requirement, captures: Captures::default(), held: None,
            phase: Phase::ReadTarget, attempted: None, stopped: false } })
    }
    pub fn phase(&self) -> Phase { self.parts.phase }
    pub fn captures(&self) -> Captures { self.parts.captures }
    pub fn into_parts(self) -> CursorParts<R> { self.parts }
    pub fn poll<S: Primitives<TailOwner=R>>(mut self, services: &mut S, owners: &mut S::Owners,
        budget: usize) -> CursorPoll<R,S::Error> {
        if self.parts.stopped { return CursorPoll::Failed { parts: self.parts, fault: Fault::AlreadyStopped }; }
        let mut spent = 0;
        while self.parts.phase != Phase::Done && spent < budget {
            self.parts.attempted = Some(self.parts.phase); spent += 1;
            if let Err(fault) = self.step(services, owners) {
                self.parts.stopped = true; return CursorPoll::Failed { parts: self.parts, fault };
            }
            self.parts.attempted = None;
        }
        if self.parts.phase == Phase::Done { CursorPoll::Ready(CursorReturned { parts: self.parts }) }
        else { CursorPoll::Pending(self) }
    }
    fn step<S: Primitives<TailOwner=R>>(&mut self, services: &mut S, owners: &mut S::Owners) -> Result<(), Fault<S::Error>> {
        let p = &mut self.parts;
        match p.phase {
            Phase::ReadTarget => {
                let target = services.read_target(owners, p.requirement.indirect_rva,
                    p.requirement.actual_addressed_slot).map_err(Fault::Service)?;
                p.captures.loaded_target = Some(target); p.phase = Phase::Tail;
            }
            Phase::Tail => {
                if p.held.is_some() { return Err(Fault::OccupiedTail); }
                let request = Request { pc: p.requirement.indirect_rva, slot: p.requirement.actual_addressed_slot,
                    loaded_target: p.captures.loaded_target.ok_or(Fault::MissingReceipt)?,
                    entry_rsp: p.actual.rsp, rax: p.actual.rax, rcx: p.actual.rcx };
                p.captures.request = Some(request);
                services.whole_tail(owners, request, &mut p.held).map_err(Fault::Service)?;
                let r = p.held.as_ref().ok_or(Fault::MissingReceipt)?.actual;
                if r.entry_rsp != request.entry_rsp || r.after_rsp != request.entry_rsp.wrapping_add(8) { return Err(Fault::ReturnBinding); }
                p.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
struct Existing<'a,S:Services>(&'a mut S);
impl<S:Services> Primitives for Existing<'_,S> {
    type Owners=S::Owners; type Error=S::Error; type TailOwner=S::IndexedTailOwner;
    fn read_target(&mut self,o:&mut Self::Owners,pc:u32,slot:u64)->Result<u64,S::Error>{self.0.read_indexed_target(o,pc,slot)}
    fn whole_tail(&mut self,o:&mut Self::Owners,r:Request,held:&mut Option<ReturnedTail<Self::TailOwner>>)->Result<(),S::Error>{self.0.indexed_whole_tail(o,r,held)}
}
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { &self.parts.held.as_ref().unwrap().actual }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {

    pub fn from_actual_indirect_tail(input: Input<S>) -> Result<Self, Rejected<S>> {
        match Cursor::from_actual_indirect_tail(input.actual) {
            Ok(cursor)=>Ok(Self { input, cursor }), Err(requirement)=>Err(Rejected { input, requirement })
        }
    }
    pub fn into_parts(self) -> Parts<S> { Self::parts(self.input,self.cursor.into_parts()) }
    fn parts(input:Input<S>,p:CursorParts<S::IndexedTailOwner>)->Parts<S>{Parts { services: input.services, owners: input.owners,
        actual: p.actual, requirement: p.requirement, captures: p.captures, held: p.held,
        phase: p.phase, attempted: p.attempted, stopped: p.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        match self.cursor.poll(&mut Existing(&mut self.input.services),&mut self.input.owners,budget) {
            CursorPoll::Pending(cursor)=>Poll::Pending(Self{input:self.input,cursor}),
            CursorPoll::Ready(done)=>Poll::Ready(Returned{parts:Self::parts(self.input,done.into_parts())}),
            CursorPoll::Failed{parts,fault}=>Poll::Failed(Stopped{pending:Self{input:self.input,cursor:Cursor{parts}},fault})
        }
    }
}
