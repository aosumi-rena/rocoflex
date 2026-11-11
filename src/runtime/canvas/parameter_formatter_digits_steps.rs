use super::{release, Fault, Owners, Phase, Prepared, Services};
struct CallRelease<'a, S: Services> { services: &'a mut S, owners: &'a mut Owners<S> }
impl<S: Services> release::SavedTextServices for CallRelease<'_, S> {
    type Error = S::Error;
    fn lock_xadd_minus_one(&mut self, header: u64, prior: &mut Option<u32>) -> Result<(), Self::Error> {
        self.services.xadd_minus_one(self.owners, 0x2b6e59, header, prior)
    }
    fn free_saved_header(&mut self, header: u64) -> Result<(), Self::Error> {
        self.services.call55da4c(self.owners, 0x2b6e61, header)
    }
}
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let s = &mut self.services; let o = &mut self.owners; let c = &mut self.captures;
        match self.phase {
            Phase::ReadLimit => { c.limit = Some(s.read_dword(o, 0x2b6d4a, self.actual.limit_r9).map_err(Fault::Service)?); self.phase = Phase::ReadValue; }
            Phase::ReadValue => { c.value_bits = Some(s.read_dword(o, 0x2b6d4d, self.actual.value_r8).map_err(Fault::Service)?); self.phase = Phase::ReadDigits; }
            Phase::ReadDigits => { c.digits = Some(s.read_dword(o, 0x2b6d52, self.actual.function_rcx.wrapping_add(8)).map_err(Fault::Service)?); self.phase = Phase::Widen; }
            Phase::Widen => { c.widened_bits = Some(s.widen_cvtss2sd(o, 0x2b6d56, c.value_bits.ok_or(Fault::MissingState)?).map_err(Fault::Service)?); self.phase = Phase::Format; }
            Phase::Format => {
                let request = super::format_request(self.actual, c).ok_or(Fault::MissingState)?;
                c.format_request = Some(request); s.format1cae80(o, request).map_err(Fault::Service)?;
                if o.temporary.is_none() { return Err(Fault::MissingState); }
                self.record_format_return();
            }
            Phase::BranchLimit => {
                self.remaining = c.limit.ok_or(Fault::MissingState)?;
                self.phase = if (self.remaining as i32) <= 0 { Phase::ReadWhole } else { Phase::ReadScanStart };
            }
            Phase::ReadScanStart => {
                let word = s.read_word(o, 0x2b6d6a, self.actual.rbp.wrapping_sub(0x10)).map_err(Fault::Service)?;
                c.scan_start = Some(word); self.cursor = word; self.phase = Phase::Scan;
            }
            Phase::Scan => {
                c.last_scan = Some(super::Scan { pointer: self.cursor, remaining: self.remaining, returned_byte: None });
                let byte = s.read_byte(o, 0x2b6d94, self.cursor).map_err(Fault::Service)?;
                if let Some(last) = c.last_scan.as_mut() { last.returned_byte = Some(byte); }
                if byte == 0 {
                    c.selected_text = c.scan_start; c.whole_store_pc = Some(0x2b6dd1); c.whole_retain_pc = Some(0x2b6de4);
                    self.phase = Phase::StoreWhole;
                } else {

                    let width = if byte & 0x80 == 0 || byte & 0x40 == 0 { 1 }
                        else if byte & 0x20 == 0 { 2 } else if byte & 0x10 == 0 { 3 } else { 4 };
                    self.cursor = self.cursor.wrapping_add(width); self.remaining = self.remaining.wrapping_sub(1);
                    if self.remaining == 0 { self.phase = Phase::CheckTruncated; }
                }
            }
            Phase::ReadWhole => {
                c.selected_text = Some(s.read_word(o, 0x2b6db5, self.actual.rbp.wrapping_sub(0x10)).map_err(Fault::Service)?);
                c.whole_store_pc = Some(0x2b6db9); c.whole_retain_pc = Some(0x2b6dcc); self.phase = Phase::StoreWhole;
            }
            Phase::StoreWhole => {
                s.write_word(o, c.whole_store_pc.ok_or(Fault::MissingState)?, self.actual.output_rdx,
                    c.selected_text.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.output_store_returned = true; self.phase = Phase::RetainWhole;
            }
            Phase::RetainWhole => {
                let header = c.selected_text.ok_or(Fault::MissingState)?.wrapping_sub(0x10);
                if header != self.actual.actual_image_base.wrapping_add(0x839cc8) {
                    s.lock_increment(o, c.whole_retain_pc.ok_or(Fault::MissingState)?, header, &mut c.retained).map_err(Fault::Service)?;
                    let actual = c.retained.ok_or(Fault::MissingState)?;
                    if actual.old.wrapping_add(1) != actual.new { return Err(Fault::IncrementBinding); }
                }
                self.phase = Phase::ReadCleanup;
            }
            Phase::CheckTruncated => {
                self.phase = if c.scan_start.ok_or(Fault::MissingState)? == 0 { Phase::StoreStatic } else { Phase::ReadFirst };
            }
            Phase::ReadFirst => {
                c.first_byte = Some(s.read_byte(o, 0x2b6dee, c.scan_start.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                if c.first_byte == Some(0) { self.phase = Phase::StoreStatic; }
                else {
                    let bytes = self.cursor.wrapping_sub(c.scan_start.ok_or(Fault::MissingState)?);
                    c.length = Some(bytes); c.rounded = Some(bytes & !3); self.phase = Phase::SaveCopySource;
                }
            }
            Phase::SaveCopySource => {
                c.allocation_bytes = Some(c.rounded.ok_or(Fault::MissingState)?.wrapping_add(0x1b));
                s.write_word(o, 0x2b6e01, self.actual.rbp.wrapping_sub(0x18), c.scan_start.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.copy_source_save_returned = true; self.phase = Phase::Allocate;
            }
            Phase::Allocate => {
                s.allocate55da80(o, 0x2b6e05, c.allocation_bytes.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.allocation_base = Some(o.allocation.as_ref().ok_or(Fault::MissingState)?.base);
                c.allocation_returned = true; self.phase = Phase::XchgReference;
            }
            Phase::XchgReference => {
                s.xchg_ref_zero(o, 0x2b6e0f, c.allocation_base.ok_or(Fault::MissingState)?, &mut c.allocation_prior).map_err(Fault::Service)?;
                if c.allocation_prior.is_none() { return Err(Fault::MissingState); }
                c.capacity = Some(c.rounded.ok_or(Fault::MissingState)?.wrapping_add(4)); self.phase = Phase::StoreCapacity;
            }
            Phase::StoreCapacity => {
                s.write_word(o, 0x2b6e15, c.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(8), c.capacity.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.capacity_store_returned = true; self.phase = Phase::StoreNewOutput;
            }
            Phase::StoreNewOutput => {
                s.write_word(o, 0x2b6e20, self.actual.output_rdx, c.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(0x10)).map_err(Fault::Service)?;
                c.output_store_returned = true; self.phase = Phase::ReloadCopySource;
            }
            Phase::ReloadCopySource => {
                c.copy_source = Some(s.read_word(o, 0x2b6e23, self.actual.rbp.wrapping_sub(0x18)).map_err(Fault::Service)?); self.phase = Phase::Copy;
            }
            Phase::Copy => {
                s.copy564f30(o, 0x2b6e2a, c.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(0x10),
                    c.copy_source.ok_or(Fault::MissingState)?, c.length.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.copy_returned = true; self.phase = Phase::Terminate;
            }
            Phase::Terminate => {
                s.write_byte(o, 0x2b6e2f, c.allocation_base.ok_or(Fault::MissingState)?.wrapping_add(c.length.ok_or(Fault::MissingState)?).wrapping_add(0x10), 0).map_err(Fault::Service)?;
                c.terminator_returned = true; self.phase = Phase::ReadCleanup;
            }
            Phase::StoreStatic => {
                s.write_word(o, 0x2b6e3d, self.actual.output_rdx, self.actual.actual_image_base.wrapping_add(0x839cd8)).map_err(Fault::Service)?;
                c.output_store_returned = true; self.phase = Phase::ReadCleanup;
            }
            Phase::ReadCleanup => {
                c.cleanup_text = Some(s.read_word(o, 0x2b6e40, self.actual.rbp.wrapping_sub(0x10)).map_err(Fault::Service)?); self.phase = Phase::Release;
            }
            Phase::Release => {
                c.cleanup = Some(release::retire_saved_text_borrowed(&mut CallRelease { services: s, owners: o },
                    c.cleanup_text.ok_or(Fault::MissingState)?, self.actual.actual_image_base).map_err(Fault::Release)?);
                self.phase = Phase::NativeReturnRequired;
            }
            Phase::NativeReturnRequired => {}
        }
        Ok(())
    }
}
