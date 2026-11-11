use super::*;

impl<P, S, H> SourceCrtQueryOwned<P, S, H> {
    pub fn hold_actual_replace_retirement_progress<D, W, E, N>(mut self,
        resources: SourceCrtReplaceRetirementResources<D, W, E, N>,
        bindings: SourceCrtReplaceRetirementBindings<D, P, S, W, E>)
        -> Result<SourceCrtReplaceRetirementRequired<D, P, S, H, W, E, N>,
            SourceCrtReplaceRetirementRejected<D, P, S, H, W, E, N>> {
        self.stopped = true; self.cursor.stop();
        let request = match &self.cursor {
            SourceCrtQueryCursor::Replace(source) => match source.awaiting_actual_completion() {
                Some(SourceCrtQueryReplaceRequest::RequiredCall(call)) if call.kind == SourceCrtQueryReplaceCallKind::Whole593ffc => Some(call), _ => None },
            _ => None,
        };
        let base = match &self.cursor { SourceCrtQueryCursor::Replace(source) => Some(source.entry().actual_module_base), _ => None };
        let description = (bindings.describe)(&resources.child);
        let entry = (bindings.actual_entry)(&resources.actual_entry);
        let owned_entry = (bindings.owned_entry)(&self.prefix, &self.services, &resources.whole_call);
        let active_call = (bindings.active_call)(&resources.whole_call);
        let actual_prefix = (bindings.actual_prefix)(&resources.child, &self.prefix, &self.services, &resources.whole_call);
        let current_prefix = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let actual_saved_registers = (bindings.actual_saved_registers)(&resources.child, &self.prefix, &self.services, &resources.whole_call);
        let valid = match (request, base, description) {
            (Some(call), Some(base), Some(description)) => {
                let child_rsp = call.actual_caller_rsp.wrapping_sub(8);
                let body_rsp = child_rsp.wrapping_sub(0x28);
                let saved = [call.required_old_rbx, entry.actual_rbp,
                    entry.call.actual_rbx_rsi_rdi.1, call.required_incoming_rdi];
                let envelope = SourceCrtQueryCallEnvelope { actual_module_base: base, source_call_pc: 0x5942bd,
                    source_resume_pc: 0x5942c2, target_rva: 0x593ffc, actual_work_rsp: call.actual_caller_rsp,
                    source_rcx: Some(call.source_rcx), source_rdx: None };
                let checked = description.checked_entry.is_some_and(|checked| checked.pc == 0x593ffc
                    && checked.actual_module_base == base && checked.actual_rsp == child_rsp
                    && checked.receiver_rcx == call.source_rcx && checked.saved == saved
                    && checked.stores_returned == [true; 3]
                    && checked.push == Some(SourceCrtReplaceRetirementPush {
                        before_rsp: child_rsp, after_rsp: child_rsp.wrapping_sub(8), actual_word: saved[3] })
                    && checked.subtract_before_after == Some((child_rsp.wrapping_sub(8), body_rsp)));
                let position = match description.position {
                    SourceCrtReplaceRetirementPosition::CheckedEntry593ffc | SourceCrtReplaceRetirementPosition::BodyEntry594010 =>
                        description.tail.is_none() && actual_prefix == (SourceCrtNativeRetFrame {
                            actual_rip: base.wrapping_add(0x594010), actual_rsp: body_rsp,
                            actual_rax: entry.call.entered.actual_rax }) && actual_saved_registers == saved,
                    SourceCrtReplaceRetirementPosition::BodyCompleted594156
                    | SourceCrtReplaceRetirementPosition::RestoreEntry594156 => description.tail.is_none()
                        && actual_prefix.actual_rip == base.wrapping_add(0x594156) && actual_prefix.actual_rsp == body_rsp,
                    SourceCrtReplaceRetirementPosition::TailRequired59416d
                    | SourceCrtReplaceRetirementPosition::TailServiceEntry59416d => description.tail.is_some_and(|tail|
                        tail.pc == 0x59416d && tail.actual_target == base.wrapping_add(0x58d910)
                            && tail.actual_rsp == child_rsp && tail.receiver_rcx == call.source_rcx
                            && actual_prefix.actual_rip == base.wrapping_add(0x59416d)
                            && actual_prefix.actual_rsp == child_rsp && actual_saved_registers == tail.restored),
                };
                call.source_rcx != 0 && call.source_rdx.is_none()
                    && call.source_pc == base.wrapping_add(0x5942bd) && call.actual_target == base.wrapping_add(0x593ffc)
                    && call.required_actual_resume == base.wrapping_add(0x5942c2)
                    && entry.call.call == envelope && active_call == Some(envelope)
                    && entry.call.before.actual_rip == call.source_pc && entry.call.before.actual_rsp == call.actual_caller_rsp
                    && entry.call.entered == (SourceCrtNativeRetFrame { actual_rip: call.actual_target,
                        actual_rsp: child_rsp, actual_rax: entry.call.before.actual_rax })
                    && entry.call.stored_return == (SourceCrtQueryStackWord { actual_address: child_rsp, actual_word: call.required_actual_resume })
                    && entry.call.actual_rcx == call.source_rcx
                    && (entry.call.actual_rbx_rsi_rdi.0, entry.call.actual_rbx_rsi_rdi.2)
                        == (call.required_old_rbx, call.required_incoming_rdi)
                    && description.body == (SourceCrtReplaceRetirementBody { pc: 0x594010,
                        actual_module_base: base, actual_rsp: body_rsp, receiver_rcx: call.source_rcx,
                        entry_rbp: saved[1], entry_rsi: saved[2], entry_rdi: saved[3] })
                    && checked && position && owned_entry == Some(entry) && actual_prefix == current_prefix
            }
            _ => false,
        };
        if !valid {
            let reason = if request.is_none() || base.is_none() || active_call.is_none() { SourceCrtReplaceRetirementReason::ActualWholeCallRequired }
                else if description.is_none() || description.is_some_and(|value| value.checked_entry.is_none()) { SourceCrtReplaceRetirementReason::CheckedEntryRequired }
                else { SourceCrtReplaceRetirementReason::ActualPrefixRequired };
            return Err(SourceCrtReplaceRetirementRejected { query: self, resources, bindings, request, description,
                entry, owned_entry, active_call, actual_prefix, current_prefix, actual_saved_registers, reason });
        }
        let (Some(request), Some(description)) = (request, description) else {
            return Err(SourceCrtReplaceRetirementRejected { query: self, resources, bindings, request, description,
                entry, owned_entry, active_call, actual_prefix, current_prefix, actual_saved_registers,
                reason: SourceCrtReplaceRetirementReason::CheckedEntryRequired });
        };
        Ok(SourceCrtReplaceRetirementRequired { query: self, resources, bindings,
            evidence: SourceCrtReplaceRetirementEvidence { request, entry, description, actual_prefix, actual_saved_registers } })
    }
}
