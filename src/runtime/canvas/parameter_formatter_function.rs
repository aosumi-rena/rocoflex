#[path = "parameter_formatter_destruction.rs"]
pub mod destruction;
#[path = "parameter_callable_storage.rs"]
pub mod selected_storage;
#[path = "parameter_formatter_function_entry.rs"]
pub mod native_entry;
#[path = "parameter_formatter_digits.rs"]
pub mod digits_callback;
#[path = "parameter_formatter_parse.rs"]
pub mod parse_callback;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Digits2b6d10, Default2b6ee0 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityEntry { At2b6ed0, At2b6f40 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityLeafRequirement { pub entry_pc: u32, pub lea_rax: u64, pub ret_pc: u32 }

pub fn identity_leaf_requirement(entry: IdentityEntry, actual_image_base: u64) -> IdentityLeafRequirement {
    let (entry_pc, offset, ret_pc) = match entry {
        IdentityEntry::At2b6ed0 => (0x2b6ed0, 0x847a80, 0x2b6ed7),
        IdentityEntry::At2b6f40 => (0x2b6f40, 0x847c40, 0x2b6f47),
    };
    IdentityLeafRequirement { entry_pc, lea_rax: actual_image_base.wrapping_add(offset), ret_pc }
}
impl Entry {
    pub fn pc(self) -> u32 { match self { Self::Digits2b6d10 => 0x2b6d10, Self::Default2b6ee0 => 0x2b6ee0 } }
    pub fn vtable(self) -> u64 { match self { Self::Digits2b6d10 => 0x627ac8, Self::Default2b6ee0 => 0x627ba8 } }
    pub fn return_pc(self) -> u32 { match self { Self::Digits2b6d10 => 0x2b6d23, Self::Default2b6ee0 => 0x2b6eed } }
    pub fn body_pc(self) -> u32 { match self { Self::Digits2b6d10 => 0x2b6d1a, Self::Default2b6ee0 => 0x2b6eea } }

