use super::*;
use super::super::crt_storage_native_return::validate_source_crt_native_ret;
use super::super::crt_query_owned::{SourceCrtQueryOwned, SourceCrtQueryOwnedBindings};
use super::transfer::SourceCrtHandlerQueryHistory;

#[must_use = "retain stopped original owner and EVERY earlier rejected/partial/service resource"]
pub struct SourceCrtHandlerAdmission<O, H> { owner: O, history: H }
impl<O, F> SourceCrtHandlerAdmission<O, F> {
    pub fn from_actual_failure(failure: SourceCrtNativeReturnFailure<O, F>) -> Self {
        Self { owner: failure.owner, history: failure.cause }
    }
}
impl<O, H> SourceCrtHandlerAdmission<O, H> {
    pub fn owner(&self) -> &O { &self.owner }
    pub fn history(&self) -> &H { &self.history }

    pub fn retain_actual_service_failure<F>(self, actual: F) -> SourceCrtHandlerAdmission<O, (H, F)> {
        SourceCrtHandlerAdmission { owner: self.owner, history: (self.history, actual) }
    }
    pub fn retain_rejected<F>(failure: SourceCrtNativeReturnFailure<Self, F>)
        -> SourceCrtHandlerAdmission<O, (H, F)> {
        let Self { owner, history } = failure.owner;
        SourceCrtHandlerAdmission { owner, history: (history, failure.cause) }
    }
}
impl<D, P, S, A, H> SourceCrtHandlerAdmission<SourceCrtCallocHandlerFrame<D, P, S, A>, H> {

    pub fn accept_actual_epilogue<R, E>(self, actual: R,
        project: fn(&R) -> SourceCrtHandlerActualEpilogue,
        apply: fn(&mut P, &mut S, &mut R) -> Result<(), E>)
        -> Result<SourceCrtHandlerAdmission<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>, H>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerEpilogueError<R, E>>> {
        let Self { owner, history } = self;
        match owner.accept_actual_epilogue(actual, project, apply) {
            Ok(owner) => Ok(SourceCrtHandlerAdmission { owner, history }),
            Err(failure) => Err(SourceCrtNativeReturnFailure {
                owner: Self { owner: failure.owner, history }, cause: failure.cause }),
        }
    }
}
impl<D, P, S, A, R, H> SourceCrtHandlerAdmission<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>, H> {

