use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryContextEntry {
    pub actual_module_base: u64,
    pub actual_entry_rsp: u64,
    pub actual_entry_rbx: u64,
    pub actual_entry_rsi: u64,
    pub actual_entry_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryStackWord { pub actual_address: u64, pub actual_word: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryIndexRead { Initial, AfterNullProbe, AfterAllocation, AfterFailedSet }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryContextCallKind {
    Probe, MarkMinusOne, Allocate, SetAllocated, ClearFailedAllocation,
    ClearFailedSet, ReleaseFailure, Initialize, ReleaseNullAfterInitialize,
    ProtectedRestore,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryContextCall {
    pub kind: SourceCrtQueryContextCallKind,
    pub source_pc: u64,
    pub actual_target: u64,

    pub source_rcx: Option<u64>,
    pub source_rdx: Option<u64>,
    pub actual_caller_rsp: u64,
    pub actual_callee_entry_rsp: u64,

    pub required_actual_resume: u64,
    pub required_actual_resume_rsp: u64,
    pub actual_extra_push: Option<SourceCrtQueryStackWord>,

    pub required_rbx: u64,
    pub required_rsi: u64,
    pub required_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryContextRequest {

    RequiredEntry9c5dfa { entry: SourceCrtQueryContextEntry,
        actual_call_pc: u64, actual_target: u64, actual_callee_entry_rsp: u64,
        required_actual_resume: u64, required_actual_resume_rsp: u64 },
    ReadIndex32 { role: SourceCrtQueryIndexRead, source_pc: u64, actual_address: u64 },
    RequiredCall(SourceCrtQueryContextCall),
    RequiredEpilogue { source_pc: u64, actual_before_rsp: u64,
        required_ret_frame: SourceCrtNativeRetFrame },

    RequiredFailure57c878 { source_pc: u64, actual_target: u64, actual_caller_rsp: u64 },
    RequiredInt3 { actual_pc: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryContextReceipt {
    EntryContinued { entry: SourceCrtQueryContextEntry, actual_frame: SourceCrtNativeRetFrame,
        actual_rdi: u64, actual_saved_words: [SourceCrtQueryStackWord; 4] },
    IndexRead { role: SourceCrtQueryIndexRead, source_pc: u64, actual_address: u64, value: u32 },
    CallContinued { request: SourceCrtQueryContextCall, actual_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_rsi: u64, actual_rdi: u64,
        actual_extra_push: Option<SourceCrtQueryStackWord> },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_rsi: u64, actual_rdi: u64,
        actual_reads: [SourceCrtNativeRetStackRead; 3] },

    FailureContinued { actual_frame: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryContextCompletion {
    entry: SourceCrtQueryContextEntry,
    actual_context_word: u64,
    actual_ret_frame: SourceCrtNativeRetFrame,
    allocation_return: Option<u64>,
}
impl SourceCrtQueryContextCompletion {
    pub fn entry(&self) -> SourceCrtQueryContextEntry { self.entry }
    pub fn actual_context_word(&self) -> u64 { self.actual_context_word }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
    pub fn allocation_return(&self) -> Option<u64> { self.allocation_return }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryContextProgress {
    Request(SourceCrtQueryContextRequest),

    BeforeRet(SourceCrtQueryContextCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryContextError {
    pub pending: SourceCrtQueryContextProgress,
    pub actual: SourceCrtQueryContextReceipt,
}
pub struct SourceCrtQueryContext58db7c {
    entry: SourceCrtQueryContextEntry,
    saved_word: Option<u32>,
    index: Option<u32>,
    allocated: Option<u64>,
    rdi: u64,
    progress: SourceCrtQueryContextProgress,
    stopped: bool,
}
impl SourceCrtQueryContext58db7c {
    pub fn begin(entry: SourceCrtQueryContextEntry) -> Self {
        let base = entry.actual_module_base;
        Self { entry, saved_word: None, index: None, allocated: None, rdi: entry.actual_entry_rdi,
            progress: SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredEntry9c5dfa {
                entry, actual_call_pc: base.wrapping_add(0x58db8c), actual_target: base.wrapping_add(0x9c5dfa),
                actual_callee_entry_rsp: entry.actual_entry_rsp.wrapping_sub(0x38),
                required_actual_resume: base.wrapping_add(0x58db91),
                required_actual_resume_rsp: entry.actual_entry_rsp.wrapping_sub(0x28) }), stopped: false }
    }
    pub fn entry(&self) -> SourceCrtQueryContextEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryContextProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtQueryContextRequest> {
        if self.stopped { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryContextRequest> {
        match self.progress { SourceCrtQueryContextProgress::Request(request) => Some(request), _ => None }
    }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryContextReceipt)
        -> Result<SourceCrtQueryContextProgress, SourceCrtQueryContextError> {
        self.stopped = true;
        let reject = SourceCrtQueryContextError { pending: self.progress, actual };
        let base = self.entry.actual_module_base;
        match (self.progress, actual) {
            (SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredEntry9c5dfa {
                entry, required_actual_resume, required_actual_resume_rsp, .. }),
                SourceCrtQueryContextReceipt::EntryContinued { entry: observed, actual_frame, actual_rdi, actual_saved_words })
                if entry == observed && actual_frame.actual_rip == required_actual_resume
                    && actual_frame.actual_rsp == required_actual_resume_rsp
                    && actual_saved_words == self.required_entry_words() => {
                self.saved_word = Some(actual_frame.actual_rax as u32); self.rdi = actual_rdi;
                self.read_index(SourceCrtQueryIndexRead::Initial, 0x58db91);
            }
            (SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::ReadIndex32 { role, source_pc, actual_address }),
                SourceCrtQueryContextReceipt::IndexRead { role: observed_role, source_pc: pc, actual_address: address, value })
                if role == observed_role && pc == source_pc && address == actual_address => {
                self.index = Some(value);
                match role {
                    SourceCrtQueryIndexRead::Initial if value != u32::MAX =>
                        self.call(SourceCrtQueryContextCallKind::Probe, 0x58dba0, 0x58e5c0, Some(u64::from(value)), None, 0x58dba5),
                    SourceCrtQueryIndexRead::Initial | SourceCrtQueryIndexRead::AfterNullProbe =>
                        self.call(SourceCrtQueryContextCallKind::MarkMinusOne, 0x58dbc1, 0x58e5c8,
                            Some(u64::from(value)), Some(u64::MAX), 0x58dbc6),
                    SourceCrtQueryIndexRead::AfterAllocation => {
                        let Some(allocated) = self.allocated else { return Err(reject); };
                        self.rdi = allocated;
                        if allocated == 0 { self.call(SourceCrtQueryContextCallKind::ClearFailedAllocation,
                            0x58dbee, 0x58e5c8, Some(u64::from(value)), Some(0), 0x58dbf3); }
                        else { self.call(SourceCrtQueryContextCallKind::SetAllocated, 0x58dbff, 0x58e5c8,
                            Some(u64::from(value)), Some(allocated), 0x58dc04); }
                    }
                    SourceCrtQueryIndexRead::AfterFailedSet => self.call(SourceCrtQueryContextCallKind::ClearFailedSet,
                        0x58dc10, 0x58e5c8, Some(u64::from(value)), Some(0), 0x58dc15),
                }
            }
            (SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredCall(expected)),
                SourceCrtQueryContextReceipt::CallContinued { request, actual_frame, actual_rbx, actual_rsi,
                    actual_rdi, actual_extra_push })
                if request == expected && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.required_actual_resume_rsp
                    && (expected.kind == SourceCrtQueryContextCallKind::ProtectedRestore
                        || (actual_rbx == expected.required_rbx && actual_rsi == expected.required_rsi
                            && actual_rdi == expected.required_rdi))
                    && actual_extra_push == expected.actual_extra_push => {
                let value = actual_frame.actual_rax;
                match expected.kind {
                    SourceCrtQueryContextCallKind::Probe => {
                        self.rdi = if value == u64::MAX { 0 } else { value };
                        if value == 0 { self.read_index(SourceCrtQueryIndexRead::AfterNullProbe, 0x58dbb7); }
                        else { self.restore(); }
                    }
                    SourceCrtQueryContextCallKind::MarkMinusOne => {
                        if value as u32 == 0 { self.rdi = 0; self.restore(); }
                        else { self.call(SourceCrtQueryContextCallKind::Allocate, 0x58dbd9, 0x58f074, Some(1), Some(0x3c8), 0x58dbde); }
                    }
                    SourceCrtQueryContextCallKind::Allocate => {
                        self.allocated = Some(value); self.read_index(SourceCrtQueryIndexRead::AfterAllocation, 0x58dbde);
                    }
                    SourceCrtQueryContextCallKind::SetAllocated => {
                        if value as u32 == 0 { self.read_index(SourceCrtQueryIndexRead::AfterFailedSet, 0x58dc08); }
                        else { self.call(SourceCrtQueryContextCallKind::Initialize, 0x58dc1d, 0x58df48, Some(self.rdi), None, 0x58dc22); }
                    }
                    SourceCrtQueryContextCallKind::ClearFailedAllocation | SourceCrtQueryContextCallKind::ClearFailedSet => {
                        self.call(SourceCrtQueryContextCallKind::ReleaseFailure, 0x58dbf5, 0x58d910, Some(self.rdi), None, 0x58dbfa);
                    }
                    SourceCrtQueryContextCallKind::ReleaseFailure => { self.rdi = 0; self.restore(); }
                    SourceCrtQueryContextCallKind::Initialize => self.call(SourceCrtQueryContextCallKind::ReleaseNullAfterInitialize,
                        0x58dc24, 0x58d910, Some(0), None, 0x58dc29),
                    SourceCrtQueryContextCallKind::ReleaseNullAfterInitialize => self.restore(),
                    SourceCrtQueryContextCallKind::ProtectedRestore => {
                        self.rdi = actual_rdi;
                        if self.rdi == 0 { self.next(SourceCrtQueryContextRequest::RequiredFailure57c878 {
                            source_pc: base.wrapping_add(0x58dc49), actual_target: base.wrapping_add(0x57c878),
                            actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28) }); }
                        else { self.next(SourceCrtQueryContextRequest::RequiredEpilogue {
                            source_pc: base.wrapping_add(0x58dc36), actual_before_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
                            required_ret_frame: SourceCrtNativeRetFrame { actual_rip: base.wrapping_add(0x58dc48),
                                actual_rsp: self.entry.actual_entry_rsp, actual_rax: self.rdi } }); }
                    }
                }
            }
            (SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredEpilogue { required_ret_frame, .. }),
                SourceCrtQueryContextReceipt::Epilogue { actual_ret_frame, actual_rbx, actual_rsi, actual_rdi, actual_reads })
                if actual_ret_frame == required_ret_frame && actual_rbx == self.entry.actual_entry_rbx
                    && actual_rsi == self.entry.actual_entry_rsi && actual_rdi == self.entry.actual_entry_rdi
                    && actual_reads == self.required_epilogue_reads() => {
                self.progress = SourceCrtQueryContextProgress::BeforeRet(SourceCrtQueryContextCompletion {
                    entry: self.entry, actual_context_word: self.rdi, actual_ret_frame, allocation_return: self.allocated });
            }
            (SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredFailure57c878 { actual_caller_rsp, .. }),
                SourceCrtQueryContextReceipt::FailureContinued { actual_frame })
                if actual_frame.actual_rip == base.wrapping_add(0x58dc4e) && actual_frame.actual_rsp == actual_caller_rsp => {
                self.next(SourceCrtQueryContextRequest::RequiredInt3 { actual_pc: base.wrapping_add(0x58dc4e) });
            }
            _ => return Err(reject),
        }
        self.stopped = false;
        Ok(self.progress)
    }
    fn next(&mut self, request: SourceCrtQueryContextRequest) { self.progress = SourceCrtQueryContextProgress::Request(request); }
    fn read_index(&mut self, role: SourceCrtQueryIndexRead, pc: u64) {
        self.next(SourceCrtQueryContextRequest::ReadIndex32 { role,
            source_pc: self.entry.actual_module_base.wrapping_add(pc),
            actual_address: self.entry.actual_module_base.wrapping_add(0x8364c0) });
    }
    fn call(&mut self, kind: SourceCrtQueryContextCallKind, pc: u64, target: u64,
        rcx: Option<u64>, rdx: Option<u64>, resume: u64) {
        let Some(saved_word) = self.saved_word else { self.stopped = true; return; };
        let body_rsp = self.entry.actual_entry_rsp.wrapping_sub(0x28);
        let extra = if kind == SourceCrtQueryContextCallKind::ProtectedRestore {
            Some(SourceCrtQueryStackWord { actual_address: body_rsp.wrapping_sub(8), actual_word: u64::from(saved_word) })
        } else { None };
        let caller_rsp = if extra.is_some() { body_rsp.wrapping_sub(8) } else { body_rsp };
        self.next(SourceCrtQueryContextRequest::RequiredCall(SourceCrtQueryContextCall {
            kind, source_pc: self.entry.actual_module_base.wrapping_add(pc),
            actual_target: self.entry.actual_module_base.wrapping_add(target), source_rcx: rcx, source_rdx: rdx,
            actual_caller_rsp: caller_rsp, actual_callee_entry_rsp: caller_rsp.wrapping_sub(8),
            required_actual_resume: self.entry.actual_module_base.wrapping_add(resume), required_actual_resume_rsp: body_rsp,
            actual_extra_push: extra, required_rbx: u64::from(saved_word), required_rsi: 0, required_rdi: self.rdi }));
    }
    fn restore(&mut self) {
        let Some(saved_word) = self.saved_word else { self.stopped = true; return; };
        self.call(SourceCrtQueryContextCallKind::ProtectedRestore, 0x58dc2c, 0xbf811d, Some(u64::from(saved_word)), None, 0x58dc31);
    }
    fn required_entry_words(&self) -> [SourceCrtQueryStackWord; 4] {
        let e = self.entry;
        [SourceCrtQueryStackWord { actual_address: e.actual_entry_rsp.wrapping_add(8), actual_word: e.actual_entry_rbx },
         SourceCrtQueryStackWord { actual_address: e.actual_entry_rsp.wrapping_add(0x10), actual_word: e.actual_entry_rsi },
         SourceCrtQueryStackWord { actual_address: e.actual_entry_rsp.wrapping_sub(8), actual_word: e.actual_entry_rdi },
         SourceCrtQueryStackWord { actual_address: e.actual_entry_rsp.wrapping_sub(0x30), actual_word: e.actual_entry_rdi }]
    }
    fn required_epilogue_reads(&self) -> [SourceCrtNativeRetStackRead; 3] {
        let words = self.required_entry_words();
        [SourceCrtNativeRetStackRead { actual_address: words[0].actual_address, actual_word: words[0].actual_word },
         SourceCrtNativeRetStackRead { actual_address: words[1].actual_address, actual_word: words[1].actual_word },
         SourceCrtNativeRetStackRead { actual_address: words[2].actual_address, actual_word: words[2].actual_word }]
    }
}
