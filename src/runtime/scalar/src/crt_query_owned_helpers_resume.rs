use super::*;

pub enum SourceCrtQueryHelperReturnBindings<P, S> {
    FreshContext(fn(&P, &S) -> (u64, u64)),
    Resolver(fn(&P, &S) -> u64),
    SlotReplacement(fn(&P, &S) -> (u64, u64)),
}
#[must_use = "retain original parent and complete nested child/native/frame/FP/resource history"]
pub struct SourceCrtQueryHelperHistory<P, S, H, CH, E, N> {
    pub parent: SourceCrtQueryHelperParent<P, S, H, E>,
    pub child_cursor: SourceCrtQueryCursor, pub child_history: CH,
    pub child_source: SourceCrtQueryCompletion,
    pub child_frame_projection: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub expected: SourceCrtNativeRetExpectation, pub retired: SourceCrtNativeRetired,
    pub actual_return: N, pub source_call: SourceCrtQueryCallEnvelope,
    pub parent_receipt: SourceCrtQueryCallerReceipt,
}
#[must_use = "retain stopped original parent plus all rejected actual child resources"]
pub enum SourceCrtQueryHelperResumeFailure<P, S, H, CH, E, N> {
    ParentAuthority {
        actual: SourceCrtQueryReturned<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N>,
        parent_frame: SourceCrtNativeRetFrame, child_frame: SourceCrtNativeRetFrame,
        actual_entry: SourceCrtQueryHelperCallEntry, owned_entry: Option<SourceCrtQueryHelperCallEntry>,
        bindings: SourceCrtQueryHelperReturnBindings<P, S>,
    },
    Caller(SourceCrtQueryCallerBindFailure<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N>),
    MissingParent(SourceCrtQueryCallerReturned<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N>),
    FreshContext { parent: SourceCrtQueryResolve5941fc,
        cause: SourceCrtQueryFreshContextError<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N> },
    Resolver { parent: SourceCrtQueryRefresh58e238,
        cause: SourceCrtQueryResolverError<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N> },
    SlotReplacement { parent: SourceCrtQueryResolve5941fc,
        cause: SourceCrtQueryUpdateError<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N> },
    WrongParent { parent: SourceCrtQueryCursor,
        actual: SourceCrtQueryCallerReturned<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N> },
}
impl<P, S, H, CH, E, N> SourceCrtQueryReturned<P, S, CH, SourceCrtQueryHelperParent<P, S, H, E>, N> {

    pub fn resume_query_helper_parent(self, project_actual_ret: fn(&N) -> SourceCrtNativeRetObservation,
        bindings: SourceCrtQueryHelperReturnBindings<P, S>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHelperHistory<P, S, H, CH, E, N>>,
            SourceCrtQueryHelperResumeFailure<P, S, H, CH, E, N>> {
        let token = &self.target_owner;
        let parent_frame = (token.bindings.actual_frame)(&self.owner.prefix, &self.owner.services);
        let child_frame = (self.owner.current_frame)(&self.owner.prefix, &self.owner.services);
        let actual_entry = (token.entry_bindings.actual_entry)(&token.actual_entry);
        let owned_entry = (token.entry_bindings.owned_entry)(&self.owner.prefix, &self.owner.services);
        let original_pending = token.parent.as_ref().is_some_and(|source|
            source.is_stopped() && source.pending() == Some(token.expected));
        let right_kind = matches!((&bindings, token.edge),
            (SourceCrtQueryHelperReturnBindings::FreshContext(_), SourceCrtQueryHelperEdge::FreshContext594206)
            | (SourceCrtQueryHelperReturnBindings::Resolver(_), SourceCrtQueryHelperEdge::Resolver58e25b)
            | (SourceCrtQueryHelperReturnBindings::SlotReplacement(_), SourceCrtQueryHelperEdge::SlotReplacement59423f));
        if !original_pending || !right_kind || actual_entry != token.entry || owned_entry != Some(token.entry)
            || parent_frame != child_frame || parent_frame != self.actual_retired.after {
            return Err(SourceCrtQueryHelperResumeFailure::ParentAuthority {
                actual: self, parent_frame, child_frame, actual_entry, owned_entry, bindings });
        }
        let mut actual = match self.admit_same_query_caller(SourceCrtQueryHelperParent::source_call, project_actual_ret) {
            Ok(actual) => actual,
            Err(cause) => return Err(SourceCrtQueryHelperResumeFailure::Caller(cause)),
        };
        let Some(parent) = actual.owner.target_owner.parent.take() else {
            return Err(SourceCrtQueryHelperResumeFailure::MissingParent(actual));
        };
        let (parent, actual) = match (parent, bindings) {
            (SourceCrtQueryCursor::Resolve(mut parent), SourceCrtQueryHelperReturnBindings::FreshContext(project)) => {
                match parent.accept_owned_fresh_context(actual, SourceCrtQueryHelperParent::source_call, project_actual_ret, project) {
                    Ok(accepted) => (SourceCrtQueryCursor::Resolve(parent), accepted.actual),
                    Err(cause) => return Err(SourceCrtQueryHelperResumeFailure::FreshContext { parent, cause }),
                }
            }
            (SourceCrtQueryCursor::Refresh(mut parent), SourceCrtQueryHelperReturnBindings::Resolver(project)) => {
                match parent.accept_owned_resolver_return(actual, SourceCrtQueryHelperParent::source_call, project_actual_ret, project) {
                    Ok(accepted) => (SourceCrtQueryCursor::Refresh(parent), accepted.actual),
                    Err(cause) => return Err(SourceCrtQueryHelperResumeFailure::Resolver { parent, cause }),
                }
            }
            (SourceCrtQueryCursor::Resolve(mut parent), SourceCrtQueryHelperReturnBindings::SlotReplacement(project)) => {
                match parent.accept_owned_update_return(actual, SourceCrtQueryHelperParent::source_call, project_actual_ret, project) {
                    Ok(accepted) => (SourceCrtQueryCursor::Resolve(parent), accepted.actual),
                    Err(cause) => return Err(SourceCrtQueryHelperResumeFailure::SlotReplacement { parent, cause }),
                }
            }
            (parent, _) => return Err(SourceCrtQueryHelperResumeFailure::WrongParent { parent, actual }),
        };
        let SourceCrtQueryCallerReturned { owner, source_call, actual_return: parent_receipt } = actual;
        let SourceCrtQueryReturned { owner, target_owner, expected, actual_retired, actual_return } = owner;
        let SourceCrtQueryBeforeRet { cursor: child_cursor, prefix, services, history: child_history,
            source: child_source, current_frame: child_frame_projection } = owner;
        let bindings = SourceCrtQueryOwnedBindings { actual_frame: target_owner.bindings.actual_frame,
            actual_receipt: target_owner.bindings.actual_receipt };
        let history = SourceCrtQueryHelperHistory { parent: target_owner, child_cursor, child_history,
            child_source, child_frame_projection, expected, retired: actual_retired, actual_return, source_call, parent_receipt };
        Ok(SourceCrtQueryOwned::from_same_cursor_and_history(parent, prefix, services, history, bindings))
    }
}
