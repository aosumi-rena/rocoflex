use super::{CallRequest, CallSite, Entry, Fault, Phase, Prepared, Services, indexed_receiver};
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::StoreConstructorIndex => {
                let index = self.actual.constructor_edx.ok_or(Fault::MissingState)?; self.captures.index = Some(index);
                self.services.write_index(&mut self.owners, 0x55e0a6, self.actual.guard_rcx, index).map_err(Fault::Service)?;
                self.captures.index_store_returned = true; self.phase = Phase::BindRbx;
            }
            Phase::BindRbx => { self.captures.bound_rbx = Some(self.actual.guard_rcx); self.phase = Phase::Branch; }
            Phase::ReadDestructorIndex => {
                let index = self.services.read_index(&mut self.owners, 0x55e0e0, self.actual.guard_rcx).map_err(Fault::Service)?;
                self.captures.index = Some(index); self.captures.rax = Some((index as i32 as i64) as u64); self.phase = Phase::Branch;
            }
            Phase::Branch => {
                let index = self.captures.index.ok_or(Fault::MissingState)?;
                self.phase = if index == 0 {
                    if self.actual.entry == Entry::Construct55e0a0 {
                        self.captures.request = Some(CallRequest { site: CallSite::ZeroConstruct, rcx: self.actual.guard_rcx, work_rsp: self.actual.work_rsp, required_rbx: self.captures.bound_rbx }); Phase::Call
                    } else { Phase::TailRestoreRequired }
                } else if (index as i32) < 8 { Phase::PrepareIndexed } else { Phase::NormalRequired };
            }
            Phase::PrepareIndexed => {
                let indexed = indexed_receiver(self.actual.source_image_base, self.captures.index.ok_or(Fault::MissingState)?);
                let site = match self.actual.entry { Entry::Construct55e0a0 => CallSite::IndexedConstruct, Entry::Destroy55e0dc => CallSite::IndexedDestroy };
                self.captures.indexed = Some(indexed); self.captures.request = Some(CallRequest { site, rcx: indexed.receiver, work_rsp: self.actual.work_rsp,
                    required_rbx: self.captures.bound_rbx }); self.phase = Phase::Call;
            }
            Phase::Call => {
                if self.owners.call.is_some() { return Err(Fault::OccupiedCall); }
                let r = self.captures.request.ok_or(Fault::MissingState)?; self.services.call(&mut self.owners, r).map_err(Fault::Service)?;
                self.record_call_return(r)?;
            }
            Phase::NormalRequired | Phase::TailRestoreRequired => {}
        }
        Ok(())
    }
    pub(super) fn record_call_return(&mut self, r: CallRequest) -> Result<(), Fault<S::Error>> {
        let a = self.owners.call.as_ref().ok_or(Fault::MissingState)?.actual;
        if a.resume_pc != r.site.resume_pc() || a.work_rsp != r.work_rsp || r.required_rbx.is_some_and(|word| word != a.rbx) { return Err(Fault::CallBinding); }
        self.captures.call_returned = true; self.captures.rax = Some(a.rax); self.phase = Phase::NormalRequired; Ok(())
    }
}
