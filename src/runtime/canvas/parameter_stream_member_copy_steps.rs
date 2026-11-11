use super::{Prepared, Services, Phase as P, CallSite as C, ErrorCause as E, Fault, Arguments, CallRequest, MemoryAttempt};
impl<S: Services> Prepared<S> {
    fn call_request(&self, site: C, args: Arguments) -> Result<CallRequest, Fault<S::Error>> {
        Ok(CallRequest { site, target: self.actual.source_image_base.wrapping_add(u64::from(site.target_rva())),
            work_rsp: self.actual.work_rsp, args, required_rbx: self.captures.rbx.ok_or(Fault::MissingState)? })
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.rbx != request.required_rbx { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    pub(super) fn accessor_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::ErrorPointer { return Err(Fault::MissingState); }
        let exhausted = self.captures.cause.ok_or(Fault::MissingState)? == E::Exhausted;
        let args = Arguments { rcx: Some(self.actual.destination_rcx), rdx: self.captures.remaining,
            r8: if exhausted { self.captures.delta } else { Some(self.actual.source_r8) },
            r9: if exhausted { self.captures.cursor } else { None } };
        self.call_request(if exhausted { C::ExhaustedPointer } else { C::InvalidPointer }, args)
    }
    pub(super) fn record_pointer_value(&mut self, rax: u64) { self.captures.pointer = Some(rax); self.phase = P::SetError; }
    pub(super) fn diagnostic_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::Diagnostic || self.captures.pointer.is_none() { return Err(Fault::MissingState); }
        self.call_request(C::Diagnostic, Arguments { rcx: None, rdx: None, r8: None, r9: None })
    }
    pub(super) fn record_diagnostic_value(&mut self, rax: u64) -> Result<(), Fault<S::Error>> {
        self.captures.diagnostic_rax = Some(rax); self.finish_value()
    }
    fn call(&mut self, site: C, args: Arguments) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = self.call_request(site, args)?;
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    fn finish_value(&mut self) -> Result<(), Fault<S::Error>> {
        self.captures.result = Some(u64::from(self.captures.rbx.ok_or(Fault::MissingState)? as u32));
        self.phase = P::NativeReturnRequired; Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            P::ZeroRbx => { self.captures.rbx = Some(0); self.captures.remaining = Some(self.actual.capacity_rdx); self.phase = P::Validate; }
            P::Validate => {
                let cause = if self.actual.destination_rcx == 0 { Some(E::NullDestination) }
                    else if self.actual.capacity_rdx == 0 { Some(E::ZeroCapacity) }
                    else if self.actual.source_r8 == 0 { Some(E::NullSource) } else { None };
                self.captures.cause = cause;
                self.phase = match cause { Some(E::NullSource) => P::ClearDestination, Some(_) => P::ErrorPointer,
                    None => { self.captures.cursor = Some(self.actual.destination_rcx);
                        self.captures.delta = Some(self.actual.source_r8.wrapping_sub(self.actual.destination_rcx)); P::ReadWord } };
            }
            P::ReadWord => {
                let address = self.captures.delta.ok_or(Fault::MissingState)?.wrapping_add(self.captures.cursor.ok_or(Fault::MissingState)?);
                self.captures.memory = Some(MemoryAttempt { pc: 0x58e349, address, width: 2, written: None });
                self.captures.word = Some(self.services.read_word16(&mut self.owners, 0x58e349, address).map_err(Fault::Service)?);
                self.phase = P::WriteWord;
            }
            P::WriteWord => {
                let address = self.captures.cursor.ok_or(Fault::MissingState)?; let word = self.captures.word.ok_or(Fault::MissingState)?;
                self.captures.memory = Some(MemoryAttempt { pc: 0x58e34e, address, width: 2, written: Some(u64::from(word)) });
                self.services.write_word16(&mut self.owners, 0x58e34e, address, word).map_err(Fault::Service)?;
                self.phase = P::Advance;
            }
            P::Advance => { self.captures.cursor = Some(self.captures.cursor.ok_or(Fault::MissingState)?.wrapping_add(2)); self.phase = P::BranchWord; }
            P::BranchWord => {
                if self.captures.word.ok_or(Fault::MissingState)? == 0 { self.finish_value()?; } else { self.phase = P::Decrement; }
            }
            P::Decrement => {
                let remaining = self.captures.remaining.ok_or(Fault::MissingState)?.wrapping_sub(1); self.captures.remaining = Some(remaining);
                self.phase = if remaining != 0 { P::ReadWord } else { P::TestRemaining };
            }
            P::TestRemaining => {
                if self.captures.remaining.ok_or(Fault::MissingState)? != 0 { self.finish_value()?; }
                else { self.captures.cause = Some(E::Exhausted); self.phase = P::ClearDestination; }
            }
            P::ClearDestination => {
                let pc = if self.captures.cause == Some(E::Exhausted) { 0x58e366 } else { 0x58e327 };
                let address = self.actual.destination_rcx;
                self.captures.memory = Some(MemoryAttempt { pc, address, width: 2, written: Some(0) });
                self.services.write_word16(&mut self.owners, pc, address, 0).map_err(Fault::Service)?;
                self.phase = P::ErrorPointer;
            }
            P::ErrorPointer => {
                let request = self.accessor_request()?; let value = self.call(request.site, request.args)?;
                self.record_pointer_value(value);
            }
            P::SetError => {
                self.captures.rbx = Some(if self.captures.cause == Some(E::Exhausted) { 0x22 } else { 0x16 });
                self.phase = P::WriteError;
            }
            P::WriteError => {
                let address = self.captures.pointer.ok_or(Fault::MissingState)?; let word = self.captures.rbx.ok_or(Fault::MissingState)? as u32;

                self.captures.memory = Some(MemoryAttempt { pc: 0x58e334, address, width: 4, written: Some(u64::from(word)) });
                self.services.write_dword(&mut self.owners, 0x58e334, address, word).map_err(Fault::Service)?;
                self.phase = P::Diagnostic;
            }
            P::Diagnostic => {

                let request = self.diagnostic_request()?;
                let value = self.call(request.site, request.args)?;
                self.record_diagnostic_value(value)?;
            }
            P::NativeReturnRequired => {}
        }
        Ok(())
    }
}
