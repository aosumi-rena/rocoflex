use super::{CallRequest, CallSite, Fault, Phase, Prepared, Services, Target};
impl<S: Services> Prepared<S> {
    fn read_word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        self.prefix.services.sentry_read_word(&mut self.prefix.owners, &mut self.calls, pc, address).map_err(Fault::Service)
    }
    fn read_dword(&mut self, pc: u32, address: u64) -> Result<u32, Fault<S::Error>> {
        self.prefix.services.sentry_read_dword(&mut self.prefix.owners, &mut self.calls, pc, address).map_err(Fault::Service)
    }
    fn byte(&mut self, pc: u32, byte: u8) -> Result<(), Fault<S::Error>> {
        self.captures.sentry_byte = Some(byte); self.captures.sentry_byte_stored = false;
        self.prefix.services.sentry_write_byte(&mut self.prefix.owners, &mut self.calls, pc, self.prefix.actual.rbp.wrapping_add(0x98), byte).map_err(Fault::Service)?;
        self.captures.sentry_byte_stored = true; Ok(())
    }
    fn zero_a4(&mut self, pc: u32) -> Result<(), Fault<S::Error>> {
        self.prefix.services.sentry_write_dword(&mut self.prefix.owners, &mut self.calls, pc, self.prefix.actual.rbp.wrapping_add(0xa4), 0).map_err(Fault::Service)?;
        self.captures.a4_zero_stored = true; Ok(())
    }
    fn offset(&mut self, pc: u32) -> Result<(), Fault<S::Error>> {
        let word = self.read_dword(pc, self.captures.current_vtable.ok_or(Fault::MissingState)?.wrapping_add(4))?;
        self.captures.current_offset_word = Some(word); self.captures.current_displacement = Some((word as i32 as i64) as u64); Ok(())
    }
    fn address(&self, field: u64) -> Result<u64, Fault<S::Error>> {
        Ok(self.prefix.actual.rbp.wrapping_add(self.captures.current_displacement.ok_or(Fault::MissingState)?).wrapping_add(field))
    }
    fn call(&mut self, site: CallSite, target: Target, rcx: u64) -> Result<(), Fault<S::Error>> {
        if self.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = CallRequest { site, target, rcx, work_rsp: self.prefix.actual.rbp.wrapping_sub(0x80), required_saved: self.prefix.saved.ok_or(Fault::MissingState)? };
        self.captures.requests[site.index()] = Some(request);
        self.prefix.services.sentry_call(&mut self.prefix.owners, &mut self.calls, request).map_err(Fault::Service)?;
        let actual = self.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if actual.resume_pc != site.resume_pc() || actual.work_rsp != request.work_rsp || actual.saved != request.required_saved { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let frame = self.prefix.actual.rbp;
        match self.phase {
            Phase::BindSentry => {
                self.prefix.saved.as_mut().ok_or(Fault::MissingState)?.rsi = frame.wrapping_add(0x20); self.phase = Phase::SaveStream;
            }
            Phase::SaveStream => {
                let word = self.prefix.saved.ok_or(Fault::MissingState)?.rsi;
                self.prefix.services.sentry_write_word(&mut self.prefix.owners, &mut self.calls, 0x1fb636, frame.wrapping_add(0x90), word).map_err(Fault::Service)?;
                self.captures.stream_saved = true; self.phase = Phase::ReadTable;
            }
            Phase::ReadTable => { self.captures.current_vtable = Some(self.read_word(0x1fb63d, frame.wrapping_add(0x20))?); self.phase = Phase::ReadOffset; }
            Phase::ReadOffset => { self.offset(0x1fb641)?; self.phase = Phase::ReadPreObject; }
            Phase::ReadPreObject => {
                let word = self.read_word(0x1fb645, self.address(0x68)?)?; self.captures.pre_object = Some(word);
                self.phase = if word == 0 { Phase::ReadInitialStatus } else { Phase::ReadPreVtable };
            }
            Phase::ReadPreVtable => { self.captures.pre_vtable = Some(self.read_word(0x1fb64f, self.captures.pre_object.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadPreTarget; }
            Phase::ReadPreTarget => { self.captures.pre_target = Some(self.read_word(0x1fb652, self.captures.pre_vtable.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = Phase::PreCall; }
            Phase::PreCall => {
                self.call(CallSite::PreVirtual, Target::LoadedIndirect(self.captures.pre_target.ok_or(Fault::MissingState)?), self.captures.pre_object.ok_or(Fault::MissingState)?)?;
                self.phase = Phase::ReloadTable;
            }
            Phase::ReloadTable => { self.captures.current_vtable = Some(self.read_word(0x1fb655, frame.wrapping_add(0x20))?); self.phase = Phase::ReloadOffset; }
            Phase::ReloadOffset => { self.offset(0x1fb659)?; self.phase = Phase::ReadInitialStatus; }
            Phase::ReadInitialStatus => {
                let word = self.read_dword(0x1fb65d, self.address(0x30)?)?; self.captures.initial_status = Some(word);
                self.phase = if word != 0 { Phase::StoreFalse } else { Phase::ReadTie };
            }
            Phase::StoreFalse => { self.byte(0x1fb664, 0)?; self.phase = Phase::ZeroInitialStatus; }
            Phase::ZeroInitialStatus => { self.zero_a4(0x1fb66b)?; self.phase = Phase::StatusRequired; }
            Phase::ReadTie => {
                let word = self.read_word(0x1fb67a, self.address(0x70)?)?; self.captures.tie_object = Some(word);
                self.captures.tie_is_self = Some(word == self.prefix.saved.ok_or(Fault::MissingState)?.rsi); self.phase = Phase::TieBranch;
            }
            Phase::TieBranch => {
                self.phase = if self.captures.tie_object.ok_or(Fault::MissingState)? == 0 || self.captures.tie_is_self.ok_or(Fault::MissingState)?
                    { Phase::StoreTrue } else { Phase::TieCall };
            }
            Phase::TieCall => { self.call(CallSite::Tied, Target::DirectRva(0x1897b0), self.captures.tie_object.ok_or(Fault::MissingState)?)?; self.phase = Phase::ReloadTieTable; }
            Phase::ReloadTieTable => { self.captures.current_vtable = Some(self.read_word(0x1fb693, frame.wrapping_add(0x20))?); self.phase = Phase::ReloadTieOffset; }
            Phase::ReloadTieOffset => { self.offset(0x1fb697)?; self.phase = Phase::ZeroTieStatus; }
            Phase::ZeroTieStatus => { self.zero_a4(0x1fb69b)?; self.phase = Phase::ReadTieStatus; }
            Phase::ReadTieStatus => { self.captures.tie_status = Some(self.read_dword(0x1fb6a5, self.address(0x30)?)?); self.phase = Phase::StoreTieEqual; }
            Phase::StoreTieEqual => {
                let equal = self.captures.tie_status.ok_or(Fault::MissingState)? == 0; self.byte(0x1fb6aa, u8::from(equal))?;
                self.phase = if equal { Phase::FacetRequired } else { Phase::StatusRequired };
            }
            Phase::StoreTrue => { self.byte(0x1fb6b9, 1)?; self.phase = Phase::FacetRequired; }
            Phase::FacetRequired | Phase::StatusRequired => {}
        }
        Ok(())
    }
}
