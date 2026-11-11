use super::{Arguments, CallRequest, CallSite, Calls, Fault, Owners, Prepared, Prior, Services, Target, facet, sentry};
pub(super) struct Resources<'a, S: Services> { pub(super) services: &'a mut S, pub(super) owners: &'a mut Owners<S>,
    pub(super) sentry: &'a mut sentry::Calls<S::SentryCallOwner>, pub(super) facet: Option<&'a mut facet::Calls<S::FacetCallOwner>>,
    pub(super) calls: &'a mut Calls<S::FinishCallOwner> }
pub(super) fn resources<'a, S: Services>(prior: &'a mut Prior<S>, calls: &'a mut Calls<S::FinishCallOwner>) -> Resources<'a, S> {
    match prior {
        Prior::Sentry(p) => Resources { services: &mut p.prefix.services, owners: &mut p.prefix.owners, sentry: &mut p.calls, facet: None, calls },
        Prior::Facet(p) => Resources { services: &mut p.sentry.prefix.services, owners: &mut p.sentry.prefix.owners, sentry: &mut p.sentry.calls, facet: Some(&mut p.calls), calls },
    }
}
impl<S: Services> Prepared<S> {
    fn resources(&mut self) -> Resources<'_, S> {
        resources(&mut self.prior, &mut self.calls)
    }
    pub(super) fn word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        let r = self.resources(); r.services.finish_read_word(r.owners, r.sentry, r.facet, r.calls, pc, address).map_err(Fault::Service)
    }
    pub(super) fn dword(&mut self, pc: u32, address: u64) -> Result<u32, Fault<S::Error>> {
        let r = self.resources(); r.services.finish_read_dword(r.owners, r.sentry, r.facet, r.calls, pc, address).map_err(Fault::Service)
    }
    pub(super) fn vector(&mut self, pc: u32, address: u64) -> Result<[u32; 4], Fault<S::Error>> {
        let r = self.resources(); r.services.finish_read_128(r.owners, r.sentry, r.facet, r.calls, pc, address).map_err(Fault::Service)
    }
    pub(super) fn store_word(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> {
        let r = self.resources(); r.services.finish_write_word(r.owners, r.sentry, r.facet, r.calls, pc, address, word).map_err(Fault::Service)
    }
    pub(super) fn store_dword(&mut self, pc: u32, address: u64, word: u32) -> Result<(), Fault<S::Error>> {
        let r = self.resources(); r.services.finish_write_dword(r.owners, r.sentry, r.facet, r.calls, pc, address, word).map_err(Fault::Service)
    }
    pub(super) fn store_vector(&mut self, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Fault<S::Error>> {
        let r = self.resources(); r.services.finish_write_128(r.owners, r.sentry, r.facet, r.calls, pc, address, words).map_err(Fault::Service)
    }
    pub(super) fn call(&mut self, site: CallSite, target: Target, args: Arguments) -> Result<u64, Fault<S::Error>> {
        if self.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let prefix = self.prior.prefix();
        let request = CallRequest { site, target, args, work_rsp: prefix.actual.rbp.wrapping_sub(0x80), required_saved: prefix.saved.ok_or(Fault::MissingState)? };
        self.captures.requests[site.index()] = Some(request);
        let r = self.resources(); r.services.finish_call(r.owners, r.sentry, r.facet, r.calls, request).map_err(Fault::Service)?;
        let a = self.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.saved != request.required_saved { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    pub(super) fn address(&self, field: u64) -> Result<u64, Fault<S::Error>> {
        Ok(self.prior.prefix().actual.rbp.wrapping_add(self.captures.displacement.ok_or(Fault::MissingState)?).wrapping_add(field))
    }
}
