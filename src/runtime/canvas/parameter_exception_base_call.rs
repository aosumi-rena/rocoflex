use super::{metadata::Action, Phase, Prepared, Resources, Stage};
use super::super::base_retirement as base;
#[path = "parameter_exception_base_entry.rs"]
pub mod native_entry;
#[path = "parameter_exception_base_tail.rs"]
pub mod native_tail;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualBaseBody {
    pub body_pc: u32, pub source_image_base: u64, pub receiver_rcx: u64,
    pub saved_rsi: u64, pub work_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualBaseReturn {
    pub caller_resume_pc: u32, pub caller_rsp: u64,
    pub actual_import_read_pc: u32, pub actual_addressed_slot: u64,
    pub actual_loaded_target: u64, pub actual_tail_target: u64,
    pub actual_tail_entry_rsp: u64, pub actual_tail_receiver_rcx: u64,
}
pub struct CompletedBaseHistory {
    pub body: ActualBaseBody, pub captures: base::Captures, pub actual_return: ActualBaseReturn,
    pub native_entry: Option<native_entry::History>, pub native_tail: Option<native_tail::History>,
}

pub struct Suspended<F, T> {
    retained: T, stage: Stage<F>, previous: [Option<Stage<F>>; 3], previous_len: usize,
    action: usize, actual: ActualBaseBody,
    native_entry: Option<native_entry::History>,
}
pub enum BindFault { NotPendingBaseCall, MissingReceiver, BodyBinding }
pub enum RejectedBind<S: super::Services + base::Services, T> {
    Before { pending: Prepared<S, T>, actual: ActualBaseBody, fault: BindFault, native_entry: Option<native_entry::History> },
    After { rejected: base::Rejected<S, Suspended<S::Function, T>>, actual: ActualBaseBody },
}
pub struct Bound<S: super::Services + base::Services, T> { child: base::Prepared<S, Suspended<S::Function, T>> }
pub enum Poll<S: super::Services + base::Services, T> {
    Pending(Bound<S, T>), Ready(TailRequired<S, T>), Failed(base::Stopped<S, Suspended<S::Function, T>>),
}
pub struct TailRequired<S: super::Services + base::Services, T> { child: base::NativeTailRequired<S, Suspended<S::Function, T>> }
pub struct RejectedReturn<S: super::Services + base::Services, T> {
    pub parts: base::Parts<S, Suspended<S::Function, T>>, pub actual: ActualBaseReturn,
    pub native_tail: Option<native_tail::History>,
}
pub struct SuspendedParts<F, T> {
    pub retained: T, pub stage: Stage<F>, pub previous: [Option<Stage<F>>; 3],
    pub previous_len: usize, pub action: usize, pub actual: ActualBaseBody,
    pub native_entry: Option<native_entry::History>,
}
impl<F, T> Suspended<F, T> {
    pub fn into_parts(self) -> SuspendedParts<F, T> {
        SuspendedParts { retained: self.retained, stage: self.stage, previous: self.previous,
            previous_len: self.previous_len, action: self.action, actual: self.actual, native_entry: self.native_entry }
    }
}
impl<S: super::Services + base::Services, T> Prepared<S, T> {

    pub fn bind_actual_base_body(self, actual: ActualBaseBody) -> Result<Bound<S, T>, RejectedBind<S, T>> {
        self.bind_base_body(actual, None)
    }

