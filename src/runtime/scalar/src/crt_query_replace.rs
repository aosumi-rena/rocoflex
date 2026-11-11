use super::crt_query_context::SourceCrtQueryStackWord;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryReplaceEntry {
    pub actual_module_base: u64, pub actual_entry_rsp: u64, pub actual_entry_rax: u64,
    pub actual_entry_rbx: u64, pub actual_entry_rdi: u64,
    pub actual_slot_rcx: u64, pub actual_incoming_rdx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReplaceCallKind { Whole593ec8, Whole593f54, Whole593ffc }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryReplaceCall {
    pub kind: SourceCrtQueryReplaceCallKind, pub source_pc: u64, pub actual_target: u64,
    pub source_rcx: u64, pub source_rdx: Option<u64>, pub actual_caller_rsp: u64,
    pub required_actual_resume: u64, pub required_old_rbx: u64, pub required_incoming_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReplaceRequest {
    RequiredPrologue { entry: SourceCrtQueryReplaceEntry, required_body_rip: u64 },
    ReadOldSlot64 { source_pc: u64, actual_address: u64 },
    StoreNewSlot64 { source_pc: u64, actual_address: u64, value: u64 },
    RequiredCall(SourceCrtQueryReplaceCall),
    ReadOldCount32 { source_pc: u64, actual_address: u64 },
    RequiredEpilogue { source_pc: u64, required_ret_frame: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReplaceReceipt {
    Prologue { entry: SourceCrtQueryReplaceEntry, actual_body: SourceCrtNativeRetFrame,
        actual_rcx: u64, actual_rdi: u64, actual_rbx: u64, actual_saved: [SourceCrtQueryStackWord; 2] },
    OldSlotRead { source_pc: u64, actual_address: u64, value: u64 },
    NewSlotStored { source_pc: u64, actual_address: u64, value: u64 },
    CallReturned { request: SourceCrtQueryReplaceCall, actual_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64, actual_rdi: u64 },
    OldCountRead { source_pc: u64, actual_address: u64, value: u32 },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame, actual_rbx: u64, actual_rdi: u64,
        actual_reads: [SourceCrtNativeRetStackRead; 2] },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryReplaceCompletion {
    entry: SourceCrtQueryReplaceEntry, old_slot: Option<u64>, new_slot_stored: bool,
    old_count_after_release: Option<u32>, actual_ret_frame: SourceCrtNativeRetFrame,
}
impl SourceCrtQueryReplaceCompletion {
    pub fn entry(&self) -> SourceCrtQueryReplaceEntry { self.entry }
    pub fn old_slot(&self) -> Option<u64> { self.old_slot }
    pub fn new_slot_stored(&self) -> bool { self.new_slot_stored }
    pub fn old_count_after_release(&self) -> Option<u32> { self.old_count_after_release }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReplaceProgress { Request(SourceCrtQueryReplaceRequest), BeforeRet(SourceCrtQueryReplaceCompletion) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryReplaceError { pub pending: SourceCrtQueryReplaceProgress, pub actual: SourceCrtQueryReplaceReceipt }
pub struct SourceCrtQueryReplace59426c {
    entry: SourceCrtQueryReplaceEntry, old: Option<u64>, stored: bool, old_count: Option<u32>,
    progress: SourceCrtQueryReplaceProgress, stopped: bool,
}
impl SourceCrtQueryReplace59426c {
    pub(super) fn saved_old_word(&self) -> Option<u64> { self.old }
    pub fn begin(entry: SourceCrtQueryReplaceEntry) -> Self {
        Self { entry, old: None, stored: false, old_count: None,
            progress: SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::RequiredPrologue {
                entry, required_body_rip: entry.actual_module_base.wrapping_add(0x594279) }), stopped: false }
    }
    pub fn entry(&self) -> SourceCrtQueryReplaceEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryReplaceProgress { self.progress }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn request(&self) -> Option<SourceCrtQueryReplaceRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryReplaceRequest> {
        match self.progress { SourceCrtQueryReplaceProgress::Request(request) => Some(request), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryReplaceReceipt)
        -> Result<SourceCrtQueryReplaceProgress, SourceCrtQueryReplaceError> {
        self.stopped = true;
        let reject = SourceCrtQueryReplaceError { pending: self.progress, actual };
        let base = self.entry.actual_module_base;
        match (self.progress, actual) {
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::RequiredPrologue { entry, required_body_rip }),
                SourceCrtQueryReplaceReceipt::Prologue { entry: observed, actual_body, actual_rcx, actual_rdi, actual_rbx, actual_saved })
                if entry == observed && actual_body == (SourceCrtNativeRetFrame { actual_rip: required_body_rip,
                    actual_rsp: entry.actual_entry_rsp.wrapping_sub(0x28), actual_rax: entry.actual_entry_rax })
                    && actual_rcx == entry.actual_slot_rcx && actual_rdi == entry.actual_incoming_rdx
                    && actual_rbx == entry.actual_entry_rbx && actual_saved == self.saved_words() => {
                if entry.actual_incoming_rdx == 0 || entry.actual_slot_rcx == 0 { self.epilogue(0); }
                else { self.next(SourceCrtQueryReplaceRequest::ReadOldSlot64 {
                    source_pc: base.wrapping_add(0x594283), actual_address: entry.actual_slot_rcx }); }
            }
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::ReadOldSlot64 { source_pc, actual_address }),
                SourceCrtQueryReplaceReceipt::OldSlotRead { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.old = Some(value);
                if value == self.entry.actual_incoming_rdx { self.epilogue(self.entry.actual_incoming_rdx); }
                else { self.next(SourceCrtQueryReplaceRequest::StoreNewSlot64 {
                    source_pc: base.wrapping_add(0x594290), actual_address: self.entry.actual_slot_rcx,
                    value: self.entry.actual_incoming_rdx }); }
            }
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::StoreNewSlot64 { source_pc, actual_address, value }),
                SourceCrtQueryReplaceReceipt::NewSlotStored { source_pc: pc, actual_address: address, value: stored })
                if pc == source_pc && address == actual_address && stored == value => {
                self.stored = true;
                self.call(SourceCrtQueryReplaceCallKind::Whole593ec8, 0x594296, 0x593ec8, self.entry.actual_incoming_rdx, 0x59429b);
            }
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::RequiredCall(expected)),
                SourceCrtQueryReplaceReceipt::CallReturned { request, actual_frame, actual_rbx, actual_rdi })
                if request == expected && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && actual_rbx == expected.required_old_rbx && actual_rdi == expected.required_incoming_rdi => {
                let Some(old) = self.old else { return Err(reject); };
                match expected.kind {
                    SourceCrtQueryReplaceCallKind::Whole593ec8 => {
                        if old == 0 { self.epilogue(self.entry.actual_incoming_rdx); }
                        else { self.call(SourceCrtQueryReplaceCallKind::Whole593f54, 0x5942a3, 0x593f54, old, 0x5942a8); }
                    }
                    SourceCrtQueryReplaceCallKind::Whole593f54 => self.next(SourceCrtQueryReplaceRequest::ReadOldCount32 {
                        source_pc: base.wrapping_add(0x5942a8), actual_address: old.wrapping_add(0x10) }),
                    SourceCrtQueryReplaceCallKind::Whole593ffc => self.epilogue(self.entry.actual_incoming_rdx),
                }
            }
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::ReadOldCount32 { source_pc, actual_address }),
                SourceCrtQueryReplaceReceipt::OldCountRead { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.old_count = Some(value);
                let Some(old) = self.old else { return Err(reject); };
                if value != 0 || old == base.wrapping_add(0x8364d0) { self.epilogue(self.entry.actual_incoming_rdx); }
                else { self.call(SourceCrtQueryReplaceCallKind::Whole593ffc, 0x5942bd, 0x593ffc, old, 0x5942c2); }
            }
            (SourceCrtQueryReplaceProgress::Request(SourceCrtQueryReplaceRequest::RequiredEpilogue { required_ret_frame, .. }),
                SourceCrtQueryReplaceReceipt::Epilogue { actual_ret_frame, actual_rbx, actual_rdi, actual_reads })
                if actual_ret_frame == required_ret_frame && actual_rbx == self.entry.actual_entry_rbx
                    && actual_rdi == self.entry.actual_entry_rdi && actual_reads == self.expected_reads() => {
                self.progress = SourceCrtQueryReplaceProgress::BeforeRet(SourceCrtQueryReplaceCompletion {
                    entry: self.entry, old_slot: self.old, new_slot_stored: self.stored,
                    old_count_after_release: self.old_count, actual_ret_frame });
            }
            _ => return Err(reject),
        }
        self.stopped = false; Ok(self.progress)
    }
    fn next(&mut self, request: SourceCrtQueryReplaceRequest) { self.progress = SourceCrtQueryReplaceProgress::Request(request); }
    fn call(&mut self, kind: SourceCrtQueryReplaceCallKind, pc: u64, target: u64, rcx: u64, resume: u64) {
        let Some(old) = self.old else { self.stopped = true; return; };
        self.next(SourceCrtQueryReplaceRequest::RequiredCall(SourceCrtQueryReplaceCall {
            kind, source_pc: self.entry.actual_module_base.wrapping_add(pc),
            actual_target: self.entry.actual_module_base.wrapping_add(target), source_rcx: rcx,
            source_rdx: None, actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
            required_actual_resume: self.entry.actual_module_base.wrapping_add(resume),
            required_old_rbx: old, required_incoming_rdi: self.entry.actual_incoming_rdx }));
    }
    fn epilogue(&mut self, value: u64) {
        self.next(SourceCrtQueryReplaceRequest::RequiredEpilogue {
            source_pc: self.entry.actual_module_base.wrapping_add(0x5942c6),
            required_ret_frame: SourceCrtNativeRetFrame { actual_rip: self.entry.actual_module_base.wrapping_add(0x5942d0),
                actual_rsp: self.entry.actual_entry_rsp, actual_rax: value } });
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
