use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryRefreshEntry {
    pub actual_module_base: u64,
    pub actual_entry_rsp: u64,
    pub actual_entry_rax: u64,
    pub actual_entry_rbx: u64,
    pub actual_thread_rcx: u64,
    pub actual_slot_rdx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryRefreshRequest {

    RequiredPrologue { entry: SourceCrtQueryRefreshEntry, actual_body_rip: u64 },
    ReadGlobal64 { source_pc: u64, actual_address: u64 },
    ReadSlot64 { source_pc: u64, actual_address: u64 },
    ReadFlags32 { source_pc: u64, actual_address: u64 },
    ReadMask32 { source_pc: u64, actual_address: u64 },
    Required5941fc { source_pc: u64, actual_target: u64, actual_rcx: u64,
        actual_rdx: u64, actual_caller_rsp: u64, required_actual_resume_rip: u64 },
    StoreSlot64 { source_pc: u64, actual_address: u64, value: u64 },
    RequiredEpilogue { source_pc: u64, actual_before_rsp: u64,
        actual_pop_address: u64, required_ret_frame: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryRefreshReceipt {
    Prologue { entry: SourceCrtQueryRefreshEntry, actual_body: SourceCrtNativeRetFrame,
        actual_rcx: u64, actual_rdx: u64, actual_rbx: u64,
        actual_saved_rbx_address: u64, actual_saved_rbx_word: u64 },
    Read64 { source_pc: u64, actual_address: u64, value: u64 },
    Read32 { source_pc: u64, actual_address: u64, value: u32 },

    Returned5941fc { request: SourceCrtQueryRefreshRequest, actual_frame: SourceCrtNativeRetFrame,
        actual_rbx: u64 },
    SlotStored { source_pc: u64, actual_address: u64, value: u64 },
    Epilogue { actual_ret_frame: SourceCrtNativeRetFrame, actual_rbx: u64,
        actual_pop: SourceCrtNativeRetStackRead },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryRefreshCompletion {
    entry: SourceCrtQueryRefreshEntry,
    source_return_rax: u64,
    actual_ret_frame: SourceCrtNativeRetFrame,
    refreshed: bool,
}
impl SourceCrtQueryRefreshCompletion {
    pub fn entry(&self) -> SourceCrtQueryRefreshEntry { self.entry }
    pub fn source_return_rax(&self) -> u64 { self.source_return_rax }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
    pub fn refreshed(&self) -> bool { self.refreshed }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryRefreshProgress {
    Request(SourceCrtQueryRefreshRequest),

    BeforeRet(SourceCrtQueryRefreshCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryRefreshError {
    pub pending: SourceCrtQueryRefreshProgress,
    pub actual: SourceCrtQueryRefreshReceipt,
}

pub struct SourceCrtQueryRefresh58e238 {
    entry: SourceCrtQueryRefreshEntry,
    global: Option<u64>,
    flags: Option<u32>,
    source_rax: u64,
    refreshed: bool,
    progress: SourceCrtQueryRefreshProgress,
    stopped: bool,
}
impl SourceCrtQueryRefresh58e238 {
    pub(super) fn source_rax(&self) -> u64 { self.source_rax }
    pub fn begin(entry: SourceCrtQueryRefreshEntry) -> Self {
        Self { entry, global: None, flags: None, source_rax: entry.actual_entry_rax,
            refreshed: false, progress: SourceCrtQueryRefreshProgress::Request(
                SourceCrtQueryRefreshRequest::RequiredPrologue { entry,
                    actual_body_rip: entry.actual_module_base.wrapping_add(0x58e23e) }), stopped: false }
    }
    pub fn entry(&self) -> SourceCrtQueryRefreshEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryRefreshProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtQueryRefreshRequest> {
        if self.stopped { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryRefreshRequest> {
        match self.progress { SourceCrtQueryRefreshProgress::Request(request) => Some(request), _ => None }
    }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryRefreshReceipt)
        -> Result<SourceCrtQueryRefreshProgress, SourceCrtQueryRefreshError> {
        self.stopped = true;
        let reject = SourceCrtQueryRefreshError { pending: self.progress, actual };
        let base = self.entry.actual_module_base;
        match (self.progress, actual) {
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::RequiredPrologue { entry, actual_body_rip }),
                SourceCrtQueryRefreshReceipt::Prologue { entry: observed, actual_body, actual_rcx, actual_rdx,
                    actual_rbx, actual_saved_rbx_address, actual_saved_rbx_word })
                if entry == observed && actual_body == (SourceCrtNativeRetFrame { actual_rip: actual_body_rip,
                    actual_rsp: entry.actual_entry_rsp.wrapping_sub(0x28), actual_rax: entry.actual_entry_rax })
                    && actual_rcx == entry.actual_thread_rcx && actual_rdx == entry.actual_slot_rdx
                    && actual_rbx == entry.actual_entry_rbx
                    && actual_saved_rbx_address == entry.actual_entry_rsp.wrapping_sub(8)
                    && actual_saved_rbx_word == entry.actual_entry_rbx => {
                self.next(SourceCrtQueryRefreshRequest::ReadGlobal64 { source_pc: actual_body_rip,
                    actual_address: base.wrapping_add(0x861640) });
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::ReadGlobal64 { source_pc, actual_address }),
                SourceCrtQueryRefreshReceipt::Read64 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.global = Some(value); self.source_rax = value;
                self.next(SourceCrtQueryRefreshRequest::ReadSlot64 { source_pc: base.wrapping_add(0x58e248),
                    actual_address: self.entry.actual_slot_rdx });
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::ReadSlot64 { source_pc, actual_address }),
                SourceCrtQueryRefreshReceipt::Read64 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                if self.global == Some(value) { self.epilogue(); }
                else { self.next(SourceCrtQueryRefreshRequest::ReadFlags32 { source_pc: base.wrapping_add(0x58e24d),
                    actual_address: self.entry.actual_thread_rcx.wrapping_add(0x3a8) }); }
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::ReadFlags32 { source_pc, actual_address }),
                SourceCrtQueryRefreshReceipt::Read32 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                self.flags = Some(value); self.source_rax = u64::from(value);
                self.next(SourceCrtQueryRefreshRequest::ReadMask32 { source_pc: base.wrapping_add(0x58e253),
                    actual_address: base.wrapping_add(0x836bd0) });
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::ReadMask32 { source_pc, actual_address }),
                SourceCrtQueryRefreshReceipt::Read32 { source_pc: pc, actual_address: address, value })
                if pc == source_pc && address == actual_address => {
                let Some(flags) = self.flags else { return Err(reject); };
                if flags & value != 0 { self.epilogue(); }
                else { self.next(SourceCrtQueryRefreshRequest::Required5941fc {
                    source_pc: base.wrapping_add(0x58e25b), actual_target: base.wrapping_add(0x5941fc),
                    actual_rcx: self.entry.actual_thread_rcx, actual_rdx: self.entry.actual_slot_rdx,
                    actual_caller_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
                    required_actual_resume_rip: base.wrapping_add(0x58e260) }); }
            }
            (SourceCrtQueryRefreshProgress::Request(expected @ SourceCrtQueryRefreshRequest::Required5941fc {
                actual_caller_rsp, required_actual_resume_rip, .. }),
                SourceCrtQueryRefreshReceipt::Returned5941fc { request, actual_frame, actual_rbx })
                if request == expected && actual_frame.actual_rip == required_actual_resume_rip
                    && actual_frame.actual_rsp == actual_caller_rsp && actual_rbx == self.entry.actual_slot_rdx => {
                self.source_rax = actual_frame.actual_rax;
                self.next(SourceCrtQueryRefreshRequest::StoreSlot64 { source_pc: base.wrapping_add(0x58e260),
                    actual_address: self.entry.actual_slot_rdx, value: self.source_rax });
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::StoreSlot64 { source_pc, actual_address, value }),
                SourceCrtQueryRefreshReceipt::SlotStored { source_pc: pc, actual_address: address, value: stored })
                if pc == source_pc && address == actual_address && stored == value => {
                self.refreshed = true; self.epilogue();
            }
            (SourceCrtQueryRefreshProgress::Request(SourceCrtQueryRefreshRequest::RequiredEpilogue {
                actual_pop_address, required_ret_frame, .. }),
                SourceCrtQueryRefreshReceipt::Epilogue { actual_ret_frame, actual_rbx, actual_pop })
                if actual_ret_frame == required_ret_frame && actual_rbx == self.entry.actual_entry_rbx
                    && actual_pop.actual_address == actual_pop_address && actual_pop.actual_word == self.entry.actual_entry_rbx => {
                self.progress = SourceCrtQueryRefreshProgress::BeforeRet(SourceCrtQueryRefreshCompletion {
                    entry: self.entry, source_return_rax: self.source_rax, actual_ret_frame, refreshed: self.refreshed });
            }
            _ => return Err(reject),
        }
        self.stopped = false;
        Ok(self.progress)
    }
    fn next(&mut self, request: SourceCrtQueryRefreshRequest) { self.progress = SourceCrtQueryRefreshProgress::Request(request); }
    fn epilogue(&mut self) {
        self.next(SourceCrtQueryRefreshRequest::RequiredEpilogue {
            source_pc: self.entry.actual_module_base.wrapping_add(0x58e263),
            actual_before_rsp: self.entry.actual_entry_rsp.wrapping_sub(0x28),
            actual_pop_address: self.entry.actual_entry_rsp.wrapping_sub(8),
            required_ret_frame: SourceCrtNativeRetFrame { actual_rip: self.entry.actual_module_base.wrapping_add(0x58e268),
                actual_rsp: self.entry.actual_entry_rsp, actual_rax: self.source_rax } });
    }
}