    fn bind_base_body(mut self, actual: ActualBaseBody, native_entry: Option<native_entry::History>) -> Result<Bound<S, T>, RejectedBind<S, T>> {

        let already_stopped = self.stopped;
        self.stopped = true;
        let action = self.stage.profile.actions.get(self.action).copied().flatten();
        if (already_stopped && native_entry.is_none()) || self.phase != Phase::CallDirect
            || !matches!(action, Some(Action::Direct { target: 0x2b3a50, .. })) {
            return Err(RejectedBind::Before { pending: self, actual, fault: BindFault::NotPendingBaseCall, native_entry });
        }
        let Some(receiver) = self.stage.captures.direct_receivers[self.action] else {
            return Err(RejectedBind::Before { pending: self, actual, fault: BindFault::MissingReceiver, native_entry });
        };

        if actual.body_pc != 0x2b3a5a || actual.receiver_rcx != receiver || actual.saved_rsi != receiver
            || actual.work_rsp != self.stage.actual.funclet_work_rsp.wrapping_sub(0x40) {
            return Err(RejectedBind::Before { pending: self, actual, fault: BindFault::BodyBinding, native_entry });
        }
        let Resources { services, parameter, frame, retained } = self.resources;
        let suspended = Suspended { retained, stage: self.stage, previous: self.previous,
            previous_len: self.previous_len, action: self.action, actual, native_entry };
        let input = base::Input { services, parameter, frame, retained: suspended,
            actual_image_base: actual.source_image_base, actual_saved_rsi: actual.saved_rsi,
            actual_initial_rcx: actual.receiver_rcx, actual_work_rsp: actual.work_rsp };
        match base::Prepared::from_actual_2b3a5a(input) {
            Ok(child) => Ok(Bound { child }),
            Err(rejected) => Err(RejectedBind::After { rejected, actual }),
        }
    }
}
impl<S: super::Services + base::Services, T> Bound<S, T> {
    pub fn into_parts(self) -> base::Parts<S, Suspended<S::Function, T>> { self.child.into_parts() }
    pub fn poll(self, budget: usize) -> Poll<S, T> {
        match self.child.poll(budget) {
            base::Poll::Pending(child) => Poll::Pending(Bound { child }),
            base::Poll::Ready(child) => Poll::Ready(TailRequired { child }),
            base::Poll::Failed(stopped) => Poll::Failed(stopped),
        }
    }
}
impl<S: super::Services + base::Services, T> TailRequired<S, T> {
    pub fn requirement(&self) -> base::TailRequirement { self.child.requirement() }
    pub fn into_parts(self) -> base::Parts<S, Suspended<S::Function, T>> { self.child.into_parts() }

    pub fn after_actual_base_return(self, actual: ActualBaseReturn) -> Result<Prepared<S, T>, RejectedReturn<S, T>> {
        let requirement = self.child.requirement();
        resume_after_base_return(self.child.into_parts(), requirement, actual, None)
    }
}
fn resume_after_base_return<S: super::Services + base::Services, T>(parts: base::Parts<S, Suspended<S::Function, T>>,
    requirement: base::TailRequirement, actual: ActualBaseReturn, native_tail: Option<native_tail::History>)
    -> Result<Prepared<S, T>, RejectedReturn<S, T>> {
    let parent = &parts.input.retained;
    let call_pc = match parent.stage.profile.actions[parent.action] {
        Some(Action::Direct { call_pc, target: 0x2b3a50, .. }) => call_pc,
        _ => return Err(RejectedReturn { parts, actual, native_tail }),
    };
    if actual.caller_resume_pc != call_pc.wrapping_add(5)
        || actual.caller_rsp != parent.stage.actual.funclet_work_rsp
        || actual.actual_import_read_pc != requirement.indirect_pc
        || actual.actual_addressed_slot != requirement.addressed_slot
        || actual.actual_loaded_target != actual.actual_tail_target
        || actual.actual_tail_entry_rsp != requirement.required_entry_rsp
        || actual.actual_tail_receiver_rcx != requirement.receiver_rcx {
        return Err(RejectedReturn { parts, actual, native_tail });
    }
    let base::Input { services, parameter, frame, retained: mut parent, .. } = parts.input;
    parent.stage.captures.direct_call_returned[parent.action] = true;
    parent.stage.base_call = Some(CompletedBaseHistory { body: parent.actual, captures: parts.captures, actual_return: actual,
        native_entry: parent.native_entry, native_tail });
    Ok(Prepared { resources: Resources { services, parameter, frame, retained: parent.retained },
        stage: parent.stage, previous: parent.previous, previous_len: parent.previous_len,
        phase: Phase::BeginAction, action: parent.action + 1, attempted: None, stopped: false })
}
