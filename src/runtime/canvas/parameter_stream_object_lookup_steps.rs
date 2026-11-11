use super::{CallSite as C, Fault, Phase as P, Prepared, Saved, Services, Target as T};
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let frame = self.actual.rbp; let base = self.actual.source_image_base;
        match self.phase {
            P::Bind => { self.saved = Some(Saved { rbx: self.actual.entry_rbx, rdi: self.actual.entry_rdi, rsi: self.actual.holder_rcx, r14: self.actual.entry_r14, rbp: frame }); self.phase = P::EnterOuter; }
            P::EnterOuter => { self.call_guard()?; }
            P::ReadCached => {
                let word = self.word(0x18c2fb, base.wrapping_add(0x863588))?; self.captures.cached = Some(word);
                self.saved.as_mut().ok_or(Fault::MissingState)?.rdi = word; self.phase = P::ReadIndex;
            }
            P::ReadIndex => {
                let word = self.word(0x18c302, base.wrapping_add(0x85f658))?; self.captures.initial_index = Some(word);
                self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = word; self.phase = if word == 0 { P::EnterInner } else { P::ReadContext };
            }
            P::EnterInner => { self.call_guard()?; }
            P::ReadCurrentIndex => {
                let word = self.word(0x18c319, base.wrapping_add(0x85f658))?; self.captures.current_index = Some(word);
                self.phase = if word == 0 { P::ReadCounter } else { P::LeaveInner };
            }
            P::ReadCounter => { self.captures.counter = Some(self.dword(0x18c323, base.wrapping_add(0x85f640))?); self.phase = P::IncrementCounter; }
            P::IncrementCounter => {

                let word = (self.captures.counter.ok_or(Fault::MissingState)? as i32 as i64) as u64;
                self.captures.next_counter = Some(word.wrapping_add(1)); self.phase = P::StoreCounter;
            }
            P::StoreCounter => {
                self.services.write_dword(&mut self.owners, 0x18c32d, base.wrapping_add(0x85f640), self.captures.next_counter.ok_or(Fault::MissingState)? as u32).map_err(Fault::Service)?;
                self.captures.counter_stored = true; self.phase = P::StoreIndex;
            }
            P::StoreIndex => { self.store(0x18c333, base.wrapping_add(0x85f658), self.captures.next_counter.ok_or(Fault::MissingState)?)?; self.captures.index_stored = true; self.phase = P::LeaveInner; }
            P::LeaveInner => { self.call_guard()?; }
            P::ReloadIndex => {
                let word = self.word(0x18c343, base.wrapping_add(0x85f658))?; self.captures.reloaded_index = Some(word);
                self.saved.as_mut().ok_or(Fault::MissingState)?.rbx = word; self.phase = P::ReadContext;
            }
            P::ReadContext => { self.captures.context = Some(self.word(0x18c34a, self.saved.ok_or(Fault::MissingState)?.rsi.wrapping_add(8))?); self.phase = P::ReadCount; }
            P::ReadCount => {
                let word = self.word(0x18c34e, self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(0x18))?; self.captures.count = Some(word);
                self.phase = if word <= self.saved.ok_or(Fault::MissingState)?.rbx { P::ReadFallbackFlag } else { P::ReadArray };
            }
            P::ReadArray => { self.captures.array = Some(self.word(0x18c354, self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(0x10))?); self.phase = P::ReadCandidate; }
            P::ReadCandidate => {
                let address = self.captures.array.ok_or(Fault::MissingState)?.wrapping_add(self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_mul(8));
                let word = self.word(0x18c358, address)?; self.captures.candidate = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.r14 = word;
                self.phase = if word == 0 { P::ReadFallbackFlag } else { P::LeaveOuter };
            }
            P::ReadFallbackFlag => {

                let byte = self.services.read_byte(&mut self.owners, 0x18c365, self.captures.context.ok_or(Fault::MissingState)?.wrapping_add(0x24)).map_err(Fault::Service)?;
                self.captures.fallback_flag = Some(byte); self.phase = if byte == 0 { P::UseCached } else { P::Fallback };
            }
            P::Fallback => { self.call_fallback()?; }
            P::ReadFallbackCount => {
                let word = self.word(0x18c370, self.captures.fallback_context.ok_or(Fault::MissingState)?.wrapping_add(0x18))?; self.captures.fallback_count = Some(word);
                self.phase = if word <= self.saved.ok_or(Fault::MissingState)?.rbx { P::UseCached } else { P::ReadFallbackArray };
            }
            P::ReadFallbackArray => { self.captures.fallback_array = Some(self.word(0x18c376, self.captures.fallback_context.ok_or(Fault::MissingState)?.wrapping_add(0x10))?); self.phase = P::ReadFallbackCandidate; }
            P::ReadFallbackCandidate => {
                let address = self.captures.fallback_array.ok_or(Fault::MissingState)?.wrapping_add(self.saved.ok_or(Fault::MissingState)?.rbx.wrapping_mul(8));
                let word = self.word(0x18c37a, address)?; self.captures.fallback_candidate = Some(word); self.saved.as_mut().ok_or(Fault::MissingState)?.r14 = word;
                self.phase = if word == 0 { P::UseCached } else { P::LeaveOuter };
            }
            P::UseCached => {
                let saved = self.saved.as_mut().ok_or(Fault::MissingState)?; saved.r14 = saved.rdi;
                self.phase = if saved.rdi == 0 { P::Allocate } else { P::LeaveOuter };
            }
            P::Allocate => { self.captures.allocation = Some(self.call(C::Allocate, T::DirectRva(0x55d9a0), Some(48), None)?); self.phase = P::ReadAllocationContext; }
            P::ReadAllocationContext => {
                self.captures.allocation_context = Some(self.word(0x18c399, self.saved.ok_or(Fault::MissingState)?.rsi.wrapping_add(8))?); self.phase = P::SaveAllocation;
            }
            P::SaveAllocation => {
                self.store(0x18c3a0, frame.wrapping_add(8), self.captures.allocation.ok_or(Fault::MissingState)?)?; self.captures.allocation_saved = true; self.phase = P::ChooseName;
            }
            P::ChooseName => {

                if self.captures.allocation_context.ok_or(Fault::MissingState)? == 0 {
                    self.captures.name_input = Some(base.wrapping_add(0x77a4df)); self.phase = P::ConstructTemporary;
                } else { self.phase = P::ReadNameOverride; }
            }
            P::ReadNameOverride => {
                let context = self.captures.allocation_context.ok_or(Fault::MissingState)?;
                let word = self.word(0x18c3a6, context.wrapping_add(0x28))?; self.captures.name_override = Some(word);
                self.captures.name_input = Some(if word == 0 { context.wrapping_add(0x30) } else { word }); self.phase = P::ConstructTemporary;
            }
            P::ConstructTemporary => { self.call(C::ConstructTemporary, T::DirectRva(0x18a7d0), Some(frame.wrapping_sub(0x60)), self.captures.name_input)?; self.phase = P::ReloadObject; }
            P::ReloadObject => {
                let word = self.word(0x18c3c7, frame.wrapping_add(8))?; self.captures.reloaded_object = Some(word);
                self.saved.as_mut().ok_or(Fault::MissingState)?.rsi = word; self.phase = P::ZeroObjectCount;
            }
            P::ZeroObjectCount => {
                self.services.write_dword(&mut self.owners, 0x18c3cb, self.saved.ok_or(Fault::MissingState)?.rsi.wrapping_add(8), 0).map_err(Fault::Service)?;
                self.captures.object_count_zeroed = true; self.phase = P::StoreVtable;
            }
            P::StoreVtable => {
                self.store(0x18c3d9, self.saved.ok_or(Fault::MissingState)?.rsi, base.wrapping_add(0x60e618))?; self.captures.vtable_stored = true; self.phase = P::ConstructMember;
            }
            P::ConstructMember => { self.call_member()?; }
            P::DestroyTemporary => { self.call(C::DestroyTemporary, T::DirectRva(0x18a930), Some(frame.wrapping_sub(0x60)), None)?; self.phase = P::Register; }
            P::Register => { self.call_registration()?; }
            P::ReloadPublishedObject => {
                let word = self.word(0x18c3f9, frame.wrapping_add(8))?; self.captures.published_object = Some(word);
                self.saved.as_mut().ok_or(Fault::MissingState)?.r14 = word; self.phase = P::ReadVtable;
            }
            P::ReadVtable => { self.captures.vtable = Some(self.word(0x18c3fd, self.saved.ok_or(Fault::MissingState)?.r14)?); self.phase = P::ReadAcquireTarget; }
            P::ReadAcquireTarget => { self.captures.acquire_target = Some(self.word(0x18c403, self.captures.vtable.ok_or(Fault::MissingState)?.wrapping_add(8))?); self.phase = P::AcquireVirtual; }
            P::AcquireVirtual => { self.call(C::AcquireVirtual, T::LoadedIndirect(self.captures.acquire_target.ok_or(Fault::MissingState)?), Some(self.saved.ok_or(Fault::MissingState)?.r14), None)?; self.phase = P::Publish; }
            P::Publish => { self.store(0x18c406, base.wrapping_add(0x863588), self.saved.ok_or(Fault::MissingState)?.r14)?; self.captures.published = true; self.phase = P::LeaveOuter; }
            P::LeaveOuter => { self.call_guard()?; }
            P::NativeReturnRequired => {}
        }
        Ok(())
    }
}
