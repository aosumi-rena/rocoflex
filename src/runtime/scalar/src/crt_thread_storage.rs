use super::crt_native_thunk::{SourceCrtNativeThunk, SourceCrtNativeThunkEntry,
    SourceCrtNativeThunkReturnedToCaller, SourceCrtNativeThunkProgress, SourceCrtNativeThunkRequest,
    SourceCrtResolver59bd70Request};
use super::crt_owned_prefix::{SourceCrtOwnedPrefix, SourceCrtOwnedMemoryObservation,
    SourceCrtOwnedMemoryStep, SourceCrtOwnedMemoryError};
use super::crt_storage_native_return::SourceCrtNativeRetFrame;

impl<O, T, N, A> super::crt_native_thunk::SourceCrtOwnedThunkAdmission<O, T, N, A, SourceCrtStorageThunkAccepted> {
    pub fn into_owned_storage_prefix(self) -> SourceCrtOwnedPrefix<Self, SourceCrtThreadStorageProgress> {
        let progress = self.source.progress;
        SourceCrtOwnedPrefix::admitted(self, progress)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThreadStorageBindings {

    pub actual_callback_address: u64,

    pub actual_context_address: u64,
    pub actual_index_address: u64,

    pub actual_context_sentinel_address: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageCall {
    Allocate59bbe0 { callback_rcx: u64 },
    Set59bcb8 { index_ecx: u32, context_rdx: u64 },
    Release59bc28 { index_ecx: u32 },
}
impl SourceCrtThreadStorageCall {
    pub fn source_thunk_entry(self) -> SourceCrtNativeThunkEntry {
        match self {
            Self::Allocate59bbe0 { callback_rcx } => SourceCrtNativeThunkEntry::Allocate59bbe0 { callback_rcx },
            Self::Set59bcb8 { index_ecx, context_rdx } => SourceCrtNativeThunkEntry::Set59bcb8 { index_ecx, context_rdx },
            Self::Release59bc28 { index_ecx } => SourceCrtNativeThunkEntry::Release59bc28 { index_ecx },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageRequest {
    Native(SourceCrtThreadStorageCall),
    StoreIndex { actual_address: u64, value: u32 },
    StoreContextSentinel { actual_address: u64, value: u32 },

    ReadIndex { actual_address: u64 },

    OrIndex { actual_address: u64, mask: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageReceipt {
    NativeReturned { call: SourceCrtThreadStorageCall, actual_eax: u32 },
    IndexStored { actual_address: u64, value: u32 },
    ContextSentinelStored { actual_address: u64, value: u32 },
    IndexRead { actual_address: u64, value: u32 },
    IndexOrCompleted { actual_address: u64, mask: u32, actual_old_value: u32, actual_new_value: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageEntry { Initialize58a4d4, Cleanup58a51c }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThreadStoragePrefix {
    entry: SourceCrtThreadStorageEntry,
    bindings: SourceCrtThreadStorageBindings,
    allocation_return_eax: Option<u32>,
    index_store_completed: bool,
    set_return_eax: Option<u32>,
    context_sentinel_stored: bool,
    cleanup_index_read: Option<u32>,
    release_return_eax: Option<u32>,
    completed_index_or: Option<(u32, u32)>,
}
impl SourceCrtThreadStoragePrefix {
    pub fn entry(&self) -> SourceCrtThreadStorageEntry { self.entry }
    pub fn bindings(&self) -> SourceCrtThreadStorageBindings { self.bindings }
    pub fn allocation_return_eax(&self) -> Option<u32> { self.allocation_return_eax }
    pub fn index_store_completed(&self) -> bool { self.index_store_completed }
    pub fn set_return_eax(&self) -> Option<u32> { self.set_return_eax }
    pub fn context_sentinel_stored(&self) -> bool { self.context_sentinel_stored }
    pub fn cleanup_index_read(&self) -> Option<u32> { self.cleanup_index_read }
    pub fn release_return_eax(&self) -> Option<u32> { self.release_return_eax }
    pub fn completed_index_or(&self) -> Option<(u32, u32)> { self.completed_index_or }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtThreadStorageCompletion {
    prefix: SourceCrtThreadStoragePrefix,
    return_al: u8,
}
impl SourceCrtThreadStorageCompletion {
    pub fn prefix(&self) -> &SourceCrtThreadStoragePrefix { &self.prefix }
    pub fn return_al(&self) -> u8 { self.return_al }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageProgress {
    Request(SourceCrtThreadStorageRequest),
    Complete(SourceCrtThreadStorageCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtThreadStorageError {
    ContextBindingRequired { supplied: SourceCrtThreadStorageBindings },
    ReceiptBindingRequired { pending: SourceCrtThreadStorageProgress, actual: SourceCrtThreadStorageReceipt },
    NativeSourceCompletionRequired { expected: SourceCrtStorageThunkBinding, actual: SourceCrtThreadStorageReceipt },
}
impl SourceCrtThreadStorageError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ContextBindingRequired { .. } => "58a4d4 actual context sentinel must bind source context+78h",
            Self::ReceiptBindingRequired { .. } => "58a4d4/58a51c actual service/memory receipt does not match pending source operation",
            Self::NativeSourceCompletionRequired { .. } => "58a4d4/58a51c native thunk has an outstanding source cursor; matching source and actual return are required together",
        }
    }
}
#[must_use = "retain actual native resource/full-state failure with its cursor"]
pub struct SourceCrtThreadStorageServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtThreadStorageRequest,
}
pub struct SourceCrtThreadStorageRejectedFailure<E> {
    pub cause: E,
    pub completed: SourceCrtThreadStorageCompletion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageThunkBinding {
    pub actual_module_base: u64,
    pub call: SourceCrtThreadStorageCall,
    pub resolver: SourceCrtResolver59bd70Request,
    pub required_actual_resume_rip: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageThunkBeginError {
    pub pending: SourceCrtThreadStorageProgress,
    pub outstanding: Option<SourceCrtStorageThunkBinding>,
    pub actual_module_base: u64,
    pub actual_bindings: SourceCrtThreadStorageBindings,
}
impl SourceCrtStorageThunkBeginError {
    pub fn reason(&self) -> &'static str {
        "storage thunk handoff requires an unfailed pending native call and exact supplied source callback/context/index addresses"
    }
}

pub type SourceCrtStorageThunkReturned = SourceCrtNativeThunkReturnedToCaller;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageThunkCompletionError {
    pub pending: SourceCrtThreadStorageProgress,
    pub expected: Option<SourceCrtStorageThunkBinding>,
    pub actual: SourceCrtStorageThunkReturned,
    pub continuation: Option<SourceCrtThreadStorageError>,
}
impl SourceCrtStorageThunkCompletionError {
    pub fn reason(&self) -> &'static str {
        match self.continuation {
            Some(error) => error.reason(),
            None => "storage thunk completion and actual return do not bind the outstanding source call/base/resume/value",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageThunkAccepted {
    pub actual: SourceCrtStorageThunkReturned,
    pub progress: SourceCrtThreadStorageProgress,
}

pub struct SourceCrtThreadStorage58a4d4 {
    prefix: SourceCrtThreadStoragePrefix,
    progress: SourceCrtThreadStorageProgress,
    failed_service: bool,
    pending_native_source: Option<SourceCrtStorageThunkBinding>,
}
impl SourceCrtThreadStorage58a4d4 {
    pub fn begin_initialize(bindings: SourceCrtThreadStorageBindings) -> Result<Self, SourceCrtThreadStorageError> {
        Self::begin(SourceCrtThreadStorageEntry::Initialize58a4d4, bindings)
    }
    pub fn begin_cleanup(bindings: SourceCrtThreadStorageBindings) -> Result<Self, SourceCrtThreadStorageError> {
        Self::begin(SourceCrtThreadStorageEntry::Cleanup58a51c, bindings)
    }
    fn begin(entry: SourceCrtThreadStorageEntry, bindings: SourceCrtThreadStorageBindings)
        -> Result<Self, SourceCrtThreadStorageError>
    {
        if bindings.actual_context_address.checked_add(0x78) != Some(bindings.actual_context_sentinel_address) {
            return Err(SourceCrtThreadStorageError::ContextBindingRequired { supplied: bindings });
        }
        let request = match entry {
            SourceCrtThreadStorageEntry::Initialize58a4d4 => SourceCrtThreadStorageRequest::Native(
                SourceCrtThreadStorageCall::Allocate59bbe0 { callback_rcx: bindings.actual_callback_address }),
            SourceCrtThreadStorageEntry::Cleanup58a51c => SourceCrtThreadStorageRequest::ReadIndex {
                actual_address: bindings.actual_index_address },
        };
        Ok(Self { prefix: SourceCrtThreadStoragePrefix {
            entry, bindings, allocation_return_eax: None, index_store_completed: false, set_return_eax: None,
            context_sentinel_stored: false, cleanup_index_read: None, release_return_eax: None, completed_index_or: None,
        }, progress: SourceCrtThreadStorageProgress::Request(request), failed_service: false, pending_native_source: None })
    }
    pub fn prefix(&self) -> &SourceCrtThreadStoragePrefix { &self.prefix }
    pub fn progress(&self) -> SourceCrtThreadStorageProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtThreadStorageRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtThreadStorageRequest> {
        match self.progress { SourceCrtThreadStorageProgress::Request(request) => Some(request), _ => None }
    }
    pub fn pending_native_source(&self) -> Option<SourceCrtStorageThunkBinding> { self.pending_native_source }

    pub(super) fn stop_for_required_parent_return(&mut self,
        value: SourceCrtThreadStorageCompletion, held: SourceCrtThreadStorageProgress) -> bool {
        let valid = !self.failed_service && self.pending_native_source.is_none()
            && self.progress == SourceCrtThreadStorageProgress::Complete(value) && held == self.progress;
        self.failed_service = true;
        valid
    }

    pub fn into_completed_owned_prefix<C>(mut self,
        prefix: SourceCrtOwnedPrefix<C, SourceCrtThreadStorageProgress>)
        -> Result<super::crt_owned_prefix::SourceCrtOwnedControllerCompletion<Self, C,
                SourceCrtThreadStorageProgress, SourceCrtThreadStorageCompletion>,
            super::crt_owned_prefix::SourceCrtOwnedControllerCompletionError<Self, C, SourceCrtThreadStorageProgress>> {
        let pending = self.progress;
        let was_failed = self.failed_service;
        self.failed_service = true;
        if !was_failed && self.pending_native_source.is_none() && prefix.progress() == pending {
            if let SourceCrtThreadStorageProgress::Complete(value) = pending {
                self.failed_service = false;
                return Ok(super::crt_owned_prefix::SourceCrtOwnedControllerCompletion::admitted(self, prefix, value));
            }
        }
        Err(super::crt_owned_prefix::SourceCrtOwnedControllerCompletionError { controller: self, prefix, pending })
    }

    pub fn begin_pending_owned_native_thunk<C>(&mut self,
        previous: SourceCrtOwnedPrefix<C, SourceCrtThreadStorageProgress>, actual_module_base: u64)
        -> Result<super::crt_owned_prefix::SourceCrtOwnedPendingChild<C, SourceCrtThreadStorageProgress,
                SourceCrtNativeThunk>,
            super::crt_owned_prefix::SourceCrtOwnedChildBeginError<C, SourceCrtThreadStorageProgress,
                SourceCrtStorageThunkBeginError>> {
        let pending = self.progress;
        if previous.progress() != pending || self.failed_service || self.pending_native_source.is_some() {
            self.failed_service = true;
            return Err(super::crt_owned_prefix::SourceCrtOwnedChildBeginError {
                previous, pending, actual_module_base, continuation: None });
        }
        match self.begin_pending_native_thunk(actual_module_base) {
            Ok(source) => Ok(super::crt_owned_prefix::SourceCrtOwnedPendingChild { previous, source }),
            Err(cause) => {
                self.failed_service = true;
                Err(super::crt_owned_prefix::SourceCrtOwnedChildBeginError {
                    previous, pending, actual_module_base, continuation: Some(cause) })
            }
        }
    }
    pub fn begin_pending_native_thunk(&mut self, actual_module_base: u64)
        -> Result<SourceCrtNativeThunk, SourceCrtStorageThunkBeginError> {
        let b = self.prefix.bindings;
        let reject = SourceCrtStorageThunkBeginError { pending: self.progress,
            outstanding: self.pending_native_source, actual_module_base, actual_bindings: b };
        if self.failed_service || self.pending_native_source.is_some()
            || b.actual_callback_address != actual_module_base.wrapping_add(0x58a6bc)
            || b.actual_context_address != actual_module_base.wrapping_add(0x861520)
            || b.actual_context_sentinel_address != actual_module_base.wrapping_add(0x861598)
            || b.actual_index_address != actual_module_base.wrapping_add(0x8364b0) { return Err(reject); }
        let SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::Native(call)) = self.progress
            else { return Err(reject); };
        let thunk = SourceCrtNativeThunk::begin(actual_module_base, call.source_thunk_entry());
        let SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Resolve(resolver)) = thunk.progress()
            else { return Err(reject); };
        let resume_rva = match call {
            SourceCrtThreadStorageCall::Allocate59bbe0 { .. } => 0x58a4e4,
            SourceCrtThreadStorageCall::Set59bcb8 { .. } => 0x58a4fd,
            SourceCrtThreadStorageCall::Release59bc28 { .. } => 0x58a530,
        };
        self.pending_native_source = Some(SourceCrtStorageThunkBinding { actual_module_base, call, resolver,
            required_actual_resume_rip: actual_module_base.wrapping_add(resume_rva) });
        self.failed_service = true;
        Ok(thunk)
    }
    pub fn accept_completed_native_thunk(&mut self, actual: SourceCrtStorageThunkReturned)
        -> Result<SourceCrtStorageThunkAccepted, SourceCrtStorageThunkCompletionError> {
        self.failed_service = true;
        let mut reject = SourceCrtStorageThunkCompletionError { pending: self.progress,
            expected: self.pending_native_source, actual, continuation: None };
        let Some(expected) = self.pending_native_source else { return Err(reject); };
        if self.progress != SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::Native(expected.call))
            || actual.source.entry() != expected.call.source_thunk_entry()
            || actual.source.resolver_request() != expected.resolver
            || actual.actual_resume_rip != expected.required_actual_resume_rip
            || actual.actual_eax != actual.source.actual_return_rax() as u32 { return Err(reject); }
        self.pending_native_source = None;
        match self.accept_actual_completion(SourceCrtThreadStorageReceipt::NativeReturned {
            call: expected.call, actual_eax: actual.actual_eax }) {
            Ok(progress) => Ok(SourceCrtStorageThunkAccepted { actual, progress }),
            Err(error) => {
                self.pending_native_source = Some(expected);
                reject.continuation = Some(error);
                Err(reject)
            }
        }
    }

    pub fn accept_completed_owned_native_thunk<O, T, N, A>(&mut self,
        actual: super::crt_native_thunk::SourceCrtThunkOwnedReturned<O, T, N, A>,
        project_frame: fn(&O) -> super::crt_storage_native_return::SourceCrtNativeRetFrame,
        project_return: fn(&O, &A) -> Option<super::crt_native_thunk::SourceCrtThunkActualCallerReturn>)
        -> Result<super::crt_native_thunk::SourceCrtOwnedThunkAdmission<O, T, N, A, SourceCrtStorageThunkAccepted>,
            super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError<O, T, N, A, SourceCrtStorageThunkCompletionError>> {
        self.failed_service = true;
        let actual_frame = project_frame(&actual.actual.actual.owner);
        let observed_return = project_return(&actual.actual.actual.owner, &actual.actual_return);
        if actual_frame != actual.completed().actual_caller_frame || observed_return != Some(actual.completed()) {
            return Err(super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError {
                actual, actual_frame, observed_return, continuation: None });
        }
        match self.accept_completed_native_thunk(actual.source()) {
            Ok(source) => Ok(super::crt_native_thunk::SourceCrtOwnedThunkAdmission { actual, source }),
            Err(cause) => Err(super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError {
                actual, actual_frame, observed_return, continuation: Some(cause) }),
        }
    }

    pub fn accept_actual_owned_memory_completion<C, A>(&mut self,
        previous: SourceCrtOwnedPrefix<C, SourceCrtThreadStorageProgress>, actual: A,
        supplied: SourceCrtOwnedMemoryObservation<SourceCrtThreadStorageReceipt>,
        project_frame: fn(&C, &A) -> SourceCrtNativeRetFrame,
        project_receipt: fn(&C, &A) -> Option<SourceCrtOwnedMemoryObservation<SourceCrtThreadStorageReceipt>>)
        -> Result<SourceCrtOwnedPrefix<SourceCrtOwnedMemoryStep<C, A, SourceCrtThreadStorageReceipt,
                SourceCrtThreadStorageProgress>, SourceCrtThreadStorageProgress>,
            SourceCrtOwnedMemoryError<C, A, SourceCrtThreadStorageReceipt, SourceCrtThreadStorageProgress,
                SourceCrtThreadStorageError>> {
        self.failed_service = true;
        let pending = self.progress;
        let actual_frame = project_frame(&previous.owner, &actual);
        let observed = project_receipt(&previous.owner, &actual);
        if previous.progress() != pending || self.pending_native_source.is_some()
            || matches!(supplied.receipt, SourceCrtThreadStorageReceipt::NativeReturned { .. })
            || observed != Some(supplied) || actual_frame != supplied.actual_frame {
            return Err(SourceCrtOwnedMemoryError { previous, actual, supplied, observed,
                actual_frame, pending, continuation: None });
        }
        match self.accept_actual_completion(supplied.receipt) {
            Ok(progress) => Ok(SourceCrtOwnedPrefix::admitted(SourceCrtOwnedMemoryStep {
                previous, actual, completed: supplied }, progress)),
            Err(cause) => Err(SourceCrtOwnedMemoryError { previous, actual, supplied, observed,
                actual_frame, pending, continuation: Some(cause) }),
        }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtThreadStorageReceipt)
        -> Result<SourceCrtThreadStorageProgress, SourceCrtThreadStorageError>
    {
        self.failed_service = true;
        if let Some(expected) = self.pending_native_source {
            return Err(SourceCrtThreadStorageError::NativeSourceCompletionRequired { expected, actual });
        }
        match (self.progress, actual) {
            (SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::Native(expected)),
                SourceCrtThreadStorageReceipt::NativeReturned { call, actual_eax }) if call == expected => {
                match call {
                    SourceCrtThreadStorageCall::Allocate59bbe0 { .. } => {
                        self.prefix.allocation_return_eax = Some(actual_eax);
                        self.next(SourceCrtThreadStorageRequest::StoreIndex {
                            actual_address: self.prefix.bindings.actual_index_address, value: actual_eax });
                    }
                    SourceCrtThreadStorageCall::Set59bcb8 { .. } => {
                        self.prefix.set_return_eax = Some(actual_eax);
                        if actual_eax != 0 {
                            self.next(SourceCrtThreadStorageRequest::StoreContextSentinel {
                                actual_address: self.prefix.bindings.actual_context_sentinel_address, value: 0xffff_fffe });
                        } else { self.cleanup(); }
                    }
                    SourceCrtThreadStorageCall::Release59bc28 { .. } => {
                        self.prefix.release_return_eax = Some(actual_eax);
                        self.next(SourceCrtThreadStorageRequest::OrIndex {
                            actual_address: self.prefix.bindings.actual_index_address, mask: u32::MAX });
                    }
                }
            }
            (SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::StoreIndex { actual_address, value }),
                SourceCrtThreadStorageReceipt::IndexStored { actual_address: address, value: stored })
                if address == actual_address && value == stored => {
                self.prefix.index_store_completed = true;
                if stored == u32::MAX { self.complete(0); }
                else { self.next(SourceCrtThreadStorageRequest::Native(SourceCrtThreadStorageCall::Set59bcb8 {
                    index_ecx: stored, context_rdx: self.prefix.bindings.actual_context_address })); }
            }
            (SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::StoreContextSentinel { actual_address, value }),
                SourceCrtThreadStorageReceipt::ContextSentinelStored { actual_address: address, value: stored })
                if address == actual_address && value == stored => {
                self.prefix.context_sentinel_stored = true;
                self.complete(1);
            }
            (SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::ReadIndex { actual_address }),
                SourceCrtThreadStorageReceipt::IndexRead { actual_address: address, value }) if address == actual_address => {
                self.prefix.cleanup_index_read = Some(value);
                if value == u32::MAX { self.complete_cleanup(); }
                else { self.next(SourceCrtThreadStorageRequest::Native(SourceCrtThreadStorageCall::Release59bc28 { index_ecx: value })); }
            }
            (SourceCrtThreadStorageProgress::Request(SourceCrtThreadStorageRequest::OrIndex { actual_address, mask }),
                SourceCrtThreadStorageReceipt::IndexOrCompleted { actual_address: address, mask: actual_mask, actual_old_value, actual_new_value })
                if address == actual_address && mask == actual_mask && actual_old_value | mask == actual_new_value => {
                self.prefix.completed_index_or = Some((actual_old_value, actual_new_value));
                self.complete_cleanup();
            }
            _ => return Err(SourceCrtThreadStorageError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtThreadStorageServiceFailure<E>, SourceCrtThreadStorageRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtThreadStorageProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtThreadStorageServiceFailure { cause, pending })
            }
            SourceCrtThreadStorageProgress::Complete(completed) => Err(SourceCrtThreadStorageRejectedFailure { cause, completed }),
        }
    }
    fn next(&mut self, request: SourceCrtThreadStorageRequest) { self.progress = SourceCrtThreadStorageProgress::Request(request); }
    fn cleanup(&mut self) { self.next(SourceCrtThreadStorageRequest::ReadIndex { actual_address: self.prefix.bindings.actual_index_address }); }
    fn complete_cleanup(&mut self) {
        self.complete(if self.prefix.entry == SourceCrtThreadStorageEntry::Initialize58a4d4 { 0 } else { 1 });
    }
    fn complete(&mut self, return_al: u8) {
        self.progress = SourceCrtThreadStorageProgress::Complete(SourceCrtThreadStorageCompletion { prefix: self.prefix, return_al });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageCallbackReturn { NullArgument, SameFallbackContext }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageCallbackPlan {

    ReturnWithoutFree(SourceCrtStorageCallbackReturn),

    RequiredFree580c24 { actual_rcx: u64 },
}

pub fn plan_source_storage_callback_58a6bc(actual_argument_rcx: u64, actual_fallback_context: u64)
    -> SourceCrtStorageCallbackPlan
{
    if actual_argument_rcx == 0 {
        SourceCrtStorageCallbackPlan::ReturnWithoutFree(SourceCrtStorageCallbackReturn::NullArgument)
    } else if actual_argument_rcx == actual_fallback_context {
        SourceCrtStorageCallbackPlan::ReturnWithoutFree(SourceCrtStorageCallbackReturn::SameFallbackContext)
    } else { SourceCrtStorageCallbackPlan::RequiredFree580c24 { actual_rcx: actual_argument_rcx } }
}
