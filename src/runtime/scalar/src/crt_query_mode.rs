use super::crt_storage_native_return::SourceCrtNativeRetFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryModeEntry {
    pub actual_module_base: u64,
    pub actual_entry_rsp: u64,
    pub actual_entry_rbx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryModeRequest {
    pub source_pc: u64,
    pub actual_address: u64,
    pub actual_entry_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryModeReceipt {
    pub request: SourceCrtQueryModeRequest,
    pub actual_read_value: u32,

    pub actual_ret_frame: SourceCrtNativeRetFrame,
    pub actual_rbx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryModeCompletion {
    entry: SourceCrtQueryModeEntry,
    actual_read_value: u32,
    actual_ret_frame: SourceCrtNativeRetFrame,
}
impl SourceCrtQueryModeCompletion {
    pub fn entry(&self) -> SourceCrtQueryModeEntry { self.entry }
    pub fn actual_read_value(&self) -> u32 { self.actual_read_value }
    pub fn actual_ret_frame(&self) -> SourceCrtNativeRetFrame { self.actual_ret_frame }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryModeProgress { Request(SourceCrtQueryModeRequest), BeforeRet(SourceCrtQueryModeCompletion) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryModeError { pub pending: SourceCrtQueryModeProgress, pub actual: SourceCrtQueryModeReceipt }
pub struct SourceCrtQueryMode59bfc8 {
    entry: SourceCrtQueryModeEntry,
    progress: SourceCrtQueryModeProgress,
    stopped: bool,
}
impl SourceCrtQueryMode59bfc8 {
    pub fn begin(entry: SourceCrtQueryModeEntry) -> Self {
        Self { entry, stopped: false, progress: SourceCrtQueryModeProgress::Request(SourceCrtQueryModeRequest {
            source_pc: entry.actual_module_base.wrapping_add(0x59bfc8),
            actual_address: entry.actual_module_base.wrapping_add(0x861d00), actual_entry_rsp: entry.actual_entry_rsp }) }
    }
    pub fn entry(&self) -> SourceCrtQueryModeEntry { self.entry }
    pub fn progress(&self) -> SourceCrtQueryModeProgress { self.progress }
    pub fn stop(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn request(&self) -> Option<SourceCrtQueryModeRequest> { if self.stopped { None } else { self.awaiting_actual_completion() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryModeRequest> {
        match self.progress { SourceCrtQueryModeProgress::Request(value) => Some(value), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtQueryModeReceipt)
        -> Result<SourceCrtQueryModeProgress, SourceCrtQueryModeError> {
        self.stopped = true;
        let reject = SourceCrtQueryModeError { pending: self.progress, actual };
        let SourceCrtQueryModeProgress::Request(expected) = self.progress else { return Err(reject); };
        let frame = SourceCrtNativeRetFrame { actual_rip: self.entry.actual_module_base.wrapping_add(0x59bfcf),
            actual_rsp: self.entry.actual_entry_rsp, actual_rax: u64::from(actual.actual_read_value) };
        if actual.request != expected || actual.actual_ret_frame != frame || actual.actual_rbx != self.entry.actual_entry_rbx {
            return Err(reject);
        }
        self.progress = SourceCrtQueryModeProgress::BeforeRet(SourceCrtQueryModeCompletion {
            entry: self.entry, actual_read_value: actual.actual_read_value, actual_ret_frame: frame });
        self.stopped = false; Ok(self.progress)
    }
}
