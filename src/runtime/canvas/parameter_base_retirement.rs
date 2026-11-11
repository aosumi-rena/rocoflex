#[path = "text_record_string_retirement.rs"]
pub mod string_release;
use string_release::{SavedTextFailure, SavedTextRetired, SavedTextServices};

pub trait Services: super::functions::attributes::Services {

    fn base_bind_loaded_receiver(&mut self, parameter: &mut Self::Parameter,
        receiver: u64, frame: &mut Self::Frame) -> Result<(), Self::Error>;
    fn base_read_dword(&mut self, pc: u32, parameter: &Self::Parameter, offset: u32) -> Result<u32, Self::Error>;
    fn base_write_dword(&mut self, pc: u32, parameter: &mut Self::Parameter, offset: u32, bits: u32) -> Result<(), Self::Error>;
    fn base_read_word(&mut self, pc: u32, address: u64) -> Result<u64, Self::Error>;

    fn base_xadd(&mut self, pc: u32, header: u64, frame: &mut Self::Frame, prior: &mut Option<u32>) -> Result<(), Self::Error>;

    fn base_call_free55da4c(&mut self, pc: u32, header: u64, frame: &mut Self::Frame) -> Result<(), Self::Error>;

    fn base_call_free580c24(&mut self, pc: u32, pointer: u64, frame: &mut Self::Frame) -> Result<(), Self::Error>;
}
struct CallRelease<'a, S: Services> {
    services: &'a mut S, frame: &'a mut S::Frame, atomic_pc: u32, free_pc: u32,
}
impl<S: Services> SavedTextServices for CallRelease<'_, S> {
    type Error = S::Error;
    fn lock_xadd_minus_one(&mut self, header: u64, held: &mut Option<u32>) -> Result<(), Self::Error> {
        self.services.base_xadd(self.atomic_pc, header, self.frame, held)
    }
    fn free_saved_header(&mut self, header: u64) -> Result<(), Self::Error> {
        self.services.base_call_free55da4c(self.free_pc, header, self.frame)
    }
}
pub struct Input<S: Services, T> {
    pub services: S, pub parameter: S::Parameter, pub frame: S::Frame, pub retained: T,
    pub actual_image_base: u64, pub actual_saved_rsi: u64, pub actual_initial_rcx: u64,
    pub actual_work_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    AdmitReceiver, StoreDerivedVtable, ReadString, ReleaseString, StoreBaseVtable, InitialCount,
    NewListTrace, ReadListBase, ReadListText, ReleaseListText, IncrementIndex, ReadLoopCount,
    ZeroCount64, ReadBuffer58, FreeBuffer58, ZeroCount54, ReadBuffer48, FreeBuffer48, NativeTailRequired,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ListEntry {
    pub index: u64, pub base: Option<u64>, pub text: Option<u64>, pub release: Option<SavedTextRetired>,
}
#[derive(Debug, Default)]
pub struct Captures {
    pub receiver_admission_returned: bool,
    pub derived_store_returned: bool, pub strings: [Option<u64>; 3], pub releases: [Option<SavedTextRetired>; 3],
    pub base_store_returned: bool, pub initial_count: Option<u32>, pub last_loop_count: Option<u32>,
    pub list: Vec<ListEntry>, pub count64_zero_returned: bool, pub buffer58: Option<u64>, pub free58_returned: bool,
    pub count54_zero_returned: bool, pub buffer48: Option<u64>, pub free48_returned: bool,
}
pub enum Fault<E> { Service(E), Release(SavedTextFailure<E>), MissingState, TraceAllocation(std::collections::TryReserveError), AlreadyStopped }
pub struct Prepared<S: Services, T> {
    input: Input<S, T>, captures: Captures, phase: Phase, string: usize,
    index: u64, attempted: Option<(Phase, u64)>, stopped: bool,
}
pub struct Rejected<S: Services, T> { pub input: Input<S, T> }
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(NativeTailRequired<S, T>), Failed(Stopped<S, T>) }
pub struct NativeTailRequired<S: Services, T> { parts: Parts<S, T> }
pub struct Parts<S: Services, T> {

