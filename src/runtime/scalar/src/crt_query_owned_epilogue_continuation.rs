use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryEpilogueCorrectionHistory {
    Prefix {
        observed: SourceCrtQueryEpilogueCompleted,
        owned: Option<SourceCrtQueryEpilogueCompleted>,
        expected: Option<SourceCrtQueryEpilogueExpectation>,
        pending: Option<SourceCrtQueryRequest>, current: SourceCrtNativeRetFrame,
    },
    Source {
        admitted: SourceCrtQueryEpilogueCompleted,
        expected: SourceCrtQueryEpilogueExpectation,
        supplied: SourceCrtQueryReceipt, observed: SourceCrtQueryReceipt,
        owned_observed: Option<SourceCrtQueryReceipt>, actual_frame: SourceCrtNativeRetFrame,
        reason: SourceCrtQueryReceiptReason, source: Option<SourceCrtQuerySourceError>,
    },
}
impl<P, S, H, R> SourceCrtQueryEpilogueFailure<P, S, H, R> {

    pub fn retain_for_actual_correction(self)
        -> (SourceCrtQueryOwned<P, S, (H, SourceCrtQueryEpilogueCorrectionHistory)>, R,
            SourceCrtQueryEpilogueBindings<P, S, R>) {
        match self {
            Self::Prefix { owner, actual, bindings, observed, owned, expected, pending, current } =>
                (owner.with_retained_history(SourceCrtQueryEpilogueCorrectionHistory::Prefix {
                    observed, owned, expected, pending, current }), actual, bindings),
            Self::Source(failure) => {
                let SourceCrtNativeReturnFailure { owner, cause } = failure;
                let SourceCrtQueryReceiptError { actual: record, supplied, observed, owned_observed,
                    actual_frame, reason, source } = cause;
                let SourceCrtQueryEpilogueRecord { actual, bindings, admitted, expected } = record;
                (owner.with_retained_history(SourceCrtQueryEpilogueCorrectionHistory::Source {
                    admitted, expected, supplied, observed, owned_observed, actual_frame, reason, source }),
                    actual, bindings)
            }
        }
    }
}
