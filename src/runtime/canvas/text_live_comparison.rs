use super::{CharacterMemory, CharacterProgress, decode_from_memory, folded_order, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Exact202e40, Folded1fbf80 }
impl Entry {
    fn return_pc(self) -> u32 { match self {
        Self::Exact202e40 => 0x203079, Self::Folded1fbf80 => 0x1fc1e8,
    } }
}
pub struct Input<O> { pub owners: O, pub left_word: u64, pub right_word: u64 }
pub trait Services {

    type Owners;
    type Error;
    fn read_byte(&mut self, owners: &mut Self::Owners, address: u64)
        -> Result<u8, Self::Error>;
    fn uppercase_58a19c(&mut self, owners: &mut Self::Owners,
        actual_call_pc: u32, saved_input_ecx: u32) -> Result<u16, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { DecodeLeft, DecodeRight, Compare, UpperRight, UpperLeft, Finish, ReturnRequired }
#[derive(Clone, Copy, Debug)]
pub struct ByteRead { pub side: Side, pub address: u64, pub returned: Option<u8> }
#[derive(Clone, Copy, Debug)]
pub struct UpperCall { pub pc: u32, pub input_ecx: u32 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub left_cursor: u64, pub right_cursor: u64,
    pub left: Option<u32>, pub right: Option<u32>,
    pub attempted_read: Option<ByteRead>, pub attempted_upper: Option<UpperCall>,
    pub decoding: Option<CharacterProgress<u64>>,
    pub upper_right: Option<u16>, pub upper_left: Option<u16>,
    pub difference: Option<i32>, pub result: Option<i32>,
}
struct LiveMemory<'a, S: Services> {
    services: &'a mut S, owners: &'a mut S::Owners,
    side: Side, attempted: &'a mut Option<ByteRead>,
    progress: &'a mut Option<CharacterProgress<u64>>,
}
impl<S: Services> CharacterMemory for LiveMemory<'_, S> {
    type Address = u64;
    type Error = S::Error;
    fn read_byte(&mut self, address: u64) -> Result<u8, Self::Error> {
        *self.attempted = Some(ByteRead { side: self.side, address, returned: None });
        let byte = self.services.read_byte(self.owners, address)?;
        *self.attempted = Some(ByteRead { side: self.side, address, returned: Some(byte) });
        Ok(byte)
    }
    fn progress(&mut self, progress: CharacterProgress<u64>) { *self.progress = Some(progress); }
}
#[derive(Debug)]
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, input: Input<S::Owners>, entry: Entry,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> {
    Pending(Prepared<S>), Ready(ReturnRequired<S>), Failed(Stopped<S>),
}
pub struct Parts<S: Services> {
    pub(super) services: S, pub(super) input: Input<S::Owners>, pub entry: Entry,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Owners> { &self.input }
}