    pub(super) input: Input<S, T>, pub captures: Captures, pub phase: Phase, pub string: usize,
    pub index: u64, pub attempted: Option<(Phase, u64)>, pub stopped: bool,
}
impl<S: Services, T> Parts<S, T> {
    pub fn input(&self) -> &Input<S, T> { &self.input }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailRequirement {
    pub next_pc: u32, pub epilogue_pc: u32, pub indirect_pc: u32,
    pub receiver_rcx: u64, pub addressed_slot: u64, pub required_entry_rsp: u64,
}
impl<S: Services, T> NativeTailRequired<S, T> {

    pub fn requirement(&self) -> TailRequirement {
        TailRequirement { next_pc: 0x2b3b31, epilogue_pc: 0x2b3b38, indirect_pc: 0x2b3b3f,
            receiver_rcx: self.parts.input.actual_saved_rsi.wrapping_add(0x18),
            addressed_slot: self.parts.input.actual_image_base.wrapping_add(0x77f8f0),
            required_entry_rsp: self.parts.input.actual_work_rsp.wrapping_add(0x38) }
    }
    pub fn into_parts(self) -> Parts<S, T> { self.parts }
}
impl<S: Services, T> Prepared<S, T> {

    pub fn from_actual_2b3a5a(input: Input<S, T>) -> Result<Self, Rejected<S, T>> {
        if input.actual_initial_rcx != input.actual_saved_rsi { return Err(Rejected { input }); }
        Ok(Self { input, captures: Captures::default(), phase: Phase::AdmitReceiver,
            string: 0, index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { input: self.input, captures: self.captures, phase: self.phase, string: self.string,
            index: self.index, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::NativeTailRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase != Phase::NativeTailRequired { return Poll::Pending(self); }
        Poll::Ready(NativeTailRequired { parts: self.into_parts() })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let i = &mut self.input;
        let c = &mut self.captures;
        match self.phase {
            Phase::AdmitReceiver => {
                i.services.base_bind_loaded_receiver(&mut i.parameter, i.actual_saved_rsi, &mut i.frame).map_err(Fault::Service)?;
                c.receiver_admission_returned = true;
                self.phase = Phase::StoreDerivedVtable;
            }
            Phase::StoreDerivedVtable => {
                i.services.write_parameter_word(&mut i.parameter, 0, i.actual_image_base.wrapping_add(0x6270e8)).map_err(Fault::Service)?;
                c.derived_store_returned = true; self.phase = Phase::ReadString;
            }
            Phase::ReadString => {
                if self.string >= 3 { return Err(Fault::MissingState); }
                c.strings[self.string] = Some(i.services.read_parameter_word(&i.parameter, [0x78, 0x70, 0x68][self.string]).map_err(Fault::Service)?);
                self.phase = Phase::ReleaseString;
            }
            Phase::ReleaseString => {
                let mut adapter = CallRelease { services: &mut i.services, frame: &mut i.frame,
                    atomic_pc: [0x2b3a7d, 0x2b3a9c, 0x2b3abb][self.string], free_pc: [0x2b3a85, 0x2b3aa4, 0x2b3ac3][self.string] };
                c.releases[self.string] = Some(string_release::retire_saved_text_borrowed(&mut adapter,
                    c.strings[self.string].ok_or(Fault::MissingState)?, i.actual_image_base).map_err(Fault::Release)?);
                self.string += 1; self.phase = if self.string == 3 { Phase::StoreBaseVtable } else { Phase::ReadString };
            }
            Phase::StoreBaseVtable => {
                i.services.write_parameter_word(&mut i.parameter, 0, i.actual_image_base.wrapping_add(0x60ef98)).map_err(Fault::Service)?;
                c.base_store_returned = true; self.phase = Phase::InitialCount;
            }
            Phase::InitialCount => {
                let count = i.services.base_read_dword(0x2b3ad2, &i.parameter, 0x64).map_err(Fault::Service)?;
                c.initial_count = Some(count);
                self.phase = if (count as i32) <= 0 { Phase::ZeroCount64 } else { Phase::NewListTrace };
            }
            Phase::NewListTrace => {

                c.list.try_reserve(1).map_err(Fault::TraceAllocation)?;
                c.list.push(ListEntry { index: self.index, ..ListEntry::default() });
                self.phase = Phase::ReadListBase;
            }
            Phase::ReadListBase => {
                let base = i.services.read_parameter_word(&i.parameter, 0x58).map_err(Fault::Service)?;
                c.list.last_mut().ok_or(Fault::MissingState)?.base = Some(base); self.phase = Phase::ReadListText;
            }
            Phase::ReadListText => {
                let step = c.list.last_mut().ok_or(Fault::MissingState)?;
                let address = step.base.ok_or(Fault::MissingState)?.wrapping_add(self.index.wrapping_mul(8));
                step.text = Some(i.services.base_read_word(0x2b3af0, address).map_err(Fault::Service)?);
                self.phase = Phase::ReleaseListText;
            }
            Phase::ReleaseListText => {
                let step = c.list.last_mut().ok_or(Fault::MissingState)?;
                let mut adapter = CallRelease { services: &mut i.services, frame: &mut i.frame, atomic_pc: 0x2b3b02, free_pc: 0x2b3b0a };
                step.release = Some(string_release::retire_saved_text_borrowed(&mut adapter,
                    step.text.ok_or(Fault::MissingState)?, i.actual_image_base).map_err(Fault::Release)?);
                self.phase = Phase::IncrementIndex;
            }
            Phase::IncrementIndex => { self.index = self.index.wrapping_add(1); self.phase = Phase::ReadLoopCount; }
            Phase::ReadLoopCount => {
                let count = i.services.base_read_dword(0x2b3ae3, &i.parameter, 0x64).map_err(Fault::Service)?;
                c.last_loop_count = Some(count);
                self.phase = if (self.index as i64) >= (count as i32 as i64) { Phase::ZeroCount64 } else { Phase::NewListTrace };
            }
            Phase::ZeroCount64 => {
                i.services.base_write_dword(0x2b3b11, &mut i.parameter, 0x64, 0).map_err(Fault::Service)?;
                c.count64_zero_returned = true; self.phase = Phase::ReadBuffer58;
            }
            Phase::ReadBuffer58 => {
                c.buffer58 = Some(i.services.read_parameter_word(&i.parameter, 0x58).map_err(Fault::Service)?); self.phase = Phase::FreeBuffer58;
            }
            Phase::FreeBuffer58 => {
                i.services.base_call_free580c24(0x2b3b1c, c.buffer58.ok_or(Fault::MissingState)?, &mut i.frame).map_err(Fault::Service)?;
                c.free58_returned = true; self.phase = Phase::ZeroCount54;
            }
            Phase::ZeroCount54 => {
                i.services.base_write_dword(0x2b3b21, &mut i.parameter, 0x54, 0).map_err(Fault::Service)?;
                c.count54_zero_returned = true; self.phase = Phase::ReadBuffer48;
            }
            Phase::ReadBuffer48 => {
                c.buffer48 = Some(i.services.read_parameter_word(&i.parameter, 0x48).map_err(Fault::Service)?); self.phase = Phase::FreeBuffer48;
            }
            Phase::FreeBuffer48 => {
                i.services.base_call_free580c24(0x2b3b2c, c.buffer48.ok_or(Fault::MissingState)?, &mut i.frame).map_err(Fault::Service)?;
                c.free48_returned = true; self.phase = Phase::NativeTailRequired;
            }
            Phase::NativeTailRequired => {}
        }
        Ok(())
    }
}
