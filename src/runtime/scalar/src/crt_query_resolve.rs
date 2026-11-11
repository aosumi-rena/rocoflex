use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};
use super::crt_query_context::SourceCrtQueryStackWord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryResolveEntry {
    pub actual_module_base: u64,
    pub actual_entry_rsp: u64,
    pub actual_entry_rax: u64,
    pub actual_entry_rbx: u64,
    pub actual_entry_rdi: u64,
    pub actual_entry_rcx: u64,
    pub actual_entry_rdx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryResolveCallKind { FreshContext58db7c, Addressed57a7a8, Update59426c, Addressed57a7c4 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryResolveCall {
    pub kind: SourceCrtQueryResolveCallKind,
    pub source_pc: u64,
    pub actual_target: u64,
    pub source_rcx: Option<u64>,
    pub source_rdx: Option<u64>,
    pub actual_caller_rsp: u64,
    pub required_actual_resume: u64,
    pub required_saved_rdi: Option<u64>,
    pub required_saved_rbx: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryResolveRequest {
    RequiredPrologue { entry: SourceCrtQueryResolveEntry, required_body_rip: u64 },
    RequiredCall(SourceCrtQueryResolveCall),
    ReadFlags32 { source_pc: u64, actual_address: u64 },
    ReadMask32 { source_pc: u64, actual_address: u64 },
    ReadSlot64 { source_pc: u64, actual_address: u64 },
    ReadGlobal64 { source_pc: u64, actual_address: u64 },
    RequiredEpilogue { source_pc: u64, required_ret_frame: SourceCrtNativeRetFrame },
    RequiredFailure57c878 { source_pc: u64, actual_target: u64, actual_caller_rsp: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryResolveReceipt {
    Prologue { entry: SourceCrtQueryResolveEntry, actual_body: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_rdi: u64, actual_saved: [SourceCrtQueryStackWord; 2] },
    CallReturned { request: SourceCrtQueryResolveCall, actual_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_rdi: u64 },
    Read32 { source_pc: u64, actual_address: u64, value: u32 },
    Read64 { source_pc: u64, actual_address: u64, value: u64 },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame, actual_rbx: u64, actual_rdi: u64,
        actual_reads: [SourceCrtNativeRetStackRead; 2] },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryResolveCompletion {
    entry: SourceCrtQueryResolveEntry,
    fresh_context: u64,
    actual_ret_frame: SourceCrtNativeRetFrame,
}
impl SourceCrtQueryResolveCompletion {
    pub fn entry(&self) -> SourceCrtQueryResolveEntry { self.entry }
    pub fn fresh_context(&self) -> u64 { self.fresh_context }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryResolveProgress {
    Request(SourceCrtQueryResolveRequest), BeforeRet(SourceCrtQueryResolveCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryResolveError { pub pending: SourceCrtQueryResolveProgress, pub actual: SourceCrtQueryResolveReceipt }
pub struct SourceCrtQueryResolve5941fc {
    entry: SourceCrtQueryResolveEntry,
    fresh_context: Option<u64>,
    slot: Option<u64>,
    flags: Option<u32>,
    result: Option<u64>,
    progress: SourceCrtQueryResolveProgress,
    stopped: bool,
}
impl SourceCrtQueryResolve5941fc {
    pub fn begin(entry: SourceCrtQueryResolveEntry) -> Self {
        Self { entry, fresh_context: None, slot: None, flags: None, result: None,
            progress: SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredPrologue {
                entry, required_body_rip: entry.actual_module_base.wrapping_add(0x594206) }), stopped: false }
    }
    pub fn entry(&self) -> SourceCrtQueryResolveEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryResolveProgress { self.progress }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn request(&self) -> Option<SourceCrtQueryResolveRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryResolveRequest> {
        match self.progress { SourceCrtQueryResolveProgress::Request(request) => Some(request), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryResolveReceipt)
        -> Result<SourceCrtQueryResolveProgress, SourceCrtQueryResolveError> {
        self.stopped = true;
        let reject = SourceCrtQueryResolveError { pending: self.progress, actual };
        let base = self.entry.actual_module_base;
        match (self.progress, actual) {
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredPrologue { entry, required_body_rip }),
                SourceCrtQueryResolveReceipt::Prologue { entry: observed, actual_body, actual_rbx, actual_rdi, actual_saved })
                if entry == observed && actual_body == (SourceCrtNativeRetFrame { actual_rip: required_body_rip,
                    actual_rsp: entry.actual_entry_rsp.wrapping_sub(0x28), actual_rax: entry.actual_entry_rax })
                    && actual_rbx == entry.actual_entry_rbx && actual_rdi == entry.actual_entry_rdi
                    && actual_saved == self.saved_words() => {
                self.call(SourceCrtQueryResolveCallKind::FreshContext58db7c, 0x594206, 0x58db7c, None, None, 0x59420b, None, None);
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredCall(expected)),
                SourceCrtQueryResolveReceipt::CallReturned { request, actual_frame, actual_rbx, actual_rdi })
                if request == expected && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && expected.required_saved_rdi.map_or(true, |value| value == actual_rdi)
                    && expected.required_saved_rbx.map_or(true, |value| value == actual_rbx)
                    && (expected.kind != SourceCrtQueryResolveCallKind::FreshContext58db7c || actual_frame.actual_rax != 0) => {
                match expected.kind {
                    SourceCrtQueryResolveCallKind::FreshContext58db7c => {
                        self.fresh_context = Some(actual_frame.actual_rax);
                        self.slot = Some(actual_frame.actual_rax.wrapping_add(0x90));
                        self.next(SourceCrtQueryResolveRequest::ReadFlags32 { source_pc: base.wrapping_add(0x594212),
                            actual_address: actual_frame.actual_rax.wrapping_add(0x3a8) });
                    }
                    SourceCrtQueryResolveCallKind::Addressed57a7a8 => self.next(SourceCrtQueryResolveRequest::ReadGlobal64 {
                        source_pc: base.wrapping_add(0x594235), actual_address: base.wrapping_add(0x861640) }),
                    SourceCrtQueryResolveCallKind::Update59426c => {
                        self.result = Some(actual_frame.actual_rax);
                        self.call(SourceCrtQueryResolveCallKind::Addressed57a7c4, 0x59424c, 0x57a7c4,
                            Some(4), None, 0x594251, self.slot, self.result);
                    }
                    SourceCrtQueryResolveCallKind::Addressed57a7c4 => {
                        let Some(result) = self.result else { return Err(reject); }; self.finish_value(result);
                    }
                }
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::ReadFlags32 { source_pc, actual_address }),
                SourceCrtQueryResolveReceipt::Read32 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.flags = Some(value); self.next(SourceCrtQueryResolveRequest::ReadMask32 {
                    source_pc: base.wrapping_add(0x594218), actual_address: base.wrapping_add(0x836bd0) });
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::ReadMask32 { source_pc, actual_address }),
                SourceCrtQueryResolveReceipt::Read32 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                let (Some(flags), Some(slot)) = (self.flags, self.slot) else { return Err(reject); };
                if flags & value != 0 { self.next(SourceCrtQueryResolveRequest::ReadSlot64 {
                    source_pc: base.wrapping_add(0x594222), actual_address: slot }); }
                else { self.acquire(); }
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::ReadSlot64 { source_pc, actual_address }),
                SourceCrtQueryResolveReceipt::Read64 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                if value != 0 { self.finish_value(value); } else { self.acquire(); }
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::ReadGlobal64 { source_pc, actual_address }),
                SourceCrtQueryResolveReceipt::Read64 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.call(SourceCrtQueryResolveCallKind::Update59426c, 0x59423f, 0x59426c,
                    self.slot, Some(value), 0x594244, self.slot, None);
            }
            (SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredEpilogue { required_ret_frame, .. }),
                SourceCrtQueryResolveReceipt::Epilogue { actual_ret_frame, actual_rbx, actual_rdi, actual_reads })
                if actual_ret_frame == required_ret_frame && actual_rbx == self.entry.actual_entry_rbx
                    && actual_rdi == self.entry.actual_entry_rdi && actual_reads == self.expected_reads() => {
                let Some(fresh_context) = self.fresh_context else { return Err(reject); };
                self.progress = SourceCrtQueryResolveProgress::BeforeRet(SourceCrtQueryResolveCompletion {
                    entry: self.entry, fresh_context, actual_ret_frame });
            }
            _ => return Err(reject),
        }
        self.stopped = false; Ok(self.progress)
    }
    fn next(&mut self, request: SourceCrtQueryResolveRequest) { self.progress = SourceCrtQueryResolveProgress::Request(request); }
    fn acquire(&mut self) {
        self.call(SourceCrtQueryResolveCallKind::Addressed57a7a8, 0x59422f, 0x57a7a8, Some(4), None, 0x594234, self.slot, None);
    }
    fn call(&mut self, kind: SourceCrtQueryResolveCallKind, pc: u64, target: u64, rcx: Option<u64>, rdx: Option<u64>,
        resume: u64, rdi: Option<u64>, rbx: Option<u64>) {
        self.next(SourceCrtQueryResolveRequest::RequiredCall(SourceCrtQueryResolveCall {
            kind, source_pc: self.entry.actual_module_base.wrapping_add(pc),
            actual_target: self.entry.actual_module_base.wrapping_add(target), source_rcx: rcx, source_rdx: rdx,
            actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
            required_actual_resume: self.entry.actual_module_base.wrapping_add(resume),
            required_saved_rdi: rdi, required_saved_rbx: rbx }));
    }
    fn finish_value(&mut self, value: u64) {
        self.result = Some(value);
        if value == 0 { self.next(SourceCrtQueryResolveRequest::RequiredFailure57c878 {
            source_pc: self.entry.actual_module_base.wrapping_add(0x594264),
            actual_target: self.entry.actual_module_base.wrapping_add(0x57c878),
            actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28) }); }
        else { self.next(SourceCrtQueryResolveRequest::RequiredEpilogue {
            source_pc: self.entry.actual_module_base.wrapping_add(0x594256),
            required_ret_frame: SourceCrtNativeRetFrame { actual_rip: self.entry.actual_module_base.wrapping_add(0x594263),
                actual_rsp: self.entry.actual_entry_rsp, actual_rax: value } }); }
    }
    fn saved_words(&self) -> [SourceCrtQueryStackWord; 2] {
        [SourceCrtQueryStackWord { actual_address: self.entry.actual_entry_rsp.wrapping_add(8), actual_word: self.entry.actual_entry_rbx },
         SourceCrtQueryStackWord { actual_address: self.entry.actual_entry_rsp.wrapping_sub(8), actual_word: self.entry.actual_entry_rdi }]
    }
    fn expected_reads(&self) -> [SourceCrtNativeRetStackRead; 2] {
        let words = self.saved_words();
        [SourceCrtNativeRetStackRead { actual_address: words[0].actual_address, actual_word: words[0].actual_word },
         SourceCrtNativeRetStackRead { actual_address: words[1].actual_address, actual_word: words[1].actual_word }]
    }
}
