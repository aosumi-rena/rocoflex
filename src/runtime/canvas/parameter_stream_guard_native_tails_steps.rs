use super::{CallRequest, Entry, Fault, Phase, Prepared, Services, TailRequest};
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Subtract => {
                self.services.sub_rsp28(&mut self.owners, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp || a.after != self.actual.rsp.wrapping_sub(0x28) { return Err(Fault::StackBinding); }
                self.live_rsp = a.after; self.phase = Phase::Call;
            }
            Phase::Call => {
                if self.owners.call.is_some() { return Err(Fault::OccupiedCall); }
                let r = self.initialization_request()?;
                self.captures.call_request = Some(r); self.services.call_58eca8(&mut self.owners, r).map_err(Fault::Service)?;
                self.record_initialization_return(r)?;
            }
            Phase::Receiver => {
                let pc = if self.actual.entry == Entry::At57a7e0 { 0x57a7e9 } else { 0x57a7fc };
                let receiver = self.actual.source_image_base.wrapping_add(0x8612a0);
                self.services.lea_rcx(&mut self.owners, pc, receiver, &mut self.captures.lea).map_err(Fault::Service)?;
                let a = self.captures.lea.ok_or(Fault::MissingReceipt)?;
                if a.rcx != receiver || a.rsp != self.live_rsp || a.rax != self.live_rax { return Err(Fault::RegisterBinding); }
                self.live_rcx = a.rcx;
                self.phase = if self.actual.entry == Entry::At57a7e0 { Phase::AddStack } else { Phase::ReadTarget };
            }
            Phase::AddStack => {
                self.services.add_rsp28(&mut self.owners, &mut self.captures.adjustment).map_err(Fault::Service)?;
                let a = self.captures.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.live_rsp || a.after != self.actual.rsp { return Err(Fault::StackBinding); }
                self.live_rsp = a.after; self.phase = Phase::ReadTarget;
            }
            Phase::ReadTarget => {
                let slot = self.actual.source_image_base.wrapping_add(self.actual.entry.target_slot_rva() as u64);
                self.captures.slot_address = Some(slot);
                let word = self.services.read_target(&mut self.owners, self.actual.entry.tail_pc(), slot).map_err(Fault::Service)?;
                self.captures.loaded_target = Some(word); self.phase = Phase::Tail;
            }
            Phase::Tail => {
                if self.owners.tail.is_some() { return Err(Fault::OccupiedTail); }
                let request = TailRequest { source_pc: self.actual.entry.tail_pc(), slot_address: self.captures.slot_address.ok_or(Fault::MissingReceipt)?,
                    loaded_target: self.captures.loaded_target.ok_or(Fault::MissingReceipt)?, entry_rsp: self.live_rsp, rcx: self.live_rcx, rax: self.live_rax };
                self.captures.tail_request = Some(request); self.services.tail(&mut self.owners, request).map_err(Fault::Service)?;
                let a = self.owners.tail.as_ref().ok_or(Fault::MissingReceipt)?.actual; self.captures.actual_return = Some(a);
                if a.entry_rsp != request.entry_rsp || a.after_rsp != self.actual.rsp.wrapping_add(8) { return Err(Fault::ReturnBinding); }

                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
    pub(super) fn initialization_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.actual.entry != Entry::At57a7e0 || self.phase != Phase::Call { return Err(Fault::MissingReceipt); }
        Ok(CallRequest { source_pc: 0x57a7e4, target_rva: 0x58eca8, expected_resume_pc: 0x57a7e9, work_rsp: self.live_rsp })
    }
    pub(super) fn record_initialization_return(&mut self, r: CallRequest) -> Result<(), Fault<S::Error>> {
        let a = self.owners.call.as_ref().ok_or(Fault::MissingReceipt)?.actual;
        if a.resume_pc != r.expected_resume_pc || a.work_rsp != r.work_rsp { return Err(Fault::CallBinding); }
        self.captures.call_returned = true; self.live_rax = a.rax; self.phase = Phase::Receiver; Ok(())
    }
}
