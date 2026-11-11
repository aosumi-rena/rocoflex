use super::{ConstructorRequest, Fault, Phase, Prepared, Saved, Services};
impl<S: Services> Prepared<S> {
    pub(super) fn constructor_request(&self) -> Result<ConstructorRequest, Fault<S::Error>> {
        Ok(ConstructorRequest { call_pc: 0x1fb5ff, rcx: self.actual.rbp.wrapping_add(0x30), rdx: self.actual.builder_rcx, r8d: 0,
            work_rsp: self.actual.rbp.wrapping_sub(0x80), required_saved: self.saved.ok_or(Fault::MissingState)? })
    }
    pub(super) fn record_constructor_return(&mut self, request: ConstructorRequest) -> Result<(), Fault<S::Error>> {
        let returned = &self.owners.constructor.as_ref().ok_or(Fault::MissingState)?.actual;
        if returned.resume_pc != 0x1fb604 || returned.work_rsp != request.work_rsp || returned.saved != request.required_saved { return Err(Fault::ConstructorBinding); }
        self.captures.constructor_returned = true; self.phase = Phase::PrecisionBranch; Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let frame = self.actual.rbp; let base = self.actual.source_image_base;
        match self.phase {
            Phase::Bind => {
                let rbx = (self.actual.entry_rbx & !0xff) | u64::from(self.actual.flag_r9d as u8);
                self.captures.bound_rbx = Some(rbx);
                self.saved = Some(Saved { rbx, rsi: u64::from(self.actual.precision_r8d), rbp: frame, xmm6: self.actual.value_xmm1 });
                self.phase = Phase::StoreTable20;
            }
            Phase::StoreTable20 => {
                self.services.write_word(&mut self.owners, 0x1fb5c7, frame.wrapping_add(0x20), base.wrapping_add(0x616b68)).map_err(Fault::Service)?;
                self.captures.table20_stored = true; self.phase = Phase::Zero38;
            }
            Phase::Zero38 => {
                self.services.write_word(&mut self.owners, 0x1fb5cf, frame.wrapping_add(0x38), 0).map_err(Fault::Service)?;
                self.captures.zero38 = true; self.phase = Phase::ZeroXmm0;
            }
            Phase::ZeroXmm0 => {
                self.services.zero_xmm0(&mut self.owners, 0x1fb5d7, &mut self.captures.xmm0).map_err(Fault::Service)?;
                if self.captures.xmm0.ok_or(Fault::MissingState)? != [0; 4] { return Err(Fault::XmmBinding); }
                self.phase = Phase::Zero60;
            }
            Phase::Zero60 => {
                self.services.write_xmm0(&mut self.owners, 0x1fb5da, frame.wrapping_add(0x60), self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.zero60 = true; self.phase = Phase::Zero70;
            }
            Phase::Zero70 => {
                self.services.write_word(&mut self.owners, 0x1fb5de, frame.wrapping_add(0x70), 0).map_err(Fault::Service)?;
                self.captures.zero70 = true; self.phase = Phase::StoreTable30;
            }
            Phase::StoreTable30 => {
                self.services.write_word(&mut self.owners, 0x1fb5ed, frame.wrapping_add(0x30), base.wrapping_add(0x616b78)).map_err(Fault::Service)?;
                self.captures.table30_stored = true; self.phase = Phase::Zero2c;
            }
            Phase::Zero2c => {
                self.services.write_dword(&mut self.owners, 0x1fb5f1, frame.wrapping_add(0x2c), 0).map_err(Fault::Service)?;
                self.captures.zero2c = true; self.phase = Phase::StoreBuilder18;
            }
            Phase::StoreBuilder18 => {
                self.services.write_word(&mut self.owners, 0x1fb5f8, frame.wrapping_add(0x18), self.actual.builder_rcx).map_err(Fault::Service)?;
                self.captures.builder18_stored = true; self.phase = Phase::Construct;
            }
            Phase::Construct => {
                if self.owners.constructor.is_some() { return Err(Fault::OccupiedConstructor); }
                let request = self.constructor_request()?;
                self.captures.constructor_request = Some(request);
                self.services.construct_1fbcf0(&mut self.owners, request).map_err(Fault::Service)?;
                self.record_constructor_return(request)?;
            }
            Phase::PrecisionBranch => {
                self.phase = if (self.actual.precision_r8d as i32) <= 0 { Phase::SentryRequired } else { Phase::ReadFlagsTable };
            }
            Phase::ReadFlagsTable => {
                self.captures.flags_table = Some(self.services.read_word(&mut self.owners, 0x1fb608, frame.wrapping_add(0x20)).map_err(Fault::Service)?);
                self.phase = Phase::ReadFlagsOffset;
            }
            Phase::ReadFlagsOffset => {
                let word = self.services.read_dword(&mut self.owners, 0x1fb60c, self.captures.flags_table.ok_or(Fault::MissingState)?.wrapping_add(4)).map_err(Fault::Service)?;
                self.captures.flags_offset_word = Some(word); self.captures.flags_displacement = Some((word as i32 as i64) as u64);
                self.phase = Phase::PrepareMask;
            }
            Phase::PrepareMask => {
                let saved = self.saved.as_mut().ok_or(Fault::MissingState)?; saved.rbx ^= 1;
                self.captures.flag_mask = Some((u32::from(saved.rbx as u8) << 12).wrapping_add(0x1000));
                self.captures.flags_address = Some(frame.wrapping_add(self.captures.flags_displacement.ok_or(Fault::MissingState)?).wrapping_add(0x38));
                self.phase = Phase::OrFlags;
            }
            Phase::OrFlags => {
                let mask = self.captures.flag_mask.ok_or(Fault::MissingState)?;
                self.services.or_dword(&mut self.owners, 0x1fb61f, self.captures.flags_address.ok_or(Fault::MissingState)?, mask, &mut self.captures.or_returned).map_err(Fault::Service)?;
                let a = self.captures.or_returned.ok_or(Fault::MissingState)?;
                if a.after != (a.before | mask) { return Err(Fault::OrBinding); }
                self.phase = Phase::ReadValueTable;
            }
            Phase::ReadValueTable => {
                self.captures.value_table = Some(self.services.read_word(&mut self.owners, 0x1fb623, frame.wrapping_add(0x20)).map_err(Fault::Service)?);
                self.phase = Phase::ReadValueOffset;
            }
            Phase::ReadValueOffset => {
                let word = self.services.read_dword(&mut self.owners, 0x1fb627, self.captures.value_table.ok_or(Fault::MissingState)?.wrapping_add(4)).map_err(Fault::Service)?;
                self.captures.value_offset_word = Some(word); self.captures.value_displacement = Some((word as i32 as i64) as u64);
                self.captures.precision_address = Some(frame.wrapping_add((word as i32 as i64) as u64).wrapping_add(0x40));
                self.phase = Phase::StorePrecision;
            }
            Phase::StorePrecision => {
                self.services.write_word(&mut self.owners, 0x1fb62d, self.captures.precision_address.ok_or(Fault::MissingState)?, u64::from(self.actual.precision_r8d)).map_err(Fault::Service)?;
                self.captures.precision_stored = true; self.phase = Phase::SentryRequired;
            }
            Phase::SentryRequired => {}
        }
        Ok(())
    }
}
