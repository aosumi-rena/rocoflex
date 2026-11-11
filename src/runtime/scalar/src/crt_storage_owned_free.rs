use core::num::NonZeroU64;
use super::crt_storage_callback::{SourceCrtStorageCallback58a6bc, SourceCrtStorageCallbackCompletion,
    SourceCrtStorageCallbackError, SourceCrtStorageCallbackProgress,
    SourceCrtStorageFree580c24Request, SourceCrtStorageFree580c24Returned};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageActualAllocation {
    pub actual_base_word: NonZeroU64,
    pub actual_accessible_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageActualFreeEntry {
    pub actual_rip: u64,
    pub actual_rcx: u64,
    pub actual_rax: u64,
    pub actual_rsp: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageBorrowedDeleteStep { NullTailReturned, FreeReturned, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStorageActualDeleteCompletion {
    pub actual_free_word: u64,

    pub actual_heap_owner_retained: bool,
    pub actual_disposition_present: bool,
}

pub struct SourceCrtStorageAllocationCallbacks<A, C> {
    pub describe: fn(&A) -> SourceCrtStorageActualAllocation,
    pub begin: fn(A) -> C,
}

pub struct SourceCrtStorageDeleteCallbacks<C, D, P, S, E> {

    pub actual_entry: fn(&P) -> SourceCrtStorageActualFreeEntry,
    pub actual_return: fn(&P) -> Option<SourceCrtStorageFree580c24Returned>,
    pub step: fn(&mut C, &mut S, &mut P) -> Result<SourceCrtStorageBorrowedDeleteStep, E>,
    pub finish: fn(C) -> Result<D, C>,
    pub describe_completed: fn(&D) -> SourceCrtStorageActualDeleteCompletion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageOwnedFreePhase {
    Ready, PendingActualDelete, OwnerCompletionRequired, ActualReturnRequired,
    CompleteBeforeRet, Blocked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageOwnedFreeBindReason { PendingFreeRequired, AllocationBindingRequired, ActualEntryRequired }
#[must_use = "retain callback, actual allocation, frame/FP/stack prefix, services and actual entry"]
pub struct SourceCrtStorageOwnedFreeBindFailure<A, P, S> {
    pub reason: SourceCrtStorageOwnedFreeBindReason,
    pub callback: SourceCrtStorageCallback58a6bc,
    pub actual_allocation: A,
    pub allocation: SourceCrtStorageActualAllocation,
    pub actual_entry: SourceCrtStorageActualFreeEntry,
    pub observed_entry: SourceCrtStorageActualFreeEntry,
    pub prefix: P,
    pub services: S,
}
pub enum SourceCrtStorageOwnedFreeError<E> {
    PhaseRequired { actual: SourceCrtStorageOwnedFreePhase },
    Primitive(E),
    UnexpectedNullTail,
    OwnerCompletionRequired,
    CompletedOwnerBindingRequired { actual: SourceCrtStorageActualDeleteCompletion },
    ActualReturnNotPending { actual: SourceCrtStorageFree580c24Returned },
    ActualPrefixReturnRequired { actual: SourceCrtStorageFree580c24Returned,
        observed: Option<SourceCrtStorageFree580c24Returned> },
    CallbackReturn(SourceCrtStorageCallbackError),
}
impl<E> SourceCrtStorageOwnedFreeError<E> {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::PhaseRequired { .. } => "owned580c24 composition is not at the required phase; no implicit invocation/retry",
            Self::Primitive(_) => "borrowed580c24 primitive failed; same delete/frame/FP/resource prefix retained",
            Self::UnexpectedNullTail => "nonnull callback allocation cannot adopt a null delete-tail completion",
            Self::OwnerCompletionRequired => "same borrowed delete cursor still requires explicit actual owner completion",
            Self::CompletedOwnerBindingRequired { .. } => "borrowed delete completion must retain the same actual free word, H and explicit physical disposition",
            Self::ActualReturnNotPending { .. } => "actual callback free return is not pending after a completed same-owner delete",
            Self::ActualPrefixReturnRequired { .. } => "actual callback return packet differs from the same owned frame/FP/resource prefix projection",
            Self::CallbackReturn(error) => error.reason(),
        }
    }
}
#[must_use = "retain the entire owned callback/delete/frame/FP/services continuation"]
pub struct SourceCrtStorageOwnedFreeFailure<T, E> { pub owner: T, pub cause: E }
pub enum SourceCrtStorageOwnedFreeReceiptError<R, E> {
    NotPending { actual: R },
    Rejected { actual: R, cause: E },
}
#[must_use = "retain completed physical D/H/disposition and actual frame/FP/stack/services"]
pub struct SourceCrtStorageOwnedFreeCompletion<D, P, S> {
    pub callback_before_ret: SourceCrtStorageCallbackCompletion,
    pub allocation: SourceCrtStorageActualAllocation,
    pub actual_entry: SourceCrtStorageActualFreeEntry,
    pub delete_completion: D,
    pub prefix: P,
    pub services: S,
}
pub struct SourceCrtStorageOwnedFreeParts<C, D, P, S, E> {
    pub callback: SourceCrtStorageCallback58a6bc,
    pub allocation: SourceCrtStorageActualAllocation,
    pub actual_entry: SourceCrtStorageActualFreeEntry,
    pub phase: SourceCrtStorageOwnedFreePhase,
    pub delete_cursor: Option<C>,
    pub delete_completion: Option<D>,
    pub prefix: P,
    pub services: S,
    pub callbacks: SourceCrtStorageDeleteCallbacks<C, D, P, S, E>,
}

#[must_use = "retain callback/delete H/frame/FP/stack/services through actual return"]
pub struct SourceCrtStorageCallbackOwnedFree<C, D, P, S, E> {
    callback: SourceCrtStorageCallback58a6bc,
    allocation: SourceCrtStorageActualAllocation,
    actual_entry: SourceCrtStorageActualFreeEntry,
    phase: SourceCrtStorageOwnedFreePhase,
    delete_cursor: Option<C>,
    delete_completion: Option<D>,
    prefix: P,
    services: S,
    callbacks: SourceCrtStorageDeleteCallbacks<C, D, P, S, E>,
}
impl<C, D, P, S, E> SourceCrtStorageCallbackOwnedFree<C, D, P, S, E> {
    pub fn bind_actual_allocation<A>(mut callback: SourceCrtStorageCallback58a6bc, actual_allocation: A,
        actual_entry: SourceCrtStorageActualFreeEntry, prefix: P, services: S,
        allocation_callbacks: SourceCrtStorageAllocationCallbacks<A, C>,
        callbacks: SourceCrtStorageDeleteCallbacks<C, D, P, S, E>)
        -> Result<Self, SourceCrtStorageOwnedFreeBindFailure<A, P, S>> {
        let allocation = (allocation_callbacks.describe)(&actual_allocation);
        let observed_entry = (callbacks.actual_entry)(&prefix);
        let pending = callback.request();
        callback.suspend_actual_free_prefix();
        let reason = match pending {
            None => Some(SourceCrtStorageOwnedFreeBindReason::PendingFreeRequired),
            Some(request) if allocation.actual_base_word.get() != request.actual_rcx =>
                Some(SourceCrtStorageOwnedFreeBindReason::AllocationBindingRequired),
            Some(request) if actual_entry.actual_rip != callback.entry().actual_module_base.wrapping_add(0x580c24)
                || actual_entry != observed_entry
                || actual_entry.actual_rcx != request.actual_rcx
                || actual_entry.actual_rax != request.source_rax_before_call
                || actual_entry.actual_rsp != request.required_callee_entry_rsp =>
                Some(SourceCrtStorageOwnedFreeBindReason::ActualEntryRequired),
            Some(_) => None,
        };
        if let Some(reason) = reason {
            return Err(SourceCrtStorageOwnedFreeBindFailure { reason, callback, actual_allocation,
                allocation, actual_entry, observed_entry, prefix, services });
        }
        let delete_cursor = (allocation_callbacks.begin)(actual_allocation);
        Ok(Self { callback, allocation, actual_entry, phase: SourceCrtStorageOwnedFreePhase::Ready,
            delete_cursor: Some(delete_cursor), delete_completion: None, prefix, services, callbacks })
    }
    pub fn phase(&self) -> SourceCrtStorageOwnedFreePhase { self.phase }
    pub fn allocation(&self) -> SourceCrtStorageActualAllocation { self.allocation }
    pub fn request(&self) -> Option<SourceCrtStorageFree580c24Request> {
        if self.phase == SourceCrtStorageOwnedFreePhase::Ready { self.callback.awaiting_actual_completion() } else { None }
    }
    pub fn prefix(&self) -> &P { &self.prefix }
    pub fn services(&self) -> &S { &self.services }
    pub fn step(mut self) -> Result<Self, SourceCrtStorageOwnedFreeFailure<Self, SourceCrtStorageOwnedFreeError<E>>> {
        if self.phase != SourceCrtStorageOwnedFreePhase::Ready {
            let actual = self.phase;
            return Err(self.fail(SourceCrtStorageOwnedFreeError::PhaseRequired { actual }));
        }
        self.phase = SourceCrtStorageOwnedFreePhase::PendingActualDelete;
        let Some(cursor) = self.delete_cursor.as_mut() else {
            return Err(self.fail(SourceCrtStorageOwnedFreeError::OwnerCompletionRequired));
        };
        match (self.callbacks.step)(cursor, &mut self.services, &mut self.prefix) {
            Err(error) => Err(self.fail(SourceCrtStorageOwnedFreeError::Primitive(error))),
            Ok(SourceCrtStorageBorrowedDeleteStep::NullTailReturned) => {
                self.phase = SourceCrtStorageOwnedFreePhase::Blocked;
                Err(self.fail(SourceCrtStorageOwnedFreeError::UnexpectedNullTail))
            }
            Ok(SourceCrtStorageBorrowedDeleteStep::FreeReturned | SourceCrtStorageBorrowedDeleteStep::Complete) => {
                self.phase = SourceCrtStorageOwnedFreePhase::OwnerCompletionRequired;
                self.finish_delete()
            }
        }
    }

    pub fn accept_actual_delete_completion<R>(mut self, actual: R,
        apply: fn(SourceCrtStorageFree580c24Request, &mut C, &mut S, &mut P, R) -> Result<(), (R, E)>)
        -> Result<Self, SourceCrtStorageOwnedFreeFailure<Self, SourceCrtStorageOwnedFreeReceiptError<R, E>>> {
        if self.phase != SourceCrtStorageOwnedFreePhase::PendingActualDelete
            && self.phase != SourceCrtStorageOwnedFreePhase::OwnerCompletionRequired {
            return Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::NotPending { actual }));
        }
        let Some(expected) = self.callback.awaiting_actual_completion() else {
            return Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::NotPending { actual }));
        };
        let Some(cursor) = self.delete_cursor.as_mut() else {
            return Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::NotPending { actual }));
        };
        match apply(expected, cursor, &mut self.services, &mut self.prefix, actual) {
            Ok(()) => { self.phase = SourceCrtStorageOwnedFreePhase::OwnerCompletionRequired; Ok(self) }
            Err((actual, cause)) => Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::Rejected { actual, cause })),
        }
    }

    pub fn finish_delete(mut self) -> Result<Self, SourceCrtStorageOwnedFreeFailure<Self, SourceCrtStorageOwnedFreeError<E>>> {
        if self.phase != SourceCrtStorageOwnedFreePhase::OwnerCompletionRequired {
            let actual = self.phase;
            return Err(self.fail(SourceCrtStorageOwnedFreeError::PhaseRequired { actual }));
        }
        let Some(cursor) = self.delete_cursor.take() else {
            return Err(self.fail(SourceCrtStorageOwnedFreeError::OwnerCompletionRequired));
        };
        let completed = match (self.callbacks.finish)(cursor) {
            Ok(completed) => completed,
            Err(cursor) => {
                self.delete_cursor = Some(cursor);
                return Err(self.fail(SourceCrtStorageOwnedFreeError::OwnerCompletionRequired));
            }
        };
        let actual = (self.callbacks.describe_completed)(&completed);
        self.delete_completion = Some(completed);
        if actual.actual_free_word != self.allocation.actual_base_word.get()
            || !actual.actual_heap_owner_retained || !actual.actual_disposition_present {
            self.phase = SourceCrtStorageOwnedFreePhase::Blocked;
            return Err(self.fail(SourceCrtStorageOwnedFreeError::CompletedOwnerBindingRequired { actual }));
        }
        self.phase = SourceCrtStorageOwnedFreePhase::ActualReturnRequired;
        Ok(self)
    }

    pub fn accept_actual_return_prefix<R>(mut self, actual: R,
        apply: fn(SourceCrtStorageFree580c24Request, &mut S, &mut P, R) -> Result<(), (R, E)>)
        -> Result<Self, SourceCrtStorageOwnedFreeFailure<Self, SourceCrtStorageOwnedFreeReceiptError<R, E>>> {
        if self.phase != SourceCrtStorageOwnedFreePhase::ActualReturnRequired {
            return Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::NotPending { actual }));
        }
        let Some(expected) = self.callback.awaiting_actual_completion() else {
            return Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::NotPending { actual }));
        };
        match apply(expected, &mut self.services, &mut self.prefix, actual) {
            Ok(()) => Ok(self),
            Err((actual, cause)) => Err(self.fail(SourceCrtStorageOwnedFreeReceiptError::Rejected { actual, cause })),
        }
    }
    pub fn accept_actual_callback_return(mut self, actual: SourceCrtStorageFree580c24Returned)
        -> Result<Self, SourceCrtStorageOwnedFreeFailure<Self, SourceCrtStorageOwnedFreeError<E>>> {
        if self.phase != SourceCrtStorageOwnedFreePhase::ActualReturnRequired {
            return Err(self.fail(SourceCrtStorageOwnedFreeError::ActualReturnNotPending { actual }));
        }
        let observed = (self.callbacks.actual_return)(&self.prefix);
        if observed != Some(actual) {
            return Err(self.fail(SourceCrtStorageOwnedFreeError::ActualPrefixReturnRequired { actual, observed }));
        }
        match self.callback.accept_actual_free_return(actual) {
            Ok(SourceCrtStorageCallbackProgress::CompleteBeforeRet(_)) => {
                self.phase = SourceCrtStorageOwnedFreePhase::CompleteBeforeRet; Ok(self)
            }
            Err(error) => Err(self.fail(SourceCrtStorageOwnedFreeError::CallbackReturn(error))),
            Ok(_) => Err(self.fail(SourceCrtStorageOwnedFreeError::ActualReturnNotPending { actual })),
        }
    }
    pub fn into_completed_before_ret(mut self) -> Result<SourceCrtStorageOwnedFreeCompletion<D, P, S>, Self> {
        if self.phase != SourceCrtStorageOwnedFreePhase::CompleteBeforeRet { return Err(self); }
        let SourceCrtStorageCallbackProgress::CompleteBeforeRet(callback_before_ret) = self.callback.progress()
            else { return Err(self); };
        let Some(delete_completion) = self.delete_completion.take() else { return Err(self); };
        Ok(SourceCrtStorageOwnedFreeCompletion { callback_before_ret, allocation: self.allocation,
            actual_entry: self.actual_entry, delete_completion, prefix: self.prefix, services: self.services })
    }
    pub fn into_owned_parts(self) -> SourceCrtStorageOwnedFreeParts<C, D, P, S, E> {
        SourceCrtStorageOwnedFreeParts { callback: self.callback, allocation: self.allocation,
            actual_entry: self.actual_entry, phase: self.phase, delete_cursor: self.delete_cursor,
            delete_completion: self.delete_completion, prefix: self.prefix, services: self.services, callbacks: self.callbacks }
    }
    fn fail<T>(self, cause: T) -> SourceCrtStorageOwnedFreeFailure<Self, T> {
        SourceCrtStorageOwnedFreeFailure { owner: self, cause }
    }
}
