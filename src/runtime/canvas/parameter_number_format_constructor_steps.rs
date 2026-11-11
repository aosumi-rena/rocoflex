use super::{Arguments, CallSite as C, Fault, Phase as P, Prepared, Saved, Services, Target as T};
fn args(rcx: Option<u64>, rdx: Option<u64>, r8: Option<u64>, cl: Option<u8>, dl: Option<u8>) -> Arguments { Arguments { rcx, rdx, r8, cl, dl } }
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let object = self.actual.object_rcx; let frame = self.actual.rbp; let base = self.actual.source_image_base;
        match self.phase {
            P::Bind => {
                let rbx = (self.actual.entry_rbx & !0xff) | u64::from(self.actual.entry_r8 as u8);
                self.captures.bound_rbx = Some(rbx);
                self.saved = Some(Saved { rbx, rdi: self.actual.builder_rdx, rsi: object, r14: self.actual.entry_r14, rbp: frame }); self.phase = P::ZeroXmm0;
            }
            P::ZeroXmm0 => {
                self.services.zero_xmm0(&mut self.owners, 0x1fbd10, &mut self.captures.xmm0).map_err(Fault::Service)?;
                if self.captures.xmm0.ok_or(Fault::MissingState)? != [0; 4] { return Err(Fault::XmmBinding); } self.phase = P::InitialStores;
            }
            P::InitialStores => {
                match self.index {
                    0 => self.services.write_xmm0(&mut self.owners, 0x1fbd13, object.wrapping_add(8), self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?,
                    1 => self.services.write_dword(&mut self.owners, 0x1fbd17, object.wrapping_add(0x18), 0x201).map_err(Fault::Service)?,
                    2 => self.store_word(0x1fbd1e, object.wrapping_add(0x20), 6)?,
                    3 => self.services.write_xmm0(&mut self.owners, 0x1fbd26, object.wrapping_add(0x38), self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?,
                    4 => self.services.write_xmm0(&mut self.owners, 0x1fbd2a, object.wrapping_add(0x28), self.captures.xmm0.ok_or(Fault::MissingState)?).map_err(Fault::Service)?,
                    _ => return Err(Fault::MissingState),
                }
                self.captures.initial_stores[self.index] = true; self.index += 1; if self.index == 5 { self.phase = P::Allocate; }
            }
            P::Allocate => {
                let word = self.call(C::Allocate, T::DirectRva(0x55d9a0), args(Some(16), None, None, None, None))?;
                self.captures.allocation = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.r14 = word; self.phase = P::SetClOne;
            }
            P::SetClOne => { self.captures.context = Some(self.call(C::SetClOne, T::DirectRva(0x55e6b4), args(None, None, None, Some(1), None))?); self.phase = P::StoreAllocationLink; }
            P::StoreAllocationLink => {
                self.store_word(0x1fbd42, self.captures.allocation.ok_or(Fault::MissingState)?.wrapping_add(8), self.captures.context.ok_or(Fault::MissingState)?)?;
                self.captures.allocation_link_stored = true; self.phase = P::StoreAllocation;
            }
            P::StoreAllocation => { self.store_word(0x1fbd46, object.wrapping_add(0x40), self.captures.allocation.ok_or(Fault::MissingState)?)?; self.captures.allocation_stored = true; self.phase = P::StoreBuilder; }
            P::StoreBuilder => { self.store_word(0x1fbd4a, object.wrapping_add(0x48), self.saved.ok_or(Fault::MissingState)?.rdi)?; self.captures.builder_stored = true; self.phase = P::Zero50; }
            P::Zero50 => { self.store_word(0x1fbd4e, object.wrapping_add(0x50), 0)?; self.captures.zero50_stored = true; self.phase = P::SaveContext; }
            P::SaveContext => { self.store_word(0x1fbd56, frame.wrapping_sub(0x28), self.captures.context.ok_or(Fault::MissingState)?)?; self.captures.context_saved = true; self.phase = P::ReadAcquireVtable; }
            P::ReadAcquireVtable => { self.captures.acquire_vtable = Some(self.word(0x1fbd5a, self.captures.context.ok_or(Fault::MissingState)?)?); self.phase = P::ReadAcquireTarget; }
            P::ReadAcquireTarget => { self.captures.acquire_target = Some(self.word(0x1fbd60, self.captures.acquire_vtable.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = P::Acquire; }
            P::Acquire => {
                self.call(C::Acquire, T::LoadedIndirect(self.captures.acquire_target.ok_or(Fault::MissingState)?),
                    args(self.captures.context, self.captures.acquire_vtable, None, None, None))?; self.phase = P::GetObject;
            }
            P::GetObject => { let request = self.lookup_request()?; let word = self.call_request(request)?; self.record_lookup_value(word); }
            P::ReadTranslateVtable => { self.captures.translate_vtable = Some(self.word(0x1fbd6c, self.captures.get_object.ok_or(Fault::MissingState)?)?); self.phase = P::ReadTranslateTarget; }
            P::ReadTranslateTarget => { self.captures.translate_target = Some(self.word(0x1fbd74, self.captures.translate_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x40))?); self.phase = P::Translate; }
            P::Translate => {
                let word = self.call(C::Translate, T::LoadedIndirect(self.captures.translate_target.ok_or(Fault::MissingState)?),
                    args(self.captures.get_object, None, self.captures.translate_vtable, None, Some(0x20)))?;
                self.captures.translated_rax = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.rdi = u64::from(word as u32); self.phase = P::ReadCleanupContext;
            }
            P::ReadCleanupContext => {
                let word = self.word(0x1fbd7a, frame.wrapping_sub(0x28))?; self.captures.cleanup_context = Some(word);
                self.phase = if word == 0 { P::StoreByte58 } else { P::ReadQueryVtable };
            }
            P::ReadQueryVtable => {
                let word = self.word(0x1fbd83, self.captures.cleanup_context.ok_or(Fault::MissingState)?)?;
                self.captures.query_vtable = Some(word); self.captures.rax = Some(word); self.phase = P::ReadQueryTarget;
            }
            P::ReadQueryTarget => { self.captures.query_target = Some(self.word(0x1fbd86, self.captures.query_vtable.ok_or(Fault::MissingState)?.wrapping_add(0x10))?); self.phase = P::Query; }
            P::Query => {
                let word = self.call(C::Query, T::LoadedIndirect(self.captures.query_target.ok_or(Fault::MissingState)?), args(self.captures.cleanup_context, None, None, None, None))?;
                self.captures.query_result = Some(word); self.phase = if word == 0 { P::StoreByte58 } else { P::ReadDeleteVtable };
            }
            P::ReadDeleteVtable => { self.captures.delete_vtable = Some(self.word(0x1fbd8e, self.captures.query_result.ok_or(Fault::MissingState)?)?); self.phase = P::ReadDeleteTarget; }
            P::ReadDeleteTarget => { self.captures.delete_target = Some(self.word(0x1fbd99, self.captures.delete_vtable.ok_or(Fault::MissingState)?)?); self.phase = P::Delete; }
            P::Delete => {
                self.call(C::Delete, T::LoadedIndirect(self.captures.delete_target.ok_or(Fault::MissingState)?), args(self.captures.query_result, Some(1), self.captures.delete_vtable, None, None))?;
                self.phase = P::StoreByte58;
            }
            P::StoreByte58 => {
                self.services.write_byte(&mut self.owners, 0x1fbd9c, object.wrapping_add(0x58), self.saved.ok_or(Fault::MissingState)?.rdi as u8).map_err(Fault::Service)?;
                self.captures.byte58_stored = true; self.phase = P::ReadCurrentBuilder;
            }
            P::ReadCurrentBuilder => {
                let word = self.word(0x1fbda0, object.wrapping_add(0x48))?; self.captures.current_builder = Some(word);
                self.phase = if word == 0 { P::ReadStatus } else { P::OptionalBranch };
            }
            P::ReadStatus => { let word = self.dword(0x1fbda7, object.wrapping_add(0x10))?; self.captures.status = Some(word); self.captures.rax = Some(u64::from(word)); self.phase = P::ComposeStatus; }
            P::ComposeStatus => { let word = (self.captures.status.ok_or(Fault::MissingState)? & 0x13) | 4; self.captures.composed = Some(word); self.captures.rax = Some(u64::from(word)); self.phase = P::StoreStatus; }
            P::StoreStatus => {
                self.services.write_dword(&mut self.owners, 0x1fbdb0, object.wrapping_add(0x10), self.captures.composed.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                self.captures.status_stored = true; self.phase = P::ReadMask;
            }
            P::ReadMask => { self.captures.mask = Some(self.dword(0x1fbdb3, object.wrapping_add(0x14))?); self.phase = P::MaskStatus; }
            P::MaskStatus => {
                let word = self.captures.composed.ok_or(Fault::MissingState)? & self.captures.mask.ok_or(Fault::MissingState)?;
                self.captures.masked = Some(word); self.captures.rax = Some(u64::from(word)); self.phase = if word == 0 { P::OptionalBranch } else { P::SelectMessage };
            }
            P::OptionalBranch => { self.phase = if self.saved.ok_or(Fault::MissingState)?.rbx as u8 == 0 { P::NativeReturnRequired } else { P::Optional }; }
            P::Optional => { self.call(C::Optional, T::DirectRva(0x55e264), args(Some(object), None, None, None, None))?; self.phase = P::NativeReturnRequired; }
            P::SelectMessage => {
                let selected = if self.captures.masked.ok_or(Fault::MissingState)? & 2 == 0 { base.wrapping_add(0x774bd5) } else { base.wrapping_add(0x774bbf) };

                let message = if self.captures.mask.ok_or(Fault::MissingState)? & 4 == 0 { selected } else { base.wrapping_add(0x774bea) };
                self.captures.selected_rdx = Some(selected); self.captures.selected_message = Some(message); self.captures.rax = Some(base.wrapping_add(0x774bd5));
                let saved = self.saved.as_mut().ok_or(Fault::MissingState)?; saved.rsi = message; saved.rdi = frame.wrapping_sub(0x40); self.phase = P::ErrorConstruct;
            }
            P::ErrorConstruct => { self.call(C::ErrorConstruct, T::DirectRva(0x189a00), args(Some(frame.wrapping_sub(0x40)), Some(1), None, None, None))?; self.phase = P::BindErrorObject; }
            P::BindErrorObject => { self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = frame.wrapping_sub(0x30); self.phase = P::ErrorMessage; }
            P::ErrorMessage => { self.call(C::ErrorMessage, T::DirectRva(0x189a80), args(Some(frame.wrapping_sub(0x30)), self.captures.selected_message, Some(frame.wrapping_sub(0x40)), None, None))?; self.phase = P::Throw; }
            P::Throw => { self.call(C::Throw, T::DirectRva(0x563354), args(Some(frame.wrapping_sub(0x30)), Some(base.wrapping_add(0x7aa058)), None, None, None))?; self.phase = P::BreakpointRequired; }
            P::NativeReturnRequired | P::BreakpointRequired => {}
        }
        Ok(())
    }
}
