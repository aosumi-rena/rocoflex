use super::{ActualMemoryEntry, Entry, Input, Memory, Words};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualEntry { pub pc: u32, pub rsp: u64, pub rax: u64, pub rcx: u64, pub rdx: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction { MoveRaxRdx, LeaRdxVtable, LeaRcxVtable }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub entry: Entry, pub pc: u32, pub next_pc: u32, pub instruction: Instruction,
    pub rsp: u64, pub value: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualInstruction {
    pub pc: u32, pub next_pc: u32, pub rsp: u64, pub rax: u64, pub rcx: u64, pub rdx: u64,
}
pub trait Services: Memory {

    fn actual_prefix_instruction(&mut self, storage: &mut Self::Storage, request: Request,
        returned: &mut Option<ActualInstruction>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, LeaVtable, MemoryRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<Request>; 2], pub returned: [Option<ActualInstruction>; 2] }
#[derive(Clone, Copy, Debug)]
pub struct History { pub entry: Entry, pub words: Words, pub actual: ActualEntry, pub captures: Captures }
pub enum Admission { EntryBinding }
pub enum Fault<E> { Service(E), MissingReceipt, ReceiptBinding, AlreadyStopped }

pub struct Cursor { entry: Entry, words: Words, actual: ActualEntry, phase: Phase,
    attempted: Option<Phase>, captures: Captures, stopped: bool }
pub struct CursorParts { pub entry: Entry, pub words: Words, pub actual: ActualEntry, pub phase: Phase,
    pub attempted: Option<Phase>, pub captures: Captures, pub stopped: bool }
pub struct CursorRejected { pub entry: Entry, pub words: Words, pub actual: ActualEntry, pub fault: Admission }
pub struct CursorStopped<E> { pub pending: Cursor, pub fault: Fault<E> }
pub enum CursorPoll<E> { Pending(Cursor), Ready(Entered), Failed(CursorStopped<E>) }

pub struct Entered { history: History, memory_entry: ActualMemoryEntry }
impl Entered {
    pub fn history(&self) -> &History { &self.history }
    pub fn memory_entry(&self) -> ActualMemoryEntry { self.memory_entry }
    pub fn into_parts(self) -> History { self.history }
    pub fn into_memory_cursor(self) -> super::Cursor {
        let mut cursor = super::Cursor::from_actual_selected_body(self.history.entry, self.history.words);
        cursor.native_entry = Some(self.history); cursor
    }
}
impl Cursor {
    pub fn from_actual_entry(entry: Entry, words: Words, actual: ActualEntry) -> Result<Self, CursorRejected> {
        if actual.pc != entry.pc() || actual.rcx != words.source_word || actual.rdx != words.destination_word {
            return Err(CursorRejected { entry, words, actual, fault: Admission::EntryBinding });
        }
        Ok(Self { entry, words, actual, phase: Phase::MoveRax, attempted: None, captures: Captures::default(), stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> CursorParts { CursorParts { entry: self.entry, words: self.words, actual: self.actual,
        phase: self.phase, attempted: self.attempted, captures: self.captures, stopped: self.stopped } }
    pub fn poll<M: Services>(mut self, memory: &mut M, storage: &mut M::Storage, budget: usize) -> CursorPoll<M::Error> {
        if self.stopped { return CursorPoll::Failed(CursorStopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::MemoryRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step(memory, storage) {
                self.stopped = true; return CursorPoll::Failed(CursorStopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::MemoryRequired { return CursorPoll::Pending(self); }
        let Some(last) = self.captures.returned[1] else {
            self.stopped = true; return CursorPoll::Failed(CursorStopped { pending: self, fault: Fault::MissingReceipt });
        };
        let memory_entry = ActualMemoryEntry { pc: last.next_pc, rax: last.rax, rcx: last.rcx, rdx: last.rdx };
        CursorPoll::Ready(Entered { history: History { entry: self.entry, words: self.words, actual: self.actual,
            captures: self.captures }, memory_entry })
    }
    fn step<M: Services>(&mut self, memory: &mut M, storage: &mut M::Storage) -> Result<(), Fault<M::Error>> {
        let (i, request) = match self.phase {
            Phase::MoveRax => (0, Request { entry: self.entry, pc: self.entry.pc(), next_pc: self.entry.pc() + 3,
                instruction: Instruction::MoveRaxRdx, rsp: self.actual.rsp, value: self.words.destination_word }),
            Phase::LeaVtable => (1, Request { entry: self.entry, pc: self.entry.pc() + 3, next_pc: self.entry.body_pc(),
                instruction: match self.entry { Entry::Digits2b6d10 => Instruction::LeaRdxVtable, Entry::Default2b6ee0 => Instruction::LeaRcxVtable },
                rsp: self.actual.rsp, value: self.words.actual_image_base.wrapping_add(self.entry.vtable()) }),
            Phase::MemoryRequired => return Ok(()),
        };
        self.captures.requests[i] = Some(request);
        memory.actual_prefix_instruction(storage, request, &mut self.captures.returned[i]).map_err(Fault::Service)?;
        let actual = self.captures.returned[i].ok_or(Fault::MissingReceipt)?;
        if actual.pc != request.pc || actual.next_pc != request.next_pc || actual.rsp != self.actual.rsp
            || actual.rax != self.words.destination_word {
            return Err(Fault::ReceiptBinding);
        }
        if self.phase == Phase::MoveRax {
            if actual.rcx != self.actual.rcx || actual.rdx != self.actual.rdx { return Err(Fault::ReceiptBinding); }
            self.phase = Phase::LeaVtable;
        } else {
            if !self.entry.matches_memory_entry(self.words, ActualMemoryEntry { pc: actual.next_pc,
                rax: actual.rax, rcx: actual.rcx, rdx: actual.rdx }) { return Err(Fault::ReceiptBinding); }
            self.phase = Phase::MemoryRequired;
        }
        Ok(())
    }
}
pub struct Prepared<M: Services> { memory: M, input: Input<M::Storage>, cursor: Cursor }
pub struct Rejected<M: Services> { pub memory: M, pub input: Input<M::Storage>, pub actual: CursorRejected }
pub struct Parts<M: Services> { pub memory: M, pub input: Input<M::Storage>, pub cursor: CursorParts }
pub struct Stopped<M: Services> { pub pending: Prepared<M>, pub fault: Fault<M::Error> }
pub enum Poll<M: Services> { Pending(Prepared<M>), Ready(MemoryRequired<M>), Failed(Stopped<M>) }
pub struct MemoryRequired<M: Services> { memory: M, input: Input<M::Storage>, entered: Entered }
impl<M: Services> Prepared<M> {
    pub fn from_actual_selected_entry(memory: M, entry: Entry, input: Input<M::Storage>, actual: ActualEntry)
        -> Result<Self, Rejected<M>> {
        let words = Words { source_word: input.source_word, destination_word: input.destination_word,
            actual_image_base: input.actual_image_base };
        match Cursor::from_actual_entry(entry, words, actual) {
            Ok(cursor) => Ok(Self { memory, input, cursor }), Err(actual) => Err(Rejected { memory, input, actual }),
        }
    }
    pub fn phase(&self) -> Phase { self.cursor.phase() }
    pub fn captures(&self) -> &Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<M> { Parts { memory: self.memory, input: self.input, cursor: self.cursor.into_parts() } }
    pub fn poll(self, budget: usize) -> Poll<M> {
        let Self { mut memory, mut input, cursor } = self;
        match cursor.poll(&mut memory, &mut input.storage, budget) {
            CursorPoll::Pending(cursor) => Poll::Pending(Self { memory, input, cursor }),
            CursorPoll::Ready(entered) => Poll::Ready(MemoryRequired { memory, input, entered }),
            CursorPoll::Failed(stopped) => Poll::Failed(Stopped { pending: Self { memory, input, cursor: stopped.pending }, fault: stopped.fault }),
        }
    }
}
impl<M: Services> MemoryRequired<M> {
    pub fn history(&self) -> &History { self.entered.history() }
    pub fn actual_memory_entry(&self) -> ActualMemoryEntry { self.entered.memory_entry() }

    pub fn into_memory_body(self) -> super::Prepared<M> {
        super::Prepared { memory: self.memory, input: self.input, cursor: self.entered.into_memory_cursor() }
    }
    pub fn into_parts(self) -> (M, Input<M::Storage>, History) { (self.memory, self.input, self.entered.into_parts()) }
}