    pub fn matches_memory_entry(self, words: Words, actual: ActualMemoryEntry) -> bool {
        actual.pc == self.body_pc() && actual.rax == words.destination_word && match self {
            Self::Digits2b6d10 => actual.rcx == words.source_word && actual.rdx == words.actual_image_base.wrapping_add(self.vtable()),
            Self::Default2b6ee0 => actual.rdx == words.destination_word && actual.rcx == words.actual_image_base.wrapping_add(self.vtable()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words { pub source_word: u64, pub destination_word: u64, pub actual_image_base: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualMemoryEntry { pub pc: u32, pub rax: u64, pub rcx: u64, pub rdx: u64 }

pub struct Input<O> {
    pub storage: O, pub source_word: u64, pub destination_word: u64,
    pub actual_image_base: u64,
}
pub trait Memory {
    type Storage;
    type Error;
    fn write_word(&mut self, storage: &mut Self::Storage, address: u64, word: u64) -> Result<(), Self::Error>;
    fn read_dword(&mut self, storage: &mut Self::Storage, address: u64) -> Result<u32, Self::Error>;
    fn write_dword(&mut self, storage: &mut Self::Storage, address: u64, bits: u32) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { StoreVtable, ReadDigits, StoreDigits, ReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub vtable_store_returned: bool, pub source_digits: Option<u32>, pub digits_store_returned: bool,
}
pub enum Fault<E> { Memory(E), MissingDigits, AlreadyStopped }
pub struct Prepared<M: Memory> {
    memory: M, input: Input<M::Storage>, cursor: Cursor,
}
pub struct Stopped<M: Memory> { pub pending: Prepared<M>, pub fault: Fault<M::Error> }
pub enum Poll<M: Memory> { Pending(Prepared<M>), Ready(NativeReturnRequired<M>), Failed(Stopped<M>) }

pub struct NativeReturnRequired<M: Memory> { parts: Parts<M> }
pub struct Parts<M: Memory> {
    pub memory: M, pub input: Input<M::Storage>, pub entry: Entry, pub vtable: u64,
    pub phase: Phase, pub attempted: Option<Phase>, pub captures: Captures, pub stopped: bool,
    pub native_entry: Option<native_entry::History>,
}
impl<M: Memory> NativeReturnRequired<M> {
    pub fn required_next_pc(&self) -> u32 { self.parts.entry.return_pc() }
    pub fn required_rax(&self) -> u64 { self.parts.input.destination_word }
    pub fn captures(&self) -> &Captures { &self.parts.captures }
    pub fn into_parts(self) -> Parts<M> { self.parts }
}
impl<M: Memory> Prepared<M> {

    pub fn from_actual_selected_body(memory: M, entry: Entry, input: Input<M::Storage>) -> Self {
        let cursor = Cursor::from_actual_selected_body(entry, Words { source_word: input.source_word,
            destination_word: input.destination_word, actual_image_base: input.actual_image_base });
        Self { memory, input, cursor }
    }
    pub fn phase(&self) -> Phase { self.cursor.phase() }
    pub fn captures(&self) -> &Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<M> {
        let cursor = self.cursor.into_parts();
        Parts { memory: self.memory, input: self.input, entry: cursor.entry, vtable: cursor.vtable,
            phase: cursor.phase, attempted: cursor.attempted, captures: cursor.captures, stopped: cursor.stopped,
            native_entry: cursor.native_entry }
    }
    pub fn poll(mut self, budget: usize) -> Poll<M> {
        match self.cursor.poll(&mut self.memory, &mut self.input.storage, budget) {
            BorrowedPoll::Pending => Poll::Pending(self),
            BorrowedPoll::Ready => Poll::Ready(NativeReturnRequired { parts: self.into_parts() }),
            BorrowedPoll::Failed(fault) => Poll::Failed(Stopped { pending: self, fault }),
        }
    }
}

pub struct Cursor { entry: Entry, words: Words, vtable: u64, phase: Phase,
    attempted: Option<Phase>, captures: Captures, stopped: bool,
    native_entry: Option<native_entry::History> }
pub struct CursorParts { pub entry: Entry, pub words: Words, pub vtable: u64, pub phase: Phase,
    pub attempted: Option<Phase>, pub captures: Captures, pub stopped: bool,
    pub native_entry: Option<native_entry::History> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub entry: Entry, pub words: Words, pub captures: Captures,
    pub native_entry: Option<native_entry::History> }
pub enum BorrowedPoll<E> { Pending, Ready, Failed(Fault<E>) }
impl Cursor {
    pub fn from_actual_selected_body(entry: Entry, words: Words) -> Self {
        Self { entry, words, vtable: words.actual_image_base.wrapping_add(entry.vtable()), phase: Phase::StoreVtable,
            attempted: None, captures: Captures::default(), stopped: false, native_entry: None }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn history(&self) -> Option<History> {
        if self.phase == Phase::ReturnRequired && !self.stopped { Some(History { entry: self.entry, words: self.words,
            captures: self.captures, native_entry: self.native_entry }) }
        else { None }
    }
    pub fn into_parts(self) -> CursorParts { CursorParts { entry: self.entry, words: self.words, vtable: self.vtable,
        phase: self.phase, attempted: self.attempted, captures: self.captures, stopped: self.stopped,
        native_entry: self.native_entry } }
    pub fn poll<M: Memory>(&mut self, memory: &mut M, storage: &mut M::Storage, budget: usize) -> BorrowedPoll<M::Error> {
        if self.stopped { return BorrowedPoll::Failed(Fault::AlreadyStopped); }
        let mut spent = 0usize;
        while self.phase != Phase::ReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step(memory, storage) {
                self.stopped = true; return BorrowedPoll::Failed(fault);
            }
            self.attempted = None;
        }
        if self.phase != Phase::ReturnRequired { BorrowedPoll::Pending } else { BorrowedPoll::Ready }
    }
    fn step<M: Memory>(&mut self, memory: &mut M, storage: &mut M::Storage) -> Result<(), Fault<M::Error>> {
        match self.phase {
            Phase::StoreVtable => {

                memory.write_word(storage, self.words.destination_word, self.vtable).map_err(Fault::Memory)?;
                self.captures.vtable_store_returned = true;
                self.phase = match self.entry { Entry::Digits2b6d10 => Phase::ReadDigits, Entry::Default2b6ee0 => Phase::ReturnRequired };
            }
            Phase::ReadDigits => {
                self.captures.source_digits = Some(memory.read_dword(storage,
                    self.words.source_word.wrapping_add(8)).map_err(Fault::Memory)?);
                self.phase = Phase::StoreDigits;
            }
            Phase::StoreDigits => {
                let bits = self.captures.source_digits.ok_or(Fault::MissingDigits)?;
                memory.write_dword(storage, self.words.destination_word.wrapping_add(8), bits).map_err(Fault::Memory)?;
                self.captures.digits_store_returned = true; self.phase = Phase::ReturnRequired;
            }
            Phase::ReturnRequired => {}
        }
        Ok(())
    }
}
