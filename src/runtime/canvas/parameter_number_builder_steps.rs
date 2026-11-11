use super::{Arguments, CallRequest, CallSite, Fault, Phase, Prepared, Saved, Services, Target, WordWrite};
impl<S: Services> Prepared<S> {
    fn call(&mut self, site: CallSite, target: Target, args: Arguments) -> Result<u64, Fault<S::Error>> {

        if self.owners.calls[site.index()].is_some() { return Err(Fault::OccupiedCall { site }); }
        let request = CallRequest { site, target, args, work_rsp: self.actual.rbp.wrapping_sub(0x30),
            required_saved: self.saved.ok_or(Fault::MissingState)? };
        self.captures.requests[site.index()] = Some(request);
        self.services.call(&mut self.owners, request).map_err(Fault::Service)?;
        let actual = self.owners.calls[site.index()].as_ref().ok_or(Fault::MissingState)?.actual;
        if actual.resume_pc != site.resume_pc() || actual.work_rsp != request.work_rsp || actual.saved != request.required_saved {
            return Err(Fault::CallBinding { site });
        }
        self.captures.completed[site.index()] = true; Ok(actual.rax)
    }
    fn write(&mut self, pc: u32, address: u64, word: u64) -> Result<(), Fault<S::Error>> {
        self.captures.last_write = Some(WordWrite { pc, address, word });
        self.services.write_word(&mut self.owners, pc, address, word).map_err(Fault::Service)
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let base = self.actual.source_image_base; let frame = self.actual.rbp;
        match self.phase {
            Phase::Bind => {
                self.saved = Some(Saved { rbx: self.actual.object_rcx, rdi: self.actual.entry_rdi,
                    rsi: self.actual.source_rdx, rbp: frame }); self.phase = Phase::StoreBaseVtable;
            }
            Phase::StoreBaseVtable => {
                self.write(0x1fb442, self.actual.object_rcx, base.wrapping_add(0x616ad8))?;
                self.captures.initial_vtable_returned = true; self.phase = Phase::Allocate;
            }
            Phase::Allocate => {
                let word = self.call(CallSite::Allocate, Target::DirectRva(0x55d9a0), Arguments { rcx: Some(16), cl: None })?;
                self.captures.allocation = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.rdi = word;
                self.phase = Phase::SetClOne;
            }
            Phase::SetClOne => {
                let word = self.call(CallSite::SetClOne, Target::DirectRva(0x55e6b4), Arguments { rcx: None, cl: Some(1) })?;
                self.captures.cl_one_result = Some(word); self.phase = Phase::StoreFixed;
            }
            Phase::StoreFixed => {
                let saved = self.saved.ok_or(Fault::MissingState)?; let object = saved.rbx;
                let stores = [
                    (0x1fb459, saved.rdi.wrapping_add(8), self.captures.cl_one_result.ok_or(Fault::MissingState)?),
                    (0x1fb45d, object.wrapping_add(0x60), saved.rdi),
                    (0x1fb465, object.wrapping_add(0x18), object.wrapping_add(8)),
                    (0x1fb46d, object.wrapping_add(0x20), object.wrapping_add(0x10)),
                    (0x1fb475, object.wrapping_add(0x38), object.wrapping_add(0x28)),
                    (0x1fb47d, object.wrapping_add(0x40), object.wrapping_add(0x30)),
                    (0x1fb485, object.wrapping_add(0x50), object.wrapping_add(0x48)),
                    (0x1fb48d, object.wrapping_add(0x58), object.wrapping_add(0x4c)),
                    (0x1fb491, object.wrapping_add(0x48), 0),
                ];
                let (pc, address, word) = *stores.get(self.index).ok_or(Fault::MissingState)?;
                self.write(pc, address, word)?; self.captures.fixed_stores[self.index] = true;
                self.index += 1; if self.index == 9 { self.phase = Phase::ZeroXmm0; }
            }
            Phase::ZeroXmm0 => {
                self.services.zero_xmm0(&mut self.owners, 0x1fb499, &mut self.captures.xmm0).map_err(Fault::Service)?;
                if self.captures.xmm0.ok_or(Fault::MissingState)? != [0; 4] { return Err(Fault::XmmBinding); }
                self.phase = Phase::StoreZeroFirst;
            }
            Phase::StoreZeroFirst => {
                self.services.write_xmm0(&mut self.owners, 0x1fb49c, self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_add(8),
                    self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.zero_first = true; self.phase = Phase::StoreZeroSecond;
            }
            Phase::StoreZeroSecond => {
                self.services.write_xmm0(&mut self.owners, 0x1fb4a0, self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_add(0x28),
                    self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.zero_second = true; self.phase = Phase::StoreVtable;
            }
            Phase::StoreVtable => {
                self.write(0x1fb4ab, self.saved.ok_or(Fault::MissingState)?.rbx, base.wrapping_add(0x6169b8))?;
                self.captures.vtable_returned = true; self.phase = Phase::ReadGlobal;
            }
            Phase::ReadGlobal => {
                self.captures.global = Some(self.services.read_dword(&mut self.owners, 0x1fb4ae, base.wrapping_add(0x8636f8)).map_err(Fault::Service)?);
                self.phase = Phase::ReadIndex;
            }
            Phase::ReadIndex => {
                self.captures.index = Some(self.services.read_dword(&mut self.owners, 0x1fb4b4, base.wrapping_add(0x860284)).map_err(Fault::Service)?);
                self.phase = Phase::ReadGs58;
            }
            Phase::ReadGs58 => {
                self.captures.gs58 = Some(self.services.read_gs58(&mut self.owners, 0x1fb4ba).map_err(Fault::Service)?);
                self.phase = Phase::ReadThread;
            }
            Phase::ReadThread => {
                let address = self.captures.gs58.ok_or(Fault::MissingState)?.wrapping_add(u64::from(self.captures.index.ok_or(Fault::MissingState)?) * 8);
                self.captures.thread = Some(self.services.read_word(&mut self.owners, 0x1fb4c3, address).map_err(Fault::Service)?);
                self.phase = Phase::ReadThreadWord;
            }
            Phase::ReadThreadWord => {
                self.captures.thread_word = Some(self.services.read_dword(&mut self.owners, 0x1fb4c7,
                    self.captures.thread.ok_or(Fault::MissingState)?.wrapping_add(4)).map_err(Fault::Service)?);
                self.phase = Phase::Compare;
            }
            Phase::Compare => {
                self.phase = if (self.captures.global.ok_or(Fault::MissingState)? as i32) > (self.captures.thread_word.ok_or(Fault::MissingState)? as i32)
                    { Phase::Enter } else { Phase::ReadFirstTarget };
            }
            Phase::Enter => {
                self.call(CallSite::Enter, Target::DirectRva(0x55db58), Arguments { rcx: Some(base.wrapping_add(0x8636f8)), cl: None })?;
                self.phase = Phase::ReadCurrentGlobal;
            }
            Phase::ReadCurrentGlobal => {
                let word = self.services.read_dword(&mut self.owners, 0x1fb4ff, base.wrapping_add(0x8636f8)).map_err(Fault::Service)?;
                self.captures.current_global = Some(word);
                self.phase = if word == u32::MAX { Phase::SaveObject } else { Phase::ReadFirstTarget };
            }
            Phase::SaveObject => {
                self.write(0x1fb508, frame.wrapping_sub(8), self.saved.ok_or(Fault::MissingState)?.rbx)?;
                self.captures.object_saved = true; self.phase = Phase::GetObject;
            }
            Phase::GetObject => {
                self.captures.object_query_return = Some(self.call(CallSite::GetObject, Target::DirectRva(0x55e69c), Arguments { rcx: None, cl: None })?);
                self.phase = Phase::ReadReturnedObject;
            }
            Phase::ReadReturnedObject => {
                self.captures.returned_object = Some(self.services.read_word(&mut self.owners, 0x1fb511,
                    self.captures.object_query_return.ok_or(Fault::MissingState)?.wrapping_add(8)).map_err(Fault::Service)?);
                self.phase = Phase::StoreGlobalObject;
            }
            Phase::StoreGlobalObject => {
                self.write(0x1fb515, base.wrapping_add(0x8636f0), self.captures.returned_object.ok_or(Fault::MissingState)?)?;
                self.captures.global_object_stored = true; self.phase = Phase::ReadVtable;
            }
            Phase::ReadVtable => {
                self.captures.vtable = Some(self.services.read_word(&mut self.owners, 0x1fb51c,
                    self.captures.returned_object.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::ReadTarget;
            }
            Phase::ReadTarget => {
                self.captures.target = Some(self.services.read_word(&mut self.owners, 0x1fb51f,
                    self.captures.vtable.ok_or(Fault::MissingState)?.wrapping_add(8)).map_err(Fault::Service)?);
                self.phase = Phase::Virtual;
            }
            Phase::Virtual => {
                self.call(CallSite::Virtual, Target::LoadedIndirect(self.captures.target.ok_or(Fault::MissingState)?),
                    Arguments { rcx: Some(self.captures.returned_object.ok_or(Fault::MissingState)?), cl: None })?;
                self.phase = Phase::Register;
            }
            Phase::Register => {
                self.call(CallSite::Register, Target::DirectRva(0x55dd20), Arguments { rcx: Some(base.wrapping_add(0x1fbaa0)), cl: None })?;
                self.phase = Phase::Leave;
            }
            Phase::Leave => {
                self.call(CallSite::Leave, Target::DirectRva(0x55dbf0), Arguments { rcx: Some(base.wrapping_add(0x8636f8)), cl: None })?;
                self.phase = Phase::ReloadObject;
            }
            Phase::ReloadObject => {
                let word = self.services.read_word(&mut self.owners, 0x1fb53a, frame.wrapping_sub(8)).map_err(Fault::Service)?;
                self.captures.reloaded_object = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = word;
                self.phase = Phase::ReadFirstTarget;
            }
            Phase::ReadFirstTarget => {
                self.captures.first_target = Some(self.services.read_word(&mut self.owners, 0x1fb4cf,
                    self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_add(0x20)).map_err(Fault::Service)?);
                self.phase = Phase::StoreFirst;
            }
            Phase::StoreFirst => {
                self.write(0x1fb4d3, self.captures.first_target.ok_or(Fault::MissingState)?, self.saved.ok_or(Fault::MissingState)?.rsi)?;
                self.captures.first_stored = true; self.phase = Phase::ReadSecondTarget;
            }
            Phase::ReadSecondTarget => {
                self.captures.second_target = Some(self.services.read_word(&mut self.owners, 0x1fb4d6,
                    self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_add(0x40)).map_err(Fault::Service)?);
                self.phase = Phase::StoreSecond;
            }
            Phase::StoreSecond => {
                self.write(0x1fb4da, self.captures.second_target.ok_or(Fault::MissingState)?, self.saved.ok_or(Fault::MissingState)?.rsi)?;
                self.captures.second_stored = true; self.phase = Phase::ReadThirdTarget;
            }
            Phase::ReadThirdTarget => {
                self.captures.third_target = Some(self.services.read_word(&mut self.owners, 0x1fb4dd,
                    self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_add(0x58)).map_err(Fault::Service)?);
                self.phase = Phase::StoreThird;
            }
            Phase::StoreThird => {
                self.services.write_dword(&mut self.owners, 0x1fb4e1, self.captures.third_target.ok_or(Fault::MissingState)?, 0x30).map_err(Fault::Service)?;
                self.captures.third_stored = true; self.phase = Phase::NativeReturnRequired;
            }
            Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
