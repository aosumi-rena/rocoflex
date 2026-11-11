use super::crt_native_thunk::{plan_source_crt_resolver_cache_word,
    SourceCrtResolverCachePlan, SourceCrtResolver59bd70Request};
use super::crt_module_load::{SourceCrtModuleLoad59bdfb, SourceCrtModuleLoadEntry,
    SourceCrtModuleLoadBeginError, SourceCrtModuleLoadCompletion, SourceCrtModuleLoadOutcome,
    SourceCrtModuleLoadPublicationAccepted, SourceCrtModuleLoadProgress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedResolverRequest {
    FunctionCache { actual_address: u64 },
    ModuleId { actual_address: u64 },
    ModuleCache { source_module_id: u32, actual_address: u64 },

    RequiredModuleLoad { source_module_id: u32, actual_module_name_pointer_address: u64 },

    RequiredNativeModuleLoad59bdf5 { source_module_id: u32, actual_name_rcx: u64, edx: u32, r8d: u32 },

    ProcedureAt59be7f { actual_module_rcx: u64, actual_name_rdx: u64,
        expected_r15: u64, expected_rdi: u64, expected_r14: u64 },

    ExchangeFunctionCache { actual_address: u64, desired_word: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedResolverReceipt {
    FunctionCache { actual_address: u64, actual_word: u64 },
    ModuleId { actual_address: u64, value: u32 },
    ModuleCache { source_module_id: u32, actual_address: u64, actual_word: u64 },

    ModuleNamePointer { source_module_id: u32, actual_address: u64, actual_word: u64 },
    ProcedureReturned { actual_module_rcx: u64, actual_name_rdx: u64, actual_rax: u64,
        actual_r15: u64, actual_rdi: u64, actual_r14: u64 },
    FunctionCacheExchanged { actual_address: u64, desired_word: u64, actual_old_word: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCachedResolverCompletion {
    call: SourceCrtResolver59bd70Request,
    return_rax: u64,
    function_cache_read: u64,
    last_module_id: Option<u32>,
    skipped_unavailable_modules: u64,
    actual_selected_module: Option<u64>,
    procedure_return_rax: Option<u64>,
    published_word: Option<u64>,
    exchanged_old_word: Option<u64>,
}
impl SourceCrtCachedResolverCompletion {
    pub fn call(&self) -> SourceCrtResolver59bd70Request { self.call }
    pub fn return_rax(&self) -> u64 { self.return_rax }
    pub fn function_cache_read(&self) -> u64 { self.function_cache_read }
    pub fn last_module_id(&self) -> Option<u32> { self.last_module_id }
    pub fn skipped_unavailable_modules(&self) -> u64 { self.skipped_unavailable_modules }
    pub fn actual_selected_module(&self) -> Option<u64> { self.actual_selected_module }
    pub fn procedure_return_rax(&self) -> Option<u64> { self.procedure_return_rax }
    pub fn published_word(&self) -> Option<u64> { self.published_word }
    pub fn exchanged_old_word(&self) -> Option<u64> { self.exchanged_old_word }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedResolverProgress {
    Request(SourceCrtCachedResolverRequest),
    Complete(SourceCrtCachedResolverCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedResolverError {
    FunctionCacheBindingRequired { actual_module_base: u64, supplied: SourceCrtResolver59bd70Request },
    ReceiptBindingRequired { pending: SourceCrtCachedResolverProgress, actual: SourceCrtCachedResolverReceipt },
    ActualNonvolatileContinuationRequired { pending: SourceCrtCachedResolverRequest, actual: SourceCrtCachedResolverReceipt },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedResolverLoadEntryError {
    EntryBindingRequired { pending: SourceCrtCachedResolverProgress,
        expected_suffix_entry: Option<SourceCrtModuleLoadEntry>, supplied: SourceCrtModuleLoadEntry },
    Begin(SourceCrtModuleLoadBeginError),
}
impl SourceCrtCachedResolverLoadEntryError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::EntryBindingRequired { .. } => "module-load suffix entry differs from the pending source binding or a suffix is already outstanding",
            Self::Begin(error) => error.reason(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCachedResolverLoadCompletionError {
    pub pending: SourceCrtCachedResolverProgress,
    pub expected_suffix_entry: Option<SourceCrtModuleLoadEntry>,
    pub actual: SourceCrtModuleLoadCompletion,
}
impl SourceCrtCachedResolverLoadCompletionError {
    pub fn reason(&self) -> &'static str { "module-load suffix completion differs from the outstanding source-bound native/load continuation" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCachedResolverLoadAccepted {
    pub actual_suffix: SourceCrtModuleLoadCompletion,
    pub progress: SourceCrtCachedResolverProgress,
}
#[must_use = "retain loaded-module/cache/native/frame/FP owners on rejected resolver adoption"]
pub enum SourceCrtCachedResolverOwnedLoadError<M, R, P, S> {
    PublishedSuffixRequired { pending: SourceCrtCachedResolverProgress,
        actual: SourceCrtModuleLoadPublicationAccepted<M, R, P, S> },
    Source { actual: SourceCrtModuleLoadPublicationAccepted<M, R, P, S>,
        cause: SourceCrtCachedResolverLoadCompletionError },
}
impl<M, R, P, S> SourceCrtCachedResolverOwnedLoadError<M, R, P, S> {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::PublishedSuffixRequired { .. } => "owned module-load capsule must retain the SAME completed publication in its completed load suffix",
            Self::Source { cause, .. } => cause.reason(),
        }
    }
}
#[must_use = "retain the full accepted module/cache/native/frame/FP capsule for the required next procedure call"]
pub struct SourceCrtCachedResolverOwnedLoadAccepted<M, R, P, S> {
    pub actual: SourceCrtModuleLoadPublicationAccepted<M, R, P, S>,
    pub source: SourceCrtCachedResolverLoadAccepted,
}
impl SourceCrtCachedResolverError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::FunctionCacheBindingRequired { .. } => "59bd70 supplied function-cache address differs from actual base+861cd8+8*zeroextended selector",
            Self::ReceiptBindingRequired { .. } => "59bd70 cached-module actual memory/procedure/atomic receipt does not bind pending source operation",
            Self::ActualNonvolatileContinuationRequired { .. } => "59bd70 actual procedure return changed source cache-base/index/sentinel registers",
        }
    }
}
#[must_use = "retain actual resolver/native/memory resources and partial cursor"]
pub struct SourceCrtCachedResolverServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtCachedResolverRequest,
}
pub struct SourceCrtCachedResolverRejectedFailure<E> {
    pub cause: E,
    pub completed: SourceCrtCachedResolverCompletion,
}

pub struct SourceCrtCachedResolver59bd70 {
    actual_module_base: u64,
    call: SourceCrtResolver59bd70Request,
    next_module_pointer: u64,
    function_cache_read: Option<u64>,
    last_module_id: Option<u32>,
    skipped_unavailable_modules: u64,
    actual_selected_module: Option<u64>,
    actual_module_name_pointer: Option<u64>,
    procedure_return: Option<u64>,
    progress: SourceCrtCachedResolverProgress,
    failed_service: bool,
    module_load_suffix_entry: Option<SourceCrtModuleLoadEntry>,
}
impl SourceCrtCachedResolver59bd70 {
    pub fn begin(actual_module_base: u64, call: SourceCrtResolver59bd70Request) -> Result<Self, SourceCrtCachedResolverError> {
        let expected = actual_module_base.wrapping_add(0x861cd8 + u64::from(call.selector_ecx) * 8);
        if expected != call.actual_function_cache_address {
            return Err(SourceCrtCachedResolverError::FunctionCacheBindingRequired { actual_module_base, supplied: call });
        }
        Ok(Self { actual_module_base, call, next_module_pointer: call.actual_r8, function_cache_read: None,
            last_module_id: None, skipped_unavailable_modules: 0, actual_selected_module: None, actual_module_name_pointer: None, procedure_return: None,
            progress: SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::FunctionCache { actual_address: expected }),
            failed_service: false, module_load_suffix_entry: None })
    }
    pub fn call(&self) -> SourceCrtResolver59bd70Request { self.call }
    pub fn progress(&self) -> SourceCrtCachedResolverProgress { self.progress }
    pub fn function_cache_read(&self) -> Option<u64> { self.function_cache_read }
    pub fn next_module_pointer(&self) -> u64 { self.next_module_pointer }
    pub fn last_module_id(&self) -> Option<u32> { self.last_module_id }
    pub fn skipped_unavailable_modules(&self) -> u64 { self.skipped_unavailable_modules }
    pub fn actual_selected_module(&self) -> Option<u64> { self.actual_selected_module }
    pub fn actual_module_name_pointer(&self) -> Option<u64> { self.actual_module_name_pointer }
    pub fn procedure_return_rax(&self) -> Option<u64> { self.procedure_return }
    pub fn request(&self) -> Option<SourceCrtCachedResolverRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtCachedResolverRequest> {
        match self.progress { SourceCrtCachedResolverProgress::Request(request) => Some(request), _ => None }
    }
    pub fn has_pending_module_load_suffix(&self) -> bool { self.module_load_suffix_entry.is_some() }
    pub fn pending_module_load_entry(&self) -> Option<SourceCrtModuleLoadEntry> { self.module_load_suffix_entry }
    pub fn begin_pending_module_load_from_actual_return(&mut self, supplied: SourceCrtModuleLoadEntry)
        -> Result<SourceCrtModuleLoad59bdfb, SourceCrtCachedResolverLoadEntryError> {

        self.failed_service = true;
        if self.module_load_suffix_entry.is_some() || !self.binds_module_load_entry(supplied) {
            return Err(SourceCrtCachedResolverLoadEntryError::EntryBindingRequired {
                pending: self.progress, expected_suffix_entry: self.module_load_suffix_entry, supplied });
        }
        let suffix = SourceCrtModuleLoad59bdfb::begin_from_actual_return(supplied)
            .map_err(SourceCrtCachedResolverLoadEntryError::Begin)?;
        self.module_load_suffix_entry = Some(supplied);
        Ok(suffix)
    }
    pub fn accept_completed_module_load(&mut self, actual: SourceCrtModuleLoadCompletion)
        -> Result<SourceCrtCachedResolverLoadAccepted, SourceCrtCachedResolverLoadCompletionError> {
        self.failed_service = true;
        let reject = SourceCrtCachedResolverLoadCompletionError { pending: self.progress,
            expected_suffix_entry: self.module_load_suffix_entry, actual };
        if self.module_load_suffix_entry != Some(actual.entry()) || !self.binds_module_load_entry(actual.entry()) { return Err(reject); }
        match actual.outcome() {
            SourceCrtModuleLoadOutcome::Published(published) => {
                let expected_next = SourceCrtCachedResolverRequest::ProcedureAt59be7f {
                    actual_module_rcx: published.actual_published_module(), actual_name_rdx: self.call.actual_rdx,
                    expected_r15: self.actual_module_base, expected_rdi: u64::from(self.call.selector_ecx), expected_r14: u64::MAX };
                if published.call() != self.call || Some(published.source_module_id()) != self.last_module_id
                    || published.next_procedure() != expected_next { return Err(reject); }
                self.actual_selected_module = Some(published.actual_published_module());
                self.next(expected_next);
            }
            SourceCrtModuleLoadOutcome::Unavailable { actual_next_metadata_pointer, next_resolver_request, .. } => {
                let expected_next_pointer = self.next_module_pointer.wrapping_add(4);
                let expected_request = if expected_next_pointer == self.call.actual_r9 {
                    SourceCrtCachedResolverRequest::ExchangeFunctionCache {
                        actual_address: self.call.actual_function_cache_address, desired_word: u64::MAX }
                } else { SourceCrtCachedResolverRequest::ModuleId { actual_address: expected_next_pointer } };
                if actual_next_metadata_pointer != expected_next_pointer || next_resolver_request != expected_request { return Err(reject); }
                self.skipped_unavailable_modules = self.skipped_unavailable_modules.wrapping_add(1);
                self.next_module_pointer = expected_next_pointer;

                self.next_candidate();
            }
        }
        self.module_load_suffix_entry = None;
        self.failed_service = false;
        Ok(SourceCrtCachedResolverLoadAccepted { actual_suffix: actual, progress: self.progress })
    }

    pub fn accept_completed_owned_module_load<M, R, P, S>(&mut self,
        actual: SourceCrtModuleLoadPublicationAccepted<M, R, P, S>)
        -> Result<SourceCrtCachedResolverOwnedLoadAccepted<M, R, P, S>, SourceCrtCachedResolverOwnedLoadError<M, R, P, S>> {
        self.failed_service = true;
        let SourceCrtModuleLoadProgress::Complete(suffix) = actual.progress else {
            return Err(SourceCrtCachedResolverOwnedLoadError::PublishedSuffixRequired { pending: self.progress, actual });
        };
        if !matches!(suffix.outcome(), SourceCrtModuleLoadOutcome::Published(publication)
            if publication == actual.actual.source
                && publication.actual_published_module() == actual.actual.actual_module_word.get()) {
            return Err(SourceCrtCachedResolverOwnedLoadError::PublishedSuffixRequired { pending: self.progress, actual });
        }
        match self.accept_completed_module_load(suffix) {
            Ok(source) => Ok(SourceCrtCachedResolverOwnedLoadAccepted { actual, source }),
            Err(cause) => Err(SourceCrtCachedResolverOwnedLoadError::Source { actual, cause }),
        }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtCachedResolverReceipt)
        -> Result<SourceCrtCachedResolverProgress, SourceCrtCachedResolverError>
    {

        self.failed_service = true;
        match (self.progress, actual) {
            (SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::FunctionCache { actual_address }),
                SourceCrtCachedResolverReceipt::FunctionCache { actual_address: address, actual_word }) if address == actual_address => {
                self.function_cache_read = Some(actual_word);
                match plan_source_crt_resolver_cache_word(actual_word) {
                    SourceCrtResolverCachePlan::ReturnRax(value) => self.complete(value, None, None),
                    SourceCrtResolverCachePlan::RequiredMissContinuation => self.next_candidate(),
                }
            }
            (SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::ModuleId { actual_address }),
                SourceCrtCachedResolverReceipt::ModuleId { actual_address: address, value }) if address == actual_address => {
                self.last_module_id = Some(value);
                self.next(SourceCrtCachedResolverRequest::ModuleCache { source_module_id: value,
                    actual_address: self.actual_module_base.wrapping_add(0x861cc0 + u64::from(value) * 8) });
            }
            (SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::ModuleCache { source_module_id, actual_address }),
                SourceCrtCachedResolverReceipt::ModuleCache { source_module_id: id, actual_address: address, actual_word })
                if id == source_module_id && address == actual_address => {
                if actual_word == u64::MAX {
                    self.skipped_unavailable_modules = self.skipped_unavailable_modules.wrapping_add(1);
                    self.next_module_pointer = self.next_module_pointer.wrapping_add(4);
                    self.next_candidate();
                } else if actual_word == 0 {
                    self.next(SourceCrtCachedResolverRequest::RequiredModuleLoad { source_module_id,
                        actual_module_name_pointer_address: self.actual_module_base.wrapping_add(0x60b710 + u64::from(source_module_id) * 8) });
                } else {
                    self.actual_selected_module = Some(actual_word);
                    self.next(SourceCrtCachedResolverRequest::ProcedureAt59be7f {
                        actual_module_rcx: actual_word, actual_name_rdx: self.call.actual_rdx,
                        expected_r15: self.actual_module_base, expected_rdi: u64::from(self.call.selector_ecx), expected_r14: u64::MAX });
                }
            }
            (SourceCrtCachedResolverProgress::Request(pending @ SourceCrtCachedResolverRequest::ProcedureAt59be7f {
                actual_module_rcx, actual_name_rdx, expected_r15, expected_rdi, expected_r14 }),
                SourceCrtCachedResolverReceipt::ProcedureReturned { actual_module_rcx: module, actual_name_rdx: name, actual_rax,
                    actual_r15, actual_rdi, actual_r14 })
                if module == actual_module_rcx && name == actual_name_rdx => {
                if (actual_r15, actual_rdi, actual_r14) != (expected_r15, expected_rdi, expected_r14) {
                    return Err(SourceCrtCachedResolverError::ActualNonvolatileContinuationRequired { pending, actual });
                }
                self.procedure_return = Some(actual_rax);
                self.publish(if actual_rax == 0 { u64::MAX } else { actual_rax });
            }
            (SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::RequiredModuleLoad {
                source_module_id, actual_module_name_pointer_address }),
                SourceCrtCachedResolverReceipt::ModuleNamePointer { source_module_id: id, actual_address, actual_word })
                if id == source_module_id && actual_address == actual_module_name_pointer_address => {

                self.actual_module_name_pointer = Some(actual_word);
                self.next(SourceCrtCachedResolverRequest::RequiredNativeModuleLoad59bdf5 {
                    source_module_id, actual_name_rcx: actual_word, edx: 0, r8d: 0x800 });
            }
            (SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::ExchangeFunctionCache { actual_address, desired_word }),
                SourceCrtCachedResolverReceipt::FunctionCacheExchanged { actual_address: address, desired_word: desired, actual_old_word })
                if address == actual_address && desired == desired_word => {

                let returned = match self.procedure_return { Some(value) => value, None => 0 };
                self.complete(returned, Some(desired), Some(actual_old_word));
            }
            _ => return Err(SourceCrtCachedResolverError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtCachedResolverServiceFailure<E>, SourceCrtCachedResolverRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtCachedResolverProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtCachedResolverServiceFailure { cause, pending })
            }
            SourceCrtCachedResolverProgress::Complete(completed) => Err(SourceCrtCachedResolverRejectedFailure { cause, completed }),
        }
    }
    fn next(&mut self, request: SourceCrtCachedResolverRequest) { self.progress = SourceCrtCachedResolverProgress::Request(request); }
    fn binds_module_load_entry(&self, supplied: SourceCrtModuleLoadEntry) -> bool {
        let SourceCrtCachedResolverProgress::Request(SourceCrtCachedResolverRequest::RequiredNativeModuleLoad59bdf5 {
            source_module_id, actual_name_rcx, edx, r8d }) = self.progress else { return false; };
        edx == 0 && r8d == 0x800 && supplied.actual_module_base == self.actual_module_base
            && supplied.call == self.call && supplied.source_module_id == source_module_id
            && Some(source_module_id) == self.last_module_id
            && supplied.actual_metadata_pointer == self.next_module_pointer
            && supplied.actual_module_name_pointer == actual_name_rcx
            && Some(actual_name_rcx) == self.actual_module_name_pointer
    }
    fn next_candidate(&mut self) {

        if self.next_module_pointer == self.call.actual_r9 { self.publish(u64::MAX); }
        else { self.next(SourceCrtCachedResolverRequest::ModuleId { actual_address: self.next_module_pointer }); }
    }
    fn publish(&mut self, desired_word: u64) { self.next(SourceCrtCachedResolverRequest::ExchangeFunctionCache {
        actual_address: self.call.actual_function_cache_address, desired_word }); }
    fn complete(&mut self, return_rax: u64, published_word: Option<u64>, exchanged_old_word: Option<u64>) {
        if let Some(function_cache_read) = self.function_cache_read {
            self.progress = SourceCrtCachedResolverProgress::Complete(SourceCrtCachedResolverCompletion {
                call: self.call, return_rax, function_cache_read, last_module_id: self.last_module_id,
                skipped_unavailable_modules: self.skipped_unavailable_modules, actual_selected_module: self.actual_selected_module,
                procedure_return_rax: self.procedure_return, published_word, exchanged_old_word });
        }
    }
}
