#[path = "source_name_install.rs"]
mod source_text;
use source_text::{CharacterMemory, decode_from_memory};
pub use source_text::CharacterProgress;
#[path = "parameter_number_text_copy_return.rs"]
pub mod native_return;
#[path = "parameter_number_text_copy_entry.rs"]
pub mod native_entry;

pub trait Services: Sized {
    type Storage;
    type Error;
    fn read_byte(&mut self, storage: &mut Self::Storage, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn read_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_byte(&mut self, storage: &mut Self::Storage, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub output_slot_rcx: u64, pub source_rdx: u64, pub count_r8d: u32 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { InitialBudget, Decode, ReadOutput, AdvanceOutput, WriteByte, AfterCharacter, ReadFinalOutput, Terminate, NativeRestoreRequired }
#[derive(Clone, Copy, Debug)]
pub struct ByteRead { pub pc: u32, pub address: u64, pub returned: Option<u8> }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub native_entry: Option<native_entry::History>,
    pub source_cursor: u64, pub remaining_r8d: u32, pub character: Option<u32>,
    pub decoding: Option<CharacterProgress<u64>>, pub reads: [Option<ByteRead>; 4],
    pub encoded: [u8; 4], pub width: u8, pub byte_index: u8,
    pub output_pointer: Option<u64>, pub advanced_pointer: Option<u64>,
    pub pointer_store_returned: bool, pub byte_store_returned: bool,
    pub final_pointer: Option<u64>, pub nul_store_returned: bool,
}
#[derive(Debug)]
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, storage: S::Storage, actual: ActualBody,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeRestoreRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) storage: S::Storage,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn storage(&self) -> &S::Storage { &self.storage }
}

pub struct NativeRestoreRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeRestoreRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb412 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
struct DecodeMemory<'a, S: Services> {
    services: &'a mut S, storage: &'a mut S::Storage,
    reads: &'a mut [Option<ByteRead>; 4], ordinal: usize,
    progress: &'a mut Option<CharacterProgress<u64>>,
}
impl<S: Services> CharacterMemory for DecodeMemory<'_, S> {
    type Address = u64;
    type Error = Fault<S::Error>;
    fn read_byte(&mut self, address: u64) -> Result<u8, Self::Error> {
        let pcs = [0x1fb24e, 0x1fb2a4, 0x1fb3ab, 0x1fb3d7];
        let pc = *pcs.get(self.ordinal).ok_or(Fault::MissingState)?;
        let slot = self.reads.get_mut(self.ordinal).ok_or(Fault::MissingState)?;
        *slot = Some(ByteRead { pc, address, returned: None }); self.ordinal += 1;
        let byte = self.services.read_byte(self.storage, pc, address).map_err(Fault::Service)?;
        *slot = Some(ByteRead { pc, address, returned: Some(byte) }); Ok(byte)
    }
    fn progress(&mut self, progress: CharacterProgress<u64>) { *self.progress = Some(progress); }
}

