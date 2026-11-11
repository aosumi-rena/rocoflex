use super::*;

#[must_use = "retain checked entry/source/provider binding history with existing body owner"]
pub struct SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E> {
    pub from: SourceCrtReplaceRetirementPosition,
    pub to: SourceCrtReplaceRetirementPosition,
    pub evidence: SourceCrtReplaceRetirementEvidence,
    pub bindings: SourceCrtReplaceRetirementBindings<D, P, S, W, E>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementTransferObservation {
    pub from: SourceCrtReplaceRetirementPosition,
    pub to: SourceCrtReplaceRetirementPosition,
    pub description: Option<SourceCrtReplaceRetirementDescription>,
    pub entry: SourceCrtReplaceRetirementCallEntry, pub owned_entry: Option<SourceCrtReplaceRetirementCallEntry>,
    pub active_call: Option<SourceCrtQueryCallEnvelope>, pub pending: Option<SourceCrtQueryRequest>,
    pub child_prefix: SourceCrtNativeRetFrame, pub current_prefix: SourceCrtNativeRetFrame,
    pub actual_saved_registers: [u64; 4],
}
#[must_use = "retain original or advanced retirement progress and every actual owner on failed transfer"]
pub enum SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N> {
    Required {
        original: SourceCrtReplaceRetirementRequired<D, P, S, H, W, E, N>,
        move_existing_body: fn(D) -> B,
        body_bindings: SourceCrtReplaceRetirementBindings<B, P, S, W, E>,
        observed: SourceCrtReplaceRetirementTransferObservation,
    },
    Rejected(SourceCrtReplaceRetirementRejected<B, P, S,
        (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>),
    Advanced(SourceCrtReplaceRetirementRequired<B, P, S,
        (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtReplaceRetirementMoveMismatch {
    pub before: SourceCrtReplaceRetirementEvidence, pub after: SourceCrtReplaceRetirementEvidence,
}
impl<D, B, P, S, H, W, E, N> SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N> {

    pub fn retain_before_move_failure(self)
        -> Result<(SourceCrtReplaceRetirementRequired<D, P, S,
                (H, SourceCrtReplaceRetirementTransferObservation), W, E, N>,
            fn(D) -> B, SourceCrtReplaceRetirementBindings<B, P, S, W, E>), Self> {
        match self {
            Self::Required { original, move_existing_body, body_bindings, observed } => {
                let SourceCrtReplaceRetirementRequired { query, resources, bindings, evidence } = original;
                Ok((SourceCrtReplaceRetirementRequired { query: query.with_retained_history(observed),
                    resources, bindings, evidence }, move_existing_body, body_bindings))
            }
            other => Err(other),
        }
    }

    pub fn retain_advanced_move_failure(self)
        -> Result<SourceCrtReplaceRetirementRequired<B, P, S,
            ((H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>),
                SourceCrtReplaceRetirementMoveMismatch), W, E, N>, Self> {
        match self {
            Self::Advanced(actual) => {
                let before = actual.query.history.1.evidence;
                let after = actual.evidence;
                let SourceCrtReplaceRetirementRequired { query, resources, bindings, evidence } = actual;
                Ok(SourceCrtReplaceRetirementRequired { query: query.with_retained_history(
                    SourceCrtReplaceRetirementMoveMismatch { before, after }), resources, bindings, evidence })
            }
            other => Err(other),
        }
    }
}
impl<D, P, S, H, W, E, N> SourceCrtReplaceRetirementRequired<D, P, S, H, W, E, N> {

    pub fn move_checked_entry_into_existing_body<B>(self, move_existing_body: fn(D) -> B,
        body_bindings: SourceCrtReplaceRetirementBindings<B, P, S, W, E>)
        -> Result<SourceCrtReplaceRetirementRequired<B, P, S,
                (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>,
            SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N>> {
        self.move_existing_progress(move_existing_body, body_bindings,
            SourceCrtReplaceRetirementPosition::CheckedEntry593ffc,
            SourceCrtReplaceRetirementPosition::BodyEntry594010)
    }

    pub fn move_completed_body_into_existing_restore<B>(self, move_existing_restore: fn(D) -> B,
        restore_bindings: SourceCrtReplaceRetirementBindings<B, P, S, W, E>)
        -> Result<SourceCrtReplaceRetirementRequired<B, P, S,
                (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>,
            SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N>> {
        self.move_existing_progress(move_existing_restore, restore_bindings,
            SourceCrtReplaceRetirementPosition::BodyCompleted594156,
            SourceCrtReplaceRetirementPosition::RestoreEntry594156)
    }

    pub fn move_restored_prefix_into_existing_tail_service<B>(self, move_existing_tail: fn(D) -> B,
        tail_bindings: SourceCrtReplaceRetirementBindings<B, P, S, W, E>)
        -> Result<SourceCrtReplaceRetirementRequired<B, P, S,
                (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>,
            SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N>> {
        self.move_existing_progress(move_existing_tail, tail_bindings,
            SourceCrtReplaceRetirementPosition::TailRequired59416d,
            SourceCrtReplaceRetirementPosition::TailServiceEntry59416d)
    }
    fn move_existing_progress<B>(self, move_existing_body: fn(D) -> B,
        body_bindings: SourceCrtReplaceRetirementBindings<B, P, S, W, E>,
        from: SourceCrtReplaceRetirementPosition, to: SourceCrtReplaceRetirementPosition)
        -> Result<SourceCrtReplaceRetirementRequired<B, P, S,
                (H, SourceCrtReplaceRetirementTransferHistory<D, P, S, W, E>), W, E, N>,
            SourceCrtReplaceRetirementTransferFailure<D, B, P, S, H, W, E, N>> {
        let current_description = (self.bindings.describe)(&self.resources.child);
        let current_entry = (self.bindings.actual_entry)(&self.resources.actual_entry);
        let current_prefix = (self.query.bindings.actual_frame)(&self.query.prefix, &self.query.services);
        let owned_entry = (self.bindings.owned_entry)(&self.query.prefix, &self.query.services, &self.resources.whole_call);
        let active_call = (self.bindings.active_call)(&self.resources.whole_call);
        let pending = self.query.awaiting_actual_completion();
        let child_prefix = (self.bindings.actual_prefix)(&self.resources.child, &self.query.prefix,
            &self.query.services, &self.resources.whole_call);
        let actual_saved_registers = (self.bindings.actual_saved_registers)(&self.resources.child, &self.query.prefix,
            &self.query.services, &self.resources.whole_call);
        let expected_pending = Some(SourceCrtQueryRequest::Replace(
            SourceCrtQueryReplaceRequest::RequiredCall(self.evidence.request)));
        let valid = self.evidence.description.position == from
            && current_description == Some(self.evidence.description) && current_entry == self.evidence.entry
            && self.query.stopped && self.query.cursor.is_stopped()
            && pending == expected_pending && owned_entry == Some(self.evidence.entry)
            && active_call == Some(self.evidence.entry.call.call) && child_prefix == self.evidence.actual_prefix
            && current_prefix == self.evidence.actual_prefix
            && actual_saved_registers == self.evidence.actual_saved_registers;
        if !valid {
            let observed = SourceCrtReplaceRetirementTransferObservation { from, to, description: current_description,
                entry: current_entry, owned_entry, active_call, pending, child_prefix, current_prefix, actual_saved_registers };
            return Err(SourceCrtReplaceRetirementTransferFailure::Required {
                original: self, move_existing_body, body_bindings, observed });
        }
        let Self { query, resources, bindings, evidence } = self;
        let SourceCrtReplaceRetirementResources { child, whole_call, actual_entry, native_resources } = resources;

        let body = move_existing_body(child);
        let query = query.with_retained_history(SourceCrtReplaceRetirementTransferHistory { from, to, evidence, bindings });
        let actual = match query.hold_actual_replace_retirement_progress(SourceCrtReplaceRetirementResources {
            child: body, whole_call, actual_entry, native_resources }, body_bindings) {
            Ok(actual) => actual,
            Err(cause) => return Err(SourceCrtReplaceRetirementTransferFailure::Rejected(cause)),
        };
        if actual.evidence.description.position != to
            || actual.evidence.description.checked_entry != evidence.description.checked_entry
            || actual.evidence.description.body != evidence.description.body
            || actual.evidence.description.tail != evidence.description.tail
            || actual.evidence.entry != evidence.entry || actual.evidence.request != evidence.request
            || actual.evidence.actual_prefix != evidence.actual_prefix
            || actual.evidence.actual_saved_registers != evidence.actual_saved_registers {
            return Err(SourceCrtReplaceRetirementTransferFailure::Advanced(actual));
        }

        Ok(actual)
    }
}
