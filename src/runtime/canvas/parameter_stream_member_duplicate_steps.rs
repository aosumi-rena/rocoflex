use super::{Prepared, Services, Phase as P, CallSite as C, Fault, Saved, Arguments, CallRequest, MemoryAttempt};
impl<S: Services> Prepared<S> {
    fn call_request(&self, site: C, args: Arguments, stack_arg5: Option<u64>) -> Result<CallRequest, Fault<S::Error>> {
        Ok(CallRequest { site, target: self.actual.source_image_base.wrapping_add(u64::from(site.target_rva())),
            work_rsp: self.actual.work_rsp, args, stack_arg5, required_saved: self.captures.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.saved != request.required_saved {
            return Err(Fault::CallBinding { site });
        }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    pub(super) fn copy_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::Copy { return Err(Fault::MissingState); }
        let s = self.captures.saved.ok_or(Fault::MissingState)?;
        self.call_request(C::Copy, Arguments { rcx: Some(self.captures.allocation.ok_or(Fault::MissingState)?),
            rdx: Some(s.rbx), r8: Some(s.rdi), r9: None }, None)
    }
    pub(super) fn record_copy_value(&mut self, rax: u64) { self.captures.copy_rax = Some(rax); self.phase = P::BranchCopy; }
    fn call(&mut self, site: C, args: Arguments, stack_arg5: Option<u64>) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = self.call_request(site, args, stack_arg5)?;
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            P::BindSaved => {

                self.captures.saved = Some(Saved { rbx: self.actual.original_rbx, rbp: 0, rsi: self.actual.original_rsi, rdi: self.actual.source_rcx });
                self.phase = P::BranchSource;
            }
            P::BranchSource => {
                if self.actual.source_rcx == 0 { self.captures.result = Some(0); self.phase = P::NativeReturnRequired; }
                else { self.captures.saved.as_mut().ok_or(Fault::MissingState)?.rbx = u64::MAX; self.phase = P::AdvanceScan; }
            }
            P::AdvanceScan => {
                let s = self.captures.saved.as_mut().ok_or(Fault::MissingState)?;
                s.rbx = s.rbx.wrapping_add(1); self.phase = P::ReadWord;
            }
            P::ReadWord => {
                let s = self.captures.saved.ok_or(Fault::MissingState)?;
                let address = self.actual.source_rcx.wrapping_add(s.rbx.wrapping_mul(2));
                self.captures.memory = Some(MemoryAttempt { pc: 0x57c475, address, width: 2, written: None });
                let word = self.services.read_word16(&mut self.owners, 0x57c475, address).map_err(Fault::Service)?;
                self.captures.word = Some(word); self.phase = if word == 0 { P::IncludeZero } else { P::AdvanceScan };
            }
            P::IncludeZero => {
                let s = self.captures.saved.as_mut().ok_or(Fault::MissingState)?;
                s.rbx = s.rbx.wrapping_add(1); self.phase = P::Allocate;
            }
            P::Allocate => {
                let count = self.captures.saved.ok_or(Fault::MissingState)?.rbx;
                let a = self.call(C::Allocate, Arguments { rcx: Some(count.wrapping_mul(2)), rdx: None, r8: None, r9: None }, None)?;

                self.captures.allocation = Some(a); self.captures.saved.as_mut().ok_or(Fault::MissingState)?.rsi = a;
                self.phase = P::BranchAllocation;
            }
            P::BranchAllocation => {
                if self.captures.allocation.ok_or(Fault::MissingState)? == 0 {
                    self.captures.result = Some(0); self.phase = P::NativeReturnRequired;
                } else { self.phase = P::Copy; }
            }
            P::Copy => {
                let request = self.copy_request()?;
                let value = self.call(request.site, request.args, request.stack_arg5)?;
                self.record_copy_value(value);
            }
            P::BranchCopy => {
                if self.captures.copy_rax.ok_or(Fault::MissingState)? as u32 == 0 {
                    self.captures.result = Some(self.captures.saved.ok_or(Fault::MissingState)?.rsi);
                    self.phase = P::NativeReturnRequired;
                } else { self.phase = P::SaveFifthArgument; }
            }
            P::SaveFifthArgument => {
                let address = self.actual.work_rsp.wrapping_add(0x20);
                let word = self.captures.saved.ok_or(Fault::MissingState)?.rbp;
                self.captures.memory = Some(MemoryAttempt { pc: 0x57c4c0, address, width: 8, written: Some(word) });
                self.services.write_word(&mut self.owners, 0x57c4c0, address, word).map_err(Fault::Service)?;
                self.phase = P::Diagnostic;
            }
            P::Diagnostic => {
                self.captures.diagnostic_rax = Some(self.call(C::Diagnostic,
                    Arguments { rcx: Some(0), rdx: Some(0), r8: Some(0), r9: Some(0) }, Some(0))?);
                self.phase = P::BreakpointRequired;
            }
            P::NativeReturnRequired | P::BreakpointRequired => {}
        }
        Ok(())
    }
}
