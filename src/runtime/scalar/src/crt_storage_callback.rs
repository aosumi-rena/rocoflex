use super::crt_thread_storage::{plan_source_storage_callback_58a6bc,
    SourceCrtStorageCallbackPlan, SourceCrtStorageCallbackReturn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageCallbackEntry {
    pub actual_module_base: u64,
    pub actual_argument_rcx: u64,
    pub actual_entry_rax: u64,
    pub actual_entry_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageFree580c24Request {
    pub actual_rcx: u64,

    pub source_rax_before_call: u64,
    pub actual_call_instruction_rip: u64,
    pub source_rsp_before_call: u64,
    pub required_callee_entry_rsp: u64,
    pub required_actual_resume_rip: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageFree580c24Returned {
    pub request: SourceCrtStorageFree580c24Request,
    pub actual_return_rax: u64,
    pub actual_resume_rip: u64,
    pub actual_rsp_after_return: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageCallbackCompletion {
    entry: SourceCrtStorageCallbackEntry,
    exclusion: Option<SourceCrtStorageCallbackReturn>,
    actual_free_return: Option<SourceCrtStorageFree580c24Returned>,
    source_rax_before_ret: u64,
    source_rsp_before_ret: u64,
    required_actual_ret_instruction_rip: u64,
}
impl SourceCrtStorageCallbackCompletion {
    pub fn entry(&self) -> SourceCrtStorageCallbackEntry { self.entry }
    pub fn exclusion(&self) -> Option<SourceCrtStorageCallbackReturn> { self.exclusion }
    pub fn actual_free_return(&self) -> Option<SourceCrtStorageFree580c24Returned> { self.actual_free_return }
    pub fn source_rax_before_ret(&self) -> u64 { self.source_rax_before_ret }
    pub fn source_rsp_before_ret(&self) -> u64 { self.source_rsp_before_ret }
    pub fn required_actual_ret_instruction_rip(&self) -> u64 { self.required_actual_ret_instruction_rip }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageCallbackProgress {
    RequiredFree(SourceCrtStorageFree580c24Request),
    CompleteBeforeRet(SourceCrtStorageCallbackCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageCallbackError {
    pub pending: SourceCrtStorageCallbackProgress,
    pub actual: SourceCrtStorageFree580c24Returned,
}
impl SourceCrtStorageCallbackError {
    pub fn reason(&self) -> &'static str {
        "58a6bc actual580c24 return differs from its pending call,58a6d6 continuation or source stack pointer"
    }
}
#[must_use = "retain actual heap/free/full-state owners and partial callback cursor"]
pub struct SourceCrtStorageCallbackServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtStorageFree580c24Request,
}
pub struct SourceCrtStorageCallbackRejectedFailure<E> {
    pub cause: E,
    pub completed: SourceCrtStorageCallbackCompletion,
}

pub struct SourceCrtStorageCallback58a6bc {
    entry: SourceCrtStorageCallbackEntry,
    progress: SourceCrtStorageCallbackProgress,
    failed_service: bool,
}
impl SourceCrtStorageCallback58a6bc {
    pub fn begin_from_supplied_entry(entry: SourceCrtStorageCallbackEntry) -> Self {
        let fallback = entry.actual_module_base.wrapping_add(0x861520);
        let plan = plan_source_storage_callback_58a6bc(entry.actual_argument_rcx, fallback);
        let progress = match plan {
            SourceCrtStorageCallbackPlan::ReturnWithoutFree(exclusion) => {
                let rax = match exclusion {
                    SourceCrtStorageCallbackReturn::NullArgument => entry.actual_entry_rax,
                    SourceCrtStorageCallbackReturn::SameFallbackContext => fallback,
                };
                SourceCrtStorageCallbackProgress::CompleteBeforeRet(Self::completion(entry, Some(exclusion), None, rax))
            }
            SourceCrtStorageCallbackPlan::RequiredFree580c24 { actual_rcx } => {

                let rsp = entry.actual_entry_rsp.wrapping_sub(0x28);
                SourceCrtStorageCallbackProgress::RequiredFree(SourceCrtStorageFree580c24Request {
                    actual_rcx, source_rax_before_call: fallback,
                    actual_call_instruction_rip: entry.actual_module_base.wrapping_add(0x58a6d1),
                    source_rsp_before_call: rsp, required_callee_entry_rsp: rsp.wrapping_sub(8),
                    required_actual_resume_rip: entry.actual_module_base.wrapping_add(0x58a6d6),
                })
            }
        };
        Self { entry, progress, failed_service: false }
    }
    pub fn entry(&self) -> SourceCrtStorageCallbackEntry { self.entry }
    pub fn progress(&self) -> SourceCrtStorageCallbackProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtStorageFree580c24Request> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtStorageFree580c24Request> {
        match self.progress { SourceCrtStorageCallbackProgress::RequiredFree(request) => Some(request), _ => None }
    }

    pub(super) fn suspend_actual_free_prefix(&mut self) { self.failed_service = true; }
    pub fn accept_actual_free_return(&mut self, actual: SourceCrtStorageFree580c24Returned)
        -> Result<SourceCrtStorageCallbackProgress, SourceCrtStorageCallbackError> {
        self.failed_service = true;
        let reject = SourceCrtStorageCallbackError { pending: self.progress, actual };
        let SourceCrtStorageCallbackProgress::RequiredFree(expected) = self.progress else { return Err(reject); };
        if actual.request != expected || actual.actual_resume_rip != expected.required_actual_resume_rip
            || actual.actual_rsp_after_return != expected.source_rsp_before_call { return Err(reject); }
        self.progress = SourceCrtStorageCallbackProgress::CompleteBeforeRet(Self::completion(
            self.entry, None, Some(actual), actual.actual_return_rax));
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtStorageCallbackServiceFailure<E>, SourceCrtStorageCallbackRejectedFailure<E>> {
        match self.progress {
            SourceCrtStorageCallbackProgress::RequiredFree(pending) => {
                self.failed_service = true;
                Ok(SourceCrtStorageCallbackServiceFailure { cause, pending })
            }
            SourceCrtStorageCallbackProgress::CompleteBeforeRet(completed) =>
                Err(SourceCrtStorageCallbackRejectedFailure { cause, completed }),
        }
    }
    fn completion(entry: SourceCrtStorageCallbackEntry, exclusion: Option<SourceCrtStorageCallbackReturn>,
        actual_free_return: Option<SourceCrtStorageFree580c24Returned>, rax: u64) -> SourceCrtStorageCallbackCompletion {
        SourceCrtStorageCallbackCompletion { entry, exclusion, actual_free_return,
            source_rax_before_ret: rax, source_rsp_before_ret: entry.actual_entry_rsp,
            required_actual_ret_instruction_rip: entry.actual_module_base.wrapping_add(0x58a6da) }
    }
}
