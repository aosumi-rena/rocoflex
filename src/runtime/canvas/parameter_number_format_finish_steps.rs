use super::{Arguments, CallSite, Fault, Phase, Prepared, Services, Target};
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let actual = self.prior.prefix().actual; let frame = actual.rbp; let base = actual.source_image_base;
        match self.phase {
            Phase::ReadTable => { self.captures.table = Some(self.word(0x1fb752, frame.wrapping_add(0x20))?); self.phase = Phase::ReadOffset; }
            Phase::ReadOffset => {
                let word = self.dword(0x1fb756, self.captures.table.ok_or(Fault::MissingState)?.wrapping_add(4))?;
                self.captures.offset_word = Some(word); self.captures.displacement = Some((word as i32 as i64) as u64); self.phase = Phase::ReadStatus;
            }
            Phase::ReadStatus => { self.captures.status = Some(self.dword(0x1fb75a, self.address(0x30)?)?); self.phase = Phase::ReadField68; }
            Phase::ReadField68 => { self.captures.field68 = Some(self.word(0x1fb760, self.address(0x68)?)?); self.phase = Phase::ReadExtra; }
            Phase::ReadExtra => { self.captures.extra = Some(self.dword(0x1fb76f, frame.wrapping_add(0xa4))?); self.phase = Phase::Compose; }
            Phase::Compose => {
                let null_bits = if self.captures.field68.ok_or(Fault::MissingState)? == 0 { 4 } else { 0 };
                self.captures.composed = Some((self.captures.status.ok_or(Fault::MissingState)? & 0x17) | self.captures.extra.ok_or(Fault::MissingState)? | null_bits);
                self.phase = Phase::StoreStatus;
            }
            Phase::StoreStatus => { self.store_dword(0x1fb777, self.address(0x30)?, self.captures.composed.ok_or(Fault::MissingState)?)?; self.captures.status_stored = true; self.phase = Phase::ReadMask; }
            Phase::ReadMask => {
                let mask = self.dword(0x1fb77b, self.address(0x34)?)?; self.captures.mask = Some(mask);
                self.captures.masked = Some(self.captures.composed.ok_or(Fault::MissingState)? & mask); self.phase = Phase::Branch;
            }
            Phase::Branch => { self.phase = if self.captures.masked.ok_or(Fault::MissingState)? != 0 { Phase::SelectMessage } else { Phase::Query }; }
            Phase::Query => {
                let word = self.call(CallSite::Query, Target::DirectRva(0x560f68), Arguments { rcx: None, rdx: None, r8: None, r9: None })?;
                self.captures.query_rax = Some(word); self.phase = if word as u8 == 0 { Phase::ReadFlushStream } else { Phase::ReadCleanupStream };
            }
            Phase::ReadFlushStream => { self.captures.flush_stream = Some(self.word(0x1fb78a, frame.wrapping_add(0x90))?); self.phase = Phase::Flush; }
            Phase::Flush => {
                self.call(CallSite::Flush, Target::DirectRva(0x18a4b0), Arguments { rcx: Some(self.captures.flush_stream.ok_or(Fault::MissingState)?), rdx: None, r8: None, r9: None })?;
                self.phase = Phase::ReadCleanupStream;
            }
            Phase::ReadCleanupStream => { self.captures.cleanup_stream = Some(self.word(0x1fb796, frame.wrapping_add(0x90))?); self.phase = Phase::ReadCleanupVtable; }
            Phase::ReadCleanupVtable => { self.captures.cleanup_vtable = Some(self.word(0x1fb79d, self.captures.cleanup_stream.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadCleanupOffset; }
            Phase::ReadCleanupOffset => {
                let word = self.dword(0x1fb7a0, self.captures.cleanup_vtable.ok_or(Fault::MissingState)?.wrapping_add(4))?;
                self.captures.cleanup_offset_word = Some(word); self.captures.cleanup_displacement = Some((word as i32 as i64) as u64); self.phase = Phase::ReadCleanupObject;
            }
            Phase::ReadCleanupObject => {
                let address = self.captures.cleanup_stream.ok_or(Fault::MissingState)?.wrapping_add(self.captures.cleanup_displacement.ok_or(Fault::MissingState)?).wrapping_add(0x48);
                let word = self.word(0x1fb7a4, address)?; self.captures.cleanup_object = Some(word);
                self.phase = if word == 0 { Phase::StoreCleanupVtable } else { Phase::ReadCleanupObjectVtable };
            }
            Phase::ReadCleanupObjectVtable => { self.captures.cleanup_object_vtable = Some(self.word(0x1fb7ae, self.captures.cleanup_object.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadCleanupTarget; }
            Phase::ReadCleanupTarget => { self.captures.cleanup_target = Some(self.word(0x1fb7b1, self.captures.cleanup_object_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x10))?); self.phase = Phase::CleanupVirtual; }
            Phase::CleanupVirtual => {
                self.call(CallSite::CleanupVirtual, Target::LoadedIndirect(self.captures.cleanup_target.ok_or(Fault::MissingState)?), Arguments { rcx: Some(self.captures.cleanup_object.ok_or(Fault::MissingState)?), rdx: None, r8: None, r9: None })?;
                self.phase = Phase::StoreCleanupVtable;
            }
            Phase::StoreCleanupVtable => { self.store_word(0x1fb7bb, frame.wrapping_add(0x30), base.wrapping_add(0x616d60))?; self.captures.cleanup_vtable_stored = true; self.phase = Phase::Destroy; }
            Phase::Destroy => {
                self.call(CallSite::Destroy, Target::DirectRva(0x55e35c), Arguments { rcx: Some(frame.wrapping_add(0x30)), rdx: None, r8: None, r9: None })?;
                self.phase = Phase::ReadBuilder;
            }
            Phase::ReadBuilder => { self.captures.builder = Some(self.word(0x1fb7c8, frame.wrapping_add(0x18))?); self.phase = Phase::ReadFirstPointer; }
            Phase::ReadFirstPointer => { self.captures.first_pointer = Some(self.word(0x1fb7cc, self.captures.builder.ok_or(Fault::MissingState)?.wrapping_add(0x20))?); self.phase = Phase::ReadSecondPointer; }
            Phase::ReadSecondPointer => { self.captures.second_pointer = Some(self.word(0x1fb7d0, self.captures.builder.ok_or(Fault::MissingState)?.wrapping_add(0x40))?); self.phase = Phase::ReadSecondValue; }
            Phase::ReadSecondValue => { self.captures.second_value = Some(self.word(0x1fb7d4, self.captures.second_pointer.ok_or(Fault::MissingState)?)?); self.phase = Phase::ReadFirstValue; }
            Phase::ReadFirstValue => {
                let word = self.word(0x1fb7d7, self.captures.first_pointer.ok_or(Fault::MissingState)?)?; self.captures.first_value = Some(word);
                self.captures.return_rax = Some(self.captures.second_value.ok_or(Fault::MissingState)?.wrapping_sub(word)); self.phase = Phase::NativeRestoreRequired;
            }
            Phase::SelectMessage => {

                let word = self.captures.masked.ok_or(Fault::MissingState)?;
                let selected = if word & 2 == 0 { base.wrapping_add(0x774bd5) } else { base.wrapping_add(0x774bbf) };
                self.captures.selected_rdx = Some(selected);
                let message = if word & 4 == 0 { selected } else { base.wrapping_add(0x774bea) };
                self.captures.selected_message = Some(message); self.prior.prefix_mut().saved.as_mut().ok_or(Fault::MissingState)?.rsi = message;
                self.phase = Phase::ErrorConstruct;
            }
            Phase::ErrorConstruct => {
                self.call(CallSite::ErrorConstruct, Target::DirectRva(0x189a00), Arguments { rcx: Some(frame.wrapping_sub(0x50)), rdx: Some(1), r8: None, r9: None })?;
                self.phase = Phase::ReadErrorVector;
            }
            Phase::ReadErrorVector => { self.captures.error_vector = Some(self.vector(0x1fb81b, frame.wrapping_sub(0x50))?); self.phase = Phase::StoreErrorVector; }
            Phase::StoreErrorVector => { self.store_vector(0x1fb81f, frame.wrapping_sub(0x30), self.captures.error_vector.ok_or(Fault::MissingState)?)?; self.captures.error_vector_stored = true; self.phase = Phase::ErrorMessage; }
            Phase::ErrorMessage => {
                self.call(CallSite::ErrorMessage, Target::DirectRva(0x189e90), Arguments { rcx: Some(frame.wrapping_sub(0x20)), rdx: Some(frame.wrapping_sub(0x30)), r8: Some(self.captures.selected_message.ok_or(Fault::MissingState)?), r9: None })?;
                self.phase = Phase::StoreErrorVtable;
            }
            Phase::StoreErrorVtable => { self.store_word(0x1fb83a, frame.wrapping_sub(0x20), base.wrapping_add(0x60e518))?; self.captures.error_vtable_stored = true; self.phase = Phase::Throw; }
            Phase::Throw => {
                self.call(CallSite::Throw, Target::DirectRva(0x563354), Arguments { rcx: Some(frame.wrapping_sub(0x20)), rdx: Some(base.wrapping_add(0x7aa058)), r8: None, r9: None })?;
                self.phase = Phase::BreakpointRequired;
            }
            Phase::NativeRestoreRequired | Phase::BreakpointRequired => {}
        }
        Ok(())
    }
}
