use super::{rows::ROWS, Arguments, CallRequest, Fault, Operation, Phase, Prepared, Register, Services, TailRequest, WriteRequest};
impl<S: Services> Prepared<S> {
    fn arguments(&self) -> Arguments {
        let row = &ROWS[self.row]; let base = self.actual.source_image_base;
        Arguments { rcx: row.ecx as u64, rdx: base.wrapping_add(row.rdx as u64), r8: base.wrapping_add(row.r8 as u64), r9: base.wrapping_add(row.r9 as u64) }
    }
    pub(super) fn resolver_call_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        if self.phase != Phase::Call || self.row >= 10 { return Err(Fault::CallBinding); }
        if self.owners.calls[self.row].is_some() { return Err(Fault::OccupiedCall); }
        let work_rsp = self.actual.rsp.wrapping_sub(0x28);
        Ok(CallRequest { index: self.row, pc: ROWS[self.row].transfer, target_rva: 0x58ee04,
            expected_resume_pc: ROWS[self.row + 1].pcs[0], work_rsp, entry_rsp: work_rsp.wrapping_sub(8), arguments: self.arguments() })
    }
    pub(super) fn record_resolver_call_return(&mut self, request: CallRequest) -> Result<(), Fault<S::Error>> {
        let a = self.owners.calls[self.row].as_ref().ok_or(Fault::MissingReceipt)?.actual;
        if a.resume_pc != request.expected_resume_pc || a.work_rsp != request.work_rsp { return Err(Fault::CallBinding); }
        self.captures.completed[self.row] = true; self.rax = a.rax; self.row += 1; self.argument = 0; self.phase = Phase::Argument;
        Ok(())
    }
    pub(super) fn resolver_tail_request(&self) -> Result<TailRequest, Fault<S::Error>> {
        if self.phase != Phase::Tail || self.row != 10 { return Err(Fault::ReturnBinding); }
        if self.owners.tail.is_some() { return Err(Fault::OccupiedTail); }
        Ok(TailRequest { pc: ROWS[10].transfer, target_rva: 0x58ee04, entry_rsp: self.actual.rsp, arguments: self.arguments(), rax: self.rax })
    }
    pub(super) fn record_resolver_tail_return(&mut self, request: TailRequest) -> Result<(), Fault<S::Error>> {
        let a = self.owners.tail.as_ref().ok_or(Fault::MissingReceipt)?.actual; self.captures.actual_return = Some(a);
        if a.entry_rsp != request.entry_rsp || a.after_rsp != self.actual.rsp.wrapping_add(8) { return Err(Fault::ReturnBinding); }
        self.phase = Phase::Done;
        Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let work_rsp = self.actual.rsp.wrapping_sub(0x28);
        match self.phase {
            Phase::Subtract => {
                self.services.sub_rsp28(&mut self.owners, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp || a.after != work_rsp { return Err(Fault::StackBinding); } self.phase = Phase::Argument;
            }
            Phase::Argument => {
                let args = self.arguments();
                let (register, operation, full_word) = match self.argument {
                    0 => (Register::R9, Operation::Lea64, args.r9),
                    1 => (Register::Rcx, if self.row == 0 { Operation::Xor32 } else { Operation::Mov32 }, args.rcx),
                    2 => (Register::R8, Operation::Lea64, args.r8), _ => (Register::Rdx, Operation::Lea64, args.rdx) };
                let request = WriteRequest { pc: ROWS[self.row].pcs[self.argument], register, operation, full_word };
                self.captures.requests[self.row][self.argument] = Some(request);
                self.services.write_argument(&mut self.owners, request, &mut self.captures.writes[self.row][self.argument]).map_err(Fault::Service)?;
                let a = self.captures.writes[self.row][self.argument].ok_or(Fault::MissingReceipt)?;
                if a.pc != request.pc || a.register != register || a.full_word != full_word || a.rsp != work_rsp || a.rax != self.rax { return Err(Fault::RegisterBinding); }
                self.argument += 1;
                if self.argument == 4 { self.phase = if self.row == 10 { Phase::AddStack } else { Phase::Call }; }
            }
            Phase::Call => {
                let request = self.resolver_call_request()?;
                self.captures.call_requests[self.row] = Some(request); self.services.call_58ee04(&mut self.owners, request).map_err(Fault::Service)?;
                self.record_resolver_call_return(request)?;
            }
            Phase::AddStack => {
                self.services.add_rsp28(&mut self.owners, &mut self.captures.adjustment).map_err(Fault::Service)?;
                let a = self.captures.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != work_rsp || a.after != self.actual.rsp { return Err(Fault::StackBinding); } self.phase = Phase::Tail;
            }
            Phase::Tail => {
                let request = self.resolver_tail_request()?;
                self.captures.tail_request = Some(request); self.services.tail_58ee04(&mut self.owners, request).map_err(Fault::Service)?;
                self.record_resolver_tail_return(request)?;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
