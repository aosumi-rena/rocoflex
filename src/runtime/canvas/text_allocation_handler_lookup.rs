#[path = "text_allocation_handler_indexed_tail.rs"]
pub mod indexed_tail;
#[path = "text_allocation_handler_lookup_borrowed.rs"]
pub mod borrowed;
pub struct Input<O> { pub owners: O, pub actual_image_base: u64 }
pub trait Services {
    type Owners;
    type Error;
    fn call57a7a8(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error>;
    fn read_word(&mut self, owners: &mut Self::Owners, address: u64) -> Result<u64, Self::Error>;
    fn call57a7c4(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexedTailEntry { At57a7a8, At57a7c4 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexedTailRequirement {
    pub entry_rva: u32, pub indirect_rva: u32,
    pub actual_index_ecx: u32, pub sign_extended_index: u64, pub index_times_five: u64,
    pub required_rax: u64, pub required_rcx: u64, pub actual_addressed_slot: u64,
}

pub fn indexed_tail_requirement(entry: IndexedTailEntry,
    actual_index_ecx: u32, actual_source_image_base: u64) -> IndexedTailRequirement {
    let sign_extended_index = actual_index_ecx as i32 as i64 as u64;
    let index_times_five = sign_extended_index.wrapping_add(sign_extended_index.wrapping_mul(4));
    let base = actual_source_image_base.wrapping_add(0x861200);
    let (entry_rva, indirect_rva, slot_rva) = match entry {
        IndexedTailEntry::At57a7a8 => (0x57a7a8, 0x57a7ba, 0x77f908),
        IndexedTailEntry::At57a7c4 => (0x57a7c4, 0x57a7d6, 0x77fb88),
    };
    IndexedTailRequirement { entry_rva, indirect_rva,
        actual_index_ecx, sign_extended_index, index_times_five,
        required_rax: base, required_rcx: base.wrapping_add(index_times_five.wrapping_mul(8)),
        actual_addressed_slot: actual_source_image_base.wrapping_add(slot_rva) }
}

pub fn decode_saved_word(cookie: u64, encoded: u64) -> u64 {
    decode_with_rotation(cookie, encoded, (cookie as u32) & 0x3f)
}
fn decode_with_rotation(cookie: u64, encoded: u64, rotation: u32) -> u64 {
    (encoded ^ cookie).rotate_right(rotation)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Enter, Cookie, RotationCount, Encoded, Decode, Leave, ReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub enter_returned: bool, pub cookie: Option<u64>, pub rotation: Option<u32>, pub encoded: Option<u64>,
    pub decoded: Option<u64>, pub leave_returned: bool,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, input: Input<S::Owners>, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(ReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> {
    pub(super) services: S, pub(super) input: Input<S::Owners>, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Owners> { &self.input }
}
pub struct ReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> Prepared<S> {
    pub fn from_actual_576624(services: S, input: Input<S::Owners>) -> Self {
        Self { services, input, captures: Captures::default(), phase: Phase::Enter,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }

    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::ReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase == Phase::ReturnRequired { Poll::Ready(ReturnRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let c = &mut self.captures;
        match self.phase {
            Phase::Enter => {
                self.services.call57a7a8(&mut self.input.owners, 0).map_err(Fault::Service)?;
                c.enter_returned = true; self.phase = Phase::Cookie;
            }
            Phase::Cookie => {
                c.cookie = Some(self.services.read_word(&mut self.input.owners,
                    self.input.actual_image_base.wrapping_add(0x820dd8)).map_err(Fault::Service)?);
                self.phase = Phase::RotationCount;
            }
            Phase::RotationCount => {

                c.rotation = Some((c.cookie.ok_or(Fault::MissingState)? as u32) & 0x3f);
                self.phase = Phase::Encoded;
            }
            Phase::Encoded => {
                c.encoded = Some(self.services.read_word(&mut self.input.owners,
                    self.input.actual_image_base.wrapping_add(0x861040)).map_err(Fault::Service)?);
                self.phase = Phase::Decode;
            }
            Phase::Decode => {
                c.decoded = Some(decode_with_rotation(c.cookie.ok_or(Fault::MissingState)?,
                    c.encoded.ok_or(Fault::MissingState)?, c.rotation.ok_or(Fault::MissingState)?));
                self.phase = Phase::Leave;
            }
            Phase::Leave => {
                self.services.call57a7c4(&mut self.input.owners, 0).map_err(Fault::Service)?;
                c.leave_returned = true; self.phase = Phase::ReturnRequired;
            }
            Phase::ReturnRequired => {}
        }
        Ok(())
    }
}
impl<S: Services> ReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x576652 }

    pub fn required_rax(&self) -> u64 { self.parts.captures.decoded.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
