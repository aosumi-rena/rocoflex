use super::*;

impl<P, S, H> SourceCrtQueryOwned<P, S, H> {
    pub fn bind_actual_replace_counter_return<D, W, E, N>(mut self,
        resources: SourceCrtReplaceCounterResources<D, W, E, N>,
        bindings: SourceCrtReplaceCounterBindings<D, P, S, W, E, N>)
        -> Result<SourceCrtReplaceCounterBound<D, P, S, H, W, E, N>,
            SourceCrtReplaceCounterRejected<D, P, S, H, W, E, N>> {
        self.stopped = true; self.cursor.stop();
        let description = resources.held.as_ref().and_then(bindings.describe);
        let entry = (bindings.actual_entry)(&resources.actual_entry);
        let owned_entry = (bindings.owned_entry)(&self.prefix, &self.services, &resources.whole_call);
        let whole = (bindings.whole_return)(&resources.whole_call);
        let observed_ret = (bindings.actual_ret)(&resources.actual_ret);
        let owned_ret = (bindings.owned_ret)(&self.prefix, &self.services, &resources.whole_call);
        let frame = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let registers = (bindings.current_registers)(&self.prefix, &self.services);
        let request = match &self.cursor {
            SourceCrtQueryCursor::Replace(source) => match source.awaiting_actual_completion() {
                Some(SourceCrtQueryReplaceRequest::RequiredCall(call))
                    if call.kind == SourceCrtQueryReplaceCallKind::Whole593ec8
                        || call.kind == SourceCrtQueryReplaceCallKind::Whole593f54 => Some(call), _ => None },
            _ => None,
        };
        let actual_module_base = match &self.cursor {
            SourceCrtQueryCursor::Replace(source) => Some(source.entry().actual_module_base), _ => None,
        };
        let retired = match observed_ret { SourceCrtNativeRetObservation::Retired(value) => Some(value), _ => None };
        let matches = match (request, description, retired, actual_module_base) {
            (Some(call), Some(description), Some(retired), Some(base)) => {
                let child_rsp = call.actual_caller_rsp.wrapping_sub(8);
                let (call_pc, resume_pc, target_rva) = if call.kind == SourceCrtQueryReplaceCallKind::Whole593ec8 {
                    (0x594296, 0x59429b, 0x593ec8) } else { (0x5942a3, 0x5942a8, 0x593f54) };
                let path = match (call.kind, description.body.profile, description.path) {
                    (SourceCrtQueryReplaceCallKind::Whole593ec8, SourceCrtReplaceCounterProfile::Increment593ec8,
                        SourceCrtReplaceCounterReturnPath::IncrementTail { tail_pc, actual_target, tail_rsp, leaf_pc, leaf_rsp, leaf_ret_pc }) => {
                        if description.body.pc == 0x593ec8 && description.body.actual_rsp == child_rsp
                            && tail_pc == 0x593f4e && actual_target == base.wrapping_add(0x594174)
                            && tail_rsp == child_rsp && leaf_pc == 0x594174 && leaf_rsp == child_rsp
                            && (leaf_ret_pc == 0x594194 || leaf_ret_pc == 0x59419a) { Some(leaf_ret_pc) } else { None }
                    }
                    (SourceCrtQueryReplaceCallKind::Whole593f54, SourceCrtReplaceCounterProfile::Decrement593f58,
                        SourceCrtReplaceCounterReturnPath::DecrementParent { checked_entry: Some(actual),
                            restore_pc, restore_rsp, restore_rax, add_before_after }) => {
                        let body_rsp = child_rsp.wrapping_sub(0x28);
                        if actual.pc == 0x593f54 && actual.actual_module_base == base && actual.actual_rsp == child_rsp
                            && actual.receiver_rcx == call.source_rcx && actual.subtract_before_after == Some((child_rsp, body_rsp))
                            && description.body.pc == 0x593f58 && description.body.actual_rsp == body_rsp
                            && restore_pc == 0x593ff7 && restore_rsp == body_rsp
                            && restore_rax == description.returned.actual_rax
                            && add_before_after == Some((body_rsp, child_rsp)) { Some(0x593ffb) } else { None }
                    }
                    _ => None,
                };
                let envelope = SourceCrtQueryCallEnvelope { actual_module_base: base, source_call_pc: call_pc,
                    source_resume_pc: resume_pc, target_rva, actual_work_rsp: call.actual_caller_rsp,
                    source_rcx: Some(call.source_rcx), source_rdx: None };
                path.is_some_and(|ret_pc| call.source_rcx != 0 && call.source_rdx.is_none()
                    && call.source_pc == base.wrapping_add(u64::from(call_pc))
                    && call.actual_target == base.wrapping_add(u64::from(target_rva))
                    && call.required_actual_resume == base.wrapping_add(u64::from(resume_pc))
                    && description.body.actual_module_base == base && description.body.receiver_rcx == call.source_rcx
                    && entry.call == envelope && whole.call == envelope
                    && entry.before.actual_rip == call.source_pc && entry.before.actual_rsp == call.actual_caller_rsp
                    && entry.entered == (SourceCrtNativeRetFrame { actual_rip: call.actual_target,
                        actual_rsp: child_rsp, actual_rax: entry.before.actual_rax })
                    && entry.stored_return.actual_address == child_rsp && entry.stored_return.actual_word == call.required_actual_resume
                    && entry.actual_rcx == call.source_rcx
                    && (entry.actual_rbx_rsi_rdi.0, entry.actual_rbx_rsi_rdi.2) == (call.required_old_rbx, call.required_incoming_rdi)
                    && owned_entry == Some(entry) && owned_ret == Some(observed_ret)
                    && description.returned.before_rsp == child_rsp && description.returned.after_rsp == call.actual_caller_rsp
                    && description.returned.actual_target == call.required_actual_resume
                    && whole.actual_frame == (SourceCrtNativeRetFrame { actual_rip: call.required_actual_resume,
                        actual_rsp: call.actual_caller_rsp, actual_rax: description.returned.actual_rax })
                    && (whole.actual_rbx, whole.actual_rdi) == (call.required_old_rbx, call.required_incoming_rdi)
                    && frame == whole.actual_frame && registers == (whole.actual_rbx, whole.actual_rdi)
                    && validate_source_crt_native_ret(SourceCrtNativeRetExpectation {
                        before: SourceCrtNativeRetFrame { actual_rip: base.wrapping_add(u64::from(ret_pc)),
                            actual_rsp: child_rsp, actual_rax: description.returned.actual_rax },
                        actual_required_target: call.required_actual_resume }, retired).is_ok()
                    && frame == retired.after)
            }
            _ => false,
        };
        if !matches {
            let reason = if request.is_none() || description.is_none() { SourceCrtReplaceCounterAdmissionReason::WholeCallRequired }
                else if owned_entry != Some(entry) || owned_ret != Some(observed_ret) || frame != whole.actual_frame
                    || registers != (whole.actual_rbx, whole.actual_rdi) { SourceCrtReplaceCounterAdmissionReason::PrefixRequired }
                else { SourceCrtReplaceCounterAdmissionReason::EntryOrReturnRequired };
            return Err(SourceCrtReplaceCounterRejected { query: self, resources, bindings, request, description,
                entry, owned_entry, whole, observed_ret, owned_ret, frame, registers, reason });
        }
        let (Some(request), Some(description), Some(retired)) = (request, description, retired) else {
            return Err(SourceCrtReplaceCounterRejected { query: self, resources, bindings, request, description,
                entry, owned_entry, whole, observed_ret, owned_ret, frame, registers,
                reason: SourceCrtReplaceCounterAdmissionReason::EntryOrReturnRequired });
        };
        Ok(SourceCrtReplaceCounterBound { query: self, resources, bindings,
            evidence: SourceCrtReplaceCounterEvidence { request, description, entry, whole, retired }, adoption_attempted: false })
    }
}