    pub fn accept_actual_ret<N, E>(self, actual: N, project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N) -> Result<(), E>)
        -> Result<SourceCrtHandlerAdmission<SourceCrtHandlerFrameReturned<D, P, S, A, R, N>, H>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerFrameReturnError<N, E>>> {
        let Self { owner, history } = self;
        match owner.accept_actual_ret(actual, project, apply) {
            Ok(owner) => Ok(SourceCrtHandlerAdmission { owner, history }),
            Err(failure) => Err(SourceCrtNativeReturnFailure {
                owner: Self { owner: failure.owner, history }, cause: failure.cause }),
        }
    }
}
impl<D, P, S, A, R, N, H> SourceCrtHandlerAdmission<SourceCrtHandlerFrameReturned<D, P, S, A, R, N>, H> {
    pub fn into_owned_query(self, bindings: SourceCrtQueryOwnedBindings<P, S>)
        -> Result<SourceCrtQueryOwned<P, S, (SourceCrtHandlerQueryHistory<D, P, S, A, R, N>, H)>, Self> {
        let Self { owner, history } = self;
        match owner.into_owned_query(bindings) {
            Ok(owner) => Ok(owner.with_retained_history(history)),
            Err(owner) => Err(Self { owner, history }),
        }
    }
    pub fn into_source_and_retained(self)
        -> Result<(SourceCrtQueryCalloc58f074, (SourceCrtHandlerFrameRetained<D, P, S, A, R, N>, H)), Self> {
        let Self { owner, history } = self;
        match owner.into_source_and_retained() {
            Ok((source, retained)) => Ok((source, (retained, history))),
            Err(owner) => Err(Self { owner, history }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtHandlerPrefixCorrectionReason {
    ActualAdapterFailed, ActualRecordChanged, RetiredProofRequired,
    OriginalPendingRequired, ActualPrefixRequired, SourceContinuationRequired,
}
#[must_use = "retain actual correction/exception and stopped original owner"]
pub struct SourceCrtHandlerPrefixCorrectionError<C, E> {
    pub actual: C, pub before: SourceCrtHandlerActualPrefix, pub after: SourceCrtHandlerActualPrefix,
    pub current: SourceCrtHandlerActualPrefix, pub reason: SourceCrtHandlerPrefixCorrectionReason,
    pub cause: Option<E>, pub source: Option<SourceCrtQueryCallocError>,
}
#[must_use = "retain original rejected RET/native resources and all actual prefix corrections"]
pub struct SourceCrtHandlerCorrectedReturn<F, H, C> {
    pub rejected_return: F, pub previous_corrections: H, pub actual_correction: C,
}
#[must_use = "retain SAME failed RET capsule; this does not qualify partial RET as retired"]
pub struct SourceCrtHandlerReturnCorrection<D, P, S, A, R, N, E, H = ()> {
    owner: SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>,
    rejected: SourceCrtHandlerFrameReturnError<N, E>, history: H,
    project_ret: fn(&N) -> SourceCrtNativeRetObservation,
}
impl<D, P, S, A, R, N, E> SourceCrtHandlerReturnCorrection<D, P, S, A, R, N, E> {
    pub fn from_actual_failure(failure: SourceCrtNativeReturnFailure<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>,
        SourceCrtHandlerFrameReturnError<N, E>>, project_ret: fn(&N) -> SourceCrtNativeRetObservation) -> Self {
        Self { owner: failure.owner, rejected: failure.cause, history: (), project_ret }
    }
}
impl<D, P, S, A, R, N, E, H> SourceCrtHandlerReturnCorrection<D, P, S, A, R, N, E, H> {
    pub fn from_admission_failure(failure: SourceCrtNativeReturnFailure<
        SourceCrtHandlerAdmission<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>, H>,
        SourceCrtHandlerFrameReturnError<N, E>>, project_ret: fn(&N) -> SourceCrtNativeRetObservation) -> Self {
        let SourceCrtHandlerAdmission { owner, history } = failure.owner;
        Self { owner, rejected: failure.cause, history, project_ret }
    }
    pub fn owner(&self) -> &SourceCrtHandlerEpilogueAccepted<D, P, S, A, R> { &self.owner }
    pub fn rejected(&self) -> &SourceCrtHandlerFrameReturnError<N, E> { &self.rejected }
    pub fn history(&self) -> &H { &self.history }

    pub fn accept_actual_retired_completion<N2, E2>(self, actual: N2,
        project: fn(&N2) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N2) -> Result<(), E2>)
        -> Result<SourceCrtHandlerAdmission<SourceCrtHandlerFrameReturned<D, P, S, A, R, N2>,
                (H, SourceCrtHandlerFrameReturnError<N, E>)>,
            SourceCrtNativeReturnFailure<SourceCrtHandlerAdmission<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>,
                (H, SourceCrtHandlerFrameReturnError<N, E>)>, SourceCrtHandlerFrameReturnError<N2, E2>>> {
        let Self { owner, rejected, history, .. } = self;
        SourceCrtHandlerAdmission { owner, history: (history, rejected) }.accept_actual_ret(actual, project, apply)
    }
    pub fn retain_rejected<C, X>(failure: SourceCrtNativeReturnFailure<Self, SourceCrtHandlerPrefixCorrectionError<C, X>>)
        -> SourceCrtHandlerReturnCorrection<D, P, S, A, R, N, E, (H, SourceCrtHandlerPrefixCorrectionError<C, X>)> {
        let Self { owner, rejected, history, project_ret } = failure.owner;
        SourceCrtHandlerReturnCorrection { owner, rejected, project_ret, history: (history, failure.cause) }
    }

    fn retired_proof(&self) -> Option<SourceCrtNativeRetired> {
        let (actual, retired) = match &self.rejected {
            SourceCrtHandlerFrameReturnError::Native(SourceCrtNativeRetError::PrefixMismatch { actual, observation, .. }) => (actual, *observation),
            SourceCrtHandlerFrameReturnError::Caller { actual, retired, .. } => (actual, *retired),
            _ => return None,
        };
        if (self.project_ret)(actual) != SourceCrtNativeRetObservation::Retired(retired) { return None; }
        let owner = &self.owner.owner;
        let expected = SourceCrtNativeRetExpectation { before: owner.ret_prefix().frame,
            actual_required_target: owner.request.required_actual_resume };
        if validate_source_crt_native_ret(expected, retired).is_err() { return None; }
        Some(retired)
    }
    pub fn accept_actual_prefix<C, X>(mut self, mut actual: C,
        project: fn(&C) -> SourceCrtHandlerActualPrefix,
        apply: fn(&mut P, &mut S, &mut C) -> Result<(), X>)
        -> Result<SourceCrtHandlerFrameReturned<D, P, S, A, R,
                SourceCrtHandlerCorrectedReturn<SourceCrtHandlerFrameReturnError<N, E>, H, C>>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerPrefixCorrectionError<C, X>>> {
        let before = project(&actual);
        let applied = apply(&mut self.owner.owner.prefix, &mut self.owner.owner.services, &mut actual);
        let after = project(&actual);
        let owner = &mut self.owner.owner;
        let current = (owner.bindings.current_prefix)(&owner.prefix, &owner.services);
        if let Err(cause) = applied {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerPrefixCorrectionError {
                actual, before, after, current, reason: SourceCrtHandlerPrefixCorrectionReason::ActualAdapterFailed,
                cause: Some(cause), source: None } });
        }
        let pending_matches = owner.phase == SourceCrtHandlerFramePhase::RetContinuationRequired
            && owner.calloc.awaiting_actual_completion() == Some(SourceCrtQueryCallocRequest::RequiredCall(owner.request));
        let same_binding = owner.same_binding();
        let retired = self.retired_proof();
        let reason = if before != after { Some(SourceCrtHandlerPrefixCorrectionReason::ActualRecordChanged) }
            else if !pending_matches { Some(SourceCrtHandlerPrefixCorrectionReason::OriginalPendingRequired) }
            else if retired.is_none() { Some(SourceCrtHandlerPrefixCorrectionReason::RetiredProofRequired) }
            else if !same_binding || current != after || current.rbx != self.owner.owner.request.required_bytes_rbx
                || current.rdi != self.owner.owner.entry.entry.rdi || retired.is_some_and(|proof| current.frame != proof.after) {
                Some(SourceCrtHandlerPrefixCorrectionReason::ActualPrefixRequired)
            } else { None };
        if let Some(reason) = reason {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerPrefixCorrectionError {
                actual, before, after, current, reason, cause: None, source: None } });
        }
        let Some(retired) = retired else {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerPrefixCorrectionError {
                actual, before, after, current, reason: SourceCrtHandlerPrefixCorrectionReason::RetiredProofRequired,
                cause: None, source: None } });
        };
        let owner = &mut self.owner.owner;
        let source = match owner.calloc.accept_actual_completion(SourceCrtQueryCallocReceipt::CallReturned {
            request: owner.request, actual_frame: retired.after, actual_rbx: current.rbx }) {
            Ok(source) => source,
            Err(source) => return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerPrefixCorrectionError {
                actual, before, after, current, reason: SourceCrtHandlerPrefixCorrectionReason::SourceContinuationRequired,
                cause: None, source: Some(source) } }),
        };
        owner.phase = SourceCrtHandlerFramePhase::Complete;
        let Self { owner, rejected, history, .. } = self;
        Ok(SourceCrtHandlerFrameReturned { owner: owner.owner, actual_epilogue: owner.actual_epilogue,
            actual_return: SourceCrtHandlerCorrectedReturn { rejected_return: rejected, previous_corrections: history,
                actual_correction: actual }, retired, source })
    }
}
