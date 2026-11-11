use super::functions::FunctionCapture;
#[path = "parameter_exception_metadata.rs"]
pub mod metadata;
#[path = "parameter_exception_base_call.rs"]
pub mod base_call;
#[path = "parameter_exception_range_call.rs"]
pub mod range_call;
#[path = "parameter_exception_return.rs"]
pub mod native_return;
#[path = "parameter_exception_entry.rs"]
pub mod native_entry;
#[path = "parameter_exception_callable.rs"]
pub mod selected_callable;
use metadata::{Action, Profile};

pub trait Services: super::functions::attributes::formatting::object::Services {

    fn eh_direct_call(&mut self, pc: u32, target_rva: u32, receiver: u64,
        frame: &mut Self::Frame) -> Result<(), Self::Error>;
}
pub struct Resources<S: Services, T> {
    pub services: S, pub parameter: S::Parameter, pub frame: S::Frame,

    pub retained: T,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualBody {
    pub state: i32, pub selected_funclet: u32, pub body_pc: u32,
    pub actual_func_info_rva: u32, pub actual_handler_rva: u32,
    pub unwind_rdx: u64, pub source_rbp: u64, pub funclet_work_rsp: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionFault { PcOutsideFunction, NoLocalAction, StateMismatch, BodyBinding, NoNextLocalState, HistoryFull }
pub struct Rejected<S: Services, T> {
    pub resources: Resources<S, T>, pub normalized_pc: u32, pub actual: ActualBody, pub fault: AdmissionFault,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { BeginAction, InlineFirst, ReloadParameter, ReadMember, BranchMember, ReadInline,
    ReadVtable, ReadTarget, CallMember, ReloadAfterCall, ClearMember, ReadDirect, CallDirect, EpilogueRequired }
#[derive(Clone, Copy, Debug)]
pub struct Attempt { pub action: usize, pub phase: Phase, pub pc: u32 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub parameters: [Option<u64>; 2], pub member_read_returned: [bool; 2], pub inline: [Option<u64>; 2],
    pub delete_storage: [Option<bool>; 2], pub vtables: [Option<u64>; 2], pub targets: [Option<u64>; 2],
    pub member_call_returned: [bool; 2], pub after_parameters: [Option<u64>; 2], pub clear_returned: [bool; 2],
    pub direct_receivers: [Option<u64>; 2], pub direct_call_returned: [bool; 2],
}
pub struct Stage<F> {
    pub initial_normalized_pc: u32,
    pub actual: ActualBody, pub profile: Profile, pub captures: Captures,
    pub functions: [Option<FunctionCapture<F>>; 2], pub actual_return: Option<ActualReturn>,
    pub base_call: Option<base_call::CompletedBaseHistory>,
    pub native_return: Option<native_return::History>,
    pub native_entry: Option<native_entry::History>,
    pub selected_callable: [Option<selected_callable::History>; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualReturn {
    pub selected_funclet: u32, pub ret_pc: u32, pub work_rsp: u64,
    pub before_rsp: u64, pub after_rsp: u64, pub target: u64,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services, T> {
    resources: Resources<S, T>, stage: Stage<S::Function>, previous: [Option<Stage<S::Function>>; 3], previous_len: usize,
    phase: Phase, action: usize, attempted: Option<Attempt>, stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(BodyComplete<S, T>), Failed(Stopped<S, T>) }
pub struct BodyComplete<S: Services, T> { parts: Parts<S, T> }
pub struct Returned<S: Services, T> { parts: Parts<S, T> }
pub struct RejectedReturn<S: Services, T> { pub parts: Parts<S, T>, pub actual: ActualReturn }
pub struct RejectedNext<S: Services, T> { pub returned: Returned<S, T>, pub actual: ActualBody, pub fault: AdmissionFault }
pub struct Parts<S: Services, T> {

    resources: Resources<S, T>, pub stage: Stage<S::Function>,
    pub previous: [Option<Stage<S::Function>>; 3], pub previous_len: usize,
    pub phase: Phase, pub action: usize, pub attempted: Option<Attempt>, pub stopped: bool,
}
impl<S: Services, T> Parts<S, T> {
    pub fn resources(&self) -> &Resources<S, T> { &self.resources }
}
fn body_matches(actual: ActualBody, profile: Profile) -> bool {
    actual.state == profile.state && actual.selected_funclet == profile.funclet && actual.body_pc == profile.body_pc
        && actual.actual_func_info_rva == 0x7c8358 && actual.actual_handler_rva == 0x5641f4
        && actual.source_rbp == actual.unwind_rdx.wrapping_add(0x80)
}
impl<S: Services, T> BodyComplete<S, T> {
    pub fn required_epilogue_pc(&self) -> u32 { self.parts.stage.profile.epilogue_pc }
    pub fn into_parts(self) -> Parts<S, T> { self.parts }

    pub fn after_actual_return(mut self, actual: ActualReturn) -> Result<Returned<S, T>, RejectedReturn<S, T>> {
        self.parts.stage.actual_return = Some(actual);
        let stage = &self.parts.stage;
        if actual.selected_funclet != stage.profile.funclet || actual.ret_pc != stage.profile.ret_pc
            || actual.work_rsp != stage.actual.funclet_work_rsp
            || actual.before_rsp != actual.work_rsp.wrapping_add(0x78)
            || actual.after_rsp != actual.before_rsp.wrapping_add(8) {
            return Err(RejectedReturn { parts: self.parts, actual });
        }
        Ok(Returned { parts: self.parts })
    }
}
impl<S: Services, T> Returned<S, T> {
    pub fn next_required_state(&self) -> i32 { self.parts.stage.profile.next_state }
    pub fn into_parts(self) -> Parts<S, T> { self.parts }

    pub fn begin_actual_next(self, actual: ActualBody) -> Result<Prepared<S, T>, RejectedNext<S, T>> {
        let next = self.next_required_state();
        if next < 0 { return Err(RejectedNext { returned: self, actual, fault: AdmissionFault::NoNextLocalState }); }
        let profile = metadata::PROFILES[next as usize];
        if !body_matches(actual, profile) || actual.unwind_rdx != self.parts.stage.actual.unwind_rdx {
            return Err(RejectedNext { returned: self, actual, fault: AdmissionFault::BodyBinding });
        }
        if self.parts.previous_len >= 3 { return Err(RejectedNext { returned: self, actual, fault: AdmissionFault::HistoryFull }); }
        let initial_normalized_pc = self.parts.stage.initial_normalized_pc;
        let mut previous = self.parts.previous;
        previous[self.parts.previous_len] = Some(self.parts.stage);
        Ok(Prepared::at(self.parts.resources, initial_normalized_pc, actual, profile, previous, self.parts.previous_len + 1))
    }
}
impl<S: Services, T> Prepared<S, T> {
    pub fn from_actual_frame(resources: Resources<S, T>, normalized_pc: u32, actual: ActualBody) -> Result<Self, Rejected<S, T>> {
        let state = match metadata::state_at_normalized_pc(normalized_pc) {
            None => return Err(Rejected { resources, normalized_pc, actual, fault: AdmissionFault::PcOutsideFunction }),
            Some(-1) => return Err(Rejected { resources, normalized_pc, actual, fault: AdmissionFault::NoLocalAction }),
            Some(state) => state,
        };
        if actual.state != state { return Err(Rejected { resources, normalized_pc, actual, fault: AdmissionFault::StateMismatch }); }
        let profile = metadata::PROFILES[state as usize];
        if !body_matches(actual, profile) { return Err(Rejected { resources, normalized_pc, actual, fault: AdmissionFault::BodyBinding }); }
        Ok(Self::at(resources, normalized_pc, actual, profile, [None, None, None], 0))
    }
    fn at(resources: Resources<S, T>, initial_normalized_pc: u32, actual: ActualBody, profile: Profile,
        previous: [Option<Stage<S::Function>>; 3], previous_len: usize) -> Self {
        Self { resources, stage: Stage { initial_normalized_pc, actual, profile, captures: Captures::default(), functions: [None, None], actual_return: None, base_call: None, native_return: None, native_entry: None, selected_callable: [None, None] },
            previous, previous_len, phase: Phase::BeginAction, action: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn stage(&self) -> &Stage<S::Function> { &self.stage }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { resources: self.resources, stage: self.stage, previous: self.previous, previous_len: self.previous_len,
            phase: self.phase, action: self.action, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::EpilogueRequired && spent < budget {
            spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::EpilogueRequired { return Poll::Pending(self); }
        Poll::Ready(BodyComplete { parts: self.into_parts() })
    }
    fn advance(&mut self) { self.action += 1; self.phase = Phase::BeginAction; }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        if self.action >= 2 || self.stage.profile.actions[self.action].is_none() { self.phase = Phase::EpilogueRequired; return Ok(()); }
        let action = self.stage.profile.actions[self.action].ok_or(Fault::MissingState)?;
        let index = self.action;
        let r = &mut self.resources;
        let c = &mut self.stage.captures;
        let pc = match (action, self.phase) {
            (Action::Member(m), Phase::InlineFirst | Phase::ReadInline) => m.inline_pc,
            (Action::Member(m), Phase::ReloadParameter) => m.parameter_pc,
            (Action::Member(m), Phase::ReadMember | Phase::BranchMember) => m.member_pc,
            (Action::Member(m), Phase::ReadVtable) => m.vtable_pc,
            (Action::Member(m), Phase::ReadTarget | Phase::CallMember) => m.call_pc,
            (Action::Member(m), Phase::ReloadAfterCall) => m.reload_pc,
            (Action::Member(m), Phase::ClearMember) => m.clear_pc,
            (Action::Direct { read_pc, .. }, Phase::ReadDirect) => read_pc,
            (Action::Direct { call_pc, .. }, Phase::CallDirect) => call_pc,
            _ => self.stage.profile.body_pc,
        };
        self.attempted = Some(Attempt { action: index, phase: self.phase, pc });
        match action {
            Action::Member(m) => match self.phase {
                Phase::BeginAction => self.phase = if m.inline_first { Phase::InlineFirst } else { Phase::ReloadParameter },
                Phase::InlineFirst | Phase::ReadInline => {
                    c.inline[index] = Some(r.services.read_frame_word(&mut r.frame, m.inline_slot).map_err(Fault::Service)?);
                    if self.phase == Phase::InlineFirst { self.phase = Phase::ReloadParameter; }
                    else {
                        c.delete_storage[index] = Some(self.stage.functions[index].as_ref().ok_or(Fault::MissingState)?.word != c.inline[index].ok_or(Fault::MissingState)?);
                        self.phase = Phase::ReadVtable;
                    }
                }
                Phase::ReloadParameter => {
                    r.services.reload_parameter40(&mut r.frame, &mut r.parameter, &mut c.parameters[index]).map_err(Fault::Service)?;
                    if c.parameters[index].is_none() { return Err(Fault::MissingState); }
                    self.phase = Phase::ReadMember;
                }
                Phase::ReadMember => {
                    r.services.read_parameter_function(&r.parameter, m.offset, &mut self.stage.functions[index]).map_err(Fault::Service)?;
                    c.member_read_returned[index] = true; self.phase = Phase::BranchMember;
                }
                Phase::BranchMember => {
                    if !self.stage.functions[index].as_ref().is_some_and(|f| f.word != 0) { self.advance(); }
                    else if m.inline_first {
                        c.delete_storage[index] = Some(self.stage.functions[index].as_ref().ok_or(Fault::MissingState)?.word != c.inline[index].ok_or(Fault::MissingState)?);
                        self.phase = Phase::ReadVtable;
                    } else { self.phase = Phase::ReadInline; }
                }
                Phase::ReadVtable => {
                    let f = self.stage.functions[index].as_mut().ok_or(Fault::MissingState)?;
                    c.vtables[index] = Some(r.services.read_clone_vtable(&mut f.owner).map_err(Fault::Service)?);
                    self.phase = Phase::ReadTarget;
                }
                Phase::ReadTarget => {
                    c.targets[index] = Some(r.services.read_retirement_target20(c.vtables[index].ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                    self.phase = Phase::CallMember;
                }
                Phase::CallMember => {
                    let f = self.stage.functions[index].as_mut().ok_or(Fault::MissingState)?;
                    r.services.retire_virtual20(m.call_pc, &mut f.owner, f.word, c.targets[index].ok_or(Fault::MissingState)?,
                        c.delete_storage[index].ok_or(Fault::MissingState)?, &mut r.frame).map_err(Fault::Service)?;
                    c.member_call_returned[index] = true; self.phase = Phase::ReloadAfterCall;
                }
                Phase::ReloadAfterCall => {
                    r.services.reload_parameter40(&mut r.frame, &mut r.parameter, &mut c.after_parameters[index]).map_err(Fault::Service)?;
                    if c.after_parameters[index].is_none() { return Err(Fault::MissingState); }
                    self.phase = Phase::ClearMember;
                }
                Phase::ClearMember => {
                    r.services.write_parameter_word(&mut r.parameter, m.offset, 0).map_err(Fault::Service)?;
                    c.clear_returned[index] = true; self.advance();
                }
                _ => return Err(Fault::MissingState),
            },
            Action::Direct { frame_slot, call_pc, target, .. } => match self.phase {
                Phase::BeginAction => self.phase = Phase::ReadDirect,
                Phase::ReadDirect => {
                    c.direct_receivers[index] = Some(r.services.read_frame_word(&mut r.frame, frame_slot).map_err(Fault::Service)?);
                    self.phase = Phase::CallDirect;
                }
                Phase::CallDirect => {
                    r.services.eh_direct_call(call_pc, target, c.direct_receivers[index].ok_or(Fault::MissingState)?, &mut r.frame).map_err(Fault::Service)?;
                    c.direct_call_returned[index] = true; self.advance();
                }
                _ => return Err(Fault::MissingState),
            },
        }
        Ok(())
    }
}
