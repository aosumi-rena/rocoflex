use super::*;
use super::helpers::SourceCrtQueryHelperCallEntry;
use super::super::crt_query_replace::{SourceCrtQueryReplaceCall, SourceCrtQueryReplaceCallKind};
use super::super::crt_query_return::SourceCrtQueryCallEnvelope;
use super::super::crt_storage_native_return::{SourceCrtNativeRetObservation, SourceCrtNativeRetExpectation,
    SourceCrtNativeRetired, validate_source_crt_native_ret};

#[path = "crt_query_replace_counter_admission.rs"]
mod admission;
#[path = "crt_query_replace_counter_adoption.rs"]
pub mod adoption;

#[path = "crt_query_replace_counter_continuation.rs"]
pub mod continuation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceCounterProfile { Increment593ec8, Decrement593f58 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterBody {
    pub profile: SourceCrtReplaceCounterProfile, pub pc: u32,
    pub actual_module_base: u64, pub actual_rsp: u64, pub receiver_rcx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterEntry {
    pub pc: u32, pub actual_module_base: u64, pub actual_rsp: u64, pub receiver_rcx: u64,
    pub subtract_before_after: Option<(u64, u64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceCounterReturnPath {
    IncrementTail { tail_pc: u32, actual_target: u64, tail_rsp: u64,
        leaf_pc: u32, leaf_rsp: u64, leaf_ret_pc: u32 },
    DecrementParent { checked_entry: Option<SourceCrtReplaceCounterEntry>,
        restore_pc: u32, restore_rsp: u64, restore_rax: u64,
        add_before_after: Option<(u64, u64)> },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterReturnWords {
    pub before_rsp: u64, pub after_rsp: u64, pub actual_target: u64, pub actual_rax: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterDescription {
    pub body: SourceCrtReplaceCounterBody, pub path: SourceCrtReplaceCounterReturnPath,
    pub returned: SourceCrtReplaceCounterReturnWords,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterWholeReturn {
    pub call: SourceCrtQueryCallEnvelope, pub actual_frame: SourceCrtNativeRetFrame,
    pub actual_rbx: u64, pub actual_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterEvidence {
    pub request: SourceCrtQueryReplaceCall,
    pub description: SourceCrtReplaceCounterDescription,
    pub entry: SourceCrtQueryHelperCallEntry,
    pub whole: SourceCrtReplaceCounterWholeReturn, pub retired: SourceCrtNativeRetired,
}

pub struct SourceCrtReplaceCounterBindings<D, P, S, W, E, N> {

    pub describe: fn(&D) -> Option<SourceCrtReplaceCounterDescription>,
    pub actual_entry: fn(&E) -> SourceCrtQueryHelperCallEntry,
    pub owned_entry: fn(&P, &S, &W) -> Option<SourceCrtQueryHelperCallEntry>,
    pub whole_return: fn(&W) -> SourceCrtReplaceCounterWholeReturn,
    pub current_registers: fn(&P, &S) -> (u64, u64),
    pub actual_ret: fn(&N) -> SourceCrtNativeRetObservation,
    pub owned_ret: fn(&P, &S, &W) -> Option<SourceCrtNativeRetObservation>,
}

#[must_use = "retain distinct completed counter, whole-call, entry and actual native owners"]
pub struct SourceCrtReplaceCounterResources<D, W, E, N> {
    held: Option<D>, pub whole_call: W, pub actual_entry: E, pub actual_ret: N,
}
impl<D, W, E, N> SourceCrtReplaceCounterResources<D, W, E, N> {
    pub fn from_actual_completed_owner(completed: D, whole_call: W, actual_entry: E, actual_ret: N) -> Self {
        Self { held: Some(completed), whole_call, actual_entry, actual_ret }
    }
    pub fn held(&self) -> Option<&D> { self.held.as_ref() }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceCounterAdmissionReason { WholeCallRequired, EntryOrReturnRequired, PrefixRequired }
#[must_use = "retain original stopped query and all actual rejected child/frame/FP/resources"]
pub struct SourceCrtReplaceCounterRejected<D, P, S, H, W, E, N> {
    pub query: SourceCrtQueryOwned<P, S, H>, pub resources: SourceCrtReplaceCounterResources<D, W, E, N>,
    pub bindings: SourceCrtReplaceCounterBindings<D, P, S, W, E, N>,
    pub request: Option<SourceCrtQueryReplaceCall>, pub description: Option<SourceCrtReplaceCounterDescription>,
    pub entry: SourceCrtQueryHelperCallEntry, pub owned_entry: Option<SourceCrtQueryHelperCallEntry>,
    pub whole: SourceCrtReplaceCounterWholeReturn,
    pub observed_ret: SourceCrtNativeRetObservation, pub owned_ret: Option<SourceCrtNativeRetObservation>,
    pub frame: SourceCrtNativeRetFrame, pub registers: (u64, u64),
    pub reason: SourceCrtReplaceCounterAdmissionReason,
}
#[must_use = "original source remains stopped until actual OWN heldNone adoption"]
pub struct SourceCrtReplaceCounterBound<D, P, S, H, W, E, N> {
    query: SourceCrtQueryOwned<P, S, H>, resources: SourceCrtReplaceCounterResources<D, W, E, N>,
    bindings: SourceCrtReplaceCounterBindings<D, P, S, W, E, N>,
    evidence: SourceCrtReplaceCounterEvidence, adoption_attempted: bool,
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceCounterBound<D, P, S, H, W, E, N> {
    pub fn evidence(&self) -> SourceCrtReplaceCounterEvidence { self.evidence }
    pub fn history(&self) -> &H { self.query.history() }
    pub fn adoption_was_attempted(&self) -> bool { self.adoption_attempted }
    pub fn source_pending(&self) -> Option<SourceCrtQueryRequest> { self.query.awaiting_actual_completion() }
    pub fn held(&self) -> Option<&D> { self.resources.held() }
    pub fn actual_prefix_services_and_call(&self) -> (&P, &S, &W) {
        (&self.query.prefix, &self.query.services, &self.resources.whole_call)
    }

    pub fn actual_prefix_services_and_call_mut(&mut self) -> (&mut P, &mut S, &mut W) {
        (&mut self.query.prefix, &mut self.query.services, &mut self.resources.whole_call)
    }
}
