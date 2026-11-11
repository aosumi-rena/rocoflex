#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Actual2c4f99Range90, Actual2c3cd8RangeMinus48 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value { Byte(u8), Dword(u32), Qword(u64) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Store { pub pc: u32, pub rbp_offset: i32, pub value: Value }
pub trait Services {
    type Frame;
    type Error;

    fn store(&mut self, frame: &mut Self::Frame, operation: Store) -> Result<(), Self::Error>;
}

fn operation(entry: Entry, index: usize) -> Option<Store> {
    let start = Value::Dword(0xc0000000);
    let end = Value::Dword(0x40000000);
    let zero = Value::Dword(0);
    let unit = Value::Dword(0x3f800000);
    let byte = Value::Byte(0);
    let null = Value::Qword(0);
    let stores = match entry {
        Entry::Actual2c4f99Range90 => [
            Store { pc: 0x2c4fa1, rbp_offset: 0x90, value: start },
            Store { pc: 0x2c4fb1, rbp_offset: 0x94, value: end },
            Store { pc: 0x2bc78c, rbp_offset: 0x98, value: zero },
            Store { pc: 0x2bc79c, rbp_offset: 0x9c, value: unit },
            Store { pc: 0x2c2e98, rbp_offset: 0xa0, value: byte },
            Store { pc: 0x2c2e9f, rbp_offset: 0xe0, value: null },
            Store { pc: 0x2bce0b, rbp_offset: 0x120, value: null },
            Store { pc: 0x2bbb0d, rbp_offset: 0x160, value: null },
        ],
        Entry::Actual2c3cd8RangeMinus48 => [
            Store { pc: 0x2c3ce0, rbp_offset: -0x48, value: start },
            Store { pc: 0x2c28ea, rbp_offset: -0x44, value: end },
            Store { pc: 0x2c28f2, rbp_offset: -0x40, value: zero },
            Store { pc: 0x2c1b13, rbp_offset: -0x3c, value: unit },
            Store { pc: 0x2bb169, rbp_offset: -0x38, value: byte },
            Store { pc: 0x2c1eb9, rbp_offset: 0x8, value: null },
            Store { pc: 0x2c1ec0, rbp_offset: 0x48, value: null },
            Store { pc: 0x2bd55f, rbp_offset: 0x88, value: null },
        ],
    };
    stores.get(index).copied()
}

pub enum Fault<E> { Service(E), MissingOperation, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S,
    frame: S::Frame,
    entry: Entry,
    next: usize,
    attempted: Option<Store>,
    completed: [Option<Store>; 8],
    stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(ArgumentWritten<S>), Failed(Stopped<S>) }
pub struct ArgumentWritten<S: Services> { returned: RequiredContinuation<S> }

pub struct RequiredContinuation<S: Services> {
    pub services: S,
    pub frame: S::Frame,
    pub entry: Entry,
    pub stores: [Option<Store>; 8],

    pub required_next_pc: u32,
}
pub struct Parts<S: Services> {
    services: S,
    frame: S::Frame,
    pub entry: Entry,
    pub next: usize,
    pub attempted: Option<Store>,
    pub completed: [Option<Store>; 8],
    pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn frame(&self) -> &S::Frame { &self.frame }
}
impl<S: Services> ArgumentWritten<S> {
    pub fn into_required_continuation(self) -> RequiredContinuation<S> { self.returned }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(services: S, frame: S::Frame, entry: Entry) -> Self {
        Self { services, frame, entry, next: 0, attempted: None, completed: [None; 8], stopped: false }
    }
    pub fn entry(&self) -> Entry { self.entry }
    pub fn services(&self) -> &S { &self.services }
    pub fn frame(&self) -> &S::Frame { &self.frame }
    pub fn attempted(&self) -> Option<Store> { self.attempted }
    pub fn completed(&self) -> &[Option<Store>; 8] { &self.completed }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, frame: self.frame, entry: self.entry, next: self.next,
            attempted: self.attempted, completed: self.completed, stopped: self.stopped }
    }

    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.next < 8 && spent < budget {
            let Some(store) = operation(self.entry, self.next) else {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::MissingOperation });
            };
            self.attempted = Some(store);
            spent += 1;
            if let Err(error) = self.services.store(&mut self.frame, store) {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::Service(error) });
            }
            self.completed[self.next] = Some(store);
            self.next += 1;
            self.attempted = None;
        }
        if self.next < 8 { return Poll::Pending(self); }
        Poll::Ready(ArgumentWritten { returned: RequiredContinuation { services: self.services,
            frame: self.frame, entry: self.entry, stores: self.completed,
            required_next_pc: match self.entry {
                Entry::Actual2c4f99Range90 => 0x2bbb14,
                Entry::Actual2c3cd8RangeMinus48 => 0x2bd56a,
            } } })
    }
}
