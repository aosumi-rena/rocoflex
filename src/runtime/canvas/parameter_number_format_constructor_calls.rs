use super::{Arguments, CallRequest, CallSite, Fault, Prepared, Services, Target};
impl<S: Services> Prepared<S> {
    pub(super) fn call(&mut self, site: CallSite, target: Target, args: Arguments) -> Result<u64, Fault<S::Error>> {
        let request = CallRequest { site, target, args, work_rsp: self.actual.rbp.wrapping_sub(0x60), required_saved: self.saved.ok_or(Fault::MissingState)? };
        self.call_request(request)
    }
    pub(super) fn lookup_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        Ok(CallRequest { site: CallSite::GetObject, target: Target::DirectRva(0x18c2d0),
            args: Arguments { rcx: Some(self.actual.rbp.wrapping_sub(0x30)), rdx: None, r8: None, cl: None, dl: None },
            work_rsp: self.actual.rbp.wrapping_sub(0x60), required_saved: self.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn record_lookup_value(&mut self, word: u64) { self.captures.get_object = Some(word); self.phase = super::Phase::ReadTranslateVtable; }
    pub(super) fn call_request(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.saved != request.required_saved { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; self.captures.rax = Some(a.rax); Ok(a.rax)
    }
    pub(super) fn word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> { self.services.read_word(&mut self.owners, pc, address).map_err(Fault::Service) }
    pub(super) fn dword(&mut self, pc: u32, address: u64) -> Result<u32, Fault<S::Error>> { self.services.read_dword(&mut self.owners, pc, address).map_err(Fault::Service) }
    pub(super) fn store_word(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> { self.services.write_word(&mut self.owners, pc, address, word).map_err(Fault::Service) }
}
