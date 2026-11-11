use super::crt_native_thunk::{SourceCrtNativeThunk, SourceCrtNativeThunkEntry,
    SourceCrtNativeThunkReturnedToCaller, SourceCrtNativeThunkProgress, SourceCrtNativeThunkRequest,
    SourceCrtResolver59bd70Request};
use super::crt_owned_prefix::{SourceCrtOwnedPrefix, SourceCrtOwnedMemoryObservation,
    SourceCrtOwnedMemoryStep, SourceCrtOwnedMemoryError};
use super::crt_storage_native_return::SourceCrtNativeRetFrame;

impl<O, T, N, A> super::crt_native_thunk::SourceCrtOwnedThunkAdmission<O, T, N, A, SourceCrtLockThunkAccepted> {
    pub fn into_owned_lock_prefix(self) -> SourceCrtOwnedPrefix<Self, SourceCrtLockProgress> {
        let progress = self.source.progress;
        SourceCrtOwnedPrefix::admitted(self, progress)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockBindings {

    pub actual_lock_array: u64,

    pub actual_count_address: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtLockRequest {

    Initialize59bd0c { actual_rcx: u64, edx: u32, r8d: u32 },
    IncrementCount { actual_address: u64 },
    ReadCleanupCount { actual_address: u64 },

    DeleteThrough77f8f0 { actual_rcx: u64, source_index: u32 },
    DecrementCount { actual_address: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtLockReceipt {
    Initialized { actual_rcx: u64, edx: u32, r8d: u32, actual_eax: u32 },
    Incremented { actual_address: u64, actual_old_value: u32, actual_new_value: u32 },
    CleanupCount { actual_address: u64, value: u32 },
    Deleted { actual_rcx: u64, source_index: u32, actual_rbx: u64 },
    Decremented { actual_address: u64, actual_old_value: u32, actual_new_value: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtLockEntry { Initialize58d8a0, Cleanup58d8d4 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockPrefix {
    entry: SourceCrtLockEntry,
    bindings: SourceCrtLockBindings,
    initialize_return_eax: Option<u32>,
    completed_increment: Option<(u32, u32)>,
    cleanup_initial_count: Option<u32>,
    next_local_ebx: Option<u32>,
    last_deleted_index: Option<u32>,
    completed_deletions: u64,
    completed_decrements: u64,
    last_completed_decrement: Option<(u32, u32)>,
}
impl SourceCrtLockPrefix {
    pub fn entry(&self) -> SourceCrtLockEntry { self.entry }
    pub fn bindings(&self) -> SourceCrtLockBindings { self.bindings }
    pub fn initialize_return_eax(&self) -> Option<u32> { self.initialize_return_eax }
    pub fn completed_increment(&self) -> Option<(u32, u32)> { self.completed_increment }
    pub fn cleanup_initial_count(&self) -> Option<u32> { self.cleanup_initial_count }
    pub fn next_local_ebx(&self) -> Option<u32> { self.next_local_ebx }
    pub fn last_deleted_index(&self) -> Option<u32> { self.last_deleted_index }
    pub fn completed_deletions(&self) -> u64 { self.completed_deletions }
    pub fn completed_decrements(&self) -> u64 { self.completed_decrements }
    pub fn last_completed_decrement(&self) -> Option<(u32, u32)> { self.last_completed_decrement }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockCompletion {
    prefix: SourceCrtLockPrefix,
    return_al: u8,
}
impl SourceCrtLockCompletion {
    pub fn prefix(&self) -> &SourceCrtLockPrefix { &self.prefix }
    pub fn return_al(&self) -> u8 { self.return_al }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtLockProgress {
    Request(SourceCrtLockRequest),
    Complete(SourceCrtLockCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtLockError {
    ReceiptBindingRequired { pending: SourceCrtLockProgress, actual: SourceCrtLockReceipt },
    ActualNonvolatileContinuationRequired { pending: SourceCrtLockRequest, actual: SourceCrtLockReceipt },
    NativeSourceCompletionRequired { expected: SourceCrtLockThunkBinding, actual: SourceCrtLockReceipt },
}
impl SourceCrtLockError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ReceiptBindingRequired { .. } => "58d8a0/58d8d4 actual native/count receipt does not match pending source operation",
            Self::ActualNonvolatileContinuationRequired { .. } => "58d8d4 actual delete return changed source local RBX loop index",
            Self::NativeSourceCompletionRequired { .. } => "58d8a0 initialization has an outstanding source thunk; matching source and actual return are required together",
        }
    }
}
#[must_use = "retain actual lock resources and advanced full state with the source cursor"]
pub struct SourceCrtLockServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtLockRequest,
}
pub struct SourceCrtLockRejectedFailure<E> {
    pub cause: E,
    pub completed: SourceCrtLockCompletion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockThunkBinding {
    pub actual_module_base: u64,
    pub entry: SourceCrtNativeThunkEntry,
    pub resolver: SourceCrtResolver59bd70Request,
    pub required_actual_resume_rip: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockThunkBeginError {
    pub pending: SourceCrtLockProgress,
    pub outstanding: Option<SourceCrtLockThunkBinding>,
    pub actual_module_base: u64,
    pub actual_bindings: SourceCrtLockBindings,
}
impl SourceCrtLockThunkBeginError {
    pub fn reason(&self) -> &'static str {
        "lock thunk handoff requires an unfailed pending59bd0c call and exact supplied source array/count addresses"
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockThunkCompletionError {
    pub pending: SourceCrtLockProgress,
    pub expected: Option<SourceCrtLockThunkBinding>,
    pub actual: SourceCrtNativeThunkReturnedToCaller,
    pub continuation: Option<SourceCrtLockError>,
}
impl SourceCrtLockThunkCompletionError {
    pub fn reason(&self) -> &'static str {
        match self.continuation {
            Some(error) => error.reason(),
            None => "lock initialization thunk and actual return differ from the outstanding source call/base/resume/value",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtLockThunkAccepted {
    pub actual: SourceCrtNativeThunkReturnedToCaller,
    pub progress: SourceCrtLockProgress,
}

pub struct SourceCrtLock58d8a0 {
    prefix: SourceCrtLockPrefix,
    progress: SourceCrtLockProgress,
    failed_service: bool,
    pending_native_source: Option<SourceCrtLockThunkBinding>,
}
impl SourceCrtLock58d8a0 {
    pub fn begin_initialize(bindings: SourceCrtLockBindings) -> Self { Self::begin(SourceCrtLockEntry::Initialize58d8a0, bindings) }
    pub fn begin_cleanup(bindings: SourceCrtLockBindings) -> Self { Self::begin(SourceCrtLockEntry::Cleanup58d8d4, bindings) }
    fn begin(entry: SourceCrtLockEntry, bindings: SourceCrtLockBindings) -> Self {
        let request = match entry {
            SourceCrtLockEntry::Initialize58d8a0 => SourceCrtLockRequest::Initialize59bd0c {
                actual_rcx: bindings.actual_lock_array, edx: 0xfa0, r8d: 0 },
            SourceCrtLockEntry::Cleanup58d8d4 => SourceCrtLockRequest::ReadCleanupCount { actual_address: bindings.actual_count_address },
        };
        Self { prefix: SourceCrtLockPrefix { entry, bindings, initialize_return_eax: None, completed_increment: None,
            cleanup_initial_count: None, next_local_ebx: None, last_deleted_index: None, completed_deletions: 0,
            completed_decrements: 0, last_completed_decrement: None },
            progress: SourceCrtLockProgress::Request(request), failed_service: false, pending_native_source: None }
    }
    pub fn prefix(&self) -> &SourceCrtLockPrefix { &self.prefix }
    pub fn progress(&self) -> SourceCrtLockProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtLockRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtLockRequest> {
        match self.progress { SourceCrtLockProgress::Request(request) => Some(request), _ => None }
    }
    pub fn pending_native_source(&self) -> Option<SourceCrtLockThunkBinding> { self.pending_native_source }

    pub(super) fn stop_for_required_parent_return(&mut self,
        value: SourceCrtLockCompletion, held: SourceCrtLockProgress) -> bool {
        let valid = !self.failed_service && self.pending_native_source.is_none()
            && self.progress == SourceCrtLockProgress::Complete(value) && held == self.progress;
        self.failed_service = true;
        valid
    }

    pub fn into_completed_owned_prefix<C>(mut self,
        prefix: SourceCrtOwnedPrefix<C, SourceCrtLockProgress>)
        -> Result<super::crt_owned_prefix::SourceCrtOwnedControllerCompletion<Self, C,
                SourceCrtLockProgress, SourceCrtLockCompletion>,
            super::crt_owned_prefix::SourceCrtOwnedControllerCompletionError<Self, C, SourceCrtLockProgress>> {
        let pending = self.progress;
        let was_failed = self.failed_service;
        self.failed_service = true;
        if !was_failed && self.pending_native_source.is_none() && prefix.progress() == pending {
            if let SourceCrtLockProgress::Complete(value) = pending {
                self.failed_service = false;
                return Ok(super::crt_owned_prefix::SourceCrtOwnedControllerCompletion::admitted(self, prefix, value));
            }
        }
        Err(super::crt_owned_prefix::SourceCrtOwnedControllerCompletionError { controller: self, prefix, pending })
    }
    pub fn begin_pending_initialize_thunk(&mut self, actual_module_base: u64)
        -> Result<SourceCrtNativeThunk, SourceCrtLockThunkBeginError> {
        let b = self.prefix.bindings;
        let reject = SourceCrtLockThunkBeginError { pending: self.progress,
            outstanding: self.pending_native_source, actual_module_base, actual_bindings: b };
        if self.failed_service || self.pending_native_source.is_some()
            || b.actual_lock_array != actual_module_base.wrapping_add(0x861608)
            || b.actual_count_address != actual_module_base.wrapping_add(0x861630) { return Err(reject); }
        let SourceCrtLockProgress::Request(SourceCrtLockRequest::Initialize59bd0c { actual_rcx, edx, r8d }) = self.progress
            else { return Err(reject); };
        if actual_rcx != b.actual_lock_array || edx != 0xfa0 || r8d != 0 { return Err(reject); }
        let entry = SourceCrtNativeThunkEntry::InitializeLock59bd0c {
            array_rcx: actual_rcx, spin_edx: edx, flags_r8d: r8d };
        let thunk = SourceCrtNativeThunk::begin(actual_module_base, entry);
        let SourceCrtNativeThunkProgress::Request(SourceCrtNativeThunkRequest::Resolve(resolver)) = thunk.progress()
            else { return Err(reject); };
        self.pending_native_source = Some(SourceCrtLockThunkBinding { actual_module_base, entry, resolver,
            required_actual_resume_rip: actual_module_base.wrapping_add(0x58d8b8) });
        self.failed_service = true;
        Ok(thunk)
    }
    pub fn accept_completed_initialize_thunk(&mut self, actual: SourceCrtNativeThunkReturnedToCaller)
        -> Result<SourceCrtLockThunkAccepted, SourceCrtLockThunkCompletionError> {
        self.failed_service = true;
        let mut reject = SourceCrtLockThunkCompletionError { pending: self.progress,
            expected: self.pending_native_source, actual, continuation: None };
        let Some(expected) = self.pending_native_source else { return Err(reject); };
        let SourceCrtNativeThunkEntry::InitializeLock59bd0c { array_rcx, spin_edx, flags_r8d } = expected.entry
            else { return Err(reject); };
        if self.progress != SourceCrtLockProgress::Request(SourceCrtLockRequest::Initialize59bd0c {
            actual_rcx: array_rcx, edx: spin_edx, r8d: flags_r8d })
            || actual.source.entry() != expected.entry || actual.source.resolver_request() != expected.resolver
            || actual.actual_resume_rip != expected.required_actual_resume_rip
            || actual.actual_eax != actual.source.actual_return_rax() as u32 { return Err(reject); }
        self.pending_native_source = None;
        match self.accept_actual_completion(SourceCrtLockReceipt::Initialized {
            actual_rcx: array_rcx, edx: spin_edx, r8d: flags_r8d, actual_eax: actual.actual_eax }) {
            Ok(progress) => Ok(SourceCrtLockThunkAccepted { actual, progress }),
            Err(error) => {
                self.pending_native_source = Some(expected);
                reject.continuation = Some(error);
                Err(reject)
            }
        }
    }

    pub fn accept_completed_owned_initialize_thunk<O, T, N, A>(&mut self,
        actual: super::crt_native_thunk::SourceCrtThunkOwnedReturned<O, T, N, A>,
        project_frame: fn(&O) -> super::crt_storage_native_return::SourceCrtNativeRetFrame,
        project_return: fn(&O, &A) -> Option<super::crt_native_thunk::SourceCrtThunkActualCallerReturn>)
        -> Result<super::crt_native_thunk::SourceCrtOwnedThunkAdmission<O, T, N, A, SourceCrtLockThunkAccepted>,
            super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError<O, T, N, A, SourceCrtLockThunkCompletionError>> {
        self.failed_service = true;
        let actual_frame = project_frame(&actual.actual.actual.owner);
        let observed_return = project_return(&actual.actual.actual.owner, &actual.actual_return);
        if actual_frame != actual.completed().actual_caller_frame || observed_return != Some(actual.completed()) {
            return Err(super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError {
                actual, actual_frame, observed_return, continuation: None });
        }
        match self.accept_completed_initialize_thunk(actual.source()) {
            Ok(source) => Ok(super::crt_native_thunk::SourceCrtOwnedThunkAdmission { actual, source }),
            Err(cause) => Err(super::crt_native_thunk::SourceCrtOwnedThunkAdmissionError {
                actual, actual_frame, observed_return, continuation: Some(cause) }),
        }
    }

    pub fn accept_actual_owned_memory_completion<C, A>(&mut self,
        previous: SourceCrtOwnedPrefix<C, SourceCrtLockProgress>, actual: A,
        supplied: SourceCrtOwnedMemoryObservation<SourceCrtLockReceipt>,
        project_frame: fn(&C, &A) -> SourceCrtNativeRetFrame,
        project_receipt: fn(&C, &A) -> Option<SourceCrtOwnedMemoryObservation<SourceCrtLockReceipt>>)
        -> Result<SourceCrtOwnedPrefix<SourceCrtOwnedMemoryStep<C, A, SourceCrtLockReceipt,
                SourceCrtLockProgress>, SourceCrtLockProgress>,
            SourceCrtOwnedMemoryError<C, A, SourceCrtLockReceipt, SourceCrtLockProgress, SourceCrtLockError>> {
        self.failed_service = true;
        let pending = self.progress;
        let actual_frame = project_frame(&previous.owner, &actual);
        let observed = project_receipt(&previous.owner, &actual);
        if previous.progress() != pending || self.pending_native_source.is_some()
            || matches!(supplied.receipt, SourceCrtLockReceipt::Initialized { .. } | SourceCrtLockReceipt::Deleted { .. })
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
    pub fn accept_actual_completion(&mut self, actual: SourceCrtLockReceipt) -> Result<SourceCrtLockProgress, SourceCrtLockError> {
        self.failed_service = true;
        if let Some(expected) = self.pending_native_source {
            return Err(SourceCrtLockError::NativeSourceCompletionRequired { expected, actual });
        }
        match (self.progress, actual) {
            (SourceCrtLockProgress::Request(SourceCrtLockRequest::Initialize59bd0c { actual_rcx, edx, r8d }),
                SourceCrtLockReceipt::Initialized { actual_rcx: rcx, edx: actual_edx, r8d: actual_r8d, actual_eax })
                if actual_rcx == rcx && edx == actual_edx && r8d == actual_r8d => {
                self.prefix.initialize_return_eax = Some(actual_eax);
                if actual_eax != 0 { self.next(SourceCrtLockRequest::IncrementCount { actual_address: self.prefix.bindings.actual_count_address }); }
                else { self.next(SourceCrtLockRequest::ReadCleanupCount { actual_address: self.prefix.bindings.actual_count_address }); }
            }
            (SourceCrtLockProgress::Request(SourceCrtLockRequest::IncrementCount { actual_address }),
                SourceCrtLockReceipt::Incremented { actual_address: address, actual_old_value, actual_new_value })
                if address == actual_address && actual_new_value == actual_old_value.wrapping_add(1) => {
                self.prefix.completed_increment = Some((actual_old_value, actual_new_value));
                self.complete(1);
            }
            (SourceCrtLockProgress::Request(SourceCrtLockRequest::ReadCleanupCount { actual_address }),
                SourceCrtLockReceipt::CleanupCount { actual_address: address, value }) if address == actual_address => {
                self.prefix.cleanup_initial_count = Some(value);
                self.cleanup_step(value);
            }
            (SourceCrtLockProgress::Request(SourceCrtLockRequest::DeleteThrough77f8f0 { actual_rcx, source_index }),
                SourceCrtLockReceipt::Deleted { actual_rcx: rcx, source_index: index, actual_rbx })
                if actual_rcx == rcx && source_index == index => {
                if actual_rbx != u64::from(index) {
                    return Err(SourceCrtLockError::ActualNonvolatileContinuationRequired {
                        pending: SourceCrtLockRequest::DeleteThrough77f8f0 { actual_rcx, source_index }, actual });
                }
                self.prefix.last_deleted_index = Some(index);
                self.prefix.completed_deletions += 1;
                self.next(SourceCrtLockRequest::DecrementCount { actual_address: self.prefix.bindings.actual_count_address });
            }
            (SourceCrtLockProgress::Request(SourceCrtLockRequest::DecrementCount { actual_address }),
                SourceCrtLockReceipt::Decremented { actual_address: address, actual_old_value, actual_new_value })
                if address == actual_address && actual_new_value == actual_old_value.wrapping_sub(1) => {
                self.prefix.last_completed_decrement = Some((actual_old_value, actual_new_value));
                self.prefix.completed_decrements += 1;
                if let Some(local) = self.prefix.next_local_ebx { self.cleanup_step(local); }
            }
            _ => return Err(SourceCrtLockError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E) -> Result<SourceCrtLockServiceFailure<E>, SourceCrtLockRejectedFailure<E>> {
        match self.progress {
            SourceCrtLockProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtLockServiceFailure { cause, pending })
            }
            SourceCrtLockProgress::Complete(completed) => Err(SourceCrtLockRejectedFailure { cause, completed }),
        }
    }
    fn next(&mut self, request: SourceCrtLockRequest) { self.progress = SourceCrtLockProgress::Request(request); }
    fn cleanup_step(&mut self, local_ebx: u32) {
        self.prefix.next_local_ebx = Some(local_ebx);
        if local_ebx == 0 {
            self.complete(if self.prefix.entry == SourceCrtLockEntry::Initialize58d8a0 { 0 } else { 1 });
        } else {
            let index = local_ebx.wrapping_sub(1);
            self.prefix.next_local_ebx = Some(index);
            let address = self.prefix.bindings.actual_lock_array.wrapping_add(u64::from(index) * 40);
            self.next(SourceCrtLockRequest::DeleteThrough77f8f0 { actual_rcx: address, source_index: index });
        }
    }
    fn complete(&mut self, return_al: u8) {
        self.progress = SourceCrtLockProgress::Complete(SourceCrtLockCompletion { prefix: self.prefix, return_al });
    }
}
