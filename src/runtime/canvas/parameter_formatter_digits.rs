pub use super::super::super::super::super::super::base_retirement::string_release as release;
#[path = "parameter_formatter_digits_steps.rs"]
mod steps;
#[path = "parameter_number_construction.rs"]
pub mod number_construction;
#[path = "parameter_number_call.rs"]
pub mod number_call;

pub struct Allocation<A> { pub base: u64, pub owner: A }
pub trait Services: Sized {
    type Storage;
    type Temporary;
    type Allocation;
    type Error;
    fn read_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_byte(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_byte(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;

    fn widen_cvtss2sd(&mut self, owners: &mut Owners<Self>, pc: u32, bits: u32) -> Result<u64, Self::Error>;

    fn format1cae80(&mut self, owners: &mut Owners<Self>, request: FormatRequest) -> Result<(), Self::Error>;

    fn allocate55da80(&mut self, owners: &mut Owners<Self>, pc: u32, bytes: u64) -> Result<(), Self::Error>;

    fn xchg_ref_zero(&mut self, owners: &mut Owners<Self>, pc: u32, header: u64, prior: &mut Option<u32>) -> Result<(), Self::Error>;
    fn lock_increment(&mut self, owners: &mut Owners<Self>, pc: u32, header: u64,
        actual: &mut Option<AtomicIncrement>) -> Result<(), Self::Error>;

    fn copy564f30(&mut self, owners: &mut Owners<Self>, pc: u32, destination: u64, source: u64, bytes: u64) -> Result<(), Self::Error>;
    fn xadd_minus_one(&mut self, owners: &mut Owners<Self>, pc: u32, header: u64, prior: &mut Option<u32>) -> Result<(), Self::Error>;

    fn call55da4c(&mut self, owners: &mut Owners<Self>, pc: u32, header: u64) -> Result<(), Self::Error>;
}

pub struct Owners<S: Services> { pub storage: S::Storage, pub temporary: Option<S::Temporary>, pub allocation: Option<Allocation<S::Allocation>> }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody {
    pub pc: u32, pub actual_image_base: u64, pub rbp: u64,
    pub function_rcx: u64, pub output_rdx: u64, pub value_r8: u64, pub limit_r9: u64,
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatRequest {
    pub pc: u32, pub destination_rcx: u64, pub value_xmm1_f64_bits: u64,
    pub digits_r8d: u32, pub fourth_r9d: u32, pub unchanged_rdx: u64,
}
fn format_request(actual: ActualBody, captures: &Captures) -> Option<FormatRequest> {
    Some(FormatRequest { pc: 0x2b6d61, destination_rcx: actual.rbp.wrapping_sub(0x10),
        value_xmm1_f64_bits: captures.widened_bits?, digits_r8d: captures.digits?, fourth_r9d: 0,
        unchanged_rdx: actual.output_rdx })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicIncrement { pub old: u32, pub new: u32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan { pub pointer: u64, pub remaining: u32, pub returned_byte: Option<u8> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    ReadLimit, ReadValue, ReadDigits, Widen, Format, BranchLimit, ReadScanStart,
    Scan, ReadWhole, StoreWhole, RetainWhole, CheckTruncated, ReadFirst,
    SaveCopySource, Allocate, XchgReference, StoreCapacity, StoreNewOutput,
    ReloadCopySource, Copy, Terminate, StoreStatic, ReadCleanup, Release, NativeReturnRequired,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub limit: Option<u32>, pub value_bits: Option<u32>, pub digits: Option<u32>, pub widened_bits: Option<u64>,
    pub format_request: Option<FormatRequest>, pub format_returned: bool,
    pub number_call: Option<number_call::History>,
    pub scan_start: Option<u64>, pub last_scan: Option<Scan>, pub selected_text: Option<u64>,
    pub whole_store_pc: Option<u32>, pub whole_retain_pc: Option<u32>, pub output_store_returned: bool,
    pub retained: Option<AtomicIncrement>, pub first_byte: Option<u8>,
    pub length: Option<u64>, pub rounded: Option<u64>, pub capacity: Option<u64>,
    pub copy_source_save_returned: bool, pub allocation_bytes: Option<u64>, pub allocation_returned: bool,
    pub allocation_base: Option<u64>,
    pub allocation_prior: Option<u32>, pub capacity_store_returned: bool,
    pub copy_source: Option<u64>, pub copy_returned: bool, pub terminator_returned: bool,
    pub cleanup_text: Option<u64>, pub cleanup: Option<release::SavedTextRetired>,
}
pub enum Fault<E> { Service(E), Release(release::SavedTextFailure<E>), MissingState, IncrementBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, cursor: u64, remaining: u32, stopped: bool,
}
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> {
    pub services: S, pub owners: Owners<S>, pub actual: ActualBody, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub cursor: u64, pub remaining: u32, pub stopped: bool,
}

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x2b6e66 }
    pub fn saved_output_this(&self) -> u64 { self.parts.actual.output_rdx }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    fn record_format_return(&mut self) { self.captures.format_returned = true; self.phase = Phase::BranchLimit; }

    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x2b6d47 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, temporary: None, allocation: None },
            actual: input.actual, captures: Captures::default(), phase: Phase::ReadLimit, attempted: None,
            cursor: 0, remaining: 0, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, cursor: self.cursor,
        remaining: self.remaining, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
}
