use super::{Arguments, CallRequest, CallSite, Fault, Nonvolatile, Phase, Prepared, Services, Target};
impl<S: Services> Prepared<S> {
    fn call(&mut self, site: CallSite, target: Target, args: Arguments) -> Result<u64, Fault<S::Error>> {
        let request = CallRequest { site, target, args, work_rsp: self.actual.rbp.wrapping_sub(0x80),
            required_saved: self.saved.ok_or(Fault::MissingState)? };
        self.call_request(request)
    }
    fn call_request(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        self.record_call_return(request)
    }
    pub(super) fn record_call_return(&mut self, request: CallRequest) -> Result<u64, Fault<S::Error>> {
        let site = request.site;
        let returned = &self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if returned.resume_pc != site.resume_pc() || returned.work_rsp != request.work_rsp || returned.saved != request.required_saved {
            return Err(Fault::ReturnBinding { site });
        }
        self.captures.completed[site.index()] = true; Ok(returned.rax)
    }
    pub(super) fn copy_request(&mut self) -> Result<CallRequest, Fault<S::Error>> {
        let count = (self.captures.length.ok_or(Fault::MissingState)? as u32).wrapping_add(1);
        self.captures.copy_count_r8d = Some(count);
        Ok(CallRequest { site: CallSite::Copy, target: Target::DirectRva(0x1fb210),
            args: Arguments { rcx: self.actual.rbp.wrapping_sub(0x30), rdx: Some(self.actual.rbp.wrapping_sub(0x60)),
                r8: Some(u64::from(count)), r9: None, xmm1: None },
            work_rsp: self.actual.rbp.wrapping_sub(0x80), required_saved: self.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn builder_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        Ok(CallRequest { site: CallSite::Builder, target: Target::DirectRva(0x1fb420),
            args: Arguments { rcx: self.actual.rbp.wrapping_sub(0x30), rdx: Some(self.actual.rbp.wrapping_sub(0x60)), r8: None, r9: None, xmm1: None },
            work_rsp: self.actual.rbp.wrapping_sub(0x80), required_saved: self.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn format_request(&self) -> Result<CallRequest, Fault<S::Error>> {
        let saved = self.saved.ok_or(Fault::MissingState)?;
        Ok(CallRequest { site: CallSite::Format, target: Target::DirectRva(0x1fb590),
            args: Arguments { rcx: self.actual.rbp.wrapping_sub(0x30), rdx: None,
                r8: Some(u64::from(saved.rdi as u32)), r9: Some(u64::from(saved.rbx as u32)), xmm1: Some(saved.xmm6) },
            work_rsp: self.actual.rbp.wrapping_sub(0x80), required_saved: saved })
    }
    pub(super) fn record_format_value(&mut self, length: u64) -> Result<(), Fault<S::Error>> {
        self.captures.length = Some(length); self.saved.as_mut().ok_or(Fault::MissingState)?.rdi = length;
        self.phase = Phase::StoreBuilderVtable; Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let frame = self.actual.rbp;
        match self.phase {
            Phase::SaveBindings => {

                let rbx = (self.actual.entry_rbx & !0xff) | u64::from(self.actual.flag_r9d as u8);
                self.captures.bl_bound_rbx = Some(rbx);
                self.saved = Some(Nonvolatile { rbx, rbp: frame, rsi: self.actual.output_rcx,
                    rdi: u64::from(self.actual.precision_r8d), xmm6: self.actual.value_xmm1 });
                self.phase = Phase::Builder;
            }
            Phase::Builder => {
                let request = self.builder_request()?; self.call_request(request)?;
                self.phase = Phase::Format;
            }
            Phase::Format => {
                let request = self.format_request()?;
                let length = self.call_request(request)?; self.record_format_value(length)?;
            }
            Phase::StoreBuilderVtable => {
                self.services.write_word(&mut self.owners, 0x1caed4, frame.wrapping_sub(0x30),
                    self.actual.actual_image_base.wrapping_add(0x616ad8)).map_err(Fault::Service)?;
                self.captures.vtable_store_returned = true; self.phase = Phase::ReadCleanupPointer;
            }
            Phase::ReadCleanupPointer => {
                let word = self.services.read_word(&mut self.owners, 0x1caed8, frame.wrapping_add(0x30)).map_err(Fault::Service)?;
                self.captures.cleanup_pointer = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = word;
                self.phase = if word == 0 { Phase::PrepareAllocation } else { Phase::ReadAttached };
            }
            Phase::ReadAttached => {
                let word = self.services.read_word(&mut self.owners, 0x1caee1,
                    self.captures.cleanup_pointer.ok_or(Fault::MissingState)?.wrapping_add(8)).map_err(Fault::Service)?;
                self.captures.attached = Some(word); self.phase = if word == 0 { Phase::Free } else { Phase::ReadQueryVtable };
            }
            Phase::ReadQueryVtable => {
                self.captures.query_vtable = Some(self.services.read_word(&mut self.owners, 0x1caeea,
                    self.captures.attached.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::ReadQueryTarget;
            }
            Phase::ReadQueryTarget => {
                self.captures.query_target = Some(self.services.read_word(&mut self.owners, 0x1caeed,
                    self.captures.query_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x10)).map_err(Fault::Service)?);
                self.phase = Phase::Query;
            }
            Phase::Query => {
                let word = self.call(CallSite::Query, Target::LoadedIndirect(self.captures.query_target.ok_or(Fault::MissingState)?),
                    Arguments { rcx: self.captures.attached.ok_or(Fault::MissingState)?, rdx: None, r8: None, r9: None, xmm1: None })?;
                self.captures.query_result = Some(word); self.phase = if word == 0 { Phase::Free } else { Phase::ReadDeleteVtable };
            }
            Phase::ReadDeleteVtable => {
                self.captures.delete_vtable = Some(self.services.read_word(&mut self.owners, 0x1caef5,
                    self.captures.query_result.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::ReadDeleteTarget;
            }
            Phase::ReadDeleteTarget => {
                self.captures.delete_target = Some(self.services.read_word(&mut self.owners, 0x1caf00,
                    self.captures.delete_vtable.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::Delete;
            }
            Phase::Delete => {
                self.call(CallSite::Delete, Target::LoadedIndirect(self.captures.delete_target.ok_or(Fault::MissingState)?),
                    Arguments { rcx: self.captures.query_result.ok_or(Fault::MissingState)?, rdx: Some(1),
                        r8: Some(self.captures.delete_vtable.ok_or(Fault::MissingState)?), r9: None, xmm1: None })?;
                self.phase = Phase::Free;
            }
            Phase::Free => {
                self.call(CallSite::Free, Target::DirectRva(0x55da3c), Arguments { rcx: self.captures.cleanup_pointer.ok_or(Fault::MissingState)?,
                    rdx: None, r8: None, r9: None, xmm1: None })?;
                self.phase = Phase::PrepareAllocation;
            }
            Phase::PrepareAllocation => {
                let rounded = self.captures.length.ok_or(Fault::MissingState)? & !3;
                let capacity = rounded.wrapping_add(4); self.captures.rounded = Some(rounded); self.captures.capacity = Some(capacity);
                self.captures.allocation_bytes = Some(rounded.wrapping_add(0x1b));
                self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = capacity; self.phase = Phase::Allocate;
            }
            Phase::Allocate => {
                let base = self.call(CallSite::Allocate, Target::DirectRva(0x55da80), Arguments { rcx: self.captures.allocation_bytes.ok_or(Fault::MissingState)?,
                    rdx: None, r8: None, r9: None, xmm1: None })?;
                self.captures.allocation_base = Some(base); self.phase = Phase::XchgReference;
            }
            Phase::XchgReference => {
                self.services.xchg_ref_zero(&mut self.owners, 0x1caf21, self.captures.allocation_base.ok_or(Fault::MissingState)?,
                    &mut self.captures.prior_reference).map_err(Fault::Service)?;
                if self.captures.prior_reference.is_none() { return Err(Fault::MissingState); }
                self.phase = Phase::StoreCapacity;
            }
            Phase::StoreCapacity => {
                self.services.write_word(&mut self.owners, 0x1caf23, self.captures.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(8),
                    self.captures.capacity.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.capacity_store_returned = true;
                self.captures.data = Some(self.captures.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(0x10)); self.phase = Phase::StoreOutput;
            }
            Phase::StoreOutput => {
                self.services.write_word(&mut self.owners, 0x1caf2b, self.actual.output_rcx,
                    self.captures.data.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.output_store_returned = true; self.phase = Phase::StoreLocalData;
            }
            Phase::StoreLocalData => {
                self.services.write_word(&mut self.owners, 0x1caf2e, frame.wrapping_sub(0x30),
                    self.captures.data.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.local_store_returned = true; self.phase = Phase::Copy;
            }
            Phase::Copy => {
                let request = self.copy_request()?;
                self.call_request(request)?;
                self.phase = Phase::NativeReturnRequired;
            }
            Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