pub struct ReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> Prepared<S> {
    pub fn from_actual_selected_entry(services: S, entry: Entry, input: Input<S::Owners>) -> Self {
        let captures = Captures { left_cursor: input.left_word,
            right_cursor: input.right_word, ..Captures::default() };
        Self { services, input, entry, captures, phase: Phase::DecodeLeft,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn services(&self) -> &S { &self.services }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, entry: self.entry,
            captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }

    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::ReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase == Phase::ReturnRequired {
            Poll::Ready(ReturnRequired { parts: self.into_parts() })
        } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let c = &mut self.captures;
        match self.phase {
            Phase::DecodeLeft => {
                c.left = None; c.right = None; c.upper_left = None; c.upper_right = None;
                c.difference = None; c.attempted_upper = None;
                let mut memory = LiveMemory { services: &mut self.services,
                    owners: &mut self.input.owners, side: Side::Left,
                    attempted: &mut c.attempted_read, progress: &mut c.decoding };
                c.left = Some(decode_from_memory(&mut memory, &mut c.left_cursor).map_err(Fault::Service)?);
                self.phase = Phase::DecodeRight;
            }
            Phase::DecodeRight => {
                let mut memory = LiveMemory { services: &mut self.services,
                    owners: &mut self.input.owners, side: Side::Right,
                    attempted: &mut c.attempted_read, progress: &mut c.decoding };
                c.right = Some(decode_from_memory(&mut memory, &mut c.right_cursor).map_err(Fault::Service)?);
                self.phase = Phase::Compare;
            }
            Phase::Compare => {
                let left = c.left.ok_or(Fault::MissingState)?;
                let right = c.right.ok_or(Fault::MissingState)?;
                if left == right { c.difference = Some(0); self.phase = Phase::Finish; }
                else if self.entry == Entry::Folded1fbf80 { self.phase = Phase::UpperRight; }
                else {

                    c.difference = Some(if (left as i32) < (right as i32) { -1 } else { 1 });
                    self.phase = Phase::Finish;
                }
            }
            Phase::UpperRight => {
                let call = UpperCall { pc: 0x1fc14a, input_ecx: c.right.ok_or(Fault::MissingState)? };
                c.attempted_upper = Some(call);
                c.upper_right = Some(self.services.uppercase_58a19c(&mut self.input.owners,
                    call.pc, call.input_ecx).map_err(Fault::Service)?);
                self.phase = Phase::UpperLeft;
            }
            Phase::UpperLeft => {
                let call = UpperCall { pc: 0x1fc154, input_ecx: c.left.ok_or(Fault::MissingState)? };
                c.attempted_upper = Some(call);
                let left = self.services.uppercase_58a19c(&mut self.input.owners,
                    call.pc, call.input_ecx).map_err(Fault::Service)?;
                c.upper_left = Some(left);
                let right = c.upper_right.ok_or(Fault::MissingState)?;
                c.difference = Some(folded_order(left, right));
                self.phase = Phase::Finish;
            }
            Phase::Finish => {
                let difference = c.difference.ok_or(Fault::MissingState)?;
                if difference != 0 || c.left.ok_or(Fault::MissingState)? == 0 {
                    c.result = Some(difference); self.phase = Phase::ReturnRequired;
                } else { self.phase = Phase::DecodeLeft; }
            }
            Phase::ReturnRequired => {}
        }
        Ok(())
    }
}
impl<S: Services> ReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { self.parts.entry.return_pc() }
    pub fn required_eax(&self) -> i32 { self.parts.captures.result.unwrap() }
    pub fn services(&self) -> &S { &self.parts.services }
    pub fn captures(&self) -> &Captures { &self.parts.captures }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}

struct BorrowedServices<'a, S: Services> { services: &'a mut S }
impl<'a, S: Services> Services for BorrowedServices<'a, S>
where S::Owners: 'a {
    type Owners = &'a mut S::Owners;
    type Error = S::Error;
    fn read_byte(&mut self, owners: &mut Self::Owners, address: u64)
        -> Result<u8, Self::Error> {
        self.services.read_byte(&mut **owners, address)
    }
    fn uppercase_58a19c(&mut self, owners: &mut Self::Owners,
        actual_call_pc: u32, saved_input_ecx: u32) -> Result<u16, Self::Error> {
        self.services.uppercase_58a19c(&mut **owners, actual_call_pc, saved_input_ecx)
    }
}
pub struct BorrowedFailure<E> {
    pub fault: Fault<E>, pub entry: Entry,
    pub left_word: u64, pub right_word: u64,
    pub captures: Captures, pub phase: Phase,
    pub attempted: Option<Phase>, pub stopped: bool,
}

pub fn compare_values_borrowed<'a, S: Services>(services: &'a mut S,
    owners: &'a mut S::Owners, entry: Entry, left_word: u64, right_word: u64)
    -> Result<i32, BorrowedFailure<S::Error>>
where S::Owners: 'a {
    let mut pending = Prepared::from_actual_selected_entry(BorrowedServices { services },
        entry, Input { owners, left_word, right_word });
    loop {
        match pending.poll(1) {
            Poll::Pending(next) => pending = next,
            Poll::Ready(returned) => return Ok(returned.required_eax()),
            Poll::Failed(Stopped { pending, fault }) => {
                let parts = pending.into_parts();
                return Err(BorrowedFailure { fault, entry: parts.entry,
                    left_word: parts.input.left_word, right_word: parts.input.right_word,
                    captures: parts.captures, phase: parts.phase,
                    attempted: parts.attempted, stopped: parts.stopped });
            }
        }
    }
}
