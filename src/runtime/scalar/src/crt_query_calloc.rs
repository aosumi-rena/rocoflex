use super::crt_query_context::SourceCrtQueryStackWord;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCallocArguments { ContextOneBy3c8, ExplicitMember100By2 }
impl SourceCrtCallocArguments {
    pub fn words(self) -> (u64, u64) {
        match self { Self::ContextOneBy3c8 => (1, 0x3c8), Self::ExplicitMember100By2 => (0x100, 2) }
    }
    pub fn source_bytes(self) -> u64 {
        let (count, element) = self.words(); count.wrapping_mul(element)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryCallocEntry {
    pub actual_module_base: u64, pub actual_entry_rsp: u64, pub actual_entry_rbx: u64,
    pub arguments: SourceCrtCallocArguments, pub actual_count_rcx: u64, pub actual_element_rdx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocProtectedRequest {
    pub source_pc: u64, pub actual_target: u64, pub actual_heap_rcx: u64,
    pub flags_edx: u32, pub bytes_r8: u64, pub actual_extra_push: SourceCrtQueryStackWord,
    pub actual_callee_entry_rsp: u64, pub required_actual_resume: u64, pub required_resume_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCallocCallKind { Mode59bfc8, Handler5765dc, ErrnoAccessor578eb0 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocCall {
    pub kind: SourceCrtCallocCallKind, pub source_pc: u64, pub actual_target: u64,
    pub source_rcx: Option<u64>, pub actual_caller_rsp: u64, pub required_actual_resume: u64,
    pub required_bytes_rbx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryCallocRequest {

    RequiredEntryAndSizePrefix { entry: SourceCrtQueryCallocEntry, required_body_rip: u64, bytes_rbx: u64 },
    ReadHeap64 { source_pc: u64, actual_address: u64 },
    RequiredProtected(SourceCrtCallocProtectedRequest),
    RequiredCall(SourceCrtCallocCall),
    WriteErrno32 { source_pc: u64, actual_address: u64, value: u32 },
    RequiredEpilogue { source_pc: u64, required_ret_frame: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryCallocReceipt {
    EntryPrefix { entry: SourceCrtQueryCallocEntry, actual_body: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_saved: SourceCrtQueryStackWord },
    HeapRead { source_pc: u64, actual_address: u64, value: u64 },

    ProtectedContinued { request: SourceCrtCallocProtectedRequest, actual_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_extra_push: SourceCrtQueryStackWord },
    CallReturned { request: SourceCrtCallocCall, actual_frame: SourceCrtNativeRetFrame, actual_rbx: u64 },
    ErrnoWritten { source_pc: u64, actual_address: u64, value: u32 },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame, actual_rbx: u64, actual_pop: SourceCrtNativeRetStackRead },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryCallocCompletion {
    entry: SourceCrtQueryCallocEntry, actual_ret_frame: SourceCrtNativeRetFrame, completed_attempts: u64,
}
impl SourceCrtQueryCallocCompletion {
    pub fn entry(&self) -> SourceCrtQueryCallocEntry { self.entry }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
    pub fn completed_attempts(&self) -> u64 { self.completed_attempts }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryCallocProgress { Request(SourceCrtQueryCallocRequest), BeforeRet(SourceCrtQueryCallocCompletion) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryCallocError { pub pending: SourceCrtQueryCallocProgress, pub actual: SourceCrtQueryCallocReceipt }
pub struct SourceCrtQueryCalloc58f074 {
    entry: SourceCrtQueryCallocEntry, bytes: u64, next_push_rax: u64, attempts: u64,
    progress: SourceCrtQueryCallocProgress, stopped: bool,
}
impl SourceCrtQueryCalloc58f074 {

    pub fn begin(entry: SourceCrtQueryCallocEntry) -> Result<Self, SourceCrtQueryCallocEntry> {
        if (entry.actual_count_rcx, entry.actual_element_rdx) != entry.arguments.words() { return Err(entry); }
        let bytes = entry.arguments.source_bytes();
        Ok(Self { entry, bytes, next_push_rax: 1, attempts: 0,
            progress: SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredEntryAndSizePrefix {
                entry, required_body_rip: entry.actual_module_base.wrapping_add(0x58f0ba), bytes_rbx: bytes }), stopped: false })
    }
    pub fn entry(&self) -> SourceCrtQueryCallocEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryCallocProgress { self.progress }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn request(&self) -> Option<SourceCrtQueryCallocRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryCallocRequest> {
        match self.progress { SourceCrtQueryCallocProgress::Request(request) => Some(request), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryCallocReceipt)
        -> Result<SourceCrtQueryCallocProgress, SourceCrtQueryCallocError> {
        self.stopped = true;
        let reject = SourceCrtQueryCallocError { pending: self.progress, actual };
        let base = self.entry.actual_module_base;
        let body_rsp = self.entry.actual_entry_rsp.wrapping_sub(0x28);
        match (self.progress, actual) {
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredEntryAndSizePrefix { entry, required_body_rip, bytes_rbx }),
                SourceCrtQueryCallocReceipt::EntryPrefix { entry: observed, actual_body, actual_rbx, actual_saved })
                if entry == observed && actual_body == (SourceCrtNativeRetFrame { actual_rip: required_body_rip,
                    actual_rsp: body_rsp, actual_rax: 1 }) && actual_rbx == bytes_rbx
                    && actual_saved == (SourceCrtQueryStackWord { actual_address: entry.actual_entry_rsp.wrapping_sub(8),
                        actual_word: entry.actual_entry_rbx }) => { self.read_heap(); }
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::ReadHeap64 { source_pc, actual_address }),
                SourceCrtQueryCallocReceipt::HeapRead { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.next(SourceCrtQueryCallocRequest::RequiredProtected(SourceCrtCallocProtectedRequest {
                    source_pc: base.wrapping_add(0x58f0ca), actual_target: base.wrapping_add(0xcd2ea2),
                    actual_heap_rcx: value, flags_edx: 8, bytes_r8: self.bytes,
                    actual_extra_push: SourceCrtQueryStackWord { actual_address: body_rsp.wrapping_sub(8), actual_word: self.next_push_rax },
                    actual_callee_entry_rsp: body_rsp.wrapping_sub(0x10),
                    required_actual_resume: base.wrapping_add(0x58f0cf), required_resume_rsp: body_rsp }));
            }
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredProtected(expected)),
                SourceCrtQueryCallocReceipt::ProtectedContinued { request, actual_frame, actual_rbx, actual_extra_push })
                if request == expected && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.required_resume_rsp && actual_rbx == self.bytes
                    && actual_extra_push == expected.actual_extra_push => {
                self.attempts = self.attempts.wrapping_add(1);
                if actual_frame.actual_rax != 0 { self.epilogue(actual_frame.actual_rax); }
                else { self.call(SourceCrtCallocCallKind::Mode59bfc8, 0x58f0a5, 0x59bfc8, None, 0x58f0aa); }
            }
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredCall(expected)),
                SourceCrtQueryCallocReceipt::CallReturned { request, actual_frame, actual_rbx })
                if request == expected && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp && actual_rbx == self.bytes => {
                match expected.kind {
                    SourceCrtCallocCallKind::Mode59bfc8 if actual_frame.actual_rax as u32 != 0 =>
                        self.call(SourceCrtCallocCallKind::Handler5765dc, 0x58f0b1, 0x5765dc, Some(self.bytes), 0x58f0b6),
                    SourceCrtCallocCallKind::Handler5765dc if actual_frame.actual_rax as u32 != 0 => {

                        self.next_push_rax = actual_frame.actual_rax; self.read_heap();
                    }
                    SourceCrtCallocCallKind::Mode59bfc8 | SourceCrtCallocCallKind::Handler5765dc =>
                        self.call(SourceCrtCallocCallKind::ErrnoAccessor578eb0, 0x58f0d6, 0x578eb0, None, 0x58f0db),
                    SourceCrtCallocCallKind::ErrnoAccessor578eb0 => self.next(SourceCrtQueryCallocRequest::WriteErrno32 {
                        source_pc: base.wrapping_add(0x58f0db), actual_address: actual_frame.actual_rax, value: 12 }),
                }
            }
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::WriteErrno32 { source_pc, actual_address, value }),
                SourceCrtQueryCallocReceipt::ErrnoWritten { source_pc: pc, actual_address: address, value: written })
                if pc == source_pc && address == actual_address && written == value => self.epilogue(0),
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredEpilogue { required_ret_frame, .. }),
                SourceCrtQueryCallocReceipt::Epilogue { actual_ret_frame, actual_rbx, actual_pop })
                if actual_ret_frame == required_ret_frame && actual_rbx == self.entry.actual_entry_rbx
                    && actual_pop.actual_address == self.entry.actual_entry_rsp.wrapping_sub(8)
                    && actual_pop.actual_word == self.entry.actual_entry_rbx => {
                self.progress = SourceCrtQueryCallocProgress::BeforeRet(SourceCrtQueryCallocCompletion {
                    entry: self.entry, actual_ret_frame, completed_attempts: self.attempts });
            }
            _ => return Err(reject),
        }
        self.stopped = false; Ok(self.progress)
    }
    fn next(&mut self, request: SourceCrtQueryCallocRequest) { self.progress = SourceCrtQueryCallocProgress::Request(request); }
    fn read_heap(&mut self) {
        self.next(SourceCrtQueryCallocRequest::ReadHeap64 {
            source_pc: self.entry.actual_module_base.wrapping_add(0x58f0ba), actual_address: self.entry.actual_module_base.wrapping_add(0x861800) });
    }
    fn call(&mut self, kind: SourceCrtCallocCallKind, pc: u64, target: u64, rcx: Option<u64>, resume: u64) {
        self.next(SourceCrtQueryCallocRequest::RequiredCall(SourceCrtCallocCall {
            kind, source_pc: self.entry.actual_module_base.wrapping_add(pc), actual_target: self.entry.actual_module_base.wrapping_add(target),
            source_rcx: rcx, actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
            required_actual_resume: self.entry.actual_module_base.wrapping_add(resume), required_bytes_rbx: self.bytes }));
    }
    fn epilogue(&mut self, value: u64) {
        self.next(SourceCrtQueryCallocRequest::RequiredEpilogue {
            source_pc: self.entry.actual_module_base.wrapping_add(0x58f0e3),
            required_ret_frame: SourceCrtNativeRetFrame { actual_rip: self.entry.actual_module_base.wrapping_add(0x58f0e8),
                actual_rsp: self.entry.actual_entry_rsp, actual_rax: value } });
    }
}
