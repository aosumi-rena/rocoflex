use super::*;
use super::super::super::crt_query_initialization::{SourceCrtQueryInitializationBody,
    SourceCrtQueryInitializationLastCall};
use super::super::super::crt_query_initialization_body::{SourceCrtQueryInitializationBodyBindings,
    SourceCrtQueryInitializationBodyCapsule};

#[must_use = "retain private body/provider/storage/two child caps and exact admission facts"]
pub struct SourceCrtContextLifecycleBodyReceipt<D, P, S> {
    pub body: D, pub bindings: SourceCrtQueryInitializationBodyBindings<D, P, S>,
    pub pending: Option<SourceCrtQueryInitializationProgress>,
    pub observed_body: SourceCrtQueryInitializationBody, pub required_next_pc: u32,
    pub last_actual_call: Option<SourceCrtQueryInitializationLastCall>,
    pub actual_frame: SourceCrtNativeRetFrame, pub owned_frame: SourceCrtNativeRetFrame,
    pub actual_rbp: u64, pub source: Option<SourceCrtQueryInitializationError>,
    pub accepted: Option<SourceCrtQueryInitializationProgress>,
}
impl<P, S, H, CH, E> SourceCrtContextLifecycleChild<P, S, H, CH, E> {

    pub fn accept_owned_initialization_body<D>(mut self, body: D,
        bindings: SourceCrtQueryInitializationBodyBindings<D, P, S>)
        -> Result<SourceCrtContextLifecycleChild<P, S, H,
                (CH, SourceCrtContextLifecycleBodyReceipt<D, P, S>), E>,
            SourceCrtNativeReturnFailure<Self, SourceCrtContextLifecycleBodyReceipt<D, P, S>>> {
        self.child.stopped = true; self.child.cursor.stop();
        let pending = match &self.child.cursor {
            SourceCrtQueryCursor::Initialization(source) => Some(source.progress()), _ => None,
        };
        let observed_body = (bindings.actual_body)(&body);
        let required_next_pc = (bindings.required_next_pc)(&body);
        let last_actual_call = (bindings.last_actual_call_return)(&body);
        let actual_frame = (bindings.actual_frame)(&body, &self.child.prefix, &self.child.services);
        let actual_rbp = (bindings.actual_rbp)(&body, &self.child.prefix, &self.child.services);
        let owned_frame = (self.child.bindings.actual_frame)(&self.child.prefix, &self.child.services);
        if pending.is_none() || actual_frame != owned_frame {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtContextLifecycleBodyReceipt { body, bindings, pending, observed_body,
                    required_next_pc, last_actual_call, actual_frame, owned_frame, actual_rbp,
                    source: None, accepted: None } });
        }
        let Self { child, parent } = self;
        let SourceCrtQueryOwned { mut cursor, prefix, services, history, bindings: owner_bindings, .. } = child;
        let actual = SourceCrtQueryInitializationBodyCapsule { body, prefix, services, bindings };
        let source = match &mut cursor {
            SourceCrtQueryCursor::Initialization(source) => source,
            _ => {
                let SourceCrtQueryInitializationBodyCapsule { body, prefix, services, bindings } = actual;
                return Err(SourceCrtNativeReturnFailure {
                    owner: Self { child: SourceCrtQueryOwned { cursor, prefix, services, history,
                        bindings: owner_bindings, stopped: true }, parent },
                    cause: SourceCrtContextLifecycleBodyReceipt { body, bindings, pending, observed_body,
                        required_next_pc, last_actual_call, actual_frame, owned_frame, actual_rbp,
                        source: None, accepted: None } });
            }
        };
        match source.accept_owned_body(actual) {
            Ok(accepted) => {
                let SourceCrtQueryInitializationBodyCapsule { body, prefix, services, bindings } = accepted.actual;
                let receipt = SourceCrtContextLifecycleBodyReceipt { body, bindings, pending, observed_body,
                    required_next_pc, last_actual_call, actual_frame, owned_frame, actual_rbp,
                    source: None, accepted: Some(accepted.source) };
                Ok(SourceCrtContextLifecycleChild {
                    child: SourceCrtQueryOwned::from_same_cursor_and_history(cursor, prefix, services,
                        (history, receipt), owner_bindings), parent })
            }
            Err(rejected) => {
                let SourceCrtQueryInitializationBodyCapsule { body, prefix, services, bindings } = rejected.actual;
                Err(SourceCrtNativeReturnFailure {
                    owner: Self { child: SourceCrtQueryOwned { cursor, prefix, services, history,
                        bindings: owner_bindings, stopped: true }, parent },
                    cause: SourceCrtContextLifecycleBodyReceipt { body, bindings, pending: Some(rejected.pending),
                        observed_body: rejected.observed_body, required_next_pc: rejected.required_next_pc,
                        last_actual_call: rejected.last_actual_call, actual_frame: rejected.actual_frame,
                        owned_frame, actual_rbp: rejected.actual_rbp, source: rejected.source, accepted: None } })
            }
        }
    }

    pub fn retain_rejected_initialization_body<D>(failure: SourceCrtNativeReturnFailure<Self,
        SourceCrtContextLifecycleBodyReceipt<D, P, S>>)
        -> SourceCrtContextLifecycleChild<P, S, H, (CH, SourceCrtContextLifecycleBodyReceipt<D, P, S>), E> {
        let Self { child, parent } = failure.owner;
        SourceCrtContextLifecycleChild { child: child.with_retained_history(failure.cause), parent }
    }
}
