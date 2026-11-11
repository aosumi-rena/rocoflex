use super::*;

pub enum SourceCrtContextLifecycleReturnBindings<P, S> {
    Initialization(fn(&P, &S) -> SourceCrtQueryInitializationReturnedRegisters),
    NullRelease(fn(&P, &S) -> (u64, u64, u64)),
}
#[must_use = "retain prior allocation/context and completed initialization/NULL child resources"]
pub struct SourceCrtContextLifecycleHistory<P, S, H, CH, E, N> {
    pub parent: SourceCrtContextLifecycleParent<P, S, H, E>,
    pub child_cursor: SourceCrtQueryCursor, pub child_history: CH,
    pub child_source: SourceCrtQueryCompletion,
    pub child_frame_projection: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub expected: SourceCrtNativeRetExpectation, pub retired: SourceCrtNativeRetired,
    pub actual_return: N, pub source_call: SourceCrtQueryCallEnvelope,
    pub parent_receipt: SourceCrtQueryCallerReceipt,
}
#[must_use = "retain original stopped parent and entire advanced child/provider capsule"]
pub enum SourceCrtContextLifecycleResumeFailure<P, S, H, CH, E, N> {
    ParentAuthority {
        actual: SourceCrtQueryReturned<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N>,
        parent_frame: SourceCrtNativeRetFrame, child_frame: SourceCrtNativeRetFrame,
        actual_entry: SourceCrtContextLifecycleCallEntry, owned_entry: Option<SourceCrtContextLifecycleCallEntry>,
        bindings: SourceCrtContextLifecycleReturnBindings<P, S>,
    },
    Caller(SourceCrtQueryCallerBindFailure<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N>),
    MissingParent(SourceCrtQueryCallerReturned<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N>),
    Initialization { parent: SourceCrtQueryContext58db7c,
        cause: SourceCrtQueryInitializationJoinError<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N> },
    NullRelease { parent: SourceCrtQueryContext58db7c,
        cause: SourceCrtQueryNullReleaseJoinError<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N> },
}
impl<P, S, H, CH, E, N> SourceCrtQueryReturned<P, S, CH, SourceCrtContextLifecycleParent<P, S, H, E>, N> {

    pub fn resume_context_lifecycle_parent(self, project_actual_ret: fn(&N) -> SourceCrtNativeRetObservation,
        bindings: SourceCrtContextLifecycleReturnBindings<P, S>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtContextLifecycleHistory<P, S, H, CH, E, N>>,
            SourceCrtContextLifecycleResumeFailure<P, S, H, CH, E, N>> {
        let token = &self.target_owner;
        let parent_frame = (token.bindings.actual_frame)(&self.owner.prefix, &self.owner.services);
        let child_frame = (self.owner.current_frame)(&self.owner.prefix, &self.owner.services);
        let actual_entry = (token.entry_bindings.actual_entry)(&token.actual_entry);
        let owned_entry = (token.entry_bindings.owned_entry)(&self.owner.prefix, &self.owner.services);
        let original_pending = token.parent.as_ref().is_some_and(|source| {
            source.is_stopped() && source.awaiting_actual_completion()
                == Some(SourceCrtQueryContextRequest::RequiredCall(token.expected))
        });
        let right_kind = match (&bindings, token.expected.kind) {
            (SourceCrtContextLifecycleReturnBindings::Initialization(_), SourceCrtQueryContextCallKind::Initialize) => true,
            (SourceCrtContextLifecycleReturnBindings::NullRelease(_), kind) =>
                (kind == SourceCrtQueryContextCallKind::ReleaseNullAfterInitialize
                    || kind == SourceCrtQueryContextCallKind::ReleaseFailure) && token.expected.source_rcx == Some(0),
            _ => false,
        };
        if !original_pending || !right_kind || actual_entry != token.entry || owned_entry != Some(token.entry)
            || parent_frame != child_frame || parent_frame != self.actual_retired.after {
            return Err(SourceCrtContextLifecycleResumeFailure::ParentAuthority {
                actual: self, parent_frame, child_frame, actual_entry, owned_entry, bindings });
        }
        let mut actual = match self.admit_same_query_caller(SourceCrtContextLifecycleParent::source_call, project_actual_ret) {
            Ok(actual) => actual,
            Err(cause) => return Err(SourceCrtContextLifecycleResumeFailure::Caller(cause)),
        };
        let Some(mut parent) = actual.owner.target_owner.parent.take() else {
            return Err(SourceCrtContextLifecycleResumeFailure::MissingParent(actual));
        };
        let actual = match bindings {
            SourceCrtContextLifecycleReturnBindings::Initialization(project_registers) => {
                match parent.accept_owned_initialization_return(actual, SourceCrtContextLifecycleParent::source_call,
                    project_actual_ret, project_registers) {
                    Ok(accepted) => accepted.actual,
                    Err(cause) => return Err(SourceCrtContextLifecycleResumeFailure::Initialization { parent, cause }),
                }
            }
            SourceCrtContextLifecycleReturnBindings::NullRelease(project_registers) => {
                match parent.accept_owned_null_release_return(actual, SourceCrtContextLifecycleParent::source_call,
                    project_actual_ret, project_registers) {
                    Ok(accepted) => accepted.actual,
                    Err(cause) => return Err(SourceCrtContextLifecycleResumeFailure::NullRelease { parent, cause }),
                }
            }
        };
        let SourceCrtQueryCallerReturned { owner, source_call, actual_return: parent_receipt } = actual;
        let SourceCrtQueryReturned { owner, target_owner, expected, actual_retired, actual_return } = owner;
        let SourceCrtQueryBeforeRet { cursor: child_cursor, prefix, services, history: child_history,
            source: child_source, current_frame: child_frame_projection } = owner;
        let bindings = SourceCrtQueryOwnedBindings { actual_frame: target_owner.bindings.actual_frame,
            actual_receipt: target_owner.bindings.actual_receipt };
        let history = SourceCrtContextLifecycleHistory { parent: target_owner, child_cursor, child_history,
            child_source, child_frame_projection, expected, retired: actual_retired, actual_return, source_call, parent_receipt };
        Ok(SourceCrtQueryOwned::from_same_cursor_and_history(SourceCrtQueryCursor::Context(parent),
            prefix, services, history, bindings))
    }
}
