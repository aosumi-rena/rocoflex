use super::*;
use super::super::crt_query_context::SourceCrtQueryStackWord;
use super::super::crt_query_resolve::SourceCrtQueryResolveCallKind;
use super::super::crt_query_return::{SourceCrtQueryCallEnvelope, SourceCrtQueryCallerReceipt,
    SourceCrtQueryNativeRet, SourceCrtQueryRetBindFailure, SourceCrtQueryReturned,
    SourceCrtQueryCallerReturned, SourceCrtQueryCallerBindFailure};
use super::super::crt_query_join::{SourceCrtQueryFreshContextError, SourceCrtQueryResolverError,
    SourceCrtQueryUpdateError};
use super::super::crt_storage_native_return::{SourceCrtNativeRetObservation,
    SourceCrtNativeRetExpectation, SourceCrtNativeRetired};

#[path = "crt_query_owned_helpers_resume.rs"]
pub mod resume;

#[path = "crt_query_owned_helpers_correction.rs"]
pub mod correction;

pub enum SourceCrtQueryHelperSource {
    FreshContext(SourceCrtQueryContext58db7c),
    Resolver(SourceCrtQueryResolve5941fc),
    SlotReplacement(SourceCrtQueryReplace59426c),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryHelperEdge { FreshContext594206, Resolver58e25b, SlotReplacement59423f }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryHelperCallEntry {
    pub call: SourceCrtQueryCallEnvelope,
    pub before: SourceCrtNativeRetFrame, pub entered: SourceCrtNativeRetFrame,
    pub stored_return: SourceCrtQueryStackWord,
    pub actual_rcx: u64, pub actual_rdx: u64,
    pub actual_rbx_rsi_rdi: (u64, u64, u64),
}
pub struct SourceCrtQueryHelperCallBindings<P, S, E> {
    pub actual_entry: fn(&E) -> SourceCrtQueryHelperCallEntry,
    pub owned_entry: fn(&P, &S) -> Option<SourceCrtQueryHelperCallEntry>,
}
#[must_use = "retain original stopped parent C/H and actual independent call authority"]
pub struct SourceCrtQueryHelperParent<P, S, H, E> {
    parent: Option<SourceCrtQueryCursor>, history: H,
    bindings: SourceCrtQueryOwnedBindings<P, S>, actual_entry: E,
    entry_bindings: SourceCrtQueryHelperCallBindings<P, S, E>,
    entry: SourceCrtQueryHelperCallEntry, expected: SourceCrtQueryRequest,
    edge: SourceCrtQueryHelperEdge,
}
impl<P, S, H, E> SourceCrtQueryHelperParent<P, S, H, E> {
    pub fn history(&self) -> &H { &self.history }
    pub fn original_pending(&self) -> Option<SourceCrtQueryRequest> {
        self.parent.as_ref().and_then(SourceCrtQueryCursor::pending)
    }
    pub fn edge(&self) -> SourceCrtQueryHelperEdge { self.edge }
    pub fn source_call(&self) -> SourceCrtQueryCallEnvelope { self.entry.call }
    pub fn required_target(&self) -> u64 {
        self.entry.call.actual_module_base.wrapping_add(u64::from(self.entry.call.source_resume_pc))
    }
}

#[must_use = "retain both child P/S/resources and independent original parent token"]
pub struct SourceCrtQueryHelperEntered<P, S, H, CH, E> {
    pub child: SourceCrtQueryOwned<P, S, CH>,
    pub parent: SourceCrtQueryHelperParent<P, S, H, E>,
}
#[must_use = "retain rejected original parent, existing child/history, actual entry and advanced P/S"]
pub struct SourceCrtQueryHelperHoldFailure<P, S, H, CH, E> {
    pub parent: SourceCrtQueryOwned<P, S, H>, pub child: SourceCrtQueryHelperSource,
    pub child_history: CH, pub actual_entry: E,
    pub entry_bindings: SourceCrtQueryHelperCallBindings<P, S, E>,
    pub child_bindings: SourceCrtQueryOwnedBindings<P, S>,
    pub observed: SourceCrtQueryHelperCallEntry, pub owned: Option<SourceCrtQueryHelperCallEntry>,
    pub parent_frame: SourceCrtNativeRetFrame, pub child_frame: SourceCrtNativeRetFrame,
}
pub enum SourceCrtQueryHelperRetBindFailure<P, S, H, CH, E> {
    BeforeRet(SourceCrtQueryHelperEntered<P, S, H, CH, E>),
    Native(SourceCrtQueryRetBindFailure<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>>),
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn hold_actual_query_helper<CH, E>(mut self, child: SourceCrtQueryHelperSource,
        child_history: CH, actual_entry: E, entry_bindings: SourceCrtQueryHelperCallBindings<P, S, E>,
        child_bindings: SourceCrtQueryOwnedBindings<P, S>)
        -> Result<SourceCrtQueryHelperEntered<P, S, H, CH, E>, SourceCrtQueryHelperHoldFailure<P, S, H, CH, E>> {
        self.stopped = true; self.cursor.stop();
        let observed = (entry_bindings.actual_entry)(&actual_entry);
        let owned = (entry_bindings.owned_entry)(&self.prefix, &self.services);
        let parent_frame = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let child_frame = (child_bindings.actual_frame)(&self.prefix, &self.services);
        let pending = self.cursor.pending();
        let expected = match (&self.cursor, pending, &child) {
            (SourceCrtQueryCursor::Resolve(parent),
                Some(SourceCrtQueryRequest::Resolve(SourceCrtQueryResolveRequest::RequiredCall(call))),
                SourceCrtQueryHelperSource::FreshContext(source))
                if call.kind == SourceCrtQueryResolveCallKind::FreshContext58db7c => {
                let parent = parent.entry(); let entry = source.entry();
                let valid = entry.actual_module_base == parent.actual_module_base
                    && entry.actual_entry_rsp == call.actual_caller_rsp.wrapping_sub(8)
                    && entry.actual_entry_rbx == parent.actual_entry_rbx
                    && entry.actual_entry_rdi == parent.actual_entry_rdi
                    && observed.actual_rbx_rsi_rdi == (entry.actual_entry_rbx, entry.actual_entry_rsi, entry.actual_entry_rdi)
                    && observed.before.actual_rax == parent.actual_entry_rax
                    && observed.actual_rcx == parent.actual_entry_rcx && observed.actual_rdx == parent.actual_entry_rdx
                    && call.source_rcx.is_none() && call.source_rdx.is_none();
                Some((SourceCrtQueryHelperEdge::FreshContext594206, valid,
                    SourceCrtQueryCallEnvelope { actual_module_base: parent.actual_module_base,
                        source_call_pc: 0x594206, source_resume_pc: 0x59420b, target_rva: 0x58db7c,
                        actual_work_rsp: call.actual_caller_rsp, source_rcx: None, source_rdx: None },
                    call.source_pc, call.actual_target, call.required_actual_resume))
            }
            (SourceCrtQueryCursor::Resolve(parent),
                Some(SourceCrtQueryRequest::Resolve(SourceCrtQueryResolveRequest::RequiredCall(call))),
                SourceCrtQueryHelperSource::SlotReplacement(source))
                if call.kind == SourceCrtQueryResolveCallKind::Update59426c => {
                let parent = parent.entry(); let entry = source.entry();
                let valid = entry.actual_module_base == parent.actual_module_base
                    && entry.actual_entry_rsp == call.actual_caller_rsp.wrapping_sub(8)
                    && entry.actual_entry_rax == observed.entered.actual_rax
                    && (entry.actual_entry_rbx, entry.actual_entry_rdi)
                        == (observed.actual_rbx_rsi_rdi.0, observed.actual_rbx_rsi_rdi.2)
                    && call.source_rcx == Some(entry.actual_slot_rcx)
                    && call.source_rdx == Some(entry.actual_incoming_rdx)
                    && call.required_saved_rdi == Some(entry.actual_entry_rdi)
                    && call.required_saved_rbx.map_or(true, |word| word == entry.actual_entry_rbx);
                Some((SourceCrtQueryHelperEdge::SlotReplacement59423f, valid,
                    SourceCrtQueryCallEnvelope { actual_module_base: parent.actual_module_base,
                        source_call_pc: 0x59423f, source_resume_pc: 0x594244, target_rva: 0x59426c,
                        actual_work_rsp: call.actual_caller_rsp, source_rcx: call.source_rcx, source_rdx: call.source_rdx },
                    call.source_pc, call.actual_target, call.required_actual_resume))
            }
            (SourceCrtQueryCursor::Refresh(parent),
                Some(SourceCrtQueryRequest::Refresh(SourceCrtQueryRefreshRequest::Required5941fc {
                    source_pc, actual_target, actual_rcx, actual_rdx, actual_caller_rsp, required_actual_resume_rip })),
                SourceCrtQueryHelperSource::Resolver(source)) => {
                let entry = source.entry(); let parent_entry = parent.entry();
                let valid = entry.actual_module_base == parent_entry.actual_module_base
                    && entry.actual_entry_rsp == actual_caller_rsp.wrapping_sub(8)
                    && entry.actual_entry_rax == parent.source_rax() && observed.before.actual_rax == parent.source_rax()
                    && entry.actual_entry_rcx == actual_rcx && entry.actual_entry_rdx == actual_rdx
                    && actual_rcx == parent_entry.actual_thread_rcx && actual_rdx == parent_entry.actual_slot_rdx
                    && entry.actual_entry_rbx == parent_entry.actual_slot_rdx
                    && (entry.actual_entry_rbx, entry.actual_entry_rdi)
                        == (observed.actual_rbx_rsi_rdi.0, observed.actual_rbx_rsi_rdi.2);
                Some((SourceCrtQueryHelperEdge::Resolver58e25b, valid,
                    SourceCrtQueryCallEnvelope { actual_module_base: parent_entry.actual_module_base,
                        source_call_pc: 0x58e25b, source_resume_pc: 0x58e260, target_rva: 0x5941fc,
                        actual_work_rsp: actual_caller_rsp, source_rcx: Some(actual_rcx), source_rdx: Some(actual_rdx) },
                    source_pc, actual_target, required_actual_resume_rip))
            }
            _ => None,
        };
        let valid = expected.is_some_and(|(_, child_matches, envelope, pc, target, resume)| {
            child_matches && observed.call == envelope && owned == Some(observed) && parent_frame == child_frame
                && pc == envelope.actual_module_base.wrapping_add(u64::from(envelope.source_call_pc))
                && target == envelope.actual_module_base.wrapping_add(u64::from(envelope.target_rva))
                && resume == envelope.actual_module_base.wrapping_add(u64::from(envelope.source_resume_pc))
                && observed.before.actual_rip == pc && observed.before.actual_rsp == envelope.actual_work_rsp
                && observed.entered == (SourceCrtNativeRetFrame { actual_rip: target,
                    actual_rsp: envelope.actual_work_rsp.wrapping_sub(8), actual_rax: observed.before.actual_rax })
                && observed.stored_return == (SourceCrtQueryStackWord {
                    actual_address: observed.entered.actual_rsp, actual_word: resume })
                && envelope.source_rcx.map_or(true, |word| word == observed.actual_rcx)
                && envelope.source_rdx.map_or(true, |word| word == observed.actual_rdx)
        });
        if !valid {
            return Err(SourceCrtQueryHelperHoldFailure { parent: self, child, child_history, actual_entry,
                entry_bindings, child_bindings, observed, owned, parent_frame, child_frame });
        }
        let (Some((edge, _, _, _, _, _)), Some(expected)) = (expected, pending) else {
            return Err(SourceCrtQueryHelperHoldFailure { parent: self, child, child_history, actual_entry,
                entry_bindings, child_bindings, observed, owned, parent_frame, child_frame });
        };
        let Self { cursor, prefix, services, history, bindings, .. } = self;
        let child = match child {
            SourceCrtQueryHelperSource::FreshContext(source) => SourceCrtQueryCursor::Context(source),
            SourceCrtQueryHelperSource::Resolver(source) => SourceCrtQueryCursor::Resolve(source),
            SourceCrtQueryHelperSource::SlotReplacement(source) => SourceCrtQueryCursor::Replace(source),
        };
        Ok(SourceCrtQueryHelperEntered {
            child: SourceCrtQueryOwned::from_same_cursor_and_history(child, prefix, services, child_history, child_bindings),
            parent: SourceCrtQueryHelperParent { parent: Some(cursor), history, bindings, actual_entry,
                entry_bindings, entry: observed, expected, edge },
        })
    }
}
impl<P, S, H, CH, E> SourceCrtQueryHelperEntered<P, S, H, CH, E> {

    pub fn into_existing_native_ret(self)
        -> Result<SourceCrtQueryNativeRet<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>>,
            SourceCrtQueryHelperRetBindFailure<P, S, H, CH, E>> {
        let Self { child, parent } = self;
        let completed = match child.into_before_ret() {
            Ok(child) => child,
            Err(child) => return Err(SourceCrtQueryHelperRetBindFailure::BeforeRet(Self { child, parent })),
        };
        SourceCrtQueryNativeRet::bind_same_before_ret(completed, parent, SourceCrtQueryHelperParent::required_target)
            .map_err(SourceCrtQueryHelperRetBindFailure::Native)
    }
}
