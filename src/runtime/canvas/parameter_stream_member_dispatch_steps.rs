use super::{Prepared, Services, Phase as P, Fault, CallSite as C, CallRequest, MemoryAttempt};
impl<S: Services> Prepared<S> {
    fn at(&mut self, pc: u32, address: u64, width: u8) { self.captures.memory = Some(MemoryAttempt { pc, address, width }); }
    fn local(&self, negative: u64) -> u64 { self.actual.rbp.wrapping_sub(negative) }
    pub(super) fn handler_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::Dispatch { return Err(Fault::MissingState); }
        Ok(CallRequest { site: C::Dispatch, target: self.actual.source_image_base.wrapping_add(0x57a200),
            work_rsp: self.actual.work_rsp, rcx: self.actual.rcx, rdx: Some(self.actual.rdx), r8: Some(self.actual.r8), r9: Some(self.actual.r9),
            required_rax: self.captures.rax.ok_or(Fault::MissingState)?, required_rbx: self.captures.rbx.ok_or(Fault::MissingState)?,
            required_rbp: self.actual.rbp, stack_arg5: Some(self.captures.fifth.ok_or(Fault::MissingState)?), stack_arg6: Some(self.local(0x40)) })
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let a = self.owners.calls[request.site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != request.site.resume() || a.work_rsp != request.work_rsp || a.rbp != request.required_rbp || a.rbx != request.required_rbx {
            return Err(Fault::CallBinding);
        }
        self.captures.completed[request.site.index()] = true; Ok(a.rax)
    }
    pub(super) fn record_handler_value(&mut self, rax: u64) { self.captures.rax = Some(rax); self.phase = P::Read18; }
    pub(super) fn metadata_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        let site = match self.phase { P::FirstRestore => C::FirstRestore, P::SecondRestore => C::SecondRestore, _ => return Err(Fault::MissingState) };
        Ok(CallRequest { site, target: self.actual.source_image_base.wrapping_add(0x568a10), work_rsp: self.actual.work_rsp,
            rcx: self.local(0x40), rdx: None, r8: None, r9: None, required_rax: self.captures.rax.ok_or(Fault::MissingState)?,
            required_rbx: self.captures.rbx.ok_or(Fault::MissingState)?, required_rbp: self.actual.rbp, stack_arg5: None, stack_arg6: None })
    }
    pub(super) fn record_metadata_value(&mut self, site: C, rax: u64) -> Result<(), Fault<S::Error>> {
        let next = match site { C::FirstRestore => P::StoreFirstValue, C::SecondRestore => P::StoreSecondValue, _ => return Err(Fault::MissingState) };
        self.captures.rax = Some(rax); self.phase = next; Ok(())
    }
    fn call_source(&mut self, site: C) -> Result<(), Fault<S::Error>> {
        let slot = site.index();
        if self.owners.calls[slot].is_some() { return Err(Fault::OccupiedCall); }
        let dispatch = site == C::Dispatch;
        let request = if dispatch { self.handler_request()? } else { self.metadata_request()? };
        if request.site != site { return Err(Fault::MissingState); }
        self.captures.requests[slot] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        let value = self.record_call_return(request)?; self.captures.rax = Some(value); Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            P::ClearContext => {
                let address = self.local(0x40); self.at(0x57a391, address, 8);
                self.services.and_qword_zero(&mut self.owners, 0x57a391, address, &mut self.captures.and_qword).map_err(Fault::Service)?;
                let a = self.captures.and_qword.ok_or(Fault::MissingState)?;
                if a.address != address || a.after != 0 { return Err(Fault::RmwBinding); }
                self.phase = P::ReadGlobal;
            }
            P::ReadGlobal => {
                let address = self.actual.source_image_base.wrapping_add(0x861168); self.at(0x57a396, address, 4);
                self.captures.global = Some(self.services.read_dword(&mut self.owners, 0x57a396, address).map_err(Fault::Service)?);
                self.phase = P::Clear30;
            }
            P::Clear30 | P::Clear18 | P::Clear10 | P::Clear8 => {
                let (pc, offset, next) = match self.phase {
                    P::Clear30 => (0x57a39d, 0x30, P::Clear18), P::Clear18 => (0x57a3a1, 0x18, P::Clear10),
                    P::Clear10 => (0x57a3a5, 0x10, P::Clear8), _ => (0x57a3a9, 8, P::BranchGlobal) };
                let address = self.local(offset); self.at(pc, address, 1);
                self.services.write_byte(&mut self.owners, pc, address, 0).map_err(Fault::Service)?; self.phase = next;
            }
            P::BranchGlobal => { self.phase = if self.captures.global.ok_or(Fault::MissingState)? != 0 { P::StoreSixth } else { P::LoadLiteral }; }
            P::LoadLiteral => {
                let address = self.actual.source_image_base.wrapping_add(0x836628); self.at(0x57a3af, address, 16);
                self.captures.literal = Some(self.services.load_movups(&mut self.owners, 0x57a3af, address).map_err(Fault::Service)?);
                self.phase = P::Mark18;
            }
            P::Mark18 => {
                let address = self.local(0x18); self.at(0x57a3b6, address, 1);
                self.services.write_byte(&mut self.owners, 0x57a3b6, address, 1).map_err(Fault::Service)?; self.phase = P::StoreLiteral;
            }
            P::StoreLiteral => {
                let address = self.local(0x28); let value = self.captures.literal.ok_or(Fault::MissingState)?; self.at(0x57a3ba, address, 16);
                self.services.store_movdqu(&mut self.owners, 0x57a3ba, address, value).map_err(Fault::Service)?; self.phase = P::StoreSixth;
            }
            P::StoreSixth => {

                let value = self.local(0x40); self.captures.rax = Some(value);
                let address = self.actual.work_rsp.wrapping_add(0x28); self.at(0x57a3c3, address, 8);
                self.services.write_qword(&mut self.owners, 0x57a3c3, address, value).map_err(Fault::Service)?; self.phase = P::LoadFifth;
            }
            P::LoadFifth => {
                let address = self.actual.rbp.wrapping_add(0x30); self.at(0x57a3c8, address, 8);
                let value = self.services.read_qword(&mut self.owners, 0x57a3c8, address).map_err(Fault::Service)?;
                self.captures.fifth = Some(value); self.captures.rax = Some(value); self.phase = P::StoreFifth;
            }
            P::StoreFifth => {
                let value = self.captures.fifth.ok_or(Fault::MissingState)?;
                let address = self.actual.work_rsp.wrapping_add(0x20); self.at(0x57a3cc, address, 8);
                self.services.write_qword(&mut self.owners, 0x57a3cc, address, value).map_err(Fault::Service)?; self.phase = P::Dispatch;
            }
            P::Dispatch => { self.call_source(C::Dispatch)?; let value = self.captures.rax.ok_or(Fault::MissingState)?; self.record_handler_value(value); }
            P::Read18 => {
                let address = self.local(0x18); self.at(0x57a3d6, address, 1);
                self.captures.flag18 = Some(self.services.read_byte(&mut self.owners, 0x57a3d6, address).map_err(Fault::Service)?); self.phase = P::Branch18;
            }
            P::Branch18 => { self.phase = if self.captures.flag18.ok_or(Fault::MissingState)? == 2 { P::ReadContext } else { P::Read10 }; }
            P::ReadContext => {
                let address = self.local(0x40); self.at(0x57a3dc, address, 8);
                self.captures.rax = Some(self.services.read_qword(&mut self.owners, 0x57a3dc, address).map_err(Fault::Service)?); self.phase = P::ClearContextFlag;
            }
            P::ClearContextFlag => {
                let address = self.captures.rax.ok_or(Fault::MissingState)?.wrapping_add(0x3a8); self.at(0x57a3e0, address, 4);
                self.services.and_dword_fd(&mut self.owners, 0x57a3e0, address, &mut self.captures.and_dword).map_err(Fault::Service)?;
                let a = self.captures.and_dword.ok_or(Fault::MissingState)?;
                if a.address != address || a.after != (a.before & 0xffff_fffd) { return Err(Fault::RmwBinding); }
                self.phase = P::Read10;
            }
            P::Read10 => {
                let address = self.local(0x10); self.at(0x57a3e7, address, 1);
                self.captures.flag10 = Some(self.services.read_byte(&mut self.owners, 0x57a3e7, address).map_err(Fault::Service)?); self.phase = P::Branch10;
            }
            P::Branch10 => { self.phase = if self.captures.flag10.ok_or(Fault::MissingState)? != 0 { P::ReadFirstValue } else { P::Read8 }; }
            P::ReadFirstValue => {
                let address = self.local(0x14); self.at(0x57a3ed, address, 4);
                self.captures.rbx = Some(u64::from(self.services.read_dword(&mut self.owners, 0x57a3ed, address).map_err(Fault::Service)?)); self.phase = P::FirstRestore;
            }
            P::FirstRestore => { self.call_source(C::FirstRestore)?; let value = self.captures.rax.ok_or(Fault::MissingState)?; self.record_metadata_value(C::FirstRestore, value)?; }
            P::StoreFirstValue => {
                let address = self.captures.rax.ok_or(Fault::MissingState)?.wrapping_add(0x20);
                let value = self.captures.rbx.ok_or(Fault::MissingState)? as u32; self.at(0x57a3f9, address, 4);
                self.services.write_dword(&mut self.owners, 0x57a3f9, address, value).map_err(Fault::Service)?; self.phase = P::Read8;
            }
            P::Read8 => {
                let address = self.local(8); self.at(0x57a3fc, address, 1);
                self.captures.flag8 = Some(self.services.read_byte(&mut self.owners, 0x57a3fc, address).map_err(Fault::Service)?); self.phase = P::Branch8;
            }
            P::Branch8 => { self.phase = if self.captures.flag8.ok_or(Fault::MissingState)? != 0 { P::ReadSecondValue } else { P::NativeRestoreRequired }; }
            P::ReadSecondValue => {
                let address = self.local(0xc); self.at(0x57a402, address, 4);
                self.captures.rbx = Some(u64::from(self.services.read_dword(&mut self.owners, 0x57a402, address).map_err(Fault::Service)?)); self.phase = P::SecondRestore;
            }
            P::SecondRestore => { self.call_source(C::SecondRestore)?; let value = self.captures.rax.ok_or(Fault::MissingState)?; self.record_metadata_value(C::SecondRestore, value)?; }
            P::StoreSecondValue => {
                let address = self.captures.rax.ok_or(Fault::MissingState)?.wrapping_add(0x24);
                let value = self.captures.rbx.ok_or(Fault::MissingState)? as u32; self.at(0x57a40e, address, 4);
                self.services.write_dword(&mut self.owners, 0x57a40e, address, value).map_err(Fault::Service)?; self.phase = P::NativeRestoreRequired;
            }
            P::NativeRestoreRequired => {}
        }
        Ok(())
    }
}
