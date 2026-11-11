use super::{metadata::Action, Phase, Prepared, Resources, Stage};
use super::super::functions::{FunctionCapture, attributes::retirement};
use retirement::standalone as range;
#[path = "parameter_exception_range_native.rs"]
pub mod native;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualRangeBody {
    pub body_pc: u32, pub receiver_rcx: u64, pub saved_rsi: u64,
    pub original_caller_rsi: u64, pub work_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualRangeReturn {
    pub caller_resume_pc: u32, pub caller_rsp: u64, pub ret_pc: u32,
    pub before_ret_rsp: u64, pub after_ret_rsp: u64, pub restored_rsi: u64,
}

pub trait BindLoadedRange<S: super::Services, T> {
    fn bind(&mut self, services: &mut S, parameter: &mut S::Parameter, frame: &mut S::Frame,
        retained: &mut T, already_loaded_receiver: u64, held: &mut Option<S::Argument>) -> Result<(), S::Error>;
}
pub struct History {
    pub body: ActualRangeBody, pub captures: retirement::Captures, pub actual_return: ActualRangeReturn,
    pub native_entry: Option<native::EntryHistory>, pub native_return: Option<native::ReturnHistory>,
}

pub struct ReturnedRange<T, A, F> {
    parent: T, argument: A, functions: [Option<FunctionCapture<F>>; 3], history: History,
}
pub struct ReturnedRangeParts<T, A, F> {
    pub parent: T, pub argument: A, pub functions: [Option<FunctionCapture<F>>; 3], pub history: History,
}
impl<T, A, F> ReturnedRange<T, A, F> {
    pub fn parent(&self) -> &T { &self.parent }
    pub fn argument(&self) -> &A { &self.argument }
    pub fn history(&self) -> &History { &self.history }
    pub fn into_parts(self) -> ReturnedRangeParts<T, A, F> {
        ReturnedRangeParts { parent: self.parent, argument: self.argument, functions: self.functions, history: self.history }
    }
}
pub struct Suspended<P, F, T> {
    parameter: P, retained: T, stage: Stage<F>, previous: [Option<Stage<F>>; 3],
    previous_len: usize, action: usize, actual: ActualRangeBody,
    native_entry: Option<native::EntryHistory>,
}
pub struct SuspendedParts<P, F, T> {
    pub parameter: P, pub retained: T, pub stage: Stage<F>, pub previous: [Option<Stage<F>>; 3],
    pub previous_len: usize, pub action: usize, pub actual: ActualRangeBody,
    pub native_entry: Option<native::EntryHistory>,
}
impl<P, F, T> Suspended<P, F, T> {
    pub fn into_parts(self) -> SuspendedParts<P, F, T> {
        SuspendedParts { parameter: self.parameter, retained: self.retained, stage: self.stage, previous: self.previous,
            previous_len: self.previous_len, action: self.action, actual: self.actual, native_entry: self.native_entry }
    }
}
type ChildBag<S, T> = Suspended<<S as super::super::Services>::Parameter,
    <S as super::super::functions::Services>::Function, T>;
pub enum BindFault<E> { NotPendingRangeCall, MissingReceiver, BodyBinding, Admission(E), MissingArgument }
pub struct RejectedBind<S: super::Services, T> {
    pub pending: Prepared<S, T>, pub actual: ActualRangeBody,
    pub admitted: Option<S::Argument>, pub fault: BindFault<S::Error>,
    pub native_entry: Option<native::EntryHistory>,
}
pub struct Bound<S: super::Services, T> { child: range::Prepared<S, ChildBag<S, T>> }
pub enum Poll<S: super::Services, T> {
    Pending(Bound<S, T>), Ready(ReturnRequired<S, T>), Failed(range::Stopped<S, ChildBag<S, T>>),
}
pub struct ReturnRequired<S: super::Services, T> { child: range::NativeReturnRequired<S, ChildBag<S, T>> }
pub struct RejectedReturn<S: super::Services, T> {
    pub parts: range::Parts<S, ChildBag<S, T>>, pub actual: ActualRangeReturn,
    pub native_return: Option<native::ReturnHistory>,
}
impl<S: super::Services, T> Prepared<S, T> {

    pub fn bind_actual_range_body<B: BindLoadedRange<S, T>>(self, actual: ActualRangeBody, binder: &mut B)
        -> Result<Bound<S, T>, RejectedBind<S, T>> {
        self.bind_range_body(actual, binder, None)
    }

