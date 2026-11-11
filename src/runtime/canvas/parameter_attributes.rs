use super::{AttributesRequired, MembersWritten, FunctionCapture, CloneReturn};

#[path = "parameter_argument_retirement.rs"]
pub mod retirement;
#[path = "parameter_default_callback.rs"]
pub mod default_callback;
#[path = "parameter_format_precision.rs"]
pub mod formatting;

pub struct Input<A> { pub attributes: A, pub actual_rdi_word: u64 }
pub trait Services: super::Services {
    type Attributes;

    fn read_attribute_function(&mut self, attributes: &Self::Attributes, offset: u32,
        held: &mut Option<FunctionCapture<Self::Function>>) -> Result<(), Self::Error>;
    fn read_parameter_word(&mut self, parameter: &Self::Parameter, offset: u32) -> Result<u64, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next { Formatting2b3307, DefaultCallback2b360f, ArgumentRetirement2b326f }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    CaptureInline, ZeroMember, ReadSource, StoreFirstFrame, BranchSource,
    ReadVtable, StoreSecondFrame, ReadTarget, Clone, ReloadParameter,
    StoreReturned, ReadFirstMember, ReloadArgument, ReloadSecondInline,
    BranchOutputs, Done,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub inline: [Option<u64>; 2], pub zero_returned: [bool; 2],
    pub source_read_returned: [bool; 2], pub frame_store_returned: [bool; 2],
    pub vtables: [Option<u64>; 2], pub targets: [Option<u64>; 2],
    pub clone_returned: [bool; 2], pub parameter_reload_words: [Option<u64>; 2],
    pub selected_clones: [Option<super::selected_clone::History>; 2],
    pub member_store_returned: [bool; 2], pub argument_reload_words: [Option<u64>; 2],
    pub first_result_word: Option<u64>, pub second_result_word: Option<u64>,
    pub first_test_word: Option<u64>, pub selected_second_inline: Option<u64>,
    pub next: Option<Next>,
}
pub struct Retained<S: Services> {
    pub parent: AttributesRequired<S>, pub input: Input<S::Attributes>,
    pub sources: [Option<FunctionCapture<S::Function>>; 2],
    pub clones: [Option<CloneReturn<S::CloneOwner>>; 2], pub captures: Captures,
    pub precision: Option<formatting::Captures>,
    pub formatter: Option<formatting::object::History<S::Function, S::CloneOwner>>,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    pub(super) retained: Retained<S>, pub(super) index: usize, pub(super) phase: Phase,
    pub(super) attempted: Option<(usize, Phase)>, pub(super) stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(BranchSelected<S>), Failed(Stopped<S>) }
pub struct BranchSelected<S: Services> { retained: Retained<S> }
pub struct Parts<S: Services> {
    retained: Retained<S>, pub index: usize, pub phase: Phase,
    pub attempted: Option<(usize, Phase)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn retained(&self) -> &Retained<S> { &self.retained }
}
impl<S: Services> BranchSelected<S> {
    pub fn next(&self) -> Next { self.retained.captures.next.unwrap() }
    pub fn retained(&self) -> &Retained<S> { &self.retained }

    pub fn into_required_continuation(self) -> Retained<S> { self.retained }
}
impl<S: Services> Prepared<S> {

