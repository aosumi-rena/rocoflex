use super::*;
use super::adoption::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceCounterAdmissionHistory {
    pub request: Option<SourceCrtQueryReplaceCall>, pub description: Option<SourceCrtReplaceCounterDescription>,
    pub entry: SourceCrtQueryHelperCallEntry, pub owned_entry: Option<SourceCrtQueryHelperCallEntry>,
    pub whole: SourceCrtReplaceCounterWholeReturn,
    pub observed_ret: SourceCrtNativeRetObservation, pub owned_ret: Option<SourceCrtNativeRetObservation>,
    pub frame: SourceCrtNativeRetFrame, pub registers: (u64, u64),
    pub reason: SourceCrtReplaceCounterAdmissionReason,
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceCounterRejected<D, P, S, H, W, E, N> {

    pub fn retain_for_actual_correction(self)
        -> (SourceCrtQueryOwned<P, S, (H, SourceCrtReplaceCounterAdmissionHistory)>,
            SourceCrtReplaceCounterResources<D, W, E, N>, SourceCrtReplaceCounterBindings<D, P, S, W, E, N>) {
        let Self { query, resources, bindings, request, description, entry, owned_entry, whole,
            observed_ret, owned_ret, frame, registers, reason } = self;
        let history = SourceCrtReplaceCounterAdmissionHistory { request, description, entry, owned_entry, whole,
            observed_ret, owned_ret, frame, registers, reason };
        (query.with_retained_history(history), resources, bindings)
    }
}
#[must_use = "retain prior ownership-transfer error or actual rejected receipt"]
pub enum SourceCrtReplaceCounterAdoptionRejection<A, F> {
    Service(F),
    Rejected { actual: Option<A>, observed: Option<SourceCrtReplaceCounterEvidence>,
        owned: Option<SourceCrtReplaceCounterEvidence>, reason: SourceCrtReplaceCounterAdoptionReason },
}
#[must_use = "retain actual stopped adoption/source continuation and all earlier owner errors"]
pub enum SourceCrtReplaceCounterContinuation<D, P, S, H, W, E, N, A, F> {
    Adoption(SourceCrtReplaceCounterAdoptionContinuation<D, P, S, H, W, E, N, A, F>),
    Source(SourceCrtQueryOwned<P, S, SourceCrtQueryRejectedHistory<H,
        SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>),
}

#[must_use = "retain SAME attempted adoption capability through actual correction"]
pub struct SourceCrtReplaceCounterAdoptionContinuation<D, P, S, H, W, E, N, A, F> {
    pending: SourceCrtReplaceCounterBound<D, P, S, (H, SourceCrtReplaceCounterAdoptionRejection<A, F>), W, E, N>,
    adoption: SourceCrtReplaceCounterAdoption<D, P, S, W, A, F>,
}
impl<D, P, S, H, W, E, N, A, F> SourceCrtReplaceCounterAdoptionContinuation<D, P, S, H, W, E, N, A, F> {
    pub fn evidence(&self) -> SourceCrtReplaceCounterEvidence { self.pending.evidence() }
    pub fn adoption_was_attempted(&self) -> bool { self.pending.adoption_was_attempted() }
    pub fn held(&self) -> Option<&D> { self.pending.held() }
    pub fn history(&self) -> &(H, SourceCrtReplaceCounterAdoptionRejection<A, F>) { self.pending.history() }
    pub fn actual_prefix_services_and_call_mut(&mut self) -> (&mut P, &mut S, &mut W) {
        self.pending.actual_prefix_services_and_call_mut()
    }

    pub fn actual_prefix_services_call_and_failure_mut(&mut self) -> (&mut P, &mut S, &mut W, Option<&mut F>) {
        let failure = match &mut self.pending.query.history.1 {
            SourceCrtReplaceCounterAdoptionRejection::Service(error) => Some(error),
            SourceCrtReplaceCounterAdoptionRejection::Rejected { .. } => None,
        };
        (&mut self.pending.query.prefix, &mut self.pending.query.services,
            &mut self.pending.resources.whole_call, failure)
    }
}
impl<D, P, S, H, W, E, N, A, F> SourceCrtReplaceCounterAdoptionFailure<D, P, S, H, W, E, N, A, F> {

    pub fn retain_for_actual_continuation(self) -> SourceCrtReplaceCounterContinuation<D, P, S, H, W, E, N, A, F> {
        match self {
            Self::Service(SourceCrtReplaceCounterAdoptionServiceFailure { pending, adoption, error }) => {
                let SourceCrtReplaceCounterBound { query, resources, bindings, evidence, adoption_attempted } = pending;
                SourceCrtReplaceCounterContinuation::Adoption(SourceCrtReplaceCounterAdoptionContinuation {
                    pending: SourceCrtReplaceCounterBound { query: query.with_retained_history(
                        SourceCrtReplaceCounterAdoptionRejection::Service(error)), resources, bindings, evidence, adoption_attempted }, adoption })
            }
            Self::Rejected(SourceCrtReplaceCounterAdoptionRejected { pending, adoption, actual, observed, owned, reason }) => {
                let SourceCrtReplaceCounterBound { query, resources, bindings, evidence, adoption_attempted } = pending;
                SourceCrtReplaceCounterContinuation::Adoption(SourceCrtReplaceCounterAdoptionContinuation {
                    pending: SourceCrtReplaceCounterBound { query: query.with_retained_history(
                        SourceCrtReplaceCounterAdoptionRejection::Rejected { actual, observed, owned, reason }),
                        resources, bindings, evidence, adoption_attempted }, adoption })
            }
            Self::Source(failure) => SourceCrtReplaceCounterContinuation::Source(SourceCrtQueryOwned::retain_rejected_receipt(failure)),
        }
    }
}
#[must_use = "retain missing/same-receipt correction failure without another adoption attempt"]
pub enum SourceCrtReplaceCounterSameReceiptFailure<D, P, S, H, W, E, N, A, F> {
    Required(SourceCrtReplaceCounterContinuation<D, P, S, H, W, E, N, A, F>),
    ActualReceiptRequired { pending: SourceCrtReplaceCounterContinuation<D, P, S, H, W, E, N, A, F>, actual: A },
    Admission(SourceCrtReplaceCounterAdoptionFailure<D, P, S,
        (H, SourceCrtReplaceCounterAdoptionRejection<A, F>), W, E, N, A, F>),
}
impl<D, P, S, H, W, E, N, A, F> SourceCrtReplaceCounterContinuation<D, P, S, H, W, E, N, A, F> {

    pub fn adopt_after_actual_prefix(self)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<
                (H, SourceCrtReplaceCounterAdoptionRejection<A, F>), SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>,
            SourceCrtReplaceCounterSameReceiptFailure<D, P, S, H, W, E, N, A, F>> {
        match self {
            Self::Adoption(continuation) if !continuation.pending.adoption_was_attempted() => {
                let SourceCrtReplaceCounterAdoptionContinuation { pending, adoption } = continuation;
                pending.adopt_completed_counter(adoption).map_err(SourceCrtReplaceCounterSameReceiptFailure::Admission)
            }
            other => Err(SourceCrtReplaceCounterSameReceiptFailure::Required(other)),
        }
    }

    pub fn accept_actual_completed_adoption(self, actual: A)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<
                (H, SourceCrtReplaceCounterAdoptionRejection<A, F>), SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>,
            SourceCrtReplaceCounterSameReceiptFailure<D, P, S, H, W, E, N, A, F>> {
        match self {
            Self::Adoption(continuation) => {
                let SourceCrtReplaceCounterAdoptionContinuation { pending, adoption } = continuation;
                pending.accept_actual_completed_adoption(actual, adoption)
                    .map_err(SourceCrtReplaceCounterSameReceiptFailure::Admission)
            }
            other => Err(SourceCrtReplaceCounterSameReceiptFailure::ActualReceiptRequired { pending: other, actual }),
        }
    }

    pub fn accept_same_receipt_after_actual_prefix(self)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<
                (H, SourceCrtReplaceCounterAdoptionRejection<A, F>), SourceCrtReplaceCounterHistory<D, P, S, W, E, N, A, F>>>,
            SourceCrtReplaceCounterSameReceiptFailure<D, P, S, H, W, E, N, A, F>> {
        let (mut pending, adoption) = match self {
            Self::Adoption(SourceCrtReplaceCounterAdoptionContinuation { pending, adoption }) => (pending, adoption),
            other => return Err(SourceCrtReplaceCounterSameReceiptFailure::Required(other)),
        };
        let actual = match &mut pending.query.history.1 {
            SourceCrtReplaceCounterAdoptionRejection::Rejected { actual, .. } => actual.take(),
            SourceCrtReplaceCounterAdoptionRejection::Service(_) => None,
        };
        let Some(actual) = actual else {
            return Err(SourceCrtReplaceCounterSameReceiptFailure::Required(Self::Adoption(
                SourceCrtReplaceCounterAdoptionContinuation { pending, adoption })));
        };
        pending.accept_actual_completed_adoption(actual, adoption)
            .map_err(SourceCrtReplaceCounterSameReceiptFailure::Admission)
    }
}
