use super::crt_query_context::SourceCrtQueryStackWord;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationEntry {
    pub actual_module_base: u64, pub actual_entry_rsp: u64, pub actual_entry_rax: u64,
    pub actual_entry_rbp: u64, pub actual_receiver_rcx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationBody {
    pub source_pc: u32, pub actual_module_base: u64, pub actual_rbp: u64, pub actual_receiver_rcx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationLastCall {
    pub resume_pc: u32, pub rsp: u64, pub rbp: u64, pub rax: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationBodyFacts {
    pub actual: SourceCrtQueryInitializationBody,
    pub required_next_pc: u32,
    pub last_actual_call: SourceCrtQueryInitializationLastCall,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryInitializationRequest {
    RequiredPrologue { entry: SourceCrtQueryInitializationEntry },
    RequiredBorrowedBody(SourceCrtQueryInitializationBody),
    RequiredEpilogue { source_pc: u64, required_ret_frame: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryInitializationReceipt {
    Prologue { entry: SourceCrtQueryInitializationEntry, actual_frame: SourceCrtNativeRetFrame,
        actual_rbp: u64, actual_rcx: u64, actual_saved: SourceCrtQueryStackWord },

    BorrowedBody { facts: SourceCrtQueryInitializationBodyFacts,
        actual_frame: SourceCrtNativeRetFrame, actual_rbp: u64 },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame, actual_rbp: u64, actual_pop: SourceCrtNativeRetStackRead },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationCompletion {
    entry: SourceCrtQueryInitializationEntry, actual_ret_frame: SourceCrtNativeRetFrame,
    body: SourceCrtQueryInitializationBodyFacts,
}
impl SourceCrtQueryInitializationCompletion {
    pub fn entry(&self) -> SourceCrtQueryInitializationEntry { self.entry }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
    pub fn body(&self) -> SourceCrtQueryInitializationBodyFacts { self.body }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryInitializationProgress {
    Request(SourceCrtQueryInitializationRequest), BeforeRet(SourceCrtQueryInitializationCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationError {
    pub pending: SourceCrtQueryInitializationProgress, pub actual: SourceCrtQueryInitializationReceipt,
}
pub struct SourceCrtQueryInitialization58df48 {
    entry: SourceCrtQueryInitializationEntry, body: Option<SourceCrtQueryInitializationBodyFacts>,
    progress: SourceCrtQueryInitializationProgress, stopped: bool,
}
impl SourceCrtQueryInitialization58df48 {
    pub fn begin(entry: SourceCrtQueryInitializationEntry) -> Self {
        Self { entry, body: None, stopped: false, progress: SourceCrtQueryInitializationProgress::Request(
            SourceCrtQueryInitializationRequest::RequiredPrologue { entry }) }
    }
    pub fn entry(&self) -> SourceCrtQueryInitializationEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryInitializationProgress { self.progress }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn request(&self) -> Option<SourceCrtQueryInitializationRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryInitializationRequest> {
        match self.progress { SourceCrtQueryInitializationProgress::Request(value) => Some(value), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryInitializationReceipt)
        -> Result<SourceCrtQueryInitializationProgress, SourceCrtQueryInitializationError> {
        self.stopped = true;
        let reject = SourceCrtQueryInitializationError { pending: self.progress, actual };
        let entry = self.entry;
        let base = entry.actual_module_base;
        let rbp = entry.actual_entry_rsp.wrapping_sub(8);
        let body_rsp = rbp.wrapping_sub(0x50);
        match (self.progress, actual) {
            (SourceCrtQueryInitializationProgress::Request(SourceCrtQueryInitializationRequest::RequiredPrologue { entry: expected }),
                SourceCrtQueryInitializationReceipt::Prologue { entry: observed, actual_frame, actual_rbp, actual_rcx, actual_saved })
                if expected == observed && actual_frame == (SourceCrtNativeRetFrame {
                    actual_rip: base.wrapping_add(0x58df51), actual_rsp: body_rsp, actual_rax: entry.actual_entry_rax })
                    && actual_rbp == rbp && actual_rcx == entry.actual_receiver_rcx
                    && actual_saved == (SourceCrtQueryStackWord { actual_address: rbp, actual_word: entry.actual_entry_rbp }) => {
                self.progress = SourceCrtQueryInitializationProgress::Request(SourceCrtQueryInitializationRequest::RequiredBorrowedBody(
                    SourceCrtQueryInitializationBody { source_pc: 0x58df51, actual_module_base: base,
                        actual_rbp: rbp, actual_receiver_rcx: entry.actual_receiver_rcx }));
            }
            (SourceCrtQueryInitializationProgress::Request(SourceCrtQueryInitializationRequest::RequiredBorrowedBody(expected)),
                SourceCrtQueryInitializationReceipt::BorrowedBody { facts, actual_frame, actual_rbp })
                if facts.actual == expected && facts.required_next_pc == 0x58e00f
                    && facts.last_actual_call.resume_pc == 0x58e00f && facts.last_actual_call.rsp == body_rsp
                    && facts.last_actual_call.rbp == rbp && actual_rbp == rbp
                    && actual_frame == (SourceCrtNativeRetFrame { actual_rip: base.wrapping_add(0x58e00f),
                        actual_rsp: body_rsp, actual_rax: facts.last_actual_call.rax }) => {
                self.body = Some(facts);
                self.progress = SourceCrtQueryInitializationProgress::Request(SourceCrtQueryInitializationRequest::RequiredEpilogue {
                    source_pc: base.wrapping_add(0x58e00f), required_ret_frame: SourceCrtNativeRetFrame {
                        actual_rip: base.wrapping_add(0x58e014), actual_rsp: entry.actual_entry_rsp, actual_rax: actual_frame.actual_rax } });
            }
            (SourceCrtQueryInitializationProgress::Request(SourceCrtQueryInitializationRequest::RequiredEpilogue { required_ret_frame, .. }),
                SourceCrtQueryInitializationReceipt::Epilogue { actual_ret_frame, actual_rbp, actual_pop })
                if actual_ret_frame == required_ret_frame && actual_rbp == entry.actual_entry_rbp
                    && actual_pop == (SourceCrtNativeRetStackRead { actual_address: rbp, actual_word: entry.actual_entry_rbp }) => {
                let Some(body) = self.body else { return Err(reject); };
                self.progress = SourceCrtQueryInitializationProgress::BeforeRet(SourceCrtQueryInitializationCompletion {
                    entry, actual_ret_frame, body });
            }
            _ => return Err(reject),
        }
        self.stopped = false; Ok(self.progress)
    }
}
