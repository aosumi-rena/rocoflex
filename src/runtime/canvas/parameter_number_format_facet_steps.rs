use super::{Arguments, CallRequest, CallSite, Fault, Phase, Prepared, Services, StackArguments, Target};
impl<S: Services> Prepared<S> {
    fn word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        self.sentry.prefix.services.facet_read_word(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, pc, address).map_err(Fault::Service)
    }
    fn dword(&mut self, pc: u32, address: u64) -> Result<u32, Fault<S::Error>> {
        self.sentry.prefix.services.facet_read_dword(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, pc, address).map_err(Fault::Service)
    }
    fn byte(&mut self, pc: u32, address: u64) -> Result<u8, Fault<S::Error>> {
        self.sentry.prefix.services.facet_read_byte(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, pc, address).map_err(Fault::Service)
    }
    fn store_word(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> {
        self.sentry.prefix.services.facet_write_word(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, pc, address, word).map_err(Fault::Service)
    }
    fn store_byte(&mut self, pc: u32, address: u64, byte: u8) -> Result<(), Fault<S::Error>> {
        self.sentry.prefix.services.facet_write_byte(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, pc, address, byte).map_err(Fault::Service)
    }
    fn call(&mut self, site: CallSite, target: Target, args: Arguments, stack: Option<StackArguments>) -> Result<u64, Fault<S::Error>> {
        if self.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = CallRequest { site, target, args, stack, work_rsp: self.sentry.prefix.actual.rbp.wrapping_sub(0x80), required_saved: self.sentry.prefix.saved.ok_or(Fault::MissingState)? };
        self.captures.requests[site.index()] = Some(request);
        self.sentry.prefix.services.facet_call(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, request).map_err(Fault::Service)?;
        let a = self.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.saved != request.required_saved { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    fn address(&self, field: u64) -> Result<u64, Fault<S::Error>> {
        Ok(self.sentry.prefix.actual.rbp.wrapping_add(self.captures.format_displacement.ok_or(Fault::MissingState)?).wrapping_add(field))
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let frame = self.sentry.prefix.actual.rbp;
        match self.phase {
            Phase::ReadInitialOffset => {
                let word = self.dword(0x1fb6c0, self.sentry.captures.current_vtable.ok_or(Fault::MissingState)?.wrapping_add(4))?;
                self.captures.initial_offset_word = Some(word); self.captures.initial_displacement = Some((word as i32 as i64) as u64);
                self.phase = Phase::ReadHolder;
            }
            Phase::ReadHolder => { self.captures.holder = Some(self.word(0x1fb6c4, frame.wrapping_add(self.captures.initial_displacement.ok_or(Fault::MissingState)?).wrapping_add(0x60))?); self.phase = Phase::ReadContext; }
            Phase::ReadContext => { self.captures.context = Some(self.word(0x1fb6c9, self.captures.holder.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = Phase::SaveContext; }
            Phase::SaveContext => {
                self.store_word(0x1fb6cd, frame.wrapping_sub(0x18), self.captures.context.ok_or(Fault::MissingState)?)?;
                self.captures.context_stored = true; self.phase = Phase::ReadAcquireVtable;
            }
            Phase::ReadAcquireVtable => { self.captures.acquire_vtable = Some(self.word(0x1fb6d1, self.captures.context.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadAcquireTarget; }
            Phase::ReadAcquireTarget => { self.captures.acquire_target = Some(self.word(0x1fb6d4, self.captures.acquire_vtable.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = Phase::Acquire; }
            Phase::Acquire => {
                self.call(CallSite::Acquire, Target::LoadedIndirect(self.captures.acquire_target.ok_or(Fault::MissingState)?), Arguments { rcx: self.captures.context.ok_or(Fault::MissingState)?, rdx: None, r8: None, r9: None }, None)?;
                self.phase = Phase::GetFacet;
            }
            Phase::GetFacet => {
                let word = self.call(CallSite::GetFacet, Target::DirectRva(0x18a5a0), Arguments { rcx: frame.wrapping_sub(0x20), rdx: None, r8: None, r9: None }, None)?;
                self.captures.facet = Some(word); self.sentry.prefix.saved.as_mut().ok_or(Fault::MissingState)?.rsi = word; self.phase = Phase::ReloadContext;
            }
            Phase::ReloadContext => {
                let word = self.word(0x1fb6e3, frame.wrapping_sub(0x18))?; self.captures.reloaded_context = Some(word);
                self.phase = if word == 0 { Phase::ReloadTable } else { Phase::ReadQueryVtable };
            }
            Phase::ReadQueryVtable => { self.captures.query_vtable = Some(self.word(0x1fb6ec, self.captures.reloaded_context.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadQueryTarget; }
            Phase::ReadQueryTarget => { self.captures.query_target = Some(self.word(0x1fb6ef, self.captures.query_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x10))?); self.phase = Phase::QueryContext; }
            Phase::QueryContext => {
                let word = self.call(CallSite::QueryContext, Target::LoadedIndirect(self.captures.query_target.ok_or(Fault::MissingState)?), Arguments { rcx: self.captures.reloaded_context.ok_or(Fault::MissingState)?, rdx: None, r8: None, r9: None }, None)?;
                self.captures.query_result = Some(word); self.phase = if word == 0 { Phase::ReloadTable } else { Phase::ReadDeleteVtable };
            }
            Phase::ReadDeleteVtable => { self.captures.delete_vtable = Some(self.word(0x1fb6f7, self.captures.query_result.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadDeleteTarget; }
            Phase::ReadDeleteTarget => { self.captures.delete_target = Some(self.word(0x1fb702, self.captures.delete_vtable.ok_or(Fault::MissingState)?)?); self.phase = Phase::DeleteObject; }
            Phase::DeleteObject => {
                self.call(CallSite::DeleteObject, Target::LoadedIndirect(self.captures.delete_target.ok_or(Fault::MissingState)?), Arguments { rcx: self.captures.query_result.ok_or(Fault::MissingState)?, rdx: Some(1), r8: Some(self.captures.delete_vtable.ok_or(Fault::MissingState)?), r9: None }, None)?;
                self.phase = Phase::ReloadTable;
            }
            Phase::ReloadTable => { self.captures.format_table = Some(self.word(0x1fb705, frame.wrapping_add(0x20))?); self.phase = Phase::ReadFormatOffset; }
            Phase::ReadFormatOffset => {
                let word = self.dword(0x1fb709, self.captures.format_table.ok_or(Fault::MissingState)?.wrapping_add(4))?;
                let displacement = (word as i32 as i64) as u64; self.captures.format_offset_word = Some(word); self.captures.format_displacement = Some(displacement);
                self.captures.format_base = Some(frame.wrapping_add(displacement).wrapping_add(0x20)); self.phase = Phase::ReadFormatByte;
            }
            Phase::ReadFormatByte => { self.captures.format_byte = Some(self.byte(0x1fb715, self.address(0x78)?)?); self.phase = Phase::ReadField68; }
            Phase::ReadField68 => { self.captures.field68 = Some(self.word(0x1fb71a, self.address(0x68)?)?); self.phase = Phase::StoreZero8; }
            Phase::StoreZero8 => { self.store_byte(0x1fb71f, frame.wrapping_add(8), 0)?; self.captures.zero8_stored = true; self.phase = Phase::StoreField10; }
            Phase::StoreField10 => { self.store_word(0x1fb723, frame.wrapping_add(0x10), self.captures.field68.ok_or(Fault::MissingState)?)?; self.captures.field10_stored = true; self.phase = Phase::ReadFacetVtable; }
            Phase::ReadFacetVtable => { self.captures.facet_vtable = Some(self.word(0x1fb727, self.captures.facet.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadFormatTarget; }
            Phase::ReadFormatTarget => { self.captures.format_target = Some(self.word(0x1fb72a, self.captures.facet_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x28))?); self.phase = Phase::StoreStackF64; }
            Phase::StoreStackF64 => {
                let words = self.sentry.prefix.saved.ok_or(Fault::MissingState)?.xmm6;
                let bits = u64::from(words[0]) | (u64::from(words[1]) << 32); self.captures.stack_f64_bits = Some(bits);
                self.store_word(0x1fb72e, frame.wrapping_sub(0x80).wrapping_add(0x28), bits)?;
                self.captures.stack_f64_stored = true; self.phase = Phase::StoreStackByte;
            }
            Phase::StoreStackByte => {
                self.store_byte(0x1fb734, frame.wrapping_sub(0x80).wrapping_add(0x20), self.captures.format_byte.ok_or(Fault::MissingState)?)?;
                self.captures.stack_byte_stored = true; self.phase = Phase::Format;
            }
            Phase::Format => {
                let work = frame.wrapping_sub(0x80);
                let stack = StackArguments { byte_address: work.wrapping_add(0x20), byte: self.captures.format_byte.ok_or(Fault::MissingState)?, f64_address: work.wrapping_add(0x28), f64_bits: self.captures.stack_f64_bits.ok_or(Fault::MissingState)? };
                self.call(CallSite::Format, Target::LoadedIndirect(self.captures.format_target.ok_or(Fault::MissingState)?), Arguments { rcx: self.captures.facet.ok_or(Fault::MissingState)?, rdx: Some(frame.wrapping_sub(0x40)), r8: Some(frame.wrapping_add(8)), r9: Some(self.captures.format_base.ok_or(Fault::MissingState)?) }, Some(stack))?;
                self.phase = Phase::ReadOutputByte;
            }
            Phase::ReadOutputByte => {
                let byte = self.byte(0x1fb745, frame.wrapping_sub(0x40))?; self.captures.output_byte = Some(byte);
                self.captures.output_status = Some(u32::from(byte) << 2); self.phase = Phase::StoreOutputStatus;
            }
            Phase::StoreOutputStatus => {
                self.sentry.prefix.services.facet_write_dword(&mut self.sentry.prefix.owners, &mut self.sentry.calls, &mut self.calls, 0x1fb74c, frame.wrapping_add(0xa4), self.captures.output_status.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.output_status_stored = true; self.phase = Phase::StatusRequired;
            }
            Phase::StatusRequired => {}
        }
        Ok(())
    }
}
