use super as p;
use super::super as r;
#[derive(Clone, Copy, Debug)]
pub struct History { pub previous_boundary: p::ActualBoundary,
    pub previous_requirement: r::ProtectedRequirement, pub previous_setup: p::Captures,
    pub actual_exit: p::ActualExit, pub next_actual: p::ActualBoundary }
pub trait Services: p::Services {

    fn retain_prior_cleanup_exit(&mut self, storage: &mut Self::Storage, history: History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error>;
}
pub enum Fault<E> { Service(E), ExitNotRetained, MissingExit, ConstructorBinding }
pub enum Rejected<S: Services> {

    Before { completed: p::ContinuationRequired<S>, actual: p::ActualBoundary },

    After { pending: p::Parts<S>, actual: p::ActualBoundary, history: Option<History>, fault: Fault<S::Error> },
    Constructor { prefix: r::Parts<S>, actual: p::ActualBoundary, history: History, fault: Fault<S::Error> },
}
impl<S: Services> p::ContinuationRequired<S> {
    pub fn bind_actual_resolve_after_prior_cleanup(self, actual: p::ActualBoundary)
        -> Result<p::Prepared<S>, Rejected<S>> {
        let prefix = &self.parts.prefix;
        let exit = self.actual_exit();

        if !matches!(self.parts.requirement, r::ProtectedRequirement::LivePriorNonzeroBeforeSetup { .. })
            || prefix.captures.prior_cleanup.is_some() || actual.pc != 0x58ef86
            || exit.pc != prefix.actual.source_image_base.wrapping_add(u64::from(actual.pc))
            || exit.rsp != actual.rsp || actual.rsp != prefix.actual.work_rsp
            || actual.registers != exit.registers {
            return Err(Rejected::Before { completed: self, actual });
        }
        let mut parts = self.into_parts();
        parts.stopped = true; parts.prefix.stopped = true;
        let actual_exit = match parts.exit.as_ref() {
            Some(exit) => exit.actual,
            None => return Err(Rejected::After { pending: parts, actual, history: None, fault: Fault::MissingExit }),
        };
        let history = History { previous_boundary: parts.actual, previous_requirement: parts.requirement,
            previous_setup: parts.captures, actual_exit, next_actual: actual };
        let prefix = &mut parts.prefix;
        if let Err(e) = prefix.services.retain_prior_cleanup_exit(&mut prefix.storage, history, &mut parts.exit) {
            return Err(Rejected::After { pending: parts, actual, history: Some(history), fault: Fault::Service(e) });
        }
        if parts.exit.is_some() {
            return Err(Rejected::After { pending: parts, actual, history: Some(history), fault: Fault::ExitNotRetained });
        }
        let mut prefix = parts.prefix;
        prefix.captures.prior_cleanup = Some(history);
        prefix.captures.protected = Some(r::ProtectedRequirement::LiveResolveBeforeArguments {
            next_pc: 0x58ef86, call_pc: 0x58ef8d, target_rva: 0xcc12e9,
            work_rsp: actual.rsp, live: actual.registers });
        prefix.phase = r::Phase::ProtectedRequired; prefix.attempted = None; prefix.stopped = false;
        match p::Prepared::from_completed_prefix(r::ProtectedRequired { parts: prefix }, actual) {
            Ok(prepared) => Ok(prepared),
            Err(e) => { let mut prefix = e.completed.into_parts(); prefix.stopped = true;
                Err(Rejected::Constructor { prefix, actual, history, fault: Fault::ConstructorBinding }) },
        }
    }
}
