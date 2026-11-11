use super::{Prepared, Services, Phase as P, Fault, CallSite as C, CallRequest, Saved, MemoryAttempt, decode_saved_handler_word};
impl<S: Services> Prepared<S> {
    fn read(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        self.captures.memory = Some(MemoryAttempt { pc, address, written: None });
        self.services.read_qword(&mut self.owners, pc, address).map_err(Fault::Service)
    }
    fn store_fifth(&mut self, pc: u32) -> Result<(), Fault<S::Error>> {
        let address = self.actual.work_rsp.wrapping_add(0x20); let value = self.captures.fifth.ok_or(Fault::MissingState)?;
        self.captures.memory = Some(MemoryAttempt { pc, address, written: Some(value) });
        self.services.write_qword(&mut self.owners, pc, address, value).map_err(Fault::Service)
    }
    fn source_call_request(&self, site: C) -> Result<CallRequest, Fault<S::Error>> {
        let saved = self.captures.saved.ok_or(Fault::MissingState)?;
        let (target, rcx, rdx, r8, r9, r10, fifth, slot) = match site {
            C::Context => (self.actual.source_image_base.wrapping_add(0x57a2cc), self.captures.sixth.ok_or(Fault::MissingState)?,
                Some(self.actual.rdx), Some(self.actual.r8), Some(self.actual.r9), None, None, None),
            C::EncodedSlot => (self.actual.source_image_base.wrapping_add(0x57a338), self.actual.source_image_base.wrapping_add(0x8611f8),
                Some(self.captures.sixth_again.ok_or(Fault::MissingState)?), None, None, None, None, None),
            C::Indirect => (self.captures.indirect_target.ok_or(Fault::MissingState)?, saved.rbp, Some(saved.rsi), Some(saved.rdi),
                Some(saved.rbx), Some(0xd6bc_1f48_3457_2a70), Some(self.captures.fifth.ok_or(Fault::MissingState)?),
                Some(self.actual.source_image_base.wrapping_add(0x77e930))),
            C::Diagnostic => (self.actual.source_image_base.wrapping_add(0x57a054), saved.rbp, Some(saved.rsi), Some(saved.rdi),
                Some(saved.rbx), None, Some(self.captures.fifth.ok_or(Fault::MissingState)?), None),
        };
        Ok(CallRequest { site, target, work_rsp: self.actual.work_rsp, rcx, rdx, r8, r9,
            required_rax: self.captures.rax.ok_or(Fault::MissingState)?, required_r10: r10, saved,
            stack_arg5: fifth, loaded_target_slot: slot })
    }
    pub(super) fn encoded_slot_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::SlotCall { return Err(Fault::MissingState); }
        self.source_call_request(C::EncodedSlot)
    }
    pub(super) fn context_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::ContextCall { return Err(Fault::MissingState); }
        self.source_call_request(C::Context)
    }
    pub(super) fn record_context_value(&mut self, value: u64) {
        self.captures.context = Some(value); self.phase = P::TestContext;
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume() || a.work_rsp != request.work_rsp || a.saved != request.saved { return Err(Fault::CallBinding); }
        self.captures.completed[site.index()] = true; self.captures.rax = Some(a.rax); Ok(a.rax)
    }
    fn call_source(&mut self, site: C) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall); }
        let request = self.source_call_request(site)?;
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            P::BindSaved => {

                self.captures.bound_rbp = Some(self.actual.rcx); self.captures.bound_rbx = Some(u64::from(self.actual.r9 as u32));
                self.phase = P::ReadSixth;
            }
            P::ReadSixth => {
                self.captures.sixth = Some(self.read(0x57a21a, self.actual.work_rsp.wrapping_add(0x68))?); self.phase = P::BindRemaining;
            }
            P::BindRemaining => {
                self.captures.saved = Some(Saved { rbp: self.captures.bound_rbp.ok_or(Fault::MissingState)?,
                    rbx: self.captures.bound_rbx.ok_or(Fault::MissingState)?, rdi: self.actual.r8, rsi: self.actual.rdx });
                self.phase = P::ContextCall;
            }
            P::ContextCall => { let value = self.call_source(C::Context)?; self.record_context_value(value); }
            P::TestContext => { self.phase = if self.captures.context.ok_or(Fault::MissingState)? == 0 { P::ReadSixthAgain } else { P::ReadContextWord }; }
            P::ReadContextWord => {
                let address = self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(0x3b8);
                let value = self.read(0x57a22f, address)?; self.captures.context_word = Some(value); self.captures.rax = Some(value);
                self.phase = P::TestContextWord;
            }
            P::TestContextWord => { self.phase = if self.captures.context_word.ok_or(Fault::MissingState)? == 0 { P::ReadSixthAgain } else { P::ReadFifthRcx }; }
            P::ReadSixthAgain => {

                self.captures.sixth_again = Some(self.read(0x57a276, self.actual.work_rsp.wrapping_add(0x68))?); self.phase = P::SlotCall;
            }
            P::SlotCall => { self.call_source(C::EncodedSlot)?; self.phase = P::ReadEncoded; }
            P::ReadEncoded => {
                let address = self.captures.rax.ok_or(Fault::MissingState)?;
                self.captures.encoded = Some(self.read(0x57a287, address)?); self.phase = P::ReadCookie;
            }
            P::ReadCookie => {
                let value = self.read(0x57a28a, self.actual.source_image_base.wrapping_add(0x820dd8))?;
                self.captures.cookie = Some(value); self.captures.rax = Some(value); self.phase = P::Decode;
            }
            P::Decode => {
                self.captures.decoded = Some(decode_saved_handler_word(self.captures.cookie.ok_or(Fault::MissingState)?, self.captures.encoded.ok_or(Fault::MissingState)?));
                self.phase = P::BranchDecoded;
            }
            P::BranchDecoded => {
                let decoded = self.captures.decoded.ok_or(Fault::MissingState)?;
                if decoded == 0 { self.phase = P::ReadFifthRax; }
                else { self.captures.rax = Some(decoded); self.phase = P::ReadFifthRcx; }
            }
            P::ReadFifthRcx => {

                self.captures.fifth = Some(self.read(0x57a245, self.actual.work_rsp.wrapping_add(0x60))?); self.phase = P::StoreFifthRcx;
            }
            P::StoreFifthRcx => { self.store_fifth(0x57a24d)?; self.phase = P::LoadIndirectTarget; }
            P::LoadIndirectTarget => {
                self.captures.indirect_target = Some(self.read(0x57a25b, self.actual.source_image_base.wrapping_add(0x77e930))?);
                self.phase = P::IndirectCall;
            }
            P::IndirectCall => { self.call_source(C::Indirect)?; self.phase = P::NativeRestoreRequired; }
            P::ReadFifthRax => {

                let value = self.read(0x57a2b0, self.actual.work_rsp.wrapping_add(0x60))?;
                self.captures.fifth = Some(value); self.captures.rax = Some(value); self.phase = P::StoreFifthRax;
            }
            P::StoreFifthRax => { self.store_fifth(0x57a2bb)?; self.phase = P::DiagnosticCall; }
            P::DiagnosticCall => {

                self.call_source(C::Diagnostic)?; self.phase = P::BreakpointRequired;
            }
            P::NativeRestoreRequired | P::BreakpointRequired => {}
        }
        Ok(())
    }
}
