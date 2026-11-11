use super::*;
use super::helpers::SourceCrtQueryHelperCallEntry;
use super::super::crt_query_context::SourceCrtQueryStackWord;
use super::super::crt_query_replace::{SourceCrtQueryReplaceCall, SourceCrtQueryReplaceCallKind};
use super::super::crt_query_return::SourceCrtQueryCallEnvelope;

#[path = "crt_query_replace_retirement_admission.rs"]
mod admission;
#[path = "crt_query_replace_retirement_transfer.rs"]
pub mod transfer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementCallEntry {
    pub call: SourceCrtQueryHelperCallEntry, pub actual_rbp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementPush {
    pub before_rsp: u64, pub after_rsp: u64, pub actual_word: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementCheckedEntry {
    pub pc: u32, pub actual_module_base: u64, pub actual_rsp: u64, pub receiver_rcx: u64,

    pub saved: [u64; 4], pub stores_returned: [bool; 3],
    pub push: Option<SourceCrtReplaceRetirementPush>, pub subtract_before_after: Option<(u64, u64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementBody {
    pub pc: u32, pub actual_module_base: u64, pub actual_rsp: u64, pub receiver_rcx: u64,
    pub entry_rbp: u64, pub entry_rsi: u64, pub entry_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceRetirementPosition {
    CheckedEntry593ffc, BodyEntry594010, BodyCompleted594156, RestoreEntry594156,
    TailRequired59416d, TailServiceEntry59416d,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementTail {
    pub pc: u32, pub actual_target: u64, pub actual_rsp: u64, pub receiver_rcx: u64,

    pub restored: [u64; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementDescription {
    pub position: SourceCrtReplaceRetirementPosition,
    pub checked_entry: Option<SourceCrtReplaceRetirementCheckedEntry>,
    pub body: SourceCrtReplaceRetirementBody,
    pub tail: Option<SourceCrtReplaceRetirementTail>,
}

pub struct SourceCrtReplaceRetirementBindings<D, P, S, W, E> {
    pub describe: fn(&D) -> Option<SourceCrtReplaceRetirementDescription>,
    pub actual_entry: fn(&E) -> SourceCrtReplaceRetirementCallEntry,
    pub owned_entry: fn(&P, &S, &W) -> Option<SourceCrtReplaceRetirementCallEntry>,
    pub active_call: fn(&W) -> Option<SourceCrtQueryCallEnvelope>,
    pub actual_prefix: fn(&D, &P, &S, &W) -> SourceCrtNativeRetFrame,
    pub actual_saved_registers: fn(&D, &P, &S, &W) -> [u64; 4],
}
#[must_use = "retain genuine retirement progress and distinct wholecall/CALL/native resources"]
pub struct SourceCrtReplaceRetirementResources<D, W, E, N> {
    pub child: D, pub whole_call: W, pub actual_entry: E, pub native_resources: N,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementEvidence {
    pub request: SourceCrtQueryReplaceCall,
    pub entry: SourceCrtReplaceRetirementCallEntry,
    pub description: SourceCrtReplaceRetirementDescription,
    pub actual_prefix: SourceCrtNativeRetFrame, pub actual_saved_registers: [u64; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceRetirementReason {
    ActualWholeCallRequired, CheckedEntryRequired, ActualPrefixRequired,
}
#[must_use = "retain stopped original C/H/P/S and complete rejected actual retirement resources"]
pub struct SourceCrtReplaceRetirementRejected<D, P, S, H, W, E, N> {
    pub query: SourceCrtQueryOwned<P, S, H>,
    pub resources: SourceCrtReplaceRetirementResources<D, W, E, N>,
    pub bindings: SourceCrtReplaceRetirementBindings<D, P, S, W, E>,
    pub request: Option<SourceCrtQueryReplaceCall>,
    pub description: Option<SourceCrtReplaceRetirementDescription>,
    pub entry: SourceCrtReplaceRetirementCallEntry, pub owned_entry: Option<SourceCrtReplaceRetirementCallEntry>,
    pub active_call: Option<SourceCrtQueryCallEnvelope>,
    pub actual_prefix: SourceCrtNativeRetFrame, pub current_prefix: SourceCrtNativeRetFrame,
    pub actual_saved_registers: [u64; 4], pub reason: SourceCrtReplaceRetirementReason,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementRejectionHistory {
    pub request: Option<SourceCrtQueryReplaceCall>,
    pub description: Option<SourceCrtReplaceRetirementDescription>,
    pub entry: SourceCrtReplaceRetirementCallEntry, pub owned_entry: Option<SourceCrtReplaceRetirementCallEntry>,
    pub active_call: Option<SourceCrtQueryCallEnvelope>,
    pub actual_prefix: SourceCrtNativeRetFrame, pub current_prefix: SourceCrtNativeRetFrame,
    pub actual_saved_registers: [u64; 4], pub reason: SourceCrtReplaceRetirementReason,
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceRetirementRejected<D, P, S, H, W, E, N> {

    pub fn retain_for_actual_correction(self)
        -> (SourceCrtQueryOwned<P, S, (H, SourceCrtReplaceRetirementRejectionHistory)>,
            SourceCrtReplaceRetirementResources<D, W, E, N>, SourceCrtReplaceRetirementBindings<D, P, S, W, E>) {
        let Self { query, resources, bindings, request, description, entry, owned_entry, active_call,
            actual_prefix, current_prefix, actual_saved_registers, reason } = self;
        let history = SourceCrtReplaceRetirementRejectionHistory { request, description, entry, owned_entry,
            active_call, actual_prefix, current_prefix, actual_saved_registers, reason };
        (query.with_retained_history(history), resources, bindings)
    }
}

#[must_use = "whole593ffc/58d910 normal return and resource adoption remain REQUIRED"]
pub struct SourceCrtReplaceRetirementRequired<D, P, S, H, W, E, N> {
    query: SourceCrtQueryOwned<P, S, H>,
    resources: SourceCrtReplaceRetirementResources<D, W, E, N>,
    bindings: SourceCrtReplaceRetirementBindings<D, P, S, W, E>,
    evidence: SourceCrtReplaceRetirementEvidence,
}

#[must_use = "raw exit still owns outstanding593ffc and unclosed58d910/native/physical/adoption requirements"]
pub struct SourceCrtReplaceRetirementRequiredParts<D, P, S, H, W, E, N> {
    pub query: SourceCrtQueryOwned<P, S, H>,
    pub resources: SourceCrtReplaceRetirementResources<D, W, E, N>,
    pub bindings: SourceCrtReplaceRetirementBindings<D, P, S, W, E>,
    pub evidence: SourceCrtReplaceRetirementEvidence,
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceRetirementRequired<D, P, S, H, W, E, N> {
    pub fn required_whole_call(&self) -> SourceCrtQueryReplaceCall { self.evidence.request }
    pub fn evidence(&self) -> SourceCrtReplaceRetirementEvidence { self.evidence }
    pub fn history(&self) -> &H { self.query.history() }
    pub fn actual_child(&self) -> &D { &self.resources.child }
    pub fn actual_prefix_services_and_call_mut(&mut self) -> (&mut P, &mut S, &mut W) {
        (&mut self.query.prefix, &mut self.query.services, &mut self.resources.whole_call)
    }
    pub fn into_required_owner_parts(self) -> SourceCrtReplaceRetirementRequiredParts<D, P, S, H, W, E, N> {
        SourceCrtReplaceRetirementRequiredParts { query: self.query, resources: self.resources,
            bindings: self.bindings, evidence: self.evidence }
    }
}
