use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryFreshReadCompleted {
    pub source: SourceCrtQueryReceipt,
    pub before: SourceCrtNativeRetFrame,
    pub after: SourceCrtNativeRetFrame,
}
pub struct SourceCrtQueryFreshReadBindings<P, S, R> {
    pub actual_read: fn(&R) -> SourceCrtQueryFreshReadCompleted,
    pub owned_read: fn(&P, &S) -> Option<SourceCrtQueryFreshReadCompleted>,
}
#[must_use = "retain actual fresh-read resource/native prefix and its independent receipt"]
pub struct SourceCrtQueryFreshReadRecord<P, S, R> {
    pub actual: R, pub bindings: SourceCrtQueryFreshReadBindings<P, S, R>,
    pub admitted: SourceCrtQueryFreshReadCompleted,
}
impl<P, S, R> SourceCrtQueryFreshReadRecord<P, S, R> {
    fn source(&self) -> SourceCrtQueryReceipt { (self.bindings.actual_read)(&self.actual).source }
}
#[must_use = "retain original stopped parent/history and entire actual advanced read failure"]
pub enum SourceCrtQueryFreshReadFailure<P, S, H, R> {
    Prefix {
        owner: SourceCrtQueryOwned<P, S, H>, actual: R,
        bindings: SourceCrtQueryFreshReadBindings<P, S, R>,
        observed: SourceCrtQueryFreshReadCompleted, owned: Option<SourceCrtQueryFreshReadCompleted>,
        current: SourceCrtNativeRetFrame,
    },
    Source(SourceCrtNativeReturnFailure<SourceCrtQueryOwned<P, S, H>,
        SourceCrtQueryReceiptError<SourceCrtQueryFreshReadRecord<P, S, R>>>),
}
#[must_use = "retain rejected read receipt/resources without repeating the read"]
pub enum SourceCrtQueryFreshReadRejection<P, S, R> {
    Prefix { actual: R, bindings: SourceCrtQueryFreshReadBindings<P, S, R>,
        observed: SourceCrtQueryFreshReadCompleted, owned: Option<SourceCrtQueryFreshReadCompleted>,
        current: SourceCrtNativeRetFrame },
    Source(SourceCrtQueryReceiptError<SourceCrtQueryFreshReadRecord<P, S, R>>),
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn retain_rejected_fresh_read<R>(failure: SourceCrtQueryFreshReadFailure<P, S, H, R>)
        -> SourceCrtQueryOwned<P, S, (H, SourceCrtQueryFreshReadRejection<P, S, R>)> {
        match failure {
            SourceCrtQueryFreshReadFailure::Prefix { owner, actual, bindings, observed, owned, current } =>
                owner.with_retained_history(SourceCrtQueryFreshReadRejection::Prefix {
                    actual, bindings, observed, owned, current }),
            SourceCrtQueryFreshReadFailure::Source(failure) =>
                failure.owner.with_retained_history(SourceCrtQueryFreshReadRejection::Source(failure.cause)),
        }
    }

    pub fn accept_actual_fresh_read<R>(mut self, actual: R, bindings: SourceCrtQueryFreshReadBindings<P, S, R>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H, SourceCrtQueryFreshReadRecord<P, S, R>>>,
            SourceCrtQueryFreshReadFailure<P, S, H, R>> {
        self.stopped = true; self.cursor.stop();
        let observed = (bindings.actual_read)(&actual);
        let owned = (bindings.owned_read)(&self.prefix, &self.services);
        let current = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let location = match (&self.cursor, self.cursor.pending(), observed.source) {
            (SourceCrtQueryCursor::Context(source),
                Some(SourceCrtQueryRequest::Context(SourceCrtQueryContextRequest::ReadIndex32 { role, source_pc, actual_address })),
                SourceCrtQueryReceipt::Context(SourceCrtQueryContextReceipt::IndexRead {
                    role: actual_role, source_pc: actual_pc, actual_address: address, .. }))
                if role == actual_role && actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Resolve(source),
                Some(SourceCrtQueryRequest::Resolve(SourceCrtQueryResolveRequest::ReadSlot64 { source_pc, actual_address }
                    | SourceCrtQueryResolveRequest::ReadGlobal64 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Resolve(SourceCrtQueryResolveReceipt::Read64 {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Resolve(source),
                Some(SourceCrtQueryRequest::Resolve(SourceCrtQueryResolveRequest::ReadFlags32 { source_pc, actual_address }
                    | SourceCrtQueryResolveRequest::ReadMask32 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Resolve(SourceCrtQueryResolveReceipt::Read32 {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Refresh(source),
                Some(SourceCrtQueryRequest::Refresh(SourceCrtQueryRefreshRequest::ReadSlot64 { source_pc, actual_address }
                    | SourceCrtQueryRefreshRequest::ReadGlobal64 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Refresh(SourceCrtQueryRefreshReceipt::Read64 {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Refresh(source),
                Some(SourceCrtQueryRequest::Refresh(SourceCrtQueryRefreshRequest::ReadFlags32 { source_pc, actual_address }
                    | SourceCrtQueryRefreshRequest::ReadMask32 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Refresh(SourceCrtQueryRefreshReceipt::Read32 {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Replace(source),
                Some(SourceCrtQueryRequest::Replace(SourceCrtQueryReplaceRequest::ReadOldSlot64 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Replace(SourceCrtQueryReplaceReceipt::OldSlotRead {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Replace(source),
                Some(SourceCrtQueryRequest::Replace(SourceCrtQueryReplaceRequest::ReadOldCount32 { source_pc, actual_address })),
                SourceCrtQueryReceipt::Replace(SourceCrtQueryReplaceReceipt::OldCountRead {
                    source_pc: actual_pc, actual_address: address, .. }))
                if actual_pc == source_pc && address == actual_address =>
                    Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            _ => None,
        };
        let matches = location.is_some_and(|(pc, rsp)| observed.before.actual_rip == pc
            && observed.before.actual_rsp == rsp && observed.after.actual_rsp == rsp);
        if !matches || owned != Some(observed) || current != observed.after {
            return Err(SourceCrtQueryFreshReadFailure::Prefix { owner: self, actual, bindings, observed, owned, current });
        }

        let record = SourceCrtQueryFreshReadRecord { actual, bindings, admitted: observed };
        self.accept_actual_receipt(record, observed.source, SourceCrtQueryFreshReadRecord::source)
            .map_err(SourceCrtQueryFreshReadFailure::Source)
    }
}
