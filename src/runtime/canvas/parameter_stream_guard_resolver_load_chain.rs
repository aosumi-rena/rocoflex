use super as l;
use super::super as p;
use super::super::super as r;
pub trait Services: p::Services {

    fn retain_load_exit(&mut self, storage: &mut Self::Storage, history: l::History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error>;
}
pub enum Fault<E> { Service(E), ExitNotRetained, MissingExit, ConstructorBinding }
pub enum Rejected<S: Services> {

    Before { required: l::RequiredNext<S>, actual: p::ActualBoundary },

    After { pending: l::Parts<S>, actual: p::ActualBoundary, history: Option<l::History>, fault: Fault<S::Error> },
    Constructor { prefix: r::Parts<S>, actual: p::ActualBoundary, history: l::History, fault: Fault<S::Error> },
}
impl<S: Services> l::RequiredNext<S> {
    pub fn bind_actual_next_call(self, actual: p::ActualBoundary) -> Result<p::Prepared<S>, Rejected<S>> {
        let requirement = self.requirement();
        let (pc, rsp, live, target, target_rva, next) = match requirement {
            l::Requirement::ZeroLoadCall { pc, target_rva, target, rsp, registers } => (pc, rsp, registers, target, target_rva,
                r::ProtectedRequirement::LiveZeroLoadCall { next_pc: pc, call_pc: pc, target_rva, work_rsp: rsp, live: registers }),
            l::Requirement::PriorNonzeroBeforeSetup { next_pc, call_pc, target_rva, target, rsp, registers, push_rbp } =>
                (next_pc, rsp, registers, target, target_rva, r::ProtectedRequirement::LivePriorNonzeroBeforeSetup {
                    next_pc, call_pc, target_rva, work_rsp: rsp, live: registers, push_rbp }),
            l::Requirement::PriorZeroBeforeSetup { next_pc, call_pc, target_rva, target, rsp, registers } =>
                (next_pc, rsp, registers, target, target_rva, r::ProtectedRequirement::LiveResolveBeforeArguments {
                    next_pc, call_pc, target_rva, work_rsp: rsp, live: registers }),
        };
        let base = self.parts.protected.prefix.actual.source_image_base;
        if actual.pc != pc || actual.rsp != rsp || actual.registers != live || target != base.wrapping_add(u64::from(target_rva)) {
            return Err(Rejected::Before { required: self, actual });
        }
        let mut parts = self.into_parts();

        parts.stopped = true;
        let actual_exit = match parts.protected.exit.as_ref() {
            Some(exit) => exit.actual,
            None => return Err(Rejected::After { pending: parts, actual, history: None, fault: Fault::MissingExit }),
        };
        let history = l::History { actual: parts.actual, previous_requirement: parts.protected.requirement,
            previous_setup: parts.protected.captures, actual_exit, captures: parts.captures, next_actual: actual };
        let prefix = &mut parts.protected.prefix;
        if let Err(e) = prefix.services.retain_load_exit(&mut prefix.storage, history, &mut parts.protected.exit) {
            return Err(Rejected::After { pending: parts, actual, history: Some(history), fault: Fault::Service(e) });
        }
        if parts.protected.exit.is_some() { return Err(Rejected::After { pending: parts, actual, history: Some(history), fault: Fault::ExitNotRetained }); }
        let mut prefix = parts.protected.prefix;
        prefix.captures.load_publication = Some(history); prefix.captures.protected = Some(next);
        prefix.phase = r::Phase::ProtectedRequired; prefix.attempted = None; prefix.stopped = false;
        match p::Prepared::from_completed_prefix(r::ProtectedRequired { parts: prefix }, actual) {
            Ok(prepared) => Ok(prepared),
            Err(e) => { let mut prefix = e.completed.into_parts(); prefix.stopped = true;
                Err(Rejected::Constructor { prefix, actual, history, fault: Fault::ConstructorBinding }) },
        }
    }
}
