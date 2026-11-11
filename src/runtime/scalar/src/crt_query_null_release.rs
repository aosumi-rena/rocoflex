use super::crt_storage_native_return::SourceCrtNativeRetFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryNullReleaseEntry {
    pub actual_module_base: u64, pub actual_entry_rsp: u64, pub actual_entry_rax: u64,
    pub actual_entry_rcx: u64, pub actual_entry_rbx_rsi_rdi: (u64, u64, u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryNullReleaseRequest {
    pub entry: SourceCrtQueryNullReleaseEntry, pub required_ret_frame: SourceCrtNativeRetFrame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryNullReleaseReceipt {
    pub request: SourceCrtQueryNullReleaseRequest, pub actual_ret_frame: SourceCrtNativeRetFrame,
    pub actual_rcx: u64, pub actual_rbx_rsi_rdi: (u64, u64, u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryNullReleaseCompletion {
    entry: SourceCrtQueryNullReleaseEntry, actual_ret_frame: SourceCrtNativeRetFrame,
}
impl SourceCrtQueryNullReleaseCompletion {
    pub fn entry(&self) -> SourceCrtQueryNullReleaseEntry { self.entry }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryNullReleaseProgress {
    Request(SourceCrtQueryNullReleaseRequest), BeforeRet(SourceCrtQueryNullReleaseCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryNullReleaseError { pub pending: SourceCrtQueryNullReleaseProgress, pub actual: SourceCrtQueryNullReleaseReceipt }
pub struct SourceCrtQueryNullRelease58d910 {
    entry: SourceCrtQueryNullReleaseEntry, progress: SourceCrtQueryNullReleaseProgress, stopped: bool,
}
impl SourceCrtQueryNullRelease58d910 {
    pub fn begin(entry: SourceCrtQueryNullReleaseEntry) -> Result<Self, SourceCrtQueryNullReleaseEntry> {
        if entry.actual_entry_rcx != 0 { return Err(entry); }
        Ok(Self { entry, stopped: false, progress: SourceCrtQueryNullReleaseProgress::Request(SourceCrtQueryNullReleaseRequest {
            entry, required_ret_frame: SourceCrtNativeRetFrame { actual_rip: entry.actual_module_base.wrapping_add(0x58d94b),
                actual_rsp: entry.actual_entry_rsp, actual_rax: entry.actual_entry_rax } }) })
    }
    pub fn entry(&self) -> SourceCrtQueryNullReleaseEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryNullReleaseProgress { self.progress }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn request(&self) -> Option<SourceCrtQueryNullReleaseRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryNullReleaseRequest> {
        match self.progress { SourceCrtQueryNullReleaseProgress::Request(value) => Some(value), _ => None }
    }

    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryNullReleaseReceipt)
        -> Result<SourceCrtQueryNullReleaseProgress, SourceCrtQueryNullReleaseError> {
        self.stopped = true;
        let reject = SourceCrtQueryNullReleaseError { pending: self.progress, actual };
        let SourceCrtQueryNullReleaseProgress::Request(expected) = self.progress else { return Err(reject); };
        if actual.request != expected || actual.actual_ret_frame != expected.required_ret_frame
            || actual.actual_rcx != 0 || actual.actual_rbx_rsi_rdi != self.entry.actual_entry_rbx_rsi_rdi { return Err(reject); }
        self.progress = SourceCrtQueryNullReleaseProgress::BeforeRet(SourceCrtQueryNullReleaseCompletion {
            entry: self.entry, actual_ret_frame: actual.actual_ret_frame });
        self.stopped = false; Ok(self.progress)
    }
}