    pub fn from_members_written(written: MembersWritten<S>, input: Input<S::Attributes>) -> Self {
        Self { retained: Retained { parent: written.into_attributes_required(), input,
            sources: [None, None], clones: [None, None], captures: Captures::default(),
            precision: None, formatter: None },
            index: 0, phase: Phase::CaptureInline, attempted: None, stopped: false }
    }
    pub fn retained(&self) -> &Retained<S> { &self.retained }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<(usize, Phase)> { self.attempted }
    pub fn into_parts(self) -> Parts<S> {
        Parts { retained: self.retained, index: self.index, phase: self.phase,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.index, self.phase)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }
        if self.retained.captures.next.is_none() {
            self.stopped = true;
            return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        }
        Poll::Ready(BranchSelected { retained: self.retained })
    }
    fn begin_second(&mut self) { self.index = 1; self.phase = Phase::CaptureInline; }
    pub(super) fn record_clone_return(&mut self, word: u64) {
        let c = &mut self.retained.captures;
        c.clone_returned[self.index] = true;
        if self.index == 0 { c.first_result_word = Some(word); c.first_test_word = Some(word); }
        else { c.second_result_word = Some(word); }
        self.phase = Phase::ReloadParameter;
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let i = self.index;
        if i > 1 { return Err(Fault::MissingState); }
        let r = &mut self.retained;
        let p = &mut r.parent;
        let c = &mut r.captures;
        let member_offset = if i == 0 { 0x1a0 } else { 0x1e0 };
        match self.phase {
            Phase::CaptureInline => {
                let offset = if i == 0 { 0x168 } else { 0x1a8 };
                let word = p.services.inline_address(&p.parameter, offset).map_err(Fault::Service)?;
                c.inline[i] = Some(word);
                if i == 1 { c.selected_second_inline = Some(word); }
                self.phase = Phase::ZeroMember;
            }
            Phase::ZeroMember => {
                p.services.write_parameter_word(&mut p.parameter, member_offset, 0).map_err(Fault::Service)?;
                c.zero_returned[i] = true; self.phase = Phase::ReadSource;
            }
            Phase::ReadSource => {
                let offset = if i == 0 { 0x48 } else { 0x88 };
                p.services.read_attribute_function(&r.input.attributes, offset, &mut r.sources[i]).map_err(Fault::Service)?;
                c.source_read_returned[i] = true;
                self.phase = if i == 0 { Phase::StoreFirstFrame } else { Phase::BranchSource };
            }
            Phase::StoreFirstFrame => {
                let word = c.inline[0].ok_or(Fault::MissingState)?;
                p.services.write_frame_word(&mut p.frame, 0x38, word).map_err(Fault::Service)?;
                c.frame_store_returned[0] = true; self.phase = Phase::BranchSource;
            }
            Phase::BranchSource => {
                if r.sources[i].as_ref().is_some_and(|function| function.word != 0) {
                    self.phase = Phase::ReadVtable;
                } else if i == 0 {

                    c.first_result_word = Some(0); c.first_test_word = Some(0);
                    self.begin_second();
                } else {

                    c.second_result_word = Some(0);
                    self.phase = Phase::BranchOutputs;
                }
            }
            Phase::ReadVtable => {
                let function = r.sources[i].as_mut().ok_or(Fault::MissingState)?;
                c.vtables[i] = Some(p.services.read_clone_vtable(&mut function.owner).map_err(Fault::Service)?);
                self.phase = if i == 0 { Phase::ReadTarget } else { Phase::StoreSecondFrame };
            }
            Phase::StoreSecondFrame => {
                let word = c.inline[1].ok_or(Fault::MissingState)?;
                p.services.write_frame_word(&mut p.frame, 0x30, word).map_err(Fault::Service)?;
                c.frame_store_returned[1] = true; self.phase = Phase::ReadTarget;
            }
            Phase::ReadTarget => {
                c.targets[i] = Some(p.services.read_clone_target(c.vtables[i].ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::Clone;
            }
            Phase::Clone => {
                let function = r.sources[i].as_mut().ok_or(Fault::MissingState)?;
                p.services.clone_virtual0(if i == 0 { 0x2b31fd } else { 0x2b323e },
                    &mut function.owner, function.word, c.targets[i].ok_or(Fault::MissingState)?,
                    c.inline[i].ok_or(Fault::MissingState)?, &mut p.frame, &mut r.clones[i]).map_err(Fault::Service)?;
                let word = r.clones[i].as_ref().ok_or(Fault::MissingState)?.word;
                self.record_clone_return(word);
            }
            Phase::ReloadParameter => {
                p.services.reload_parameter40(&mut p.frame, &mut p.parameter, &mut c.parameter_reload_words[i]).map_err(Fault::Service)?;
                if c.parameter_reload_words[i].is_none() { return Err(Fault::MissingState); }
                self.phase = Phase::StoreReturned;
            }
            Phase::StoreReturned => {
                let word = r.clones[i].as_ref().ok_or(Fault::MissingState)?.word;
                p.services.write_parameter_word(&mut p.parameter, member_offset, word).map_err(Fault::Service)?;
                c.member_store_returned[i] = true;
                self.phase = if i == 0 { Phase::ReloadArgument } else { Phase::ReadFirstMember };
            }
            Phase::ReadFirstMember => {

                c.first_test_word = Some(p.services.read_parameter_word(&p.parameter, 0x1a0).map_err(Fault::Service)?);
                self.phase = Phase::ReloadArgument;
            }
            Phase::ReloadArgument => {
                p.services.reload_argument28(&mut p.frame, &mut p.argument, &mut c.argument_reload_words[i]).map_err(Fault::Service)?;
                if c.argument_reload_words[i].is_none() { return Err(Fault::MissingState); }
                if i == 0 { self.begin_second(); } else { self.phase = Phase::ReloadSecondInline; }
            }
            Phase::ReloadSecondInline => {
                c.selected_second_inline = Some(p.services.read_frame_word(&mut p.frame, 0x30).map_err(Fault::Service)?);
                self.phase = Phase::BranchOutputs;
            }
            Phase::BranchOutputs => {
                let first = c.first_test_word.ok_or(Fault::MissingState)?;
                let second = c.second_result_word.ok_or(Fault::MissingState)?;
                c.next = Some(if first == 0 { Next::Formatting2b3307 }
                    else if second == 0 { Next::DefaultCallback2b360f }
                    else { Next::ArgumentRetirement2b326f });
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
