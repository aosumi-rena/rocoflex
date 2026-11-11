use super::crt_lock_initialization::{SourceCrtLockBindings, SourceCrtLockCompletion, SourceCrtLockEntry};
use super::crt_thread_storage::{SourceCrtThreadStorageBindings, SourceCrtThreadStorageCompletion, SourceCrtThreadStorageEntry};
use super::crt_lock_initialization::{SourceCrtLock58d8a0, SourceCrtLockProgress};
use super::crt_thread_storage::{SourceCrtThreadStorage58a4d4, SourceCrtThreadStorageProgress};
use super::crt_owned_prefix::SourceCrtOwnedControllerCompletion;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetObservation,
    SourceCrtNativeReturnFailure};

pub type SourceCrtOwnedStorageParent<C> = SourceCrtOwnedControllerCompletion<SourceCrtThreadStorage58a4d4,
    C, SourceCrtThreadStorageProgress, SourceCrtThreadStorageCompletion>;
pub type SourceCrtOwnedLockParent<C> = SourceCrtOwnedControllerCompletion<SourceCrtLock58d8a0,
    C, SourceCrtLockProgress, SourceCrtLockCompletion>;

#[must_use = "retain original storage/lock controller and complete owned prefix"]
pub enum SourceCrtOwnedResourceParent<C> {
    Storage(SourceCrtOwnedStorageParent<C>),
    Lock(SourceCrtOwnedLockParent<C>),
}
impl<C> SourceCrtOwnedResourceParent<C> {
    fn logical_receipt(&self) -> SourceCrtResourcesReceipt {
        match self {
            Self::Storage(actual) => SourceCrtResourcesReceipt::ThreadStorage(actual.value()),
            Self::Lock(actual) => SourceCrtResourcesReceipt::Lock(actual.value()),
        }
    }
    fn stop_original(&mut self) -> bool {
        match self {
            Self::Storage(actual) => {
                let value = actual.value();
                actual.controller.stop_for_required_parent_return(value, actual.prefix.progress())
            }
            Self::Lock(actual) => {
                let value = actual.value();
                actual.controller.stop_for_required_parent_return(value, actual.prefix.progress())
            }
        }
    }
    fn prefix(&self) -> &C {
        match self { Self::Storage(actual) => &actual.prefix.owner, Self::Lock(actual) => &actual.prefix.owner }
    }
    fn prefix_mut(&mut self) -> &mut C {
        match self { Self::Storage(actual) => &mut actual.prefix.owner, Self::Lock(actual) => &mut actual.prefix.owner }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtRequiredActualParentReturn {
    pub pending_call: SourceCrtResourcesRequest,
    pub logical_completion: SourceCrtResourcesReceipt,
}
impl SourceCrtRequiredActualParentReturn {
    pub fn reason(&self) -> &'static str {
        "storage/lock actual whole epilogue/caller return remains required; logical AL is not full RAX or a native RET receipt"
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtOwnedParentBindReason { CallerAlreadyStopped, CallerCallRequired, SourceParentNotHeldComplete, CallBinding }
#[must_use = "retain stopped original caller, original child and all nested owners"]
pub struct SourceCrtOwnedParentBindFailure<C> {
    pub caller: SourceCrtResources564ccc,
    pub parent: SourceCrtOwnedResourceParent<C>,
    pub pending: SourceCrtResourcesProgress,
    pub logical_completion: SourceCrtResourcesReceipt,
    pub reason: SourceCrtOwnedParentBindReason,
}

#[must_use = "retain whole original caller/parent/module/cache/native/frame/FP capsule"]
pub struct SourceCrtResourcesOwnedParentReturn<C, H = ()> {
    caller: SourceCrtResources564ccc,
    parent: SourceCrtOwnedResourceParent<C>,
    required: SourceCrtRequiredActualParentReturn,
    history: H,
}
#[must_use = "retain both stopped original controllers and every actual prefix owner"]
pub struct SourceCrtOwnedParentReturnParts<C, H> {
    pub caller: SourceCrtResources564ccc,
    pub parent: SourceCrtOwnedResourceParent<C>,
    pub required: SourceCrtRequiredActualParentReturn,
    pub history: H,
}
#[must_use = "retain prior history and raw actual prefix resources"]
pub struct SourceCrtParentPrefixHistory<H, A> {
    pub previous: H,
    pub actual: A,

    pub observation: SourceCrtNativeRetObservation,
}
pub enum SourceCrtParentPrefixError<A, E> {
    BeforeFrame { actual: A, observation: SourceCrtNativeRetObservation, actual_frame: SourceCrtNativeRetFrame },
    Adapter { actual: A, observation: SourceCrtNativeRetObservation, cause: E },
    Changed { actual: A, before: SourceCrtNativeRetObservation, after: SourceCrtNativeRetObservation },
    Frame { actual: A, observation: SourceCrtNativeRetObservation, actual_frame: SourceCrtNativeRetFrame },
}
#[must_use = "retain rejected actual prefix/error and all previous owned history"]
pub struct SourceCrtRejectedParentPrefixHistory<H, A, E> {
    pub previous: H,
    pub rejected: SourceCrtParentPrefixError<A, E>,
}
impl<C, H> SourceCrtResourcesOwnedParentReturn<C, H> {
    pub fn requirement(&self) -> SourceCrtRequiredActualParentReturn { self.required }
    pub fn caller_progress(&self) -> SourceCrtResourcesProgress { self.caller.progress() }
    pub fn prefix(&self) -> &C { self.parent.prefix() }
    pub fn history(&self) -> &H { &self.history }

    pub fn retain_actual_parent_prefix<A, E>(mut self, mut actual: A,
        project: fn(&A) -> SourceCrtNativeRetObservation,
        apply: fn(&mut C, &mut A) -> Result<(), E>,
        current_frame: fn(&C) -> SourceCrtNativeRetFrame)
        -> Result<SourceCrtResourcesOwnedParentReturn<C, SourceCrtParentPrefixHistory<H, A>>,
            SourceCrtNativeReturnFailure<Self, SourceCrtParentPrefixError<A, E>>> {
        let observation = project(&actual);
        let before = match observation {
            SourceCrtNativeRetObservation::Retired(actual) => actual.before,
            SourceCrtNativeRetObservation::Partial { before, .. } => before,
        };
        let actual_frame = current_frame(self.parent.prefix());
        if actual_frame != before {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtParentPrefixError::BeforeFrame { actual, observation, actual_frame } });
        }
        if let Err(cause) = apply(self.parent.prefix_mut(), &mut actual) {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtParentPrefixError::Adapter { actual, observation, cause } });
        }
        let after = project(&actual);
        if after != observation {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtParentPrefixError::Changed { actual, before: observation, after } });
        }
        let expected_frame = match observation {
            SourceCrtNativeRetObservation::Retired(actual) => actual.after,
            SourceCrtNativeRetObservation::Partial { after, .. } => after,
        };
        let actual_frame = current_frame(self.parent.prefix());
        if actual_frame != expected_frame {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtParentPrefixError::Frame { actual, observation, actual_frame } });
        }
        Ok(SourceCrtResourcesOwnedParentReturn { caller: self.caller, parent: self.parent,
            required: self.required, history: SourceCrtParentPrefixHistory {
                previous: self.history, actual, observation } })
    }

    pub fn retain_rejected_parent_prefix<A, E>(
        failure: SourceCrtNativeReturnFailure<Self, SourceCrtParentPrefixError<A, E>>)
        -> SourceCrtResourcesOwnedParentReturn<C, SourceCrtRejectedParentPrefixHistory<H, A, E>> {
        let SourceCrtNativeReturnFailure { owner, cause } = failure;
        SourceCrtResourcesOwnedParentReturn { caller: owner.caller, parent: owner.parent,
            required: owner.required, history: SourceCrtRejectedParentPrefixHistory {
                previous: owner.history, rejected: cause } }
    }
    pub fn into_owned_parts(self) -> SourceCrtOwnedParentReturnParts<C, H> {
        SourceCrtOwnedParentReturnParts { caller: self.caller, parent: self.parent,
            required: self.required, history: self.history }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtResourcesEntry { Initialize564ccc, Cleanup564cf4 { source_entry_ecx: u32 } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtResourcesRequest {
    Lock { entry: SourceCrtLockEntry, bindings: SourceCrtLockBindings },
    ThreadStorage { entry: SourceCrtThreadStorageEntry, bindings: SourceCrtThreadStorageBindings },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtResourcesReceipt {
    Lock(SourceCrtLockCompletion),
    ThreadStorage(SourceCrtThreadStorageCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtResourcesPrefix {
    entry: SourceCrtResourcesEntry,
    lock_bindings: SourceCrtLockBindings,
    thread_storage_bindings: SourceCrtThreadStorageBindings,
    lock_initialization: Option<SourceCrtLockCompletion>,
    thread_storage_initialization: Option<SourceCrtThreadStorageCompletion>,
    thread_storage_cleanup: Option<SourceCrtThreadStorageCompletion>,
    lock_cleanup: Option<SourceCrtLockCompletion>,
}
impl SourceCrtResourcesPrefix {
    pub fn entry(&self) -> SourceCrtResourcesEntry { self.entry }
    pub fn lock_bindings(&self) -> SourceCrtLockBindings { self.lock_bindings }
    pub fn thread_storage_bindings(&self) -> SourceCrtThreadStorageBindings { self.thread_storage_bindings }
    pub fn lock_initialization(&self) -> Option<&SourceCrtLockCompletion> { self.lock_initialization.as_ref() }
    pub fn thread_storage_initialization(&self) -> Option<&SourceCrtThreadStorageCompletion> { self.thread_storage_initialization.as_ref() }
    pub fn thread_storage_cleanup(&self) -> Option<&SourceCrtThreadStorageCompletion> { self.thread_storage_cleanup.as_ref() }
    pub fn lock_cleanup(&self) -> Option<&SourceCrtLockCompletion> { self.lock_cleanup.as_ref() }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtResourcesCompletion {
    prefix: SourceCrtResourcesPrefix,
    return_al: u8,
}
impl SourceCrtResourcesCompletion {
    pub fn prefix(&self) -> &SourceCrtResourcesPrefix { &self.prefix }
    pub fn return_al(&self) -> u8 { self.return_al }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtResourcesProgress {
    Request(SourceCrtResourcesRequest),
    Complete(SourceCrtResourcesCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtResourcesError {
    pub pending: SourceCrtResourcesProgress,
    pub actual: SourceCrtResourcesReceipt,
}
impl SourceCrtResourcesError {
    pub fn reason(self) -> &'static str { "564ccc/564cf4 actual inner completion has different entry/address bindings or pending role" }
}
#[must_use = "retain the source resource prefix, nested native resources and advanced state"]
pub struct SourceCrtResourcesServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtResourcesRequest,
}
pub struct SourceCrtResourcesRejectedFailure<E> {
    pub cause: E,
    pub completed: SourceCrtResourcesCompletion,
}

pub struct SourceCrtResources564ccc {
    prefix: SourceCrtResourcesPrefix,
    progress: SourceCrtResourcesProgress,
    failed_service: bool,
}
impl SourceCrtResources564ccc {
    pub fn begin_initialize(lock_bindings: SourceCrtLockBindings, thread_storage_bindings: SourceCrtThreadStorageBindings) -> Self {
        Self::begin(SourceCrtResourcesEntry::Initialize564ccc, lock_bindings, thread_storage_bindings)
    }
    pub fn begin_cleanup(source_entry_ecx: u32, lock_bindings: SourceCrtLockBindings,
        thread_storage_bindings: SourceCrtThreadStorageBindings) -> Self
    { Self::begin(SourceCrtResourcesEntry::Cleanup564cf4 { source_entry_ecx }, lock_bindings, thread_storage_bindings) }
    fn begin(entry: SourceCrtResourcesEntry, lock_bindings: SourceCrtLockBindings,
        thread_storage_bindings: SourceCrtThreadStorageBindings) -> Self
    {
        let prefix = SourceCrtResourcesPrefix { entry, lock_bindings, thread_storage_bindings,
            lock_initialization: None, thread_storage_initialization: None, thread_storage_cleanup: None, lock_cleanup: None };
        let progress = match entry {
            SourceCrtResourcesEntry::Initialize564ccc => SourceCrtResourcesProgress::Request(SourceCrtResourcesRequest::Lock {
                entry: SourceCrtLockEntry::Initialize58d8a0, bindings: lock_bindings }),
            SourceCrtResourcesEntry::Cleanup564cf4 { source_entry_ecx } if source_entry_ecx as u8 != 0 =>
                SourceCrtResourcesProgress::Complete(SourceCrtResourcesCompletion { prefix, return_al: 1 }),
            SourceCrtResourcesEntry::Cleanup564cf4 { .. } => SourceCrtResourcesProgress::Request(SourceCrtResourcesRequest::ThreadStorage {
                entry: SourceCrtThreadStorageEntry::Cleanup58a51c, bindings: thread_storage_bindings }),
        };
        Self { prefix, progress, failed_service: false }
    }
    pub fn prefix(&self) -> &SourceCrtResourcesPrefix { &self.prefix }
    pub fn progress(&self) -> SourceCrtResourcesProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtResourcesRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtResourcesRequest> {
        match self.progress { SourceCrtResourcesProgress::Request(request) => Some(request), _ => None }
    }

    pub fn hold_owned_parent_return<C>(mut self, mut parent: SourceCrtOwnedResourceParent<C>)
        -> Result<SourceCrtResourcesOwnedParentReturn<C>, SourceCrtOwnedParentBindFailure<C>> {
        let pending = self.progress;
        let was_failed = self.failed_service;
        self.failed_service = true;
        let logical_completion = parent.logical_receipt();

        let child_held_complete = parent.stop_original();
        let source_call = match logical_completion {
            SourceCrtResourcesReceipt::Lock(value) => SourceCrtResourcesRequest::Lock {
                entry: value.prefix().entry(), bindings: value.prefix().bindings() },
            SourceCrtResourcesReceipt::ThreadStorage(value) => SourceCrtResourcesRequest::ThreadStorage {
                entry: value.prefix().entry(), bindings: value.prefix().bindings() },
        };
        let reason = if was_failed { Some(SourceCrtOwnedParentBindReason::CallerAlreadyStopped) }
            else if !child_held_complete { Some(SourceCrtOwnedParentBindReason::SourceParentNotHeldComplete) }
            else { match pending {
                SourceCrtResourcesProgress::Request(call) if call == source_call => None,
                SourceCrtResourcesProgress::Request(_) => Some(SourceCrtOwnedParentBindReason::CallBinding),
                SourceCrtResourcesProgress::Complete(_) => Some(SourceCrtOwnedParentBindReason::CallerCallRequired),
            }};
        if let Some(reason) = reason {
            return Err(SourceCrtOwnedParentBindFailure { caller: self, parent, pending, logical_completion, reason });
        }
        Ok(SourceCrtResourcesOwnedParentReturn { caller: self, parent,
            required: SourceCrtRequiredActualParentReturn { pending_call: source_call, logical_completion }, history: () })
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtResourcesReceipt)
        -> Result<SourceCrtResourcesProgress, SourceCrtResourcesError>
    {
        self.failed_service = true;
        match (self.progress, actual) {
            (SourceCrtResourcesProgress::Request(SourceCrtResourcesRequest::Lock { entry, bindings }),
                SourceCrtResourcesReceipt::Lock(completion))
                if completion.prefix().entry() == entry && completion.prefix().bindings() == bindings => {
                match entry {
                    SourceCrtLockEntry::Initialize58d8a0 => {
                        self.prefix.lock_initialization = Some(completion);
                        if completion.return_al() == 0 { self.complete(0); }
                        else { self.next(SourceCrtResourcesRequest::ThreadStorage {
                            entry: SourceCrtThreadStorageEntry::Initialize58a4d4, bindings: self.prefix.thread_storage_bindings }); }
                    }
                    SourceCrtLockEntry::Cleanup58d8d4 => {
                        self.prefix.lock_cleanup = Some(completion);
                        self.complete(if self.prefix.entry == SourceCrtResourcesEntry::Initialize564ccc { 0 } else { 1 });
                    }
                }
            }
            (SourceCrtResourcesProgress::Request(SourceCrtResourcesRequest::ThreadStorage { entry, bindings }),
                SourceCrtResourcesReceipt::ThreadStorage(completion))
                if completion.prefix().entry() == entry && completion.prefix().bindings() == bindings => {
                match entry {
                    SourceCrtThreadStorageEntry::Initialize58a4d4 => {
                        self.prefix.thread_storage_initialization = Some(completion);
                        if completion.return_al() != 0 { self.complete(1); }
                        else { self.cleanup_locks(); }
                    }
                    SourceCrtThreadStorageEntry::Cleanup58a51c => {
                        self.prefix.thread_storage_cleanup = Some(completion);
                        self.cleanup_locks();
                    }
                }
            }
            _ => return Err(SourceCrtResourcesError { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtResourcesServiceFailure<E>, SourceCrtResourcesRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtResourcesProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtResourcesServiceFailure { cause, pending })
            }
            SourceCrtResourcesProgress::Complete(completed) => Err(SourceCrtResourcesRejectedFailure { cause, completed }),
        }
    }
    fn cleanup_locks(&mut self) { self.next(SourceCrtResourcesRequest::Lock {
        entry: SourceCrtLockEntry::Cleanup58d8d4, bindings: self.prefix.lock_bindings }); }
    fn next(&mut self, request: SourceCrtResourcesRequest) { self.progress = SourceCrtResourcesProgress::Request(request); }
    fn complete(&mut self, return_al: u8) {
        self.progress = SourceCrtResourcesProgress::Complete(SourceCrtResourcesCompletion { prefix: self.prefix, return_al });
    }
}
