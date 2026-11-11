use super::*;
use super::super::crt_query_context::{SourceCrtQueryContextCall, SourceCrtQueryContextCallKind, SourceCrtQueryStackWord};
use super::super::crt_query_calloc::SourceCrtCallocArguments;
use super::super::crt_query_return::{SourceCrtQueryCallEnvelope, SourceCrtQueryCallerReceipt,
    SourceCrtQueryNativeRet, SourceCrtQueryRetBindFailure, SourceCrtQueryReturned,
    SourceCrtQueryCallerReturned, SourceCrtQueryCallerBindFailure};
use super::super::crt_storage_native_return::{SourceCrtNativeRetObservation, SourceCrtNativeRetExpectation, SourceCrtNativeRetired};
use super::super::crt_query_calloc_join::SourceCrtQueryCallocJoinError;

#[path = "crt_query_context_allocation_resume.rs"]
pub mod resume;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtContextAllocationCallEntry {
    pub call: SourceCrtQueryCallEnvelope, pub before: SourceCrtNativeRetFrame,
    pub entered: SourceCrtNativeRetFrame, pub stored_return: SourceCrtQueryStackWord,
    pub actual_rcx: u64, pub actual_rdx: u64, pub actual_rbx_rsi_rdi: (u64, u64, u64),
}
pub struct SourceCrtContextAllocationCallBindings<P, S, E> {
    pub actual_entry: fn(&E) -> SourceCrtContextAllocationCallEntry,
    pub owned_entry: fn(&P, &S) -> Option<SourceCrtContextAllocationCallEntry>,
}
#[must_use = "retain original stopped parent/history/entry authority while SAME P/S belong to child"]
pub struct SourceCrtContextAllocationParent<P, S, H, E> {
    parent: Option<SourceCrtQueryContext58db7c>, history: H,
    bindings: SourceCrtQueryOwnedBindings<P, S>, actual_entry: E,
    entry_bindings: SourceCrtContextAllocationCallBindings<P, S, E>,
    entry: SourceCrtContextAllocationCallEntry, expected: SourceCrtQueryContextCall,
}
impl<P, S, H, E> SourceCrtContextAllocationParent<P, S, H, E> {
    pub fn history(&self) -> &H { &self.history }
    pub fn parent_source(&self) -> Option<&SourceCrtQueryContext58db7c> { self.parent.as_ref() }
    pub fn source_call(&self) -> SourceCrtQueryCallEnvelope { self.entry.call }
    pub fn required_target(&self) -> u64 { self.expected.required_actual_resume }
}
#[must_use = "retain both original parent and SAME child/provider/advanced history"]
pub struct SourceCrtContextAllocationChild<P, S, H, CH, E> {
    child: SourceCrtQueryOwned<P, S, CH>, parent: SourceCrtContextAllocationParent<P, S, H, E>,
}
#[must_use = "retain all rejected handoff inputs with original parent stopped"]
pub struct SourceCrtContextAllocationHoldFailure<P, S, H, CH, E> {
    pub parent: SourceCrtQueryOwned<P, S, H>, pub child: SourceCrtQueryCalloc58f074, pub child_history: CH,
    pub actual_entry: E, pub entry_bindings: SourceCrtContextAllocationCallBindings<P, S, E>,
    pub child_bindings: SourceCrtQueryOwnedBindings<P, S>,
    pub observed: SourceCrtContextAllocationCallEntry, pub owned: Option<SourceCrtContextAllocationCallEntry>,
    pub parent_frame: SourceCrtNativeRetFrame, pub child_frame: SourceCrtNativeRetFrame,
}
pub enum SourceCrtContextAllocationRetBindFailure<P, S, H, CH, E> {
    BeforeRet(SourceCrtContextAllocationChild<P, S, H, CH, E>),
    Native(SourceCrtQueryRetBindFailure<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>>),
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn hold_actual_context_allocation_child<CH, E>(mut self, child: SourceCrtQueryCalloc58f074,
        child_history: CH, actual_entry: E, entry_bindings: SourceCrtContextAllocationCallBindings<P, S, E>,
        child_bindings: SourceCrtQueryOwnedBindings<P, S>)
        -> Result<SourceCrtContextAllocationChild<P, S, H, CH, E>, SourceCrtContextAllocationHoldFailure<P, S, H, CH, E>> {
        self.stopped = true; self.cursor.stop();
        let observed = (entry_bindings.actual_entry)(&actual_entry);
        let owned = (entry_bindings.owned_entry)(&self.prefix, &self.services);
        let parent_frame = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let child_frame = (child_bindings.actual_frame)(&self.prefix, &self.services);
        let expected = match &self.cursor {
            SourceCrtQueryCursor::Context(source) => match source.awaiting_actual_completion() {
                Some(SourceCrtQueryContextRequest::RequiredCall(call)) if call.kind == SourceCrtQueryContextCallKind::Allocate => Some((source.entry(), call)), _ => None },
            _ => None,
        };
        let valid = match expected {
            Some((parent, call)) => {
                let child_entry = child.entry();
                child_entry.actual_module_base == parent.actual_module_base
                    && child_entry.actual_entry_rsp == call.actual_callee_entry_rsp
                    && child_entry.actual_entry_rbx == call.required_rbx
                    && child_entry.arguments == SourceCrtCallocArguments::ContextOneBy3c8
                    && child_entry.actual_count_rcx == 1 && child_entry.actual_element_rdx == 0x3c8
                    && observed.call == (SourceCrtQueryCallEnvelope { actual_module_base: parent.actual_module_base,
                        source_call_pc: 0x58dbd9, source_resume_pc: 0x58dbde, target_rva: 0x58f074,
                        actual_work_rsp: call.actual_caller_rsp, source_rcx: Some(1), source_rdx: Some(0x3c8) })
                    && observed.before.actual_rip == call.source_pc && observed.before.actual_rsp == call.actual_caller_rsp
                    && observed.entered == (SourceCrtNativeRetFrame { actual_rip: call.actual_target,
                        actual_rsp: call.actual_callee_entry_rsp, actual_rax: observed.before.actual_rax })
                    && observed.stored_return == (SourceCrtQueryStackWord { actual_address: call.actual_callee_entry_rsp,
                        actual_word: call.required_actual_resume })
                    && observed.actual_rcx == 1 && observed.actual_rdx == 0x3c8
                    && observed.actual_rbx_rsi_rdi == (call.required_rbx, call.required_rsi, call.required_rdi)
                    && owned == Some(observed) && parent_frame == child_frame
            }
            None => false,
        };
        if !valid {
            return Err(SourceCrtContextAllocationHoldFailure { parent: self, child, child_history, actual_entry,
                entry_bindings, child_bindings, observed, owned, parent_frame, child_frame });
        }
        let Some((_, expected)) = expected else {
            return Err(SourceCrtContextAllocationHoldFailure { parent: self, child, child_history, actual_entry,
                entry_bindings, child_bindings, observed, owned, parent_frame, child_frame });
        };
        let Self { cursor, prefix, services, history, bindings, .. } = self;
        let parent = match cursor {
            SourceCrtQueryCursor::Context(parent) => parent,
            cursor => return Err(SourceCrtContextAllocationHoldFailure {
                parent: Self { cursor, prefix, services, history, bindings, stopped: true }, child, child_history,
                actual_entry, entry_bindings, child_bindings, observed, owned, parent_frame, child_frame }),
        };
        Ok(SourceCrtContextAllocationChild {
            child: SourceCrtQueryOwned::from_same_cursor_and_history(SourceCrtQueryCursor::Calloc(child), prefix, services,
                child_history, child_bindings),
            parent: SourceCrtContextAllocationParent { parent: Some(parent), history, bindings, actual_entry,
                entry_bindings, entry: observed, expected },
        })
    }
}
impl<P, S, H, CH, E> SourceCrtContextAllocationChild<P, S, H, CH, E> {
    pub fn request(&self) -> Option<SourceCrtQueryRequest> { self.child.request() }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryRequest> { self.child.awaiting_actual_completion() }
    pub fn child_history(&self) -> &CH { self.child.history() }
    pub fn parent_history(&self) -> &H { self.parent.history() }

