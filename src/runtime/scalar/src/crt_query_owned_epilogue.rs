use super::*;

#[path = "crt_query_owned_epilogue_continuation.rs"]
pub mod continuation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryEpilogueCompleted {
    pub source: SourceCrtQueryReceipt,

    pub before: SourceCrtNativeRetFrame,
    pub after: SourceCrtNativeRetFrame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryEpilogueExpectation {
    pub source_pc: u64,
    pub actual_before_rsp: u64,
    pub required_ret_frame: SourceCrtNativeRetFrame,
}
pub struct SourceCrtQueryEpilogueBindings<P, S, R> {
    pub actual_epilogue: fn(&R) -> SourceCrtQueryEpilogueCompleted,
    pub owned_epilogue: fn(&P, &S) -> Option<SourceCrtQueryEpilogueCompleted>,
}
#[must_use = "retain completed actual restore/native/frame/FP/stack resources before RET"]
pub struct SourceCrtQueryEpilogueRecord<P, S, R> {
    pub actual: R,
    pub bindings: SourceCrtQueryEpilogueBindings<P, S, R>,
    pub admitted: SourceCrtQueryEpilogueCompleted,
    pub expected: SourceCrtQueryEpilogueExpectation,
}
impl<P, S, R> SourceCrtQueryEpilogueRecord<P, S, R> {
    fn source(&self) -> SourceCrtQueryReceipt { (self.bindings.actual_epilogue)(&self.actual).source }
}
#[must_use = "retain stopped original query, actual restore owner and advanced prefix"]
pub enum SourceCrtQueryEpilogueFailure<P, S, H, R> {
    Prefix {
        owner: SourceCrtQueryOwned<P, S, H>, actual: R,
        bindings: SourceCrtQueryEpilogueBindings<P, S, R>,
        observed: SourceCrtQueryEpilogueCompleted, owned: Option<SourceCrtQueryEpilogueCompleted>,
        expected: Option<SourceCrtQueryEpilogueExpectation>,
        pending: Option<SourceCrtQueryRequest>, current: SourceCrtNativeRetFrame,
    },
    Source(SourceCrtNativeReturnFailure<SourceCrtQueryOwned<P, S, H>,
        SourceCrtQueryReceiptError<SourceCrtQueryEpilogueRecord<P, S, R>>>),
}
#[must_use = "retain actual rejected restore without repeating the epilogue or its reads"]
pub enum SourceCrtQueryEpilogueRejection<P, S, R> {
    Prefix {
        actual: R, bindings: SourceCrtQueryEpilogueBindings<P, S, R>,
        observed: SourceCrtQueryEpilogueCompleted, owned: Option<SourceCrtQueryEpilogueCompleted>,
        expected: Option<SourceCrtQueryEpilogueExpectation>,
        pending: Option<SourceCrtQueryRequest>, current: SourceCrtNativeRetFrame,
    },
    Source(SourceCrtQueryReceiptError<SourceCrtQueryEpilogueRecord<P, S, R>>),
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn accept_actual_query_epilogue<R>(mut self, actual: R,
        bindings: SourceCrtQueryEpilogueBindings<P, S, R>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H, SourceCrtQueryEpilogueRecord<P, S, R>>>,
            SourceCrtQueryEpilogueFailure<P, S, H, R>> {
        self.stopped = true; self.cursor.stop();
        let observed = (bindings.actual_epilogue)(&actual);
        let owned = (bindings.owned_epilogue)(&self.prefix, &self.services);
        let current = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let pending = self.cursor.pending();
        let expected = match (&self.cursor, pending, observed.source) {
            (SourceCrtQueryCursor::Context(_),
                Some(SourceCrtQueryRequest::Context(SourceCrtQueryContextRequest::RequiredEpilogue {
                    source_pc, actual_before_rsp, required_ret_frame })),
                SourceCrtQueryReceipt::Context(SourceCrtQueryContextReceipt::Epilogue { .. })) =>
                    Some(SourceCrtQueryEpilogueExpectation { source_pc, actual_before_rsp, required_ret_frame }),
            (SourceCrtQueryCursor::Refresh(_),
                Some(SourceCrtQueryRequest::Refresh(SourceCrtQueryRefreshRequest::RequiredEpilogue {
                    source_pc, actual_before_rsp, required_ret_frame, .. })),
                SourceCrtQueryReceipt::Refresh(SourceCrtQueryRefreshReceipt::Epilogue { .. })) =>
                    Some(SourceCrtQueryEpilogueExpectation { source_pc, actual_before_rsp, required_ret_frame }),
            (SourceCrtQueryCursor::Resolve(source),
                Some(SourceCrtQueryRequest::Resolve(SourceCrtQueryResolveRequest::RequiredEpilogue {
                    source_pc, required_ret_frame })),
                SourceCrtQueryReceipt::Resolve(SourceCrtQueryResolveReceipt::Epilogue { .. })) =>
                    Some(SourceCrtQueryEpilogueExpectation { source_pc,
                        actual_before_rsp: source.entry().actual_entry_rsp.wrapping_sub(0x28), required_ret_frame }),
            (SourceCrtQueryCursor::Replace(source),
                Some(SourceCrtQueryRequest::Replace(SourceCrtQueryReplaceRequest::RequiredEpilogue {
                    source_pc, required_ret_frame })),
                SourceCrtQueryReceipt::Replace(SourceCrtQueryReplaceReceipt::Epilogue { .. })) =>
                    Some(SourceCrtQueryEpilogueExpectation { source_pc,
                        actual_before_rsp: source.entry().actual_entry_rsp.wrapping_sub(0x28), required_ret_frame }),
            _ => None,
        };
        let matches = expected.is_some_and(|expected| observed.before.actual_rip == expected.source_pc
            && observed.before.actual_rsp == expected.actual_before_rsp
            && observed.after == expected.required_ret_frame
            && observed.source.supplied_frame() == Some(observed.after));
        if !matches || owned != Some(observed) || current != observed.after {
            return Err(SourceCrtQueryEpilogueFailure::Prefix { owner: self, actual, bindings, observed,
                owned, expected, pending, current });
        }
        let Some(expected) = expected else {
            return Err(SourceCrtQueryEpilogueFailure::Prefix { owner: self, actual, bindings, observed,
                owned, expected, pending, current });
        };
        let record = SourceCrtQueryEpilogueRecord { actual, bindings, admitted: observed, expected };
        self.accept_actual_receipt(record, observed.source, SourceCrtQueryEpilogueRecord::source)
            .map_err(SourceCrtQueryEpilogueFailure::Source)
    }
    pub fn retain_rejected_query_epilogue<R>(failure: SourceCrtQueryEpilogueFailure<P, S, H, R>)
        -> SourceCrtQueryOwned<P, S, (H, SourceCrtQueryEpilogueRejection<P, S, R>)> {
        match failure {
            SourceCrtQueryEpilogueFailure::Prefix { owner, actual, bindings, observed, owned,
                expected, pending, current } => owner.with_retained_history(SourceCrtQueryEpilogueRejection::Prefix {
                    actual, bindings, observed, owned, expected, pending, current }),
            SourceCrtQueryEpilogueFailure::Source(failure) => failure.owner.with_retained_history(
                SourceCrtQueryEpilogueRejection::Source(failure.cause)),
        }
    }
}
