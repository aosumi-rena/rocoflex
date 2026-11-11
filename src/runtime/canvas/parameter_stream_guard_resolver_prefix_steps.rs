use super::{saved_word_source, Fault, Phase, Prepared, ProtectedRequirement, ReturnReason, Services};
impl<S: Services> Prepared<S> {
    pub(super) fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let a = self.input.actual; let c = &mut self.captures; let s = &mut self.input.services; let o = &mut self.input.storage;
        match self.phase {
            Phase::Bind => {

                let selector = a.ecx as u64; c.selector = Some(selector);
                c.cache_slot = Some(a.source_image_base.wrapping_add(0x861700).wrapping_add(selector.wrapping_mul(8)));
                c.cursor = Some(a.r8); self.phase = Phase::Encoded;
            }
            Phase::Encoded => { c.encoded = Some(s.read_word(o, 0x58ee37, c.cache_slot.ok_or(Fault::MissingState)?).map_err(Fault::Service)?); self.phase = Phase::Cookie; }
            Phase::Cookie => { c.cookie = Some(s.read_word(o, 0x58ee40, a.source_image_base.wrapping_add(0x820dd8)).map_err(Fault::Service)?); self.phase = Phase::Decode; }
            Phase::Decode => {

                c.decoded = Some(saved_word_source::decode_saved_word(c.cookie.ok_or(Fault::MissingState)?, c.encoded.ok_or(Fault::MissingState)?)); self.phase = Phase::CacheBranch;
            }
            Phase::CacheBranch => {
                let word = c.decoded.ok_or(Fault::MissingState)?;
                self.phase = if word == u64::MAX { c.reason = Some(ReturnReason::DecodedUnavailable); Phase::ZeroReturn }
                    else if word != 0 { Phase::CachedReturn } else { Phase::CompareRange };
            }
            Phase::CompareRange => {
                self.phase = if a.r8 == a.r9 { c.publication_cookie = c.cookie; Phase::PreparePublication } else { Phase::ReadIndex };
            }
            Phase::ReadIndex => {
                c.last_index = Some(s.read_dword(o, 0x58ee72, c.cursor.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);

                c.last_provider = None; c.last_name = None; self.phase = Phase::ReadProvider;
            }
            Phase::ReadProvider => {
                let index = c.last_index.ok_or(Fault::MissingState)? as u64;
                c.last_provider = Some(s.read_word(o, 0x58ee75, a.source_image_base.wrapping_add(0x861650).wrapping_add(index.wrapping_mul(8))).map_err(Fault::Service)?);
                self.phase = Phase::ProviderBranch;
            }
            Phase::ProviderBranch => {
                let word = c.last_provider.ok_or(Fault::MissingState)?;
                self.phase = if word == 0 { Phase::ReadName } else if word == u64::MAX { Phase::Advance } else { Phase::PrepareResolve };
            }
            Phase::Advance => {
                let cursor = c.cursor.ok_or(Fault::MissingState)?.wrapping_add(4); c.cursor = Some(cursor);
                c.completed_skips = c.completed_skips.wrapping_add(1);
                self.phase = if cursor == a.r9 { Phase::RefreshCookie } else { Phase::ReadIndex };
            }
            Phase::RefreshCookie => {
                let cookie = s.read_word(o, 0x58ef25, a.source_image_base.wrapping_add(0x820dd8)).map_err(Fault::Service)?;
                c.refreshed_cookie = Some(cookie); c.publication_cookie = Some(cookie); self.phase = Phase::PreparePublication;
            }
            Phase::ReadName => {
                let index = c.last_index.ok_or(Fault::MissingState)? as u64;
                c.last_name = Some(s.read_word(o, 0x58ee91, a.source_image_base.wrapping_add(0x606ea0).wrapping_add(index.wrapping_mul(8))).map_err(Fault::Service)?);
                self.phase = Phase::PrepareLoad;
            }
            Phase::PrepareLoad => {
                let name = c.last_name.ok_or(Fault::MissingState)?;
                c.protected = Some(ProtectedRequirement::LoadBeforePush { next_pc: 0x58eea4, call_pc: 0x58eea5, target_rva: 0xb9ee12,
                    work_rsp: a.work_rsp, rcx: name, edx: 0, r8d: 0x800, push_rax: a.entry_rax, saved_r14: name });
                self.phase = Phase::ProtectedRequired;
            }
            Phase::PrepareResolve => {
                c.protected = Some(ProtectedRequirement::ResolveBeforeArguments { next_pc: 0x58ef86, call_pc: 0x58ef8d, target_rva: 0xcc12e9,
                    work_rsp: a.work_rsp, saved_rbx: c.last_provider.ok_or(Fault::MissingState)?, saved_r13: a.rdx, saved_r14: a.source_image_base });
                self.phase = Phase::ProtectedRequired;
            }
            Phase::PreparePublication => {
                let cookie = c.publication_cookie.ok_or(Fault::MissingState)?;
                c.source_rotation_count = Some(64u32.wrapping_sub((cookie as u32) & 0x3f));

                c.encoded_sentinel = Some(super::encode_word(cookie, u64::MAX, c.source_rotation_count.ok_or(Fault::MissingState)?)); self.phase = Phase::Publish;
            }
            Phase::Publish => {
                let address = c.cache_slot.ok_or(Fault::MissingState)?; let word = c.encoded_sentinel.ok_or(Fault::MissingState)?;
                s.xchg_word(o, 0x58ef3f, address, word, &mut c.exchange).map_err(Fault::Service)?;
                let returned = c.exchange.ok_or(Fault::MissingState)?;
                if returned.address != address || returned.written != word { return Err(Fault::ExchangeBinding); }
                c.reason = Some(ReturnReason::Exhausted); self.phase = Phase::ZeroReturn;
            }
            Phase::ZeroReturn => { c.rax = Some(0); self.phase = Phase::NativeReturnRequired; }
            Phase::CachedReturn => { c.reason = Some(ReturnReason::CachedWord); c.rax = c.decoded; self.phase = Phase::NativeReturnRequired; }
            Phase::NativeReturnRequired | Phase::ProtectedRequired => {}
        }
        Ok(())
    }
}
