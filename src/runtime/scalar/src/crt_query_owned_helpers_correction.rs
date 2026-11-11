use super::*;
use super::resume::{SourceCrtQueryHelperResumeFailure, SourceCrtQueryHelperReturnBindings};

pub enum SourceCrtQueryHelperRejection<P, S> {
    ParentAuthority { parent_frame: SourceCrtNativeRetFrame, child_frame: SourceCrtNativeRetFrame,
        actual_entry: SourceCrtQueryHelperCallEntry, owned_entry: Option<SourceCrtQueryHelperCallEntry>,
        bindings: SourceCrtQueryHelperReturnBindings<P, S> },
    Caller { source_call: SourceCrtQueryCallEnvelope, actual_frame: SourceCrtNativeRetFrame,
        observed_ret: SourceCrtNativeRetObservation },
    FreshContext { pending: SourceCrtQueryResolveProgress, actual_frame: SourceCrtNativeRetFrame,
        observed_call: SourceCrtQueryCallEnvelope, observed_ret: SourceCrtNativeRetObservation,
        actual_rbx_rdi: (u64, u64), source: Option<SourceCrtQueryResolveError> },
    Resolver { pending: SourceCrtQueryRefreshProgress, actual_frame: SourceCrtNativeRetFrame,
        observed_call: SourceCrtQueryCallEnvelope, observed_ret: SourceCrtNativeRetObservation,
        actual_rbx: u64, source: Option<SourceCrtQueryRefreshError> },
    SlotReplacement { pending: SourceCrtQueryResolveProgress, actual_frame: SourceCrtNativeRetFrame,
        observed_call: SourceCrtQueryCallEnvelope, observed_ret: SourceCrtNativeRetObservation,
        actual_rbx_rdi: (u64, u64), source: Option<SourceCrtQueryResolveError> },
}
#[must_use = "retain every prior rejected helper/native/advanced prefix through correction"]
pub struct SourceCrtQueryHelperRejectedHistory<CH, P, S> {
    pub previous: CH, pub rejection: SourceCrtQueryHelperRejection<P, S>,
    pub admitted_call: Option<SourceCrtQueryCallEnvelope>,
    pub caller_receipt: Option<SourceCrtQueryCallerReceipt>,
}
impl<P, S, CH, T, N> SourceCrtQueryReturned<P, S, CH, T, N> {
    fn retain_helper_rejection(self, rejection: SourceCrtQueryHelperRejection<P, S>,
        admitted_call: Option<SourceCrtQueryCallEnvelope>, caller_receipt: Option<SourceCrtQueryCallerReceipt>)
        -> SourceCrtQueryReturned<P, S, SourceCrtQueryHelperRejectedHistory<CH, P, S>, T, N> {
        let Self { owner, target_owner, expected, actual_retired, actual_return } = self;
        let SourceCrtQueryBeforeRet { cursor, prefix, services, history, source, current_frame } = owner;
        SourceCrtQueryReturned { owner: SourceCrtQueryBeforeRet { cursor, prefix, services,
            history: SourceCrtQueryHelperRejectedHistory { previous: history, rejection, admitted_call, caller_receipt },
            source, current_frame }, target_owner, expected, actual_retired, actual_return }
    }
}
impl<P, S, H, CH, E, N> SourceCrtQueryHelperResumeFailure<P, S, H, CH, E, N> {

    pub fn retain_for_actual_correction(self)
        -> Result<SourceCrtQueryReturned<P, S, SourceCrtQueryHelperRejectedHistory<CH, P, S>,
                SourceCrtQueryHelperParent<P, S, H, E>, N>, Self> {
        match self {
            Self::ParentAuthority { actual, parent_frame, child_frame, actual_entry, owned_entry, bindings } =>
                Ok(actual.retain_helper_rejection(SourceCrtQueryHelperRejection::ParentAuthority {
                    parent_frame, child_frame, actual_entry, owned_entry, bindings }, None, None)),
            Self::Caller(cause) => {
                let SourceCrtQueryCallerBindFailure { owner, source_call, actual_frame, observed_ret } = cause;
                Ok(owner.retain_helper_rejection(SourceCrtQueryHelperRejection::Caller {
                    source_call, actual_frame, observed_ret }, None, None))
            }
            Self::FreshContext { parent, mut cause } => {
                let token = &cause.actual.owner.target_owner;
                if token.parent.is_some() || !parent.is_stopped()
                    || parent.awaiting_actual_completion().map(SourceCrtQueryRequest::Resolve) != Some(token.expected) {
                    return Err(Self::FreshContext { parent, cause });
                }
                cause.actual.owner.target_owner.parent = Some(SourceCrtQueryCursor::Resolve(parent));
                let SourceCrtQueryFreshContextError { actual, pending, actual_frame, observed_call,
                    observed_ret, actual_rbx_rdi, source } = cause;
                let SourceCrtQueryCallerReturned { owner, source_call, actual_return } = actual;
                Ok(owner.retain_helper_rejection(SourceCrtQueryHelperRejection::FreshContext {
                    pending, actual_frame, observed_call, observed_ret, actual_rbx_rdi, source },
                    Some(source_call), Some(actual_return)))
            }
            Self::Resolver { parent, mut cause } => {
                let token = &cause.actual.owner.target_owner;
                if token.parent.is_some() || !parent.is_stopped()
                    || parent.awaiting_actual_completion().map(SourceCrtQueryRequest::Refresh) != Some(token.expected) {
                    return Err(Self::Resolver { parent, cause });
                }
                cause.actual.owner.target_owner.parent = Some(SourceCrtQueryCursor::Refresh(parent));
                let SourceCrtQueryResolverError { actual, pending, actual_frame, observed_call,
                    observed_ret, actual_rbx, source } = cause;
                let SourceCrtQueryCallerReturned { owner, source_call, actual_return } = actual;
                Ok(owner.retain_helper_rejection(SourceCrtQueryHelperRejection::Resolver {
                    pending, actual_frame, observed_call, observed_ret, actual_rbx, source },
                    Some(source_call), Some(actual_return)))
            }
            Self::SlotReplacement { parent, mut cause } => {
                let token = &cause.actual.owner.target_owner;
                if token.parent.is_some() || !parent.is_stopped()
                    || parent.awaiting_actual_completion().map(SourceCrtQueryRequest::Resolve) != Some(token.expected) {
                    return Err(Self::SlotReplacement { parent, cause });
                }
                cause.actual.owner.target_owner.parent = Some(SourceCrtQueryCursor::Resolve(parent));
                let SourceCrtQueryUpdateError { actual, pending, actual_frame, observed_call,
                    observed_ret, actual_rbx_rdi, source } = cause;
                let SourceCrtQueryCallerReturned { owner, source_call, actual_return } = actual;
                Ok(owner.retain_helper_rejection(SourceCrtQueryHelperRejection::SlotReplacement {
                    pending, actual_frame, observed_call, observed_ret, actual_rbx_rdi, source },
                    Some(source_call), Some(actual_return)))
            }
            failure @ (Self::MissingParent(_) | Self::WrongParent { .. }) => Err(failure),
        }
    }
}
