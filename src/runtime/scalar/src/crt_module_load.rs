use super::crt_cached_resolver::SourceCrtCachedResolverRequest;
use super::crt_native_thunk::{SourceCrtArgumentOverrides, SourceCrtResolver59bd70Request};
use super::crt_module_publish::{SourceCrtModulePublish59be59, SourceCrtModulePublishCompletion,
    SourceCrtModulePublishError, SourceCrtModulePublishProgress, SourceCrtModulePublishReceipt,
    SourceCrtModulePublishRequest};
use super::crt_module_compare::{SourceCrtModuleCompare593e68, SourceCrtModuleCompareCompletion,
    SourceCrtModuleCompareError, SourceCrtModuleCompareLive};
use super::crt_module_publish_owned::SourceCrtModulePublicationCompletion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadSaved {
    pub rbx: u64, pub rbp: u64, pub rsi: u64, pub rdi: u64,
    pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadEntry {
    pub actual_module_base: u64,
    pub call: SourceCrtResolver59bd70Request,
    pub source_module_id: u32,
    pub actual_metadata_pointer: u64,
    pub actual_module_name_pointer: u64,
    pub actual_resume_rip: u64,
    pub actual_return_rax: u64,
    pub actual_saved: SourceCrtModuleLoadSaved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadNativeKind { Status59be03, Compare593e68, Retry59be2d }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadNativeCall {
    pub kind: SourceCrtModuleLoadNativeKind,

    pub overrides: SourceCrtArgumentOverrides,
    pub required_actual_resume_rip: u64,
    pub required_actual_saved: SourceCrtModuleLoadSaved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadNativeReceipt {
    pub call: SourceCrtModuleLoadNativeCall,
    pub actual_return_rax: u64,
    pub actual_resume_rip: u64,
    pub actual_saved: SourceCrtModuleLoadSaved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadRequest {
    Native(SourceCrtModuleLoadNativeCall),
    ExchangeUnavailableModuleCache { actual_address: u64, desired_word: u64 },

    Publication(SourceCrtModulePublishRequest),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadReceipt {
    Native(SourceCrtModuleLoadNativeReceipt),
    UnavailableExchanged { actual_address: u64, desired_word: u64, actual_old_word: u64 },
    Publication(SourceCrtModulePublishReceipt),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceCrtModuleLoadHistory {
    pub status_return: Option<SourceCrtModuleLoadNativeReceipt>,
    pub compare_return: Option<SourceCrtModuleLoadNativeReceipt>,
    pub retry_return: Option<SourceCrtModuleLoadNativeReceipt>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadOutcome {
    Published(SourceCrtModulePublishCompletion),
    Unavailable { actual_exchanged_old_word: u64, actual_next_metadata_pointer: u64,
        next_resolver_request: SourceCrtCachedResolverRequest },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadCompletion {
    entry: SourceCrtModuleLoadEntry,
    history: SourceCrtModuleLoadHistory,
    outcome: SourceCrtModuleLoadOutcome,
}
impl SourceCrtModuleLoadCompletion {
    pub fn entry(&self) -> SourceCrtModuleLoadEntry { self.entry }
    pub fn history(&self) -> SourceCrtModuleLoadHistory { self.history }
    pub fn outcome(&self) -> SourceCrtModuleLoadOutcome { self.outcome }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadProgress {
    Request(SourceCrtModuleLoadRequest),
    Complete(SourceCrtModuleLoadCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadBeginReason {
    CallBindingRequired, ActualResumeRequired, ActualSavedContinuationRequired,
    Publication(SourceCrtModulePublishError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadBeginError {
    pub entry: SourceCrtModuleLoadEntry,
    pub reason: SourceCrtModuleLoadBeginReason,
}
impl SourceCrtModuleLoadBeginError {
    pub fn reason(&self) -> &'static str {
        match self.reason {
            SourceCrtModuleLoadBeginReason::CallBindingRequired => "59bdfb entry needs the same source-bound function-cache/base/selector",
            SourceCrtModuleLoadBeginReason::ActualResumeRequired => "59bdfb entry needs its explicitly supplied actual native-return point",
            SourceCrtModuleLoadBeginReason::ActualSavedContinuationRequired => "59bdfb entry needs the actual saved source continuation registers",
            SourceCrtModuleLoadBeginReason::Publication(error) => error.reason(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadError {
    ReceiptRequired { pending: SourceCrtModuleLoadProgress, actual: SourceCrtModuleLoadReceipt },
    CompareSuffixCompletionRequired { expected: SourceCrtModuleLoadNativeCall, actual: SourceCrtModuleLoadReceipt },
    OwnedPublicationCompletionRequired { expected: SourceCrtModuleLoadPublicationBinding, actual: SourceCrtModuleLoadReceipt },
    ActualNativeContinuationRequired { pending: SourceCrtModuleLoadNativeCall, actual: SourceCrtModuleLoadNativeReceipt },
    Publication(SourceCrtModulePublishError),
}
impl SourceCrtModuleLoadError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ReceiptRequired { .. } => "59bdfb suffix receipt differs from its pending operation",
            Self::CompareSuffixCompletionRequired { .. } => "59bdfb comparison has an outstanding source cursor; its matching reads and actual return must complete together",
            Self::OwnedPublicationCompletionRequired { .. } => "59bdfb publication cursor is held with actual module/cache resources; matching owned completion is required",
            Self::ActualNativeContinuationRequired { .. } => "59bdfb suffix requires actual saved-register/resume continuation",
            Self::Publication(error) => error.reason(),
        }
    }
}
#[must_use = "retain actual native/module/atomic/full-state resources and pending cursor"]
pub struct SourceCrtModuleLoadServiceFailure<E> { pub cause: E, pub pending: SourceCrtModuleLoadRequest }
pub struct SourceCrtModuleLoadRejectedFailure<E> { pub cause: E, pub completed: SourceCrtModuleLoadCompletion }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleLoadCompareBeginError {
    PendingCompareRequired { pending: SourceCrtModuleLoadProgress,
        outstanding: Option<SourceCrtModuleLoadNativeCall> },
    Begin(SourceCrtModuleCompareError),
}
impl SourceCrtModuleLoadCompareBeginError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::PendingCompareRequired { .. } => "59bdfb comparison handoff requires one pending593e68 call and no outstanding comparison cursor",
            Self::Begin(error) => error.reason(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleCompareReturned {
    pub source: SourceCrtModuleCompareCompletion,
    pub actual_return: SourceCrtModuleLoadNativeReceipt,
    pub actual_live_at_return: SourceCrtModuleCompareLive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadCompareCompletionError {
    pub pending: SourceCrtModuleLoadProgress,
    pub expected_call: Option<SourceCrtModuleLoadNativeCall>,
    pub actual: SourceCrtModuleCompareReturned,
    pub continuation: Option<SourceCrtModuleLoadError>,
}
impl SourceCrtModuleLoadCompareCompletionError {
    pub fn reason(&self) -> &'static str {
        match self.continuation {
            Some(error) => error.reason(),
            None => "593e68 fresh-read completion and actual return differ from the outstanding source-bound comparison",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadCompareAccepted {
    pub actual: SourceCrtModuleCompareReturned,
    pub progress: SourceCrtModuleLoadProgress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadPublicationBinding {
    pub entry: SourceCrtModuleLoadEntry,
    pub actual_module_word: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleLoadPublicationBeginError {
    pub pending: SourceCrtModuleLoadProgress,
    pub outstanding: Option<SourceCrtModuleLoadPublicationBinding>,
}
impl SourceCrtModuleLoadPublicationBeginError {
    pub fn reason(&self) -> &'static str { "module publication handoff requires one unfailed initial SAME publication cursor" }
}
#[must_use = "retain actual loaded module/cache/native/frame/FP owners on rejected parent adoption"]
pub struct SourceCrtModuleLoadPublicationCompletionError<M, R, P, S> {
    pub pending: SourceCrtModuleLoadProgress,
    pub expected: Option<SourceCrtModuleLoadPublicationBinding>,
    pub actual: SourceCrtModulePublicationCompletion<M, R, P, S>,
}
impl<M, R, P, S> SourceCrtModuleLoadPublicationCompletionError<M, R, P, S> {
    pub fn reason(&self) -> &'static str { "owned publication completion differs from the outstanding module-load source binding" }
}
#[must_use = "retain the accepted actual module/cache/native/frame/FP resource capsule"]
pub struct SourceCrtModuleLoadPublicationAccepted<M, R, P, S> {
    pub actual: SourceCrtModulePublicationCompletion<M, R, P, S>,
    pub progress: SourceCrtModuleLoadProgress,
}

pub struct SourceCrtModuleLoad59bdfb {
    entry: SourceCrtModuleLoadEntry,
    history: SourceCrtModuleLoadHistory,
    publication: Option<SourceCrtModulePublish59be59>,
    progress: SourceCrtModuleLoadProgress,
    failed_service: bool,
    compare_suffix_call: Option<SourceCrtModuleLoadNativeCall>,
    owned_publication_binding: Option<SourceCrtModuleLoadPublicationBinding>,
}
impl SourceCrtModuleLoad59bdfb {
    pub fn begin_from_actual_return(entry: SourceCrtModuleLoadEntry) -> Result<Self, SourceCrtModuleLoadBeginError> {
        let fail = |reason| SourceCrtModuleLoadBeginError { entry, reason };
        let expected_cache = entry.actual_module_base.wrapping_add(0x861cd8 + u64::from(entry.call.selector_ecx) * 8);
        if entry.call.actual_function_cache_address != expected_cache {
            return Err(fail(SourceCrtModuleLoadBeginReason::CallBindingRequired));
        }
        if entry.actual_resume_rip != entry.actual_module_base.wrapping_add(0x59bdfb) {
            return Err(fail(SourceCrtModuleLoadBeginReason::ActualResumeRequired));
        }

        let expected = SourceCrtModuleLoadSaved { rbx: entry.actual_saved.rbx,
            rbp: entry.actual_metadata_pointer, rsi: u64::from(entry.source_module_id),
            rdi: u64::from(entry.call.selector_ecx), r12: entry.call.actual_r9,
            r13: entry.call.actual_rdx, r14: u64::MAX, r15: entry.actual_module_name_pointer };
        if entry.actual_saved != expected {
            return Err(fail(SourceCrtModuleLoadBeginReason::ActualSavedContinuationRequired));
        }
        let initial = SourceCrtModuleLoadRequest::Native(Self::native_call(entry, SourceCrtModuleLoadNativeKind::Status59be03));
        let mut cursor = Self { entry, history: SourceCrtModuleLoadHistory::default(), publication: None,
            progress: SourceCrtModuleLoadProgress::Request(initial), failed_service: false, compare_suffix_call: None,
            owned_publication_binding: None };
        if entry.actual_return_rax != 0 {
            cursor.begin_publication(entry.actual_return_rax)
                .map_err(|error| fail(SourceCrtModuleLoadBeginReason::Publication(error)))?;
        }
        Ok(cursor)
    }
    pub fn entry(&self) -> SourceCrtModuleLoadEntry { self.entry }
    pub fn history(&self) -> SourceCrtModuleLoadHistory { self.history }
    pub fn progress(&self) -> SourceCrtModuleLoadProgress { self.progress }
    pub fn publication_progress(&self) -> Option<SourceCrtModulePublishProgress> {
        self.publication.as_ref().map(SourceCrtModulePublish59be59::progress)
    }
    pub fn request(&self) -> Option<SourceCrtModuleLoadRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtModuleLoadRequest> {
        match self.progress { SourceCrtModuleLoadProgress::Request(request) => Some(request), _ => None }
    }
    pub fn pending_compare_source_call(&self) -> Option<SourceCrtModuleLoadNativeCall> { self.compare_suffix_call }
    pub fn pending_owned_publication(&self) -> Option<SourceCrtModuleLoadPublicationBinding> { self.owned_publication_binding }

    pub fn take_pending_publication_source(&mut self)
        -> Result<SourceCrtModulePublish59be59, SourceCrtModuleLoadPublicationBeginError> {
        let reject = SourceCrtModuleLoadPublicationBeginError { pending: self.progress,
            outstanding: self.owned_publication_binding };
        if self.failed_service || self.owned_publication_binding.is_some() { return Err(reject); }
        let SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Publication(expected)) = self.progress
            else { return Err(reject); };
        let Some(publication) = self.publication.as_ref() else { return Err(reject); };
        if publication.request() != Some(expected) || publication.actual_exchanged_old_word().is_some()
            || !matches!(expected, SourceCrtModulePublishRequest::ExchangeModuleCache { .. }) { return Err(reject); }
        let binding = SourceCrtModuleLoadPublicationBinding { entry: self.entry,
            actual_module_word: publication.actual_module_word() };
        let Some(publication) = self.publication.take() else { return Err(reject); };
        self.owned_publication_binding = Some(binding);
        self.failed_service = true;
        Ok(publication)
    }
    pub fn accept_completed_owned_publication<M, R, P, S>(&mut self,
        actual: SourceCrtModulePublicationCompletion<M, R, P, S>)
        -> Result<SourceCrtModuleLoadPublicationAccepted<M, R, P, S>, SourceCrtModuleLoadPublicationCompletionError<M, R, P, S>> {
        self.failed_service = true;
        let valid = match self.owned_publication_binding {
            None => false,
            Some(binding) => {
                let expected_request = SourceCrtModulePublishRequest::ExchangeModuleCache {
                    actual_address: self.entry.actual_module_base.wrapping_add(0x861cc0 + u64::from(self.entry.source_module_id) * 8),
                    desired_word: binding.actual_module_word };
                let expected_next = SourceCrtCachedResolverRequest::ProcedureAt59be7f {
                    actual_module_rcx: binding.actual_module_word, actual_name_rdx: self.entry.call.actual_rdx,
                    expected_r15: self.entry.actual_module_base, expected_rdi: u64::from(self.entry.call.selector_ecx), expected_r14: u64::MAX };
                binding.entry == self.entry
                    && self.progress == SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Publication(expected_request))
                    && actual.source.call() == self.entry.call && actual.source.source_module_id() == self.entry.source_module_id
                    && actual.source.actual_published_module() == binding.actual_module_word
                    && actual.actual_module_word.get() == binding.actual_module_word
                    && actual.source.next_procedure() == expected_next
            }
        };
        if !valid { return Err(SourceCrtModuleLoadPublicationCompletionError {
            pending: self.progress, expected: self.owned_publication_binding, actual }); }
        self.adopt_publication_progress(SourceCrtModulePublishProgress::Complete(actual.source));
        self.owned_publication_binding = None;
        self.failed_service = false;
        Ok(SourceCrtModuleLoadPublicationAccepted { actual, progress: self.progress })
    }

    pub fn begin_pending_compare_source(&mut self)
        -> Result<SourceCrtModuleCompare593e68, SourceCrtModuleLoadCompareBeginError> {
        let reject = SourceCrtModuleLoadCompareBeginError::PendingCompareRequired {
            pending: self.progress, outstanding: self.compare_suffix_call };
        if self.failed_service || self.compare_suffix_call.is_some() { return Err(reject); }
        let SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Native(call)) = self.progress
            else { return Err(reject); };
        if call.kind != SourceCrtModuleLoadNativeKind::Compare593e68 { return Err(reject); }
        self.failed_service = true;
        let source = SourceCrtModuleCompare593e68::begin_bound_module_call(self.entry.actual_module_base, call)
            .map_err(SourceCrtModuleLoadCompareBeginError::Begin)?;
        self.compare_suffix_call = Some(call);
        Ok(source)
    }
    pub fn accept_completed_compare_source(&mut self, actual: SourceCrtModuleCompareReturned)
        -> Result<SourceCrtModuleLoadCompareAccepted, SourceCrtModuleLoadCompareCompletionError> {
        self.failed_service = true;
        let mut reject = SourceCrtModuleLoadCompareCompletionError { pending: self.progress,
            expected_call: self.compare_suffix_call, actual, continuation: None };
        let Some(call) = self.compare_suffix_call else { return Err(reject); };
        if self.progress != SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Native(call))
            || actual.source.call() != call || actual.actual_return.call != call
            || actual.actual_live_at_return != actual.source.source_live_at_ret()
            || actual.actual_return.actual_return_rax != actual.source.source_live_at_ret().rax {
            return Err(reject);
        }

        self.compare_suffix_call = None;
        match self.accept_actual_completion(SourceCrtModuleLoadReceipt::Native(actual.actual_return)) {
            Ok(progress) => Ok(SourceCrtModuleLoadCompareAccepted { actual, progress }),
            Err(error) => {
                self.compare_suffix_call = Some(call);
                reject.continuation = Some(error);
                Err(reject)
            }
        }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtModuleLoadReceipt)
        -> Result<SourceCrtModuleLoadProgress, SourceCrtModuleLoadError> {

        self.failed_service = true;
        if let Some(expected) = self.compare_suffix_call {
            return Err(SourceCrtModuleLoadError::CompareSuffixCompletionRequired { expected, actual });
        }
        if let Some(expected) = self.owned_publication_binding {
            return Err(SourceCrtModuleLoadError::OwnedPublicationCompletionRequired { expected, actual });
        }
        match (self.progress, actual) {
            (SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Native(pending)),
                SourceCrtModuleLoadReceipt::Native(receipt)) if receipt.call == pending => {
                if receipt.actual_resume_rip != pending.required_actual_resume_rip
                    || receipt.actual_saved != pending.required_actual_saved {
                    return Err(SourceCrtModuleLoadError::ActualNativeContinuationRequired { pending, actual: receipt });
                }
                match pending.kind {
                    SourceCrtModuleLoadNativeKind::Status59be03 => {
                        self.history.status_return = Some(receipt);
                        if receipt.actual_return_rax as u32 == 0x57 {
                            self.next_native(SourceCrtModuleLoadNativeKind::Compare593e68);
                        } else { self.next_unavailable(); }
                    }
                    SourceCrtModuleLoadNativeKind::Compare593e68 => {
                        self.history.compare_return = Some(receipt);
                        if receipt.actual_return_rax as u32 == 0 { self.next_unavailable(); }
                        else { self.next_native(SourceCrtModuleLoadNativeKind::Retry59be2d); }
                    }
                    SourceCrtModuleLoadNativeKind::Retry59be2d => {
                        self.history.retry_return = Some(receipt);
                        if receipt.actual_return_rax == 0 { self.next_unavailable(); }
                        else { self.begin_publication(receipt.actual_return_rax).map_err(SourceCrtModuleLoadError::Publication)?; }
                    }
                }
            }
            (SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::ExchangeUnavailableModuleCache { actual_address, desired_word }),
                SourceCrtModuleLoadReceipt::UnavailableExchanged { actual_address: address, desired_word: desired, actual_old_word })
                if address == actual_address && desired == desired_word => {
                let next = self.entry.actual_metadata_pointer.wrapping_add(4);
                let next_resolver_request = if next == self.entry.call.actual_r9 {
                    SourceCrtCachedResolverRequest::ExchangeFunctionCache {
                        actual_address: self.entry.call.actual_function_cache_address, desired_word: u64::MAX }
                } else { SourceCrtCachedResolverRequest::ModuleId { actual_address: next } };
                self.complete(SourceCrtModuleLoadOutcome::Unavailable { actual_exchanged_old_word: actual_old_word,
                    actual_next_metadata_pointer: next, next_resolver_request });
            }
            (SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Publication(_)),
                SourceCrtModuleLoadReceipt::Publication(receipt)) => {
                let Some(publication) = &mut self.publication else {
                    return Err(SourceCrtModuleLoadError::ReceiptRequired { pending: self.progress, actual });
                };
                let progress = publication.accept_actual_completion(receipt).map_err(SourceCrtModuleLoadError::Publication)?;
                self.adopt_publication_progress(progress);
            }
            _ => return Err(SourceCrtModuleLoadError::ReceiptRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtModuleLoadServiceFailure<E>, SourceCrtModuleLoadRejectedFailure<E>> {
        match self.progress {
            SourceCrtModuleLoadProgress::Request(pending) => {
                self.failed_service = true; Ok(SourceCrtModuleLoadServiceFailure { cause, pending })
            }
            SourceCrtModuleLoadProgress::Complete(completed) => Err(SourceCrtModuleLoadRejectedFailure { cause, completed }),
        }
    }
    fn native_call(entry: SourceCrtModuleLoadEntry, kind: SourceCrtModuleLoadNativeKind) -> SourceCrtModuleLoadNativeCall {
        let mut saved = entry.actual_saved;
        saved.rbx = 0;
        let (overrides, resume_rva) = match kind {
            SourceCrtModuleLoadNativeKind::Status59be03 => (SourceCrtArgumentOverrides { rcx: None, rdx: None, r8: None }, 0x59be09),
            SourceCrtModuleLoadNativeKind::Compare593e68 => (SourceCrtArgumentOverrides {
                rcx: Some(entry.actual_module_name_pointer), rdx: Some(entry.actual_module_base.wrapping_add(0x77e3a0)),
                r8: Some((saved.rbx as u32).wrapping_add(7) as u64) }, 0x59be21),
            SourceCrtModuleLoadNativeKind::Retry59be2d => (SourceCrtArgumentOverrides {
                rcx: Some(entry.actual_module_name_pointer), rdx: Some(0), r8: Some(0) }, 0x59be33),
        };
        SourceCrtModuleLoadNativeCall { kind, overrides,
            required_actual_resume_rip: entry.actual_module_base.wrapping_add(resume_rva), required_actual_saved: saved }
    }
    fn next_native(&mut self, kind: SourceCrtModuleLoadNativeKind) {
        self.progress = SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Native(Self::native_call(self.entry, kind)));
    }
    fn next_unavailable(&mut self) {
        self.progress = SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::ExchangeUnavailableModuleCache {
            actual_address: self.entry.actual_module_base.wrapping_add(0x861cc0 + u64::from(self.entry.source_module_id) * 8), desired_word: u64::MAX });
    }
    fn begin_publication(&mut self, module: u64) -> Result<(), SourceCrtModulePublishError> {
        let publication = SourceCrtModulePublish59be59::begin_from_actual_entry(self.entry.actual_module_base,
            self.entry.call, self.entry.source_module_id, module)?;
        let progress = publication.progress();
        self.publication = Some(publication);
        self.adopt_publication_progress(progress);
        Ok(())
    }
    fn adopt_publication_progress(&mut self, progress: SourceCrtModulePublishProgress) {
        match progress {
            SourceCrtModulePublishProgress::Request(request) => self.progress = SourceCrtModuleLoadProgress::Request(SourceCrtModuleLoadRequest::Publication(request)),
            SourceCrtModulePublishProgress::Complete(completed) => self.complete(SourceCrtModuleLoadOutcome::Published(completed)),
        }
    }
    fn complete(&mut self, outcome: SourceCrtModuleLoadOutcome) {
        self.progress = SourceCrtModuleLoadProgress::Complete(SourceCrtModuleLoadCompletion { entry: self.entry, history: self.history, outcome });
    }
}
