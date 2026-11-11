use super::*;

#[must_use = "retain prior parent and completed child/heap/mode/handler/accessor/native resources"]
pub struct SourceCrtContextAllocationHistory<P, S, H, CH, E, N> {
    pub parent: SourceCrtContextAllocationParent<P, S, H, E>,
    pub child_cursor: SourceCrtQueryCursor, pub child_history: CH,
    pub child_source: SourceCrtQueryCompletion,
    pub child_frame_projection: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub expected: SourceCrtNativeRetExpectation, pub retired: SourceCrtNativeRetired,
    pub actual_return: N, pub source_call: SourceCrtQueryCallEnvelope,
    pub parent_receipt: SourceCrtQueryCallerReceipt,
}
#[must_use = "retain ORIGINAL parent and whole advanced child capsule on rejection"]
pub enum SourceCrtContextAllocationResumeFailure<P, S, H, CH, E, N> {
    ParentAuthority {
        actual: SourceCrtQueryReturned<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>, N>,
        parent_frame: SourceCrtNativeRetFrame, child_frame: SourceCrtNativeRetFrame,
        actual_entry: SourceCrtContextAllocationCallEntry, owned_entry: Option<SourceCrtContextAllocationCallEntry>,
    },
    Caller(SourceCrtQueryCallerBindFailure<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>, N>),
    MissingParent(SourceCrtQueryCallerReturned<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>, N>),
    Source {
        parent: SourceCrtQueryContext58db7c,
        cause: SourceCrtQueryCallocJoinError<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>, N>,
    },
}
impl<P, S, H, CH, E, N> SourceCrtQueryReturned<P, S, CH, SourceCrtContextAllocationParent<P, S, H, E>, N> {

    pub fn resume_context_allocation_parent(self,
        project_actual_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_registers: fn(&P, &S) -> (u64, u64, u64))
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtContextAllocationHistory<P, S, H, CH, E, N>>,
            SourceCrtContextAllocationResumeFailure<P, S, H, CH, E, N>> {
        let token = &self.target_owner;
        let parent_frame = (token.bindings.actual_frame)(&self.owner.prefix, &self.owner.services);
        let child_frame = (self.owner.current_frame)(&self.owner.prefix, &self.owner.services);
        let actual_entry = (token.entry_bindings.actual_entry)(&token.actual_entry);
        let owned_entry = (token.entry_bindings.owned_entry)(&self.owner.prefix, &self.owner.services);
        let original_pending = token.parent.as_ref().is_some_and(|source| {
            source.is_stopped() && source.awaiting_actual_completion()
                == Some(SourceCrtQueryContextRequest::RequiredCall(token.expected))
        });
        if !original_pending || actual_entry != token.entry || owned_entry != Some(token.entry)
            || parent_frame != child_frame || parent_frame != self.actual_retired.after {
            return Err(SourceCrtContextAllocationResumeFailure::ParentAuthority {
                actual: self, parent_frame, child_frame, actual_entry, owned_entry });
        }
        let mut actual = match self.admit_same_query_caller(SourceCrtContextAllocationParent::source_call, project_actual_ret) {
            Ok(actual) => actual,
            Err(cause) => return Err(SourceCrtContextAllocationResumeFailure::Caller(cause)),
        };
        let Some(mut parent) = actual.owner.target_owner.parent.take() else {
            return Err(SourceCrtContextAllocationResumeFailure::MissingParent(actual));
        };
        let accepted = match parent.accept_owned_calloc_return(actual,
            SourceCrtContextAllocationParent::source_call, project_actual_ret, project_registers) {
            Ok(accepted) => accepted,
            Err(cause) => return Err(SourceCrtContextAllocationResumeFailure::Source { parent, cause }),
        };
        let SourceCrtQueryCallerReturned { owner, source_call, actual_return: parent_receipt } = accepted.actual;
        let SourceCrtQueryReturned { owner, target_owner, expected, actual_retired, actual_return } = owner;
        let SourceCrtQueryBeforeRet { cursor: child_cursor, prefix, services, history: child_history,
            source: child_source, current_frame: child_frame_projection } = owner;

        let bindings = SourceCrtQueryOwnedBindings { actual_frame: target_owner.bindings.actual_frame,
            actual_receipt: target_owner.bindings.actual_receipt };
        let history = SourceCrtContextAllocationHistory { parent: target_owner, child_cursor, child_history,
            child_source, child_frame_projection, expected, retired: actual_retired, actual_return, source_call, parent_receipt };
        Ok(SourceCrtQueryOwned::from_same_cursor_and_history(SourceCrtQueryCursor::Context(parent),
            prefix, services, history, bindings))
    }
}