    pub fn prefix_mut(&mut self) -> &mut P { &mut self.child.prefix }
    pub fn services_mut(&mut self) -> &mut S { &mut self.child.services }
    pub fn actual_prefix_and_services_mut(&mut self) -> (&mut P, &mut S) {
        (&mut self.child.prefix, &mut self.child.services)
    }
    pub fn accept_actual_receipt<R>(self, actual: R, supplied: SourceCrtQueryReceipt,
        project: fn(&R) -> SourceCrtQueryReceipt)
        -> Result<SourceCrtContextAllocationChild<P, S, H, SourceCrtQueryHistory<CH, R>, E>,
            SourceCrtNativeReturnFailure<Self, SourceCrtQueryReceiptError<R>>> {
        let Self { child, parent } = self;
        match child.accept_actual_receipt(actual, supplied, project) {
            Ok(child) => Ok(SourceCrtContextAllocationChild { child, parent }),
            Err(failure) => Err(SourceCrtNativeReturnFailure { owner: Self { child: failure.owner, parent }, cause: failure.cause }),
        }
    }
    pub fn retain_rejected_receipt<R>(failure: SourceCrtNativeReturnFailure<Self, SourceCrtQueryReceiptError<R>>)
        -> SourceCrtContextAllocationChild<P, S, H, SourceCrtQueryRejectedHistory<CH, R>, E> {
        let Self { child, parent } = failure.owner;
        SourceCrtContextAllocationChild { child: SourceCrtQueryOwned::retain_rejected_receipt(
            SourceCrtNativeReturnFailure { owner: child, cause: failure.cause }), parent }
    }
    pub fn retain_actual_service_failure<F>(self, actual: F)
        -> SourceCrtContextAllocationChild<P, S, H, SourceCrtQueryServiceFailureHistory<CH, F>, E> {
        SourceCrtContextAllocationChild { child: self.child.retain_actual_service_failure(actual), parent: self.parent }
    }
    pub fn accept_actual_ret_entry_prefix<R>(self, actual: R, project: fn(&R) -> SourceCrtNativeRetFrame)
        -> Result<SourceCrtContextAllocationChild<P, S, H, SourceCrtQueryRetPrefixHistory<CH, R>, E>,
            SourceCrtNativeReturnFailure<Self, SourceCrtQueryRetPrefixError<R>>> {
        let Self { child, parent } = self;
        match child.accept_actual_ret_entry_prefix(actual, project) {
            Ok(child) => Ok(SourceCrtContextAllocationChild { child, parent }),
            Err(failure) => Err(SourceCrtNativeReturnFailure { owner: Self { child: failure.owner, parent }, cause: failure.cause }),
        }
    }
    pub fn into_existing_native_ret(self)
        -> Result<SourceCrtQueryNativeRet<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>>,
            SourceCrtContextAllocationRetBindFailure<P, S, H, CH, E>> {
        let Self { child, parent } = self;
        let completed = match child.into_before_ret() {
            Ok(child) => child,
            Err(child) => return Err(SourceCrtContextAllocationRetBindFailure::BeforeRet(Self { child, parent })),
        };
        SourceCrtQueryNativeRet::bind_same_before_ret(completed, parent, SourceCrtContextAllocationParent::required_target)
            .map_err(SourceCrtContextAllocationRetBindFailure::Native)
    }
}
