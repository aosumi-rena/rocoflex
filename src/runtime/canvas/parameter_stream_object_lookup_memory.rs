use super::{Arguments, CallRequest, CallSite, Fault, Prepared, Services, Target};
impl<S: Services> Prepared<S> {
    pub(super) fn word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> { self.services.read_word(&mut self.owners, pc, address).map_err(Fault::Service) }
    pub(super) fn dword(&mut self, pc: u32, address: u64) -> Result<u32, Fault<S::Error>> { self.services.read_dword(&mut self.owners, pc, address).map_err(Fault::Service) }
    pub(super) fn store(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> { self.services.write_word(&mut self.owners, pc, address, word).map_err(Fault::Service) }
    pub(super) fn call(&mut self, site: CallSite, target: Target, rcx: Option<u64>, rdx: Option<u64>) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = self.call_request(site, target, rcx, rdx)?;
        self.captures.requests[site.index()] = Some(request); self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    pub(super) fn call_request(&self, site: CallSite, target: Target, rcx: Option<u64>, rdx: Option<u64>) -> Result<CallRequest, Fault<S::Error>> {
        Ok(CallRequest { site, target, args: Arguments { rcx, rdx, r8: None, cl: None, dl: None },
            work_rsp: self.actual.rbp.wrapping_sub(0x80), required_saved: self.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.saved != request.required_saved { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    pub(super) fn fallback_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        self.call_request(CallSite::Fallback, Target::DirectRva(0x55e7ac), None, None)
    }
    pub(super) fn record_fallback_value(&mut self, word: u64) {
        self.captures.fallback_context = Some(word); self.phase = super::Phase::ReadFallbackCount;
    }
    pub(super) fn call_fallback(&mut self) -> Result<(), Fault<S::Error>> {
        let request = self.fallback_request()?;
        let word = self.call(request.site, request.target, request.args.rcx, request.args.rdx)?;
        self.record_fallback_value(word); Ok(())
    }
    pub(super) fn registration_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        self.call_request(CallSite::Register, Target::DirectRva(0x55e3d4), Some(self.saved.ok_or(Fault::MissingState)?.rsi), None)
    }
    pub(super) fn record_registration_return(&mut self) { self.phase = super::Phase::ReloadPublishedObject; }
    pub(super) fn call_registration(&mut self) -> Result<(), Fault<S::Error>> {
        let request = self.registration_request()?;
        self.call(request.site, request.target, request.args.rcx, request.args.rdx)?;
        self.record_registration_return(); Ok(())
    }
    pub(super) fn member_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        self.call_request(CallSite::ConstructMember, Target::DirectRva(0x5612f0), Some(self.saved.ok_or(Fault::MissingState)?.rsi.wrapping_add(0x10)), None)
    }
    pub(super) fn record_member_return(&mut self) { self.phase = super::Phase::DestroyTemporary; }
    pub(super) fn call_member(&mut self) -> Result<(), Fault<S::Error>> {
        let request = self.member_request()?;
        self.call(request.site, request.target, request.args.rcx, request.args.rdx)?;
        self.record_member_return(); Ok(())
    }
    pub(super) fn guard_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        use super::Phase as P;
        let (site, offset, construct) = match self.phase {
            P::EnterOuter => (CallSite::EnterOuter, 0x14u64, true),
            P::EnterInner => (CallSite::EnterInner, 0u64.wrapping_sub(0x60), true),
            P::LeaveInner => (CallSite::LeaveInner, 0u64.wrapping_sub(0x60), false),
            P::LeaveOuter => (CallSite::LeaveOuter, 0x14u64, false),
            _ => return Err(Fault::MissingState),
        };
        self.call_request(site, Target::DirectRva(if construct { 0x55e0a0 } else { 0x55e0dc }),
            Some(self.actual.rbp.wrapping_add(offset)), if construct { Some(0) } else { None })
    }
    pub(super) fn guard_call_returned(&mut self, site: CallSite) -> Result<(), Fault<S::Error>> {
        use super::Phase as P;
        self.phase = match site { CallSite::EnterOuter => P::ReadCached, CallSite::EnterInner => P::ReadCurrentIndex,
            CallSite::LeaveInner => P::ReloadIndex, CallSite::LeaveOuter => P::NativeReturnRequired, _ => return Err(Fault::MissingState) };
        Ok(())
    }
    pub(super) fn call_guard(&mut self) -> Result<(), Fault<S::Error>> {
        let r = self.guard_request()?; self.call(r.site, r.target, r.args.rcx, r.args.rdx)?;
        self.guard_call_returned(r.site)
    }
}
