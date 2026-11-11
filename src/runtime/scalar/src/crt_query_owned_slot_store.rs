use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQuerySlotStorePrefix {
    pub frame: SourceCrtNativeRetFrame,
    pub actual_rcx: u64, pub actual_rbx: u64, pub actual_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQuerySlotStoreCompleted {
    pub source: SourceCrtQueryReceipt,
    pub before: SourceCrtQuerySlotStorePrefix, pub after: SourceCrtQuerySlotStorePrefix,
}

pub struct SourceCrtQuerySlotStoreBindings<P, S, R> {
    pub actual_store: fn(&R) -> SourceCrtQuerySlotStoreCompleted,
    pub owned_store: fn(&P, &S) -> Option<SourceCrtQuerySlotStoreCompleted>,
}
#[must_use = "retain actual store/alias/resource receipt and original advanced prefix"]
pub struct SourceCrtQuerySlotStoreRecord<P, S, R> {
    pub actual: R, pub bindings: SourceCrtQuerySlotStoreBindings<P, S, R>,
    pub admitted: SourceCrtQuerySlotStoreCompleted,
}
impl<P, S, R> SourceCrtQuerySlotStoreRecord<P, S, R> {
    fn source(&self) -> SourceCrtQueryReceipt { (self.bindings.actual_store)(&self.actual).source }
}
#[must_use = "retain stopped original query/history and actual partial or rejected store"]
pub enum SourceCrtQuerySlotStoreFailure<P, S, H, R> {
    Prefix { owner: SourceCrtQueryOwned<P, S, H>, actual: R,
        bindings: SourceCrtQuerySlotStoreBindings<P, S, R>,
        observed: SourceCrtQuerySlotStoreCompleted, owned: Option<SourceCrtQuerySlotStoreCompleted>,
        current: SourceCrtNativeRetFrame },
    Source(SourceCrtNativeReturnFailure<SourceCrtQueryOwned<P, S, H>,
        SourceCrtQueryReceiptError<SourceCrtQuerySlotStoreRecord<P, S, R>>>),
}
#[must_use = "retain actual rejected publication without a second write"]
pub enum SourceCrtQuerySlotStoreRejection<P, S, R> {
    Prefix { actual: R, bindings: SourceCrtQuerySlotStoreBindings<P, S, R>,
        observed: SourceCrtQuerySlotStoreCompleted, owned: Option<SourceCrtQuerySlotStoreCompleted>,
        current: SourceCrtNativeRetFrame },
    Source(SourceCrtQueryReceiptError<SourceCrtQuerySlotStoreRecord<P, S, R>>),
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn accept_actual_slot_store<R>(mut self, actual: R, bindings: SourceCrtQuerySlotStoreBindings<P, S, R>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H, SourceCrtQuerySlotStoreRecord<P, S, R>>>,
            SourceCrtQuerySlotStoreFailure<P, S, H, R>> {
        self.stopped = true; self.cursor.stop();
        let observed = (bindings.actual_store)(&actual);
        let owned = (bindings.owned_store)(&self.prefix, &self.services);
        let current = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let location = match (&self.cursor, self.cursor.pending(), observed.source) {
            (SourceCrtQueryCursor::Refresh(source),
                Some(SourceCrtQueryRequest::Refresh(SourceCrtQueryRefreshRequest::StoreSlot64 {
                    source_pc, actual_address, value })),
                SourceCrtQueryReceipt::Refresh(SourceCrtQueryRefreshReceipt::SlotStored {
                    source_pc: actual_pc, actual_address: address, value: actual_value }))
                if actual_pc == source_pc && address == actual_address && actual_value == value
                    && observed.before.actual_rbx == source.entry().actual_slot_rdx
                    && observed.before.actual_rbx == actual_address && observed.before.frame.actual_rax == value =>
                        Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            (SourceCrtQueryCursor::Replace(source),
                Some(SourceCrtQueryRequest::Replace(SourceCrtQueryReplaceRequest::StoreNewSlot64 {
                    source_pc, actual_address, value })),
                SourceCrtQueryReceipt::Replace(SourceCrtQueryReplaceReceipt::NewSlotStored {
                    source_pc: actual_pc, actual_address: address, value: actual_value }))
                if actual_pc == source_pc && address == actual_address && actual_value == value
                    && observed.before.actual_rcx == actual_address && observed.before.actual_rdi == value
                    && source.saved_old_word() == Some(observed.before.actual_rbx) =>
                        Some((source_pc, source.entry().actual_entry_rsp.wrapping_sub(0x28))),
            _ => None,
        };
        let matches = location.is_some_and(|(pc, rsp)| observed.before.frame.actual_rip == pc
            && observed.before.frame.actual_rsp == rsp && observed.after.frame.actual_rsp == rsp)
            && observed.before.frame.actual_rax == observed.after.frame.actual_rax
            && observed.before.actual_rcx == observed.after.actual_rcx
            && observed.before.actual_rbx == observed.after.actual_rbx
            && observed.before.actual_rdi == observed.after.actual_rdi;
        if !matches || owned != Some(observed) || current != observed.after.frame {
            return Err(SourceCrtQuerySlotStoreFailure::Prefix { owner: self, actual, bindings, observed, owned, current });
        }
        let record = SourceCrtQuerySlotStoreRecord { actual, bindings, admitted: observed };

        self.accept_actual_receipt(record, observed.source, SourceCrtQuerySlotStoreRecord::source)
            .map_err(SourceCrtQuerySlotStoreFailure::Source)
    }
    pub fn retain_rejected_slot_store<R>(failure: SourceCrtQuerySlotStoreFailure<P, S, H, R>)
        -> SourceCrtQueryOwned<P, S, (H, SourceCrtQuerySlotStoreRejection<P, S, R>)> {
        match failure {
            SourceCrtQuerySlotStoreFailure::Prefix { owner, actual, bindings, observed, owned, current } =>
                owner.with_retained_history(SourceCrtQuerySlotStoreRejection::Prefix { actual, bindings, observed, owned, current }),
            SourceCrtQuerySlotStoreFailure::Source(failure) =>
                failure.owner.with_retained_history(SourceCrtQuerySlotStoreRejection::Source(failure.cause)),
        }
    }
}