    fn bind_range_body<B: BindLoadedRange<S, T>>(mut self, actual: ActualRangeBody, binder: &mut B,
        native_entry: Option<native::EntryHistory>) -> Result<Bound<S, T>, RejectedBind<S, T>> {
        let already_stopped = self.stopped;
        self.stopped = true;
        let action = self.stage.profile.actions.get(self.action).copied().flatten();
        if (already_stopped && native_entry.is_none()) || self.phase != Phase::CallDirect
            || !matches!(action, Some(Action::Direct { target: 0x2b39d0, .. })) {
            return Err(RejectedBind { pending: self, actual, admitted: None, fault: BindFault::NotPendingRangeCall, native_entry });
        }
        let Some(receiver) = self.stage.captures.direct_receivers[self.action] else {
            return Err(RejectedBind { pending: self, actual, admitted: None, fault: BindFault::MissingReceiver, native_entry });
        };
        if actual.body_pc != 0x2b39d8 || actual.receiver_rcx != receiver || actual.saved_rsi != receiver
            || actual.work_rsp != self.stage.actual.funclet_work_rsp.wrapping_sub(0x30) {
            return Err(RejectedBind { pending: self, actual, admitted: None, fault: BindFault::BodyBinding, native_entry });
        }
        let mut admitted = None;
        if let Err(error) = binder.bind(&mut self.resources.services, &mut self.resources.parameter,
            &mut self.resources.frame, &mut self.resources.retained, receiver, &mut admitted) {
            return Err(RejectedBind { pending: self, actual, admitted, fault: BindFault::Admission(error), native_entry });
        }
        let Some(argument) = admitted else {
            return Err(RejectedBind { pending: self, actual, admitted: None, fault: BindFault::MissingArgument, native_entry });
        };
        let Resources { services, parameter, frame, retained } = self.resources;
        let suspended = Suspended { parameter, retained, stage: self.stage, previous: self.previous,
            previous_len: self.previous_len, action: self.action, actual, native_entry };
        let child = range::Prepared::from_actual_2b39d8(range::Input { services, argument, frame,
            actual_saved_rsi: actual.saved_rsi, retained: suspended });
        Ok(Bound { child })
    }
}
impl<S: super::Services, T> Bound<S, T> {
    pub fn into_parts(self) -> range::Parts<S, ChildBag<S, T>> { self.child.into_parts() }
    pub fn poll(self, budget: usize) -> Poll<S, T> {
        match self.child.poll(budget) {
            range::Poll::Pending(child) => Poll::Pending(Bound { child }),
            range::Poll::Ready(child) => Poll::Ready(ReturnRequired { child }),
            range::Poll::Failed(stopped) => Poll::Failed(stopped),
        }
    }
}
impl<S: super::Services, T> ReturnRequired<S, T> {
    pub fn required_epilogue_pc(&self) -> u32 { self.child.required_next_pc() }
    pub fn into_parts(self) -> range::Parts<S, ChildBag<S, T>> { self.child.into_parts() }

    pub fn after_actual_range_return(self, actual: ActualRangeReturn)
        -> Result<Prepared<S, ReturnedRange<T, S::Argument, S::Function>>, RejectedReturn<S, T>> {
        resume_after_range_return(self.child.into_parts(), actual, None)
    }
}
fn resume_after_range_return<S: super::Services, T>(parts: range::Parts<S, ChildBag<S, T>>,
    actual: ActualRangeReturn, native_return: Option<native::ReturnHistory>)
    -> Result<Prepared<S, ReturnedRange<T, S::Argument, S::Function>>, RejectedReturn<S, T>> {
    let parent = &parts.input.retained;
    let call_pc = match parent.stage.profile.actions[parent.action] {
        Some(Action::Direct { call_pc, target: 0x2b39d0, .. }) => call_pc,
        _ => return Err(RejectedReturn { parts, actual, native_return }),
    };
    if actual.caller_resume_pc != call_pc.wrapping_add(5)
        || actual.caller_rsp != parent.stage.actual.funclet_work_rsp || actual.ret_pc != 0x2b3a4f
        || actual.before_ret_rsp != parent.actual.work_rsp.wrapping_add(0x28)
        || actual.after_ret_rsp != parent.stage.actual.funclet_work_rsp
        || actual.restored_rsi != parent.actual.original_caller_rsi {
        return Err(RejectedReturn { parts, actual, native_return });
    }
    let range::Input { services, argument, frame, retained: mut parent, .. } = parts.input;
    parent.stage.captures.direct_call_returned[parent.action] = true;
    let retained = ReturnedRange { parent: parent.retained, argument, functions: parts.functions,
        history: History { body: parent.actual, captures: parts.captures, actual_return: actual,
            native_entry: parent.native_entry, native_return } };
    Ok(Prepared { resources: Resources { services, parameter: parent.parameter, frame, retained },
        stage: parent.stage, previous: parent.previous, previous_len: parent.previous_len,
        phase: Phase::BeginAction, action: parent.action + 1, attempted: None, stopped: false })
}
