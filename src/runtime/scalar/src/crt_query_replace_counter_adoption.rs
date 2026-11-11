use super::*;

pub struct SourceCrtReplaceCounterAdoption<D, P, S, W, A, F> {
    pub adopt: fn(&mut P, &mut S, &mut W, &mut Option<D>, SourceCrtReplaceCounterEvidence) -> Result<A, F>,
    pub receipt: fn(&A) -> SourceCrtReplaceCounterEvidence,
    pub owned_receipt: fn(&P, &S, &W) -> Option<SourceCrtReplaceCounterEvidence>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtReplaceCounterAdoptionReason {
    AlreadyAttempted, HeldResourcesRemain, ActualReceiptRequired, OriginalSourceRequired, AdvancedPrefixRequired,
}
#[must_use = "retain actual whole-call/adoption/native owners with continuing original query"]
pub struct SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F> {
    pub whole_call: W, pub actual_entry: E, pub actual_ret: N, pub adoption_receipt: A,
    pub evidence: SourceCrtReplaceCounterEvidence,
    pub bindings: SourceCrtReplaceCounterBindings<D, P, S, W, E, N>,
    pub adoption_bindings: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>,
}
impl<D, P, S, W, E, N, A, F> SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F> {
    fn source(&self) -> SourceCrtQueryReceipt {
        let actual = (self.bindings.whole_return)(&self.whole_call);
        SourceCrtQueryReceipt::Replace(SourceCrtQueryReplaceReceipt::CallReturned {
            request: self.evidence.request, actual_frame: actual.actual_frame,
            actual_rbx: actual.actual_rbx, actual_rdi: actual.actual_rdi })
    }
}
#[must_use = "retain same stopped query/counter/call/native resources and advanced adoption errors"]
pub enum SourceCrtReplaceCounterAdoptionFailure<D, P, S, H, W, E, N, A, F> {
    Service(SourceCrtReplaceCounterAdoptionServiceFailure<D, P, S, H, W, E, N, A, F>),
    Rejected(SourceCrtReplaceCounterAdoptionRejected<D, P, S, H, W, E, N, A, F>),
    Source(SourceCrtNativeReturnFailure<SourceCrtQueryOwned<P, S, H>,
        SourceCrtQueryReceiptError<SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>),
}

#[must_use = "retain original adoption bindings and actual advanced service error"]
pub struct SourceCrtReplaceCounterAdoptionServiceFailure<D, P, S, H, W, E, N, A, F> {
    pub(super) pending: SourceCrtReplaceCounterBound<D, P, S, H, W, E, N>,
    pub(super) adoption: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>,
    pub(super) error: F,
}
impl<D, P, S, H, W, E, N, A, F> SourceCrtReplaceCounterAdoptionServiceFailure<D, P, S, H, W, E, N, A, F> {
    pub fn pending(&self) -> &SourceCrtReplaceCounterBound<D, P, S, H, W, E, N> { &self.pending }
    pub fn error(&self) -> &F { &self.error }
}
#[must_use = "retain original adoption bindings and actual rejected ownership receipt"]
pub struct SourceCrtReplaceCounterAdoptionRejected<D, P, S, H, W, E, N, A, F> {
    pub(super) pending: SourceCrtReplaceCounterBound<D, P, S, H, W, E, N>,
    pub(super) adoption: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>,
    pub(super) actual: Option<A>,
    pub(super) observed: Option<SourceCrtReplaceCounterEvidence>,
    pub(super) owned: Option<SourceCrtReplaceCounterEvidence>,
    pub(super) reason: SourceCrtReplaceCounterAdoptionReason,
}
impl<D, P, S, H, W, E, N, A, F> SourceCrtReplaceCounterAdoptionRejected<D, P, S, H, W, E, N, A, F> {
    pub fn pending(&self) -> &SourceCrtReplaceCounterBound<D, P, S, H, W, E, N> { &self.pending }
    pub fn actual(&self) -> Option<&A> { self.actual.as_ref() }
    pub fn observed(&self) -> Option<SourceCrtReplaceCounterEvidence> { self.observed }
    pub fn owned(&self) -> Option<SourceCrtReplaceCounterEvidence> { self.owned }
    pub fn reason(&self) -> SourceCrtReplaceCounterAdoptionReason { self.reason }
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceCounterBound<D, P, S, H, W, E, N> {
    fn original_source_and_prefix_match(&self) -> bool {
        self.query.stopped && self.query.cursor.is_stopped()
            && self.query.awaiting_actual_completion() == Some(SourceCrtQueryRequest::Replace(
                SourceCrtQueryReplaceRequest::RequiredCall(self.evidence.request)))
            && (self.bindings.actual_entry)(&self.resources.actual_entry) == self.evidence.entry
            && (self.bindings.owned_entry)(&self.query.prefix, &self.query.services, &self.resources.whole_call) == Some(self.evidence.entry)
            && (self.bindings.actual_ret)(&self.resources.actual_ret) == SourceCrtNativeRetObservation::Retired(self.evidence.retired)
            && (self.bindings.owned_ret)(&self.query.prefix, &self.query.services, &self.resources.whole_call)
                == Some(SourceCrtNativeRetObservation::Retired(self.evidence.retired))
            && (self.bindings.whole_return)(&self.resources.whole_call) == self.evidence.whole
            && (self.query.bindings.actual_frame)(&self.query.prefix, &self.query.services) == self.evidence.retired.after
            && (self.bindings.current_registers)(&self.query.prefix, &self.query.services)
                == (self.evidence.request.required_old_rbx, self.evidence.request.required_incoming_rdi)
    }

    pub fn adopt_completed_counter<A, F>(mut self, adoption: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H,
                SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>,
            SourceCrtReplaceCounterAdoptionFailure<D, P, S, H, W, E, N, A, F>> {
        if self.adoption_attempted {
            return Err(SourceCrtReplaceCounterAdoptionFailure::Rejected(SourceCrtReplaceCounterAdoptionRejected {
                pending: self, adoption, actual: None, observed: None, owned: None,
                reason: SourceCrtReplaceCounterAdoptionReason::AlreadyAttempted }));
        }
        if !self.original_source_and_prefix_match()
            || self.resources.held.as_ref().and_then(self.bindings.describe) != Some(self.evidence.description) {
            return Err(SourceCrtReplaceCounterAdoptionFailure::Rejected(SourceCrtReplaceCounterAdoptionRejected {
                pending: self, adoption, actual: None, observed: None, owned: None,
                reason: SourceCrtReplaceCounterAdoptionReason::AdvancedPrefixRequired }));
        }
        self.adoption_attempted = true;
        let actual = match (adoption.adopt)(&mut self.query.prefix, &mut self.query.services,
            &mut self.resources.whole_call, &mut self.resources.held, self.evidence) {
            Ok(actual) => actual,
            Err(error) => return Err(SourceCrtReplaceCounterAdoptionFailure::Service(SourceCrtReplaceCounterAdoptionServiceFailure {
                pending: self, adoption, error })),
        };
        self.accept_actual_completed_adoption(actual, adoption)
    }

    pub(super) fn accept_actual_completed_adoption<A, F>(self, actual: A,
        adoption: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H,
                SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>,
            SourceCrtReplaceCounterAdoptionFailure<D, P, S, H, W, E, N, A, F>> {
        let observed = (adoption.receipt)(&actual);
        let owned = (adoption.owned_receipt)(&self.query.prefix, &self.query.services, &self.resources.whole_call);
        let pending_matches = self.query.stopped && self.query.cursor.is_stopped()
            && self.query.awaiting_actual_completion() == Some(SourceCrtQueryRequest::Replace(
                SourceCrtQueryReplaceRequest::RequiredCall(self.evidence.request)));
        let prefix_matches = self.original_source_and_prefix_match();
        let reason = if !self.adoption_attempted { Some(SourceCrtReplaceCounterAdoptionReason::ActualReceiptRequired) }
            else if self.resources.held.is_some() { Some(SourceCrtReplaceCounterAdoptionReason::HeldResourcesRemain) }
            else if observed != self.evidence || owned != Some(self.evidence) { Some(SourceCrtReplaceCounterAdoptionReason::ActualReceiptRequired) }
            else if !pending_matches { Some(SourceCrtReplaceCounterAdoptionReason::OriginalSourceRequired) }
            else if !prefix_matches { Some(SourceCrtReplaceCounterAdoptionReason::AdvancedPrefixRequired) } else { None };
        if let Some(reason) = reason {
            return Err(SourceCrtReplaceCounterAdoptionFailure::Rejected(SourceCrtReplaceCounterAdoptionRejected {
                pending: self, adoption, actual: Some(actual), observed: Some(observed), owned, reason }));
        }
        let Self { query, resources, bindings, evidence, .. } = self;
        let SourceCrtReplaceCounterResources { held, whole_call, actual_entry, actual_ret } = resources;

        let _ = held;
        let record = SourceCrtReplaceCounterHistory { whole_call, actual_entry, actual_ret,
            adoption_receipt: actual, evidence, bindings, adoption_bindings: adoption };
        let supplied = SourceCrtReplaceCounterHistory::source(&record);
        query.accept_actual_receipt(record, supplied, SourceCrtReplaceCounterHistory::source)
            .map_err(SourceCrtReplaceCounterAdoptionFailure::Source)
    }
}
