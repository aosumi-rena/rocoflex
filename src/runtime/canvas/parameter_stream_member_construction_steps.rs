use super::{Prepared, Services, Phase as P, CallSite as C, Fault, CallRequest, Arguments, MemoryAttempt};
impl<S: Services> Prepared<S> {
    pub(super) fn call_request(&self, site: C, rcx: Option<u64>, rdx: Option<u64>) -> CallRequest {
        CallRequest { site, target: self.actual.source_image_base.wrapping_add(u64::from(site.target_rva())),
            args: Arguments { rcx, rdx }, work_rsp: self.actual.work_rsp, required_rbx: self.actual.receiver_rcx }
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let a = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != site.resume_pc() || a.work_rsp != request.work_rsp || a.rbx != request.required_rbx { return Err(Fault::CallBinding { site }); }
        self.captures.completed[site.index()] = true; Ok(a.rax)
    }
    pub(super) fn query_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        let site = match self.phase { P::QueryWord => C::QueryWord, P::CopySource => C::CopySource,
            P::FallbackSource => C::FallbackSource, P::QueryContext => C::QueryContext, _ => return Err(Fault::MissingState) };
        Ok(self.call_request(site, None, None))
    }
    pub(super) fn record_query_value(&mut self, site: C, rax: u64) {
        match site {
            C::QueryWord => { self.captures.query_word = Some(rax); self.phase = P::StoreWord; }
            C::CopySource => { self.captures.source = Some(rax); self.phase = P::ReloadDestination; }
            C::FallbackSource => { self.captures.fallback = Some(rax); self.phase = P::ClearOwnedFlag; }
            C::QueryContext => { self.captures.context = Some(rax); self.phase = P::ReadContextMember; }
            C::Allocate | C::Duplicate => unreachable!(),
        }
    }
    pub(super) fn duplicate_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != P::Duplicate { return Err(Fault::MissingState); }
        Ok(self.call_request(C::Duplicate, Some(self.captures.context_member.ok_or(Fault::MissingState)?), None))
    }
    pub(super) fn record_duplicate_value(&mut self, rax: u64) { self.captures.duplicate = Some(rax); self.phase = P::StoreDuplicate; }
    fn call(&mut self, site: C, rcx: Option<u64>, rdx: Option<u64>) -> Result<u64, Fault<S::Error>> {
        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = self.call_request(site, rcx, rdx);
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    fn attempt(&mut self, pc: u32, address: u64, width: u8) { self.captures.memory = Some(MemoryAttempt { pc, address, width }); }
    fn store_word(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> {
        self.attempt(pc, address, 8); self.services.write_word(&mut self.owners, pc, address, word).map_err(Fault::Service)
    }
    fn store_dword(&mut self, pc: u32, address: u64, word: u32) -> Result<(), Fault<S::Error>> {
        self.attempt(pc, address, 4); self.services.write_dword(&mut self.owners, pc, address, word).map_err(Fault::Service)
    }
    fn read_word(&mut self, pc: u32, address: u64) -> Result<u64, Fault<S::Error>> {
        self.attempt(pc, address, 8); self.services.read_word(&mut self.owners, pc, address).map_err(Fault::Service)
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let root = self.actual.receiver_rcx;
        match self.phase {
            P::QueryWord => { let value = self.call(C::QueryWord, None, None)?; self.record_query_value(C::QueryWord, value); }
            P::StoreWord => { self.store_dword(0x561303, root, self.captures.query_word.ok_or(Fault::MissingState)? as u32)?; self.phase = P::Allocate; }
            P::Allocate => { self.captures.allocation = Some(self.call(C::Allocate, Some(0x100), Some(2))?); self.phase = P::StoreAllocation; }
            P::StoreAllocation => { self.store_word(0x56130f, root.wrapping_add(8), self.captures.allocation.ok_or(Fault::MissingState)?)?; self.phase = P::BranchAllocation; }
            P::BranchAllocation => { self.phase = if self.captures.allocation.ok_or(Fault::MissingState)? == 0 { P::FallbackSource } else { P::CopySource }; }
            P::CopySource => { let value = self.call(C::CopySource, None, None)?; self.record_query_value(C::CopySource, value); }
            P::ReloadDestination => {
                self.captures.destination = Some(self.read_word(0x56131d, root.wrapping_add(8))?);
                self.captures.remaining = 4; self.captures.lane = 0; self.phase = P::LoadVector;
            }
            P::LoadVector => {
                let lane = self.captures.lane; if lane >= 8 { return Err(Fault::CopyPhase); }
                let pcs = [0x56132a, 0x561330, 0x561338, 0x561340, 0x561348, 0x561350, 0x561358, 0x561363];
                let address = self.captures.source.ok_or(Fault::MissingState)?.wrapping_add((lane as u64) * 16);
                self.attempt(pcs[lane], address, 16);
                self.captures.vector = Some(self.services.read_vector(&mut self.owners, pcs[lane], address).map_err(Fault::Service)?);
                self.phase = if lane == 7 { P::AdvanceSource } else { P::StoreVector };
            }
            P::StoreVector => {
                let lane = self.captures.lane; if lane >= 8 { return Err(Fault::CopyPhase); }
                let pcs = [0x56132d, 0x561334, 0x56133c, 0x561344, 0x56134c, 0x561354, 0x56135c, 0x56136a];
                let destination = self.captures.destination.ok_or(Fault::MissingState)?;
                let address = if lane == 7 { destination.wrapping_sub(16) } else { destination.wrapping_add((lane as u64) * 16) };
                self.attempt(pcs[lane], address, 16);
                self.services.write_vector(&mut self.owners, pcs[lane], address, self.captures.vector.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.copied_vectors += 1;
                if lane == 7 { self.phase = P::DecrementCopy; }
                else if lane == 6 { self.phase = P::AdvanceDestination; }
                else { self.captures.lane += 1; self.phase = P::LoadVector; }
            }
            P::AdvanceDestination => { self.captures.destination = Some(self.captures.destination.ok_or(Fault::MissingState)?.wrapping_add(0x80)); self.captures.lane = 7; self.phase = P::LoadVector; }
            P::AdvanceSource => { self.captures.source = Some(self.captures.source.ok_or(Fault::MissingState)?.wrapping_add(0x80)); self.phase = P::StoreVector; }
            P::DecrementCopy => { self.captures.remaining = self.captures.remaining.wrapping_sub(1); self.captures.lane = 0;
                self.phase = if self.captures.remaining != 0 { P::LoadVector } else { P::StoreOwnedFlag }; }
            P::StoreOwnedFlag => { self.store_dword(0x561374, root.wrapping_add(0x10), 1)?; self.phase = P::QueryContext; }
            P::FallbackSource => { let value = self.call(C::FallbackSource, None, None)?; self.record_query_value(C::FallbackSource, value); }
            P::ClearOwnedFlag => {
                let address = root.wrapping_add(0x10); self.attempt(0x561382, address, 4);
                self.services.and_dword_zero(&mut self.owners, 0x561382, address, &mut self.captures.cleared).map_err(Fault::Service)?;
                let a = self.captures.cleared.ok_or(Fault::MissingState)?;
                if a.address != address || a.after != 0 { return Err(Fault::RmwBinding); } self.phase = P::StoreFallback;
            }
            P::StoreFallback => { self.store_word(0x561386, root.wrapping_add(8), self.captures.fallback.ok_or(Fault::MissingState)?)?; self.phase = P::QueryContext; }
            P::QueryContext => { let value = self.call(C::QueryContext, None, None)?; self.record_query_value(C::QueryContext, value); }
            P::ReadContextMember => { self.captures.context_member = Some(self.read_word(0x56138f, self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = P::StoreContextMember; }
            P::StoreContextMember => {
                let word = self.captures.context_member.ok_or(Fault::MissingState)?;
                self.store_word(0x561393, root.wrapping_add(0x18), word)?;
                self.phase = if word == 0 { P::NativeReturnRequired } else { P::Duplicate };
            }
            P::Duplicate => { let request = self.duplicate_request()?;
                let value = self.call(request.site, request.args.rcx, request.args.rdx)?; self.record_duplicate_value(value); }
            P::StoreDuplicate => { self.store_word(0x5613a1, root.wrapping_add(0x18), self.captures.duplicate.ok_or(Fault::MissingState)?)?; self.phase = P::NativeReturnRequired; }
            P::NativeReturnRequired => {}
        }
        Ok(())
    }
}