fn encoding(character: u32) -> ([u8; 4], u8) {
    if character < 0x80 { return ([character as u8, 0, 0, 0], 1); }
    let mut continuation = if character < 0x10000 { 2_u32 } else { 3_u32 };
    if character < 0x800 { continuation = 1; }
    let prefix = 0xff_u32.wrapping_shl(continuation ^ 7);
    let mut bytes = [0_u8; 4];
    bytes[0] = ((character >> (6 * continuation)) | prefix) as u8;
    bytes[1] = (((character >> (6 * (continuation - 1))) & 0x3f) | 0x80) as u8;
    if character >= 0x800 {
        bytes[2] = (((character >> (6 * continuation - 12)) & 0x3f) | 0x80) as u8;
        if continuation == 3 { bytes[3] = ((character & 0x3f) | 0x80) as u8; }
    }
    (bytes, continuation as u8 + 1)
}
fn output_pcs(width: u8, index: u8) -> Option<[u32; 3]> {
    if width == 1 && index == 0 || width == 4 && index == 3 { Some([0x1fb230, 0x1fb237, 0x1fb23a]) }
    else if width >= 2 && width <= 4 { match index {
        0 => Some([0x1fb32d, 0x1fb334, 0x1fb337]),
        1 => Some([0x1fb34e, 0x1fb355, 0x1fb358]),
        2 if width >= 3 => Some([0x1fb37e, 0x1fb385, 0x1fb388]),
        _ => None,
    } } else { None }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fb214 { return Err(Rejected { input }); }
        let captures = Captures { source_cursor: input.actual.source_rdx,
            remaining_r8d: input.actual.count_r8d, ..Captures::default() };
        Ok(Self { services: input.services, storage: input.storage, actual: input.actual, captures,
            phase: Phase::InitialBudget, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, storage: self.storage,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }

    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeRestoreRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeRestoreRequired { Poll::Ready(NativeRestoreRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let c = &mut self.captures;
        match self.phase {
            Phase::InitialBudget => {
                if (c.remaining_r8d as i32) < 2 { self.phase = Phase::ReadFinalOutput; }
                else { c.remaining_r8d = c.remaining_r8d.wrapping_add(1); self.phase = Phase::Decode; }
            }
            Phase::Decode => {
                c.character = None; c.decoding = None; c.reads = [None; 4];
                let mut memory = DecodeMemory { services: &mut self.services, storage: &mut self.storage,
                    reads: &mut c.reads, ordinal: 0, progress: &mut c.decoding };
                let character = decode_from_memory(&mut memory, &mut c.source_cursor)?;
                c.character = Some(character);
                if character == 0 { self.phase = Phase::ReadFinalOutput; }
                else {
                    let (bytes, width) = encoding(character); c.encoded = bytes; c.width = width; c.byte_index = 0;
                    self.phase = Phase::ReadOutput;
                }
            }
            Phase::ReadOutput => {
                let pcs = output_pcs(c.width, c.byte_index).ok_or(Fault::MissingState)?;
                c.output_pointer = None; c.advanced_pointer = None;
                c.pointer_store_returned = false; c.byte_store_returned = false;
                c.output_pointer = Some(self.services.read_word(&mut self.storage, pcs[0], self.actual.output_slot_rcx).map_err(Fault::Service)?);
                self.phase = Phase::AdvanceOutput;
            }
            Phase::AdvanceOutput => {
                let pcs = output_pcs(c.width, c.byte_index).ok_or(Fault::MissingState)?;
                let next = c.output_pointer.ok_or(Fault::MissingState)?.wrapping_add(1);
                c.advanced_pointer = Some(next);
                self.services.write_word(&mut self.storage, pcs[1], self.actual.output_slot_rcx, next).map_err(Fault::Service)?;
                c.pointer_store_returned = true; self.phase = Phase::WriteByte;
            }
            Phase::WriteByte => {
                let pcs = output_pcs(c.width, c.byte_index).ok_or(Fault::MissingState)?;
                let byte = *c.encoded.get(usize::from(c.byte_index)).ok_or(Fault::MissingState)?;
                self.services.write_byte(&mut self.storage, pcs[2], c.output_pointer.ok_or(Fault::MissingState)?, byte).map_err(Fault::Service)?;
                c.byte_store_returned = true; c.byte_index += 1;
                self.phase = if c.byte_index == c.width { Phase::AfterCharacter } else { Phase::ReadOutput };
            }
            Phase::AfterCharacter => {
                c.remaining_r8d = c.remaining_r8d.wrapping_sub(1);
                self.phase = if (c.remaining_r8d as i32) <= 2 { Phase::ReadFinalOutput } else { Phase::Decode };
            }
            Phase::ReadFinalOutput => {
                c.final_pointer = Some(self.services.read_word(&mut self.storage, 0x1fb40c, self.actual.output_slot_rcx).map_err(Fault::Service)?);
                self.phase = Phase::Terminate;
            }
            Phase::Terminate => {
                self.services.write_byte(&mut self.storage, 0x1fb40f, c.final_pointer.ok_or(Fault::MissingState)?, 0).map_err(Fault::Service)?;
                c.nul_store_returned = true; self.phase = Phase::NativeRestoreRequired;
            }
            Phase::NativeRestoreRequired => {}
        }
        Ok(())
    }
}
