use super::{Prepared, Services, Phase as P, Entry, CallSite as C, CallRequest, Arguments, MemoryAttempt, Fault};
impl<S: Services> Prepared<S> {
    fn call(&mut self, site: C, pc: u32, target_rva: u32, rcx: Option<u64>, rdx: Option<u64>) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = CallRequest { site, pc, resume_pc: pc + 5, target_rva, target: self.actual.source_image_base.wrapping_add(u64::from(target_rva)),
            work_rsp: self.actual.work_rsp, args: Arguments { rcx, rdx } };
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        let actual = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if actual.resume_pc != request.resume_pc || actual.work_rsp != request.work_rsp { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(actual.rax)
    }
    fn read_word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        self.captures.memory = Some(MemoryAttempt { pc, address, width: 8, written: None });
        self.services.read_word(&mut self.owners, pc, address).map_err(Fault::Service)
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let entry = self.actual.entry; let slot = self.actual.work_rsp.wrapping_add(0x30);
        match self.phase {
            P::Context => { self.captures.context = Some(self.call(C::Context, entry.body_pc(), 0x58db7c, None, None)?); self.phase = P::ReadReference; }
            P::ReadReference => {
                self.captures.reference = Some(self.read_word(entry.read_reference_pc(), self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(0x90))?);
                self.phase = P::SaveReference;
            }
            P::SaveReference => {
                let word = self.captures.reference.ok_or(Fault::MissingState)?; let pc = entry.save_reference_pc();
                self.captures.memory = Some(MemoryAttempt { pc, address: slot, width: 8, written: Some(word) });
                self.services.write_word(&mut self.owners, pc, slot, word).map_err(Fault::Service)?;
                self.captures.saved_reference = true; self.phase = P::Refresh;
            }
            P::Refresh => {

                self.call(C::Refresh, entry.refresh_pc(), 0x58e238, self.captures.context, Some(slot))?;
                self.phase = P::ReloadReference;
            }
            P::ReloadReference => { self.captures.refreshed_reference = Some(self.read_word(entry.reload_pc(), slot)?); self.phase = P::Project; }
            P::Project => {
                let reference = self.captures.refreshed_reference.ok_or(Fault::MissingState)?;
                let pc = entry.projection_pc();
                self.captures.rax = Some(match entry {
                    Entry::Context567c68 => reference.wrapping_add(0x128),
                    Entry::Word567c9c => { let address = reference.wrapping_add(0xc);
                        self.captures.memory = Some(MemoryAttempt { pc, address, width: 4, written: None });
                        u64::from(self.services.read_dword(&mut self.owners, pc, address).map_err(Fault::Service)?) },
                    Entry::Source56807c => self.read_word(pc, reference)?,
                });
                self.phase = P::NativeReturnRequired;
            }
            P::NativeReturnRequired => {}
        }
        Ok(())
    }
}
