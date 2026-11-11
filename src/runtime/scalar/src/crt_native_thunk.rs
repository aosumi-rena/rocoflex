use super::crt_cached_resolver::{SourceCrtCachedResolver59bd70, SourceCrtCachedResolverCompletion,
    SourceCrtCachedResolverError};
use super::crt_cached_native_return::SourceCrtCachedResolverReturned;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, validate_source_crt_native_ret};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtResolverCachePlan {
    ReturnRax(u64),

    RequiredMissContinuation,
}

pub fn plan_source_crt_resolver_cache_word(actual_word: u64) -> SourceCrtResolverCachePlan {
    match actual_word {
        u64::MAX => SourceCrtResolverCachePlan::ReturnRax(0),
        0 => SourceCrtResolverCachePlan::RequiredMissContinuation,
        word => SourceCrtResolverCachePlan::ReturnRax(word),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeThunkEntry {
    Allocate59bbe0 { callback_rcx: u64 },
    Release59bc28 { index_ecx: u32 },
    Set59bcb8 { index_ecx: u32, context_rdx: u64 },
    InitializeLock59bd0c { array_rcx: u64, spin_edx: u32, flags_r8d: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtResolver59bd70Request {
    pub selector_ecx: u32,
    pub actual_rdx: u64,
    pub actual_r8: u64,
    pub actual_r9: u64,
    pub actual_function_cache_address: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtArgumentOverrides {
    pub rcx: Option<u64>,
    pub rdx: Option<u64>,
    pub r8: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtDispatchStyle { TailJumpAfterEpilogue, CallThenEpilogue }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtGuard77e920Request {
    pub actual_target_rax: u64,
    pub overrides: SourceCrtArgumentOverrides,
    pub style: SourceCrtDispatchStyle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThunkFallbackRequest {
    pub source_entry_rva: u32,
    pub known_prefix_overrides: SourceCrtArgumentOverrides,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeThunkRequest {
    Resolve(SourceCrtResolver59bd70Request),

    Dispatch(SourceCrtGuard77e920Request),

    RequiredFallback(SourceCrtThunkFallbackRequest),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeThunkReceipt {
    Resolved { request: SourceCrtResolver59bd70Request, actual_rax: u64,
        actual_rbx: u64, actual_rsi: u64, actual_rdi: u64 },
    Dispatched { request: SourceCrtGuard77e920Request, actual_return_rax: u64 },
    ActualFallbackReturned { request: SourceCrtThunkFallbackRequest, actual_return_rax: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeThunkCompletion {
    entry: SourceCrtNativeThunkEntry,
    resolver_request: SourceCrtResolver59bd70Request,
    resolver_return_rax: u64,
    actual_return_rax: u64,
}
impl SourceCrtNativeThunkCompletion {
    pub fn entry(&self) -> SourceCrtNativeThunkEntry { self.entry }
    pub fn resolver_request(&self) -> SourceCrtResolver59bd70Request { self.resolver_request }
    pub fn resolver_return_rax(&self) -> u64 { self.resolver_return_rax }
    pub fn actual_return_rax(&self) -> u64 { self.actual_return_rax }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeThunkReturnedToCaller {
    pub source: SourceCrtNativeThunkCompletion,
    pub actual_resume_rip: u64,
    pub actual_eax: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeThunkProgress {
    Request(SourceCrtNativeThunkRequest),
    Complete(SourceCrtNativeThunkCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeThunkError {
    ReceiptBindingRequired { pending: SourceCrtNativeThunkProgress, actual: SourceCrtNativeThunkReceipt },
    ActualNonvolatileContinuationRequired { entry: SourceCrtNativeThunkEntry, actual: SourceCrtNativeThunkReceipt },
    ResolverSourceCompletionRequired { expected: SourceCrtResolver59bd70Request, actual: SourceCrtNativeThunkReceipt },
}
impl SourceCrtNativeThunkError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ReceiptBindingRequired { .. } => "CRT native thunk actual resolver/guard/fallback return does not bind its pending request",
            Self::ActualNonvolatileContinuationRequired { .. } => "CRT resolver actual return changed source saved argument registers",
            Self::ResolverSourceCompletionRequired { .. } => "CRT thunk resolver has an outstanding source cursor; matching source and actual return are required together",
        }
    }
}
#[must_use = "retain actual native resolver/guard/fallback resources and advanced state"]
pub struct SourceCrtNativeThunkServiceFailure<E> { pub cause: E, pub pending: SourceCrtNativeThunkRequest }
pub struct SourceCrtNativeThunkRejectedFailure<E> { pub cause: E, pub completed: SourceCrtNativeThunkCompletion }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThunkResolverBeginError {
    PendingResolverRequired { actual_module_base: u64, pending: SourceCrtNativeThunkProgress,
        source_outstanding: bool },
    Begin(SourceCrtCachedResolverError),
}
impl SourceCrtThunkResolverBeginError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::PendingResolverRequired { .. } => "CRT thunk requires a pending resolver with no failure or outstanding source cursor",
            Self::Begin(error) => error.reason(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThunkResolverReturned {
    pub source: SourceCrtCachedResolverCompletion,
    pub actual_return: SourceCrtNativeThunkReceipt,
    pub actual_resume_rip: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThunkResolverCompletionError {
    pub pending: SourceCrtNativeThunkProgress,
    pub source_outstanding: bool,
    pub expected_resume_rip: u64,
    pub actual: SourceCrtThunkResolverReturned,
    pub continuation: Option<SourceCrtNativeThunkError>,
}
impl SourceCrtThunkResolverCompletionError {
    pub fn reason(&self) -> &'static str {
        match self.continuation {
            Some(error) => error.reason(),
            None => "CRT resolver source completion and actual whole-return packet do not bind the outstanding thunk request",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThunkResolverAccepted {
    pub actual: SourceCrtThunkResolverReturned,
    pub progress: SourceCrtNativeThunkProgress,
}
#[must_use = "retain SAME module/cache/native/frame/FP/raw-RET owners on rejected thunk adoption"]
pub struct SourceCrtThunkOwnedResolverError<O, T, N> {
    pub actual: SourceCrtCachedResolverReturned<O, T, N>,
    pub actual_receipt: SourceCrtNativeThunkReceipt,
    pub observed_receipt: Option<SourceCrtNativeThunkReceipt>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub cause: Option<SourceCrtThunkResolverCompletionError>,
}
impl<O, T, N> SourceCrtThunkOwnedResolverError<O, T, N> {
    pub fn reason(&self) -> &'static str {
        match &self.cause {
            Some(cause) => cause.reason(),
            None => "owned resolver RET, actual thunk target/frame/full-RAX and SAME owned return receipt must match",
        }
    }
}
#[must_use = "retain accepted module/cache/native/frame/FP/raw-RET owners for required next guard/fallback"]
pub struct SourceCrtThunkOwnedResolverAccepted<O, T, N> {
    pub actual: SourceCrtCachedResolverReturned<O, T, N>,
    pub source: SourceCrtThunkResolverAccepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThunkActualCallerReturn {
    pub receipt: SourceCrtNativeThunkReceipt,
    pub actual_caller_frame: SourceCrtNativeRetFrame,
}
#[must_use = "retain entire module/cache/native/frame/FP capsule and actual return resources"]
pub struct SourceCrtThunkOwnedReturnError<O, T, N, A> {
    pub actual: SourceCrtThunkOwnedResolverAccepted<O, T, N>,
    pub actual_return: A,
    pub supplied: SourceCrtThunkActualCallerReturn,
    pub observed: Option<SourceCrtThunkActualCallerReturn>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub pending: SourceCrtNativeThunkProgress,
    pub continuation: Option<SourceCrtNativeThunkError>,
}
impl<O, T, N, A> SourceCrtThunkOwnedReturnError<O, T, N, A> {
    pub fn reason(&self) -> &'static str {
        match self.continuation {
            Some(error) => error.reason(),
            None => "owned thunk requires SAME accepted resolver, pending guard/fallback and actual whole caller-return prefix",
        }
    }
}

#[must_use = "retain owned thunk/module/cache/native/frame/FP and caller return resources"]
pub struct SourceCrtThunkOwnedReturned<O, T, N, A> {
    pub actual: SourceCrtThunkOwnedResolverAccepted<O, T, N>,
    pub actual_return: A,
    source: SourceCrtNativeThunkReturnedToCaller,
    completed: SourceCrtThunkActualCallerReturn,
}
impl<O, T, N, A> SourceCrtThunkOwnedReturned<O, T, N, A> {
    pub fn source(&self) -> SourceCrtNativeThunkReturnedToCaller { self.source }
    pub fn completed(&self) -> SourceCrtThunkActualCallerReturn { self.completed }

    pub fn matches_actual_prefix(&self,
        project_frame: fn(&O) -> SourceCrtNativeRetFrame,
        project_return: fn(&O, &A) -> Option<SourceCrtThunkActualCallerReturn>) -> bool {
        project_frame(&self.actual.actual.owner) == self.completed.actual_caller_frame
            && project_return(&self.actual.actual.owner, &self.actual_return) == Some(self.completed)
    }
}
#[must_use = "retain entire actual child capsule when storage/lock admission rejects"]
pub struct SourceCrtOwnedThunkAdmissionError<O, T, N, A, E> {
    pub actual: SourceCrtThunkOwnedReturned<O, T, N, A>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_return: Option<SourceCrtThunkActualCallerReturn>,

    pub continuation: Option<E>,
}
#[must_use = "retain accepted physical/native/frame/FP owners alongside parent progress"]
pub struct SourceCrtOwnedThunkAdmission<O, T, N, A, S> {
    pub actual: SourceCrtThunkOwnedReturned<O, T, N, A>,
    pub(super) source: S,
}
impl<O, T, N, A, S> SourceCrtOwnedThunkAdmission<O, T, N, A, S> {
    pub fn source(&self) -> &S { &self.source }
}

pub struct SourceCrtNativeThunk {
    actual_module_base: u64,
    entry: SourceCrtNativeThunkEntry,
    resolver: SourceCrtResolver59bd70Request,
    resolver_return: Option<u64>,
    progress: SourceCrtNativeThunkProgress,
    failed_service: bool,
    resolver_source_outstanding: bool,
}
impl SourceCrtNativeThunk {
    pub fn begin(actual_module_base: u64, entry: SourceCrtNativeThunkEntry) -> Self {
        let (selector, rdx_rva, r8_rva, r9_rva) = match entry {
            SourceCrtNativeThunkEntry::Allocate59bbe0 { .. } => (0, 0x77b848, 0x60b728, 0x60b730),
            SourceCrtNativeThunkEntry::Release59bc28 { .. } => (1, 0x77b5d8, 0x60b730, 0x60b738),
            SourceCrtNativeThunkEntry::Set59bcb8 { .. } => (3, 0x77b360, 0x60b740, 0x60b748),
            SourceCrtNativeThunkEntry::InitializeLock59bd0c { .. } => (4, 0x77ac90, 0x60b748, 0x60b750),
        };

        let resolver = SourceCrtResolver59bd70Request { selector_ecx: selector,
            actual_rdx: actual_module_base.wrapping_add(rdx_rva), actual_r8: actual_module_base.wrapping_add(r8_rva),
            actual_r9: actual_module_base.wrapping_add(r9_rva),
            actual_function_cache_address: actual_module_base.wrapping_add(0x861cd8 + u64::from(selector) * 8) };
        Self { actual_module_base, entry, resolver, resolver_return: None, progress: SourceCrtNativeThunkProgress::Request(
            SourceCrtNativeThunkRequest::Resolve(resolver)), failed_service: false, resolver_source_outstanding: false }
    }
    pub fn entry(&self) -> SourceCrtNativeThunkEntry { self.entry }
    pub fn resolver_return_rax(&self) -> Option<u64> { self.resolver_return }
    pub fn progress(&self) -> SourceCrtNativeThunkProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtNativeThunkRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtNativeThunkRequest> {
        match self.progress { SourceCrtNativeThunkProgress::Request(request) => Some(request), _ => None }
    }
    pub fn has_pending_resolver_source(&self) -> bool { self.resolver_source_outstanding }
    pub fn required_resolver_return_rip(&self) -> u64 {
        let rva = match self.entry {
            SourceCrtNativeThunkEntry::Allocate59bbe0 { .. } => 0x59bc05,
            SourceCrtNativeThunkEntry::Release59bc28 { .. } => 0x59bc4f,
            SourceCrtNativeThunkEntry::Set59bcb8 { .. } => 0x59bce6,
            SourceCrtNativeThunkEntry::InitializeLock59bd0c { .. } => 0x59bd42,
        };
        self.actual_module_base.wrapping_add(rva)
    }

    pub fn begin_pending_cached_resolver(&mut self, actual_module_base: u64)
        -> Result<SourceCrtCachedResolver59bd70, SourceCrtThunkResolverBeginError> {
        if self.failed_service || self.resolver_source_outstanding
            || self.progress != SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Resolve(self.resolver)) {
            return Err(SourceCrtThunkResolverBeginError::PendingResolverRequired {
                actual_module_base, pending: self.progress, source_outstanding: self.resolver_source_outstanding });
        }

        let source = SourceCrtCachedResolver59bd70::begin(actual_module_base, self.resolver)
            .map_err(SourceCrtThunkResolverBeginError::Begin)?;
        self.resolver_source_outstanding = true;
        self.failed_service = true;
        Ok(source)
    }
    pub fn accept_completed_cached_resolver(&mut self, actual: SourceCrtThunkResolverReturned)
        -> Result<SourceCrtThunkResolverAccepted, SourceCrtThunkResolverCompletionError> {
        self.failed_service = true;
        let mut reject = SourceCrtThunkResolverCompletionError { pending: self.progress,
            source_outstanding: self.resolver_source_outstanding,
            expected_resume_rip: self.required_resolver_return_rip(), actual, continuation: None };
        if !self.resolver_source_outstanding || actual.source.call() != self.resolver
            || actual.actual_resume_rip != self.required_resolver_return_rip()
            || self.progress != SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Resolve(self.resolver)) {
            return Err(reject);
        }
        let SourceCrtNativeThunkReceipt::Resolved { request, actual_rax, .. } = actual.actual_return
            else { return Err(reject); };
        if request != self.resolver || actual_rax != actual.source.return_rax() { return Err(reject); }
        self.resolver_source_outstanding = false;
        match self.accept_actual_completion(actual.actual_return) {
            Ok(progress) => Ok(SourceCrtThunkResolverAccepted { actual, progress }),
            Err(error) => {
                self.resolver_source_outstanding = true;
                reject.continuation = Some(error);
                Err(reject)
            }
        }
    }

    pub fn accept_completed_owned_cached_resolver<O, T, N>(&mut self,
        actual: SourceCrtCachedResolverReturned<O, T, N>, actual_receipt: SourceCrtNativeThunkReceipt,
        project_frame: fn(&O) -> SourceCrtNativeRetFrame,
        project_receipt: fn(&O) -> Option<SourceCrtNativeThunkReceipt>)
        -> Result<SourceCrtThunkOwnedResolverAccepted<O, T, N>, SourceCrtThunkOwnedResolverError<O, T, N>> {
        self.failed_service = true;
        let actual_frame = project_frame(&actual.owner);
        let observed_receipt = project_receipt(&actual.owner);
        if actual_frame != actual.actual_retired.after || observed_receipt != Some(actual_receipt)
            || validate_source_crt_native_ret(actual.expected, actual.actual_retired).is_err()
            || actual.actual_call_frame.actual_module_base != self.actual_module_base
            || actual.actual_call_frame.actual_entry_rip != self.actual_module_base.wrapping_add(0x59bd70)
            || actual.actual_call_frame.actual_call != actual.source.call()
            || actual.expected.before.actual_rip != self.actual_module_base.wrapping_add(0x59bebd)
            || actual.expected.before.actual_rsp != actual.actual_call_frame.actual_return_slot_address
            || actual.expected.before.actual_rax != actual.source.return_rax()
            || actual.expected.actual_required_target != self.required_resolver_return_rip()
            || actual.actual_retired.after.actual_rip != self.required_resolver_return_rip()
            || actual.actual_retired.after.actual_rax != actual.source.return_rax() {
            return Err(SourceCrtThunkOwnedResolverError { actual, actual_receipt, observed_receipt, actual_frame, cause: None });
        }
        let value = SourceCrtThunkResolverReturned { source: actual.source, actual_return: actual_receipt,
            actual_resume_rip: actual.actual_retired.after.actual_rip };
        match self.accept_completed_cached_resolver(value) {
            Ok(source) => Ok(SourceCrtThunkOwnedResolverAccepted { actual, source }),
            Err(cause) => Err(SourceCrtThunkOwnedResolverError {
                actual, actual_receipt, observed_receipt, actual_frame, cause: Some(cause) }),
        }
    }

    pub fn accept_completed_owned_native_return<O, T, N, A>(&mut self,
        actual: SourceCrtThunkOwnedResolverAccepted<O, T, N>, actual_return: A,
        supplied: SourceCrtThunkActualCallerReturn,
        project_frame: fn(&O) -> SourceCrtNativeRetFrame,
        project_return: fn(&O, &A) -> Option<SourceCrtThunkActualCallerReturn>)
        -> Result<SourceCrtThunkOwnedReturned<O, T, N, A>, SourceCrtThunkOwnedReturnError<O, T, N, A>> {
        self.failed_service = true;
        let actual_frame = project_frame(&actual.actual.owner);
        let observed = project_return(&actual.actual.owner, &actual_return);
        let pending = self.progress;
        let expected_value = match (pending, supplied.receipt) {
            (SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Dispatch(expected)),
                SourceCrtNativeThunkReceipt::Dispatched { request, actual_return_rax }) if request == expected => Some(actual_return_rax),
            (SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::RequiredFallback(expected)),
                SourceCrtNativeThunkReceipt::ActualFallbackReturned { request, actual_return_rax }) if request == expected => Some(actual_return_rax),
            _ => None,
        };
        if self.resolver_source_outstanding || actual.source.progress != pending
            || actual.actual.actual_call_frame.actual_module_base != self.actual_module_base
            || actual.actual.source != actual.source.actual.source
            || actual.source.actual.source.call() != self.resolver
            || actual.source.actual.actual_resume_rip != self.required_resolver_return_rip()
            || self.resolver_return != Some(actual.actual.source.return_rax())
            || actual_frame != supplied.actual_caller_frame || observed != Some(supplied)
            || expected_value != Some(actual_frame.actual_rax) {
            return Err(SourceCrtThunkOwnedReturnError { actual, actual_return, supplied, observed,
                actual_frame, pending, continuation: None });
        }
        match self.accept_actual_completion(supplied.receipt) {
            Ok(SourceCrtNativeThunkProgress::Complete(source)) => {
                let source = SourceCrtNativeThunkReturnedToCaller { source,
                    actual_resume_rip: actual_frame.actual_rip, actual_eax: actual_frame.actual_rax as u32 };
                Ok(SourceCrtThunkOwnedReturned { actual, actual_return, source, completed: supplied })
            }
            Err(cause) => Err(SourceCrtThunkOwnedReturnError { actual, actual_return, supplied, observed,
                actual_frame, pending, continuation: Some(cause) }),
            Ok(_) => {

                self.failed_service = true;
                Err(SourceCrtThunkOwnedReturnError { actual, actual_return, supplied, observed,
                    actual_frame, pending, continuation: None })
            }
        }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtNativeThunkReceipt)
        -> Result<SourceCrtNativeThunkProgress, SourceCrtNativeThunkError>
    {
        self.failed_service = true;
        if self.resolver_source_outstanding {
            return Err(SourceCrtNativeThunkError::ResolverSourceCompletionRequired { expected: self.resolver, actual });
        }
        match (self.progress, actual) {
            (SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Resolve(expected)),
                SourceCrtNativeThunkReceipt::Resolved { request, actual_rax, actual_rbx, actual_rsi, actual_rdi }) if request == expected => {
                let preserved = match self.entry {
                    SourceCrtNativeThunkEntry::Allocate59bbe0 { callback_rcx } => actual_rbx == callback_rcx,
                    SourceCrtNativeThunkEntry::Release59bc28 { index_ecx } => actual_rbx == u64::from(index_ecx),
                    SourceCrtNativeThunkEntry::Set59bcb8 { index_ecx, context_rdx } => actual_rbx == context_rdx && actual_rdi == u64::from(index_ecx),
                    SourceCrtNativeThunkEntry::InitializeLock59bd0c { array_rcx, spin_edx, flags_r8d } =>
                        actual_rbx == u64::from(spin_edx) && actual_rsi == u64::from(flags_r8d) && actual_rdi == array_rcx,
                };
                if !preserved { return Err(SourceCrtNativeThunkError::ActualNonvolatileContinuationRequired { entry: self.entry, actual }); }
                self.resolver_return = Some(actual_rax);
                let (dispatch, fallback_rva, fallback) = source_argument_overrides(self.entry);
                self.progress = SourceCrtNativeThunkProgress::Request(if actual_rax != 0 {
                    let style = match self.entry {
                        SourceCrtNativeThunkEntry::Allocate59bbe0 { .. } | SourceCrtNativeThunkEntry::Release59bc28 { .. } => SourceCrtDispatchStyle::TailJumpAfterEpilogue,
                        _ => SourceCrtDispatchStyle::CallThenEpilogue,
                    };
                    SourceCrtNativeThunkRequest::Dispatch(SourceCrtGuard77e920Request { actual_target_rax: actual_rax, overrides: dispatch, style })
                } else { SourceCrtNativeThunkRequest::RequiredFallback(SourceCrtThunkFallbackRequest {
                    source_entry_rva: fallback_rva, known_prefix_overrides: fallback }) });
            }
            (SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Dispatch(expected)),
                SourceCrtNativeThunkReceipt::Dispatched { request, actual_return_rax }) if request == expected => self.complete(actual_return_rax),
            (SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::RequiredFallback(expected)),
                SourceCrtNativeThunkReceipt::ActualFallbackReturned { request, actual_return_rax }) if request == expected => self.complete(actual_return_rax),
            _ => return Err(SourceCrtNativeThunkError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtNativeThunkServiceFailure<E>, SourceCrtNativeThunkRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtNativeThunkProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtNativeThunkServiceFailure { cause, pending })
            }
            SourceCrtNativeThunkProgress::Complete(completed) => Err(SourceCrtNativeThunkRejectedFailure { cause, completed }),
        }
    }
    fn complete(&mut self, actual_return_rax: u64) {
        if let Some(resolver_return_rax) = self.resolver_return {
            self.progress = SourceCrtNativeThunkProgress::Complete(SourceCrtNativeThunkCompletion {
                entry: self.entry, resolver_request: self.resolver, resolver_return_rax, actual_return_rax });
        }
    }
}

fn source_argument_overrides(entry: SourceCrtNativeThunkEntry)
    -> (SourceCrtArgumentOverrides, u32, SourceCrtArgumentOverrides)
{
    let none = SourceCrtArgumentOverrides { rcx: None, rdx: None, r8: None };
    match entry {
        SourceCrtNativeThunkEntry::Allocate59bbe0 { callback_rcx } =>
            (SourceCrtArgumentOverrides { rcx: Some(callback_rcx), ..none }, 0x59bc19, none),
        SourceCrtNativeThunkEntry::Release59bc28 { index_ecx } => {
            let args = SourceCrtArgumentOverrides { rcx: Some(u64::from(index_ecx)), ..none };
            (args, 0x59bc62, args)
        }
        SourceCrtNativeThunkEntry::Set59bcb8 { index_ecx, context_rdx } => {
            let args = SourceCrtArgumentOverrides { rcx: Some(u64::from(index_ecx)), rdx: Some(context_rdx), ..none };
            (args, 0x59bcf8, args)
        }
        SourceCrtNativeThunkEntry::InitializeLock59bd0c { array_rcx, spin_edx, flags_r8d } => {
            let prefix = SourceCrtArgumentOverrides { rcx: Some(array_rcx), rdx: Some(u64::from(spin_edx)), ..none };

            (SourceCrtArgumentOverrides { r8: Some(u64::from(flags_r8d)), ..prefix }, 0x59bd57, prefix)
        }
    }
}
