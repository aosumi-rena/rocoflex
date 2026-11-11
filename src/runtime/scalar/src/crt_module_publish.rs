use super::crt_cached_resolver::SourceCrtCachedResolverRequest;
use super::crt_native_thunk::SourceCrtResolver59bd70Request;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModulePublishRequest {
    ExchangeModuleCache { actual_address: u64, desired_word: u64 },

    CleanupAt59be73 { actual_rcx: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModulePublishReceipt {
    Exchanged { actual_address: u64, desired_word: u64, actual_old_word: u64 },

    CleanupReturned { actual_rcx_argument: u64, actual_return_rax: u64,
        actual_resume_rip: u64, actual_rbx: u64, actual_r15: u64,
        actual_rsi: u64, actual_rdi: u64, actual_r13: u64, actual_r14: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModulePublishCompletion {
    call: SourceCrtResolver59bd70Request,
    source_module_id: u32,
    actual_published_module: u64,
    actual_exchanged_old_word: u64,
    actual_cleanup_return_rax: Option<u64>,

    next_procedure: SourceCrtCachedResolverRequest,
}
impl SourceCrtModulePublishCompletion {
    pub fn call(&self) -> SourceCrtResolver59bd70Request { self.call }
    pub fn source_module_id(&self) -> u32 { self.source_module_id }
    pub fn actual_published_module(&self) -> u64 { self.actual_published_module }
    pub fn actual_exchanged_old_word(&self) -> u64 { self.actual_exchanged_old_word }
    pub fn actual_cleanup_return_rax(&self) -> Option<u64> { self.actual_cleanup_return_rax }
    pub fn next_procedure(&self) -> SourceCrtCachedResolverRequest { self.next_procedure }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModulePublishProgress {
    Request(SourceCrtModulePublishRequest),
    Complete(SourceCrtModulePublishCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModulePublishError {
    EntryRequired { actual_module_base: u64, call: SourceCrtResolver59bd70Request, actual_rbx_module: u64 },
    ReceiptRequired { pending: SourceCrtModulePublishProgress, actual: SourceCrtModulePublishReceipt },
    ActualContinuationRequired { actual: SourceCrtModulePublishReceipt },
}
impl SourceCrtModulePublishError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::EntryRequired { .. } => "59be59 requires actual nonzero loaded module and source-bound resolver/cache entry",
            Self::ReceiptRequired { .. } => "59be59 actual atomic/native receipt differs from pending operation",
            Self::ActualContinuationRequired { .. } => "59be73 actual return needs matching59be79 and saved source registers",
        }
    }
}
#[must_use = "retain actual module/native/atomic resource owners and pending suffix"]
pub struct SourceCrtModulePublishServiceFailure<E> { pub cause: E, pub pending: SourceCrtModulePublishRequest }
pub struct SourceCrtModulePublishRejectedFailure<E> { pub cause: E, pub completed: SourceCrtModulePublishCompletion }

pub struct SourceCrtModulePublish59be59 {
    actual_module_base: u64,
    call: SourceCrtResolver59bd70Request,
    source_module_id: u32,
    actual_module: u64,
    actual_old_word: Option<u64>,
    progress: SourceCrtModulePublishProgress,
    failed_service: bool,
}
impl SourceCrtModulePublish59be59 {
    pub fn begin_from_actual_entry(actual_module_base: u64, call: SourceCrtResolver59bd70Request,
        source_module_id: u32, actual_rbx_module: u64) -> Result<Self, SourceCrtModulePublishError> {
        if actual_rbx_module == 0 || call.actual_function_cache_address != actual_module_base.wrapping_add(0x861cd8 + u64::from(call.selector_ecx) * 8) {
            return Err(SourceCrtModulePublishError::EntryRequired { actual_module_base, call, actual_rbx_module });
        }
        let request = SourceCrtModulePublishRequest::ExchangeModuleCache {
            actual_address: actual_module_base.wrapping_add(0x861cc0 + u64::from(source_module_id) * 8), desired_word: actual_rbx_module,
        };
        Ok(Self { actual_module_base, call, source_module_id, actual_module: actual_rbx_module,
            actual_old_word: None, progress: SourceCrtModulePublishProgress::Request(request), failed_service: false })
    }
    pub fn progress(&self) -> SourceCrtModulePublishProgress { self.progress }
    pub fn actual_module_word(&self) -> u64 { self.actual_module }
    pub fn actual_exchanged_old_word(&self) -> Option<u64> { self.actual_old_word }
    pub fn request(&self) -> Option<SourceCrtModulePublishRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtModulePublishRequest> {
        match self.progress { SourceCrtModulePublishProgress::Request(request) => Some(request), _ => None }
    }
    pub(super) fn suspend_actual_prefix(&mut self) { self.failed_service = true; }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtModulePublishReceipt)
        -> Result<SourceCrtModulePublishProgress, SourceCrtModulePublishError> {
        self.failed_service = true;
        match (self.progress, actual) {
            (SourceCrtModulePublishProgress::Request(SourceCrtModulePublishRequest::ExchangeModuleCache { actual_address, desired_word }),
                SourceCrtModulePublishReceipt::Exchanged { actual_address: address, desired_word: desired, actual_old_word })
                if actual_address == address && desired_word == desired => {
                self.actual_old_word = Some(actual_old_word);

                if actual_old_word == 0 { self.complete(actual_old_word, None); }
                else { self.progress = SourceCrtModulePublishProgress::Request(SourceCrtModulePublishRequest::CleanupAt59be73 { actual_rcx: self.actual_module }); }
            }
            (SourceCrtModulePublishProgress::Request(SourceCrtModulePublishRequest::CleanupAt59be73 { actual_rcx }),
                SourceCrtModulePublishReceipt::CleanupReturned { actual_rcx_argument, actual_return_rax,
                    actual_resume_rip, actual_rbx, actual_r15, actual_rsi, actual_rdi, actual_r13, actual_r14 })
                if actual_rcx_argument == actual_rcx => {
                if actual_resume_rip != self.actual_module_base.wrapping_add(0x59be79)
                    || actual_rbx != self.actual_module || actual_r15 != self.actual_module_base
                    || actual_rsi != u64::from(self.source_module_id) || actual_rdi != u64::from(self.call.selector_ecx)
                    || actual_r13 != self.call.actual_rdx || actual_r14 != u64::MAX {
                    return Err(SourceCrtModulePublishError::ActualContinuationRequired { actual });
                }
                let Some(old) = self.actual_old_word else {
                    return Err(SourceCrtModulePublishError::ReceiptRequired { pending: self.progress, actual });
                };
                self.complete(old, Some(actual_return_rax));
            }
            _ => return Err(SourceCrtModulePublishError::ReceiptRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtModulePublishServiceFailure<E>, SourceCrtModulePublishRejectedFailure<E>> {
        match self.progress {
            SourceCrtModulePublishProgress::Request(pending) => {
                self.failed_service = true; Ok(SourceCrtModulePublishServiceFailure { cause, pending })
            }
            SourceCrtModulePublishProgress::Complete(completed) => Err(SourceCrtModulePublishRejectedFailure { cause, completed }),
        }
    }
    fn complete(&mut self, actual_exchanged_old_word: u64, actual_cleanup_return_rax: Option<u64>) {
        self.progress = SourceCrtModulePublishProgress::Complete(SourceCrtModulePublishCompletion {
            call: self.call, source_module_id: self.source_module_id, actual_published_module: self.actual_module,
            actual_exchanged_old_word, actual_cleanup_return_rax,
            next_procedure: SourceCrtCachedResolverRequest::ProcedureAt59be7f {
                actual_module_rcx: self.actual_module, actual_name_rdx: self.call.actual_rdx,
                expected_r15: self.actual_module_base, expected_rdi: u64::from(self.call.selector_ecx), expected_r14: u64::MAX,
            },
        });
    }
}
