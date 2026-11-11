use super::{Captures as RangeCaptures, RangeWritten};

#[path = "parameter_attributes.rs"]
pub mod attributes;
#[path = "parameter_function_clone.rs"]
pub mod selected_clone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member { First, Second, Third }
impl Member {
    fn index(self) -> usize { match self { Self::First => 0, Self::Second => 1, Self::Third => 2 } }
    pub fn source_offset(self) -> u32 { match self { Self::First => 0x50, Self::Second => 0x90, Self::Third => 0xd0 } }
    pub fn inline_offset(self) -> u32 { match self { Self::First => 0xa0, Self::Second => 0xe0, Self::Third => 0x120 } }
    pub fn pointer_offset(self) -> u32 { match self { Self::First => 0xd8, Self::Second => 0x118, Self::Third => 0x158 } }
    pub fn frame_inline_slot(self) -> i32 { match self { Self::First => 0x38, Self::Second => 0x30, Self::Third => 0x20 } }
    pub fn clone_pc(self) -> u32 { match self { Self::First => 0x2b3135, Self::Second => 0x2b316f, Self::Third => 0x2b31a5 } }
}

pub struct FunctionCapture<F> { pub word: u64, pub owner: F }

pub struct CloneReturn<C> { pub word: u64, pub owner: C }

pub trait Services: super::Services {
    type Frame;
    type Function;
    type CloneOwner;

    fn inline_address(&mut self, parameter: &Self::Parameter, offset: u32) -> Result<u64, Self::Error>;
    fn write_frame_word(&mut self, frame: &mut Self::Frame, offset: i32, word: u64) -> Result<(), Self::Error>;
    fn read_frame_word(&mut self, frame: &mut Self::Frame, offset: i32) -> Result<u64, Self::Error>;

    fn write_parameter_word(&mut self, parameter: &mut Self::Parameter, offset: u32, word: u64)
        -> Result<(), Self::Error>;

    fn read_source_function(&mut self, argument: &Self::Argument, offset: u32,
        held: &mut Option<FunctionCapture<Self::Function>>) -> Result<(), Self::Error>;
    fn save_argument28(&mut self, frame: &mut Self::Frame, argument: &Self::Argument) -> Result<(), Self::Error>;
    fn save_parameter40(&mut self, frame: &mut Self::Frame, parameter: &Self::Parameter) -> Result<(), Self::Error>;

    fn read_clone_vtable(&mut self, function: &mut Self::Function) -> Result<u64, Self::Error>;
    fn read_clone_target(&mut self, saved_vtable: u64) -> Result<u64, Self::Error>;

    fn clone_virtual0(&mut self, pc: u32, function: &mut Self::Function, source_word: u64,
        target: u64, destination: u64, frame: &mut Self::Frame,
        held: &mut Option<CloneReturn<Self::CloneOwner>>) -> Result<(), Self::Error>;

    fn reload_parameter40(&mut self, frame: &mut Self::Frame, parameter: &mut Self::Parameter,
        actual_word: &mut Option<u64>) -> Result<(), Self::Error>;
    fn reload_argument28(&mut self, frame: &mut Self::Frame, argument: &mut Self::Argument,
        actual_word: &mut Option<u64>) -> Result<(), Self::Error>;

    fn read_default_d0(&mut self, frame: &mut Self::Frame) -> Result<u32, Self::Error>;
    fn write_parameter_f32(&mut self, parameter: &mut Self::Parameter, offset: u32, bits: u32)
        -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    CaptureInline, StoreInline, ZeroMember, ReadSource, SaveArgument, SaveParameter,
    BranchSource, ReadVtable, ReadInline, ReadTarget, Clone, ReloadParameter, StoreReturned,
    ReloadArgument, ReadDefault, CaptureRange, StoreRangeSlot, StorePublic,
    StoreDefault, AttributesRequired,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub inline_addresses: [Option<u64>; 3],
    pub inline_store_returned: [bool; 3],
    pub member_zero_returned: [bool; 3],
    pub source_read_returned: [bool; 3],
    pub argument_save_returned: bool,
    pub parameter_save_returned: bool,
    pub vtables: [Option<u64>; 3],
    pub targets: [Option<u64>; 3],
    pub clone_destinations: [Option<u64>; 3],
    pub clone_returned: [bool; 3],
    pub selected_clones: [Option<selected_clone::History>; 3],
    pub parameter_reload_words: [Option<u64>; 3],
    pub member_store_returned: [bool; 3],
    pub argument_reload_words: [Option<u64>; 3],
    pub default_bits: Option<u32>,
    pub range_address: Option<u64>,
    pub range_slot_returned: bool,
    pub public_store_returned: bool,
    pub default_store_returned: bool,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S,
    parameter: S::Parameter,
    argument: S::Argument,
    frame: S::Frame,
    range_copy: RangeCaptures,
    sources: [Option<FunctionCapture<S::Function>>; 3],
    clones: [Option<CloneReturn<S::CloneOwner>>; 3],
    captures: Captures,
    member: Member,
    phase: Phase,
    attempted: Option<(Member, Phase)>,
    stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(MembersWritten<S>), Failed(Stopped<S>) }
pub struct MembersWritten<S: Services> { returned: AttributesRequired<S> }

pub struct AttributesRequired<S: Services> {
    pub services: S, pub parameter: S::Parameter, pub argument: S::Argument,
    pub frame: S::Frame, pub range_copy: RangeCaptures,
    pub sources: [Option<FunctionCapture<S::Function>>; 3],
    pub clones: [Option<CloneReturn<S::CloneOwner>>; 3],
    pub captures: Captures,
}

pub struct Parts<S: Services> {
    pub(super) services: S, pub(super) parameter: S::Parameter, pub(super) argument: S::Argument,
    pub(super) frame: S::Frame, pub range_copy: RangeCaptures,
    pub sources: [Option<FunctionCapture<S::Function>>; 3],
    pub clones: [Option<CloneReturn<S::CloneOwner>>; 3],
    pub captures: Captures, pub member: Member, pub phase: Phase,
    pub attempted: Option<(Member, Phase)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn parameter(&self) -> &S::Parameter { &self.parameter }
    pub fn argument(&self) -> &S::Argument { &self.argument }
    pub fn frame(&self) -> &S::Frame { &self.frame }
}
impl<S: Services> MembersWritten<S> {
    pub fn into_attributes_required(self) -> AttributesRequired<S> { self.returned }
}
impl<S: Services> Prepared<S> {

    pub fn from_range_written(written: RangeWritten<S>, frame: S::Frame) -> Self {
        let returned = written.into_function_members_required();
        Self { services: returned.services, parameter: returned.parameter, argument: returned.argument,
            frame, range_copy: returned.captures, sources: [None, None, None], clones: [None, None, None],
            captures: Captures::default(), member: Member::First, phase: Phase::CaptureInline,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn member(&self) -> Member { self.member }
    pub fn attempted(&self) -> Option<(Member, Phase)> { self.attempted }
    pub fn services(&self) -> &S { &self.services }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, parameter: self.parameter, argument: self.argument,
            frame: self.frame, range_copy: self.range_copy, sources: self.sources, clones: self.clones,
            captures: self.captures, member: self.member, phase: self.phase,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::AttributesRequired && spent < budget {
            self.attempted = Some((self.member, self.phase)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::AttributesRequired { return Poll::Pending(self); }
        Poll::Ready(MembersWritten { returned: AttributesRequired { services: self.services,
            parameter: self.parameter, argument: self.argument, frame: self.frame,
            range_copy: self.range_copy, sources: self.sources, clones: self.clones, captures: self.captures } })
    }
    fn next_member(&mut self) {
        match self.member {
            Member::First => { self.member = Member::Second; self.phase = Phase::CaptureInline; }
            Member::Second => { self.member = Member::Third; self.phase = Phase::ZeroMember; }
            Member::Third => self.phase = Phase::ReadDefault,
        }
    }
    fn record_clone_return(&mut self) {
        self.captures.clone_returned[self.member.index()] = true;
        self.phase = Phase::ReloadParameter;
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let index = self.member.index();
        match self.phase {
            Phase::CaptureInline => {
                let word = self.services.inline_address(&self.parameter, self.member.inline_offset()).map_err(Fault::Service)?;
                self.captures.inline_addresses[index] = Some(word);
                self.phase = if self.member == Member::Third { Phase::ReadVtable } else { Phase::StoreInline };
            }
            Phase::StoreInline => {
                let word = self.captures.inline_addresses[index].ok_or(Fault::MissingState)?;
                self.services.write_frame_word(&mut self.frame, self.member.frame_inline_slot(), word).map_err(Fault::Service)?;
                self.captures.inline_store_returned[index] = true;
                self.phase = if self.member == Member::Third { Phase::ReadTarget } else { Phase::ZeroMember };
            }
            Phase::ZeroMember => {
                self.services.write_parameter_word(&mut self.parameter, self.member.pointer_offset(), 0).map_err(Fault::Service)?;
                self.captures.member_zero_returned[index] = true; self.phase = Phase::ReadSource;
            }
            Phase::ReadSource => {
                self.services.read_source_function(&self.argument, self.member.source_offset(), &mut self.sources[index]).map_err(Fault::Service)?;
                self.captures.source_read_returned[index] = true;
                self.phase = if self.member == Member::First { Phase::SaveArgument } else { Phase::BranchSource };
            }
            Phase::SaveArgument => {
                self.services.save_argument28(&mut self.frame, &self.argument).map_err(Fault::Service)?;
                self.captures.argument_save_returned = true; self.phase = Phase::SaveParameter;
            }
            Phase::SaveParameter => {
                self.services.save_parameter40(&mut self.frame, &self.parameter).map_err(Fault::Service)?;
                self.captures.parameter_save_returned = true; self.phase = Phase::BranchSource;
            }
            Phase::BranchSource => {
                if self.sources[index].as_ref().is_some_and(|function| function.word != 0) {
                    self.phase = if self.member == Member::Third { Phase::CaptureInline } else { Phase::ReadVtable };
                } else { self.next_member(); }
            }
            Phase::ReadVtable => {
                let function = self.sources[index].as_mut().ok_or(Fault::MissingState)?;
                self.captures.vtables[index] = Some(self.services.read_clone_vtable(&mut function.owner).map_err(Fault::Service)?);
                self.phase = if self.member == Member::Third { Phase::StoreInline } else { Phase::ReadInline };
            }
            Phase::ReadInline => {
                let word = self.services.read_frame_word(&mut self.frame, self.member.frame_inline_slot()).map_err(Fault::Service)?;
                self.captures.clone_destinations[index] = Some(word); self.phase = Phase::ReadTarget;
            }
            Phase::ReadTarget => {
                let vtable = self.captures.vtables[index].ok_or(Fault::MissingState)?;
                self.captures.targets[index] = Some(self.services.read_clone_target(vtable).map_err(Fault::Service)?);
                self.phase = Phase::Clone;
            }
            Phase::Clone => {

                let destination = if self.member == Member::Third {
                    self.captures.inline_addresses[index].ok_or(Fault::MissingState)?
                } else { self.captures.clone_destinations[index].ok_or(Fault::MissingState)? };
                self.captures.clone_destinations[index] = Some(destination);
                let function = self.sources[index].as_mut().ok_or(Fault::MissingState)?;
                self.services.clone_virtual0(self.member.clone_pc(), &mut function.owner, function.word,
                    self.captures.targets[index].ok_or(Fault::MissingState)?, destination,
                    &mut self.frame, &mut self.clones[index]).map_err(Fault::Service)?;
                if self.clones[index].is_none() { return Err(Fault::MissingState); }
                self.record_clone_return();
            }
            Phase::ReloadParameter => {
                self.services.reload_parameter40(&mut self.frame, &mut self.parameter,
                    &mut self.captures.parameter_reload_words[index]).map_err(Fault::Service)?;
                if self.captures.parameter_reload_words[index].is_none() { return Err(Fault::MissingState); }
                self.phase = Phase::StoreReturned;
            }
            Phase::StoreReturned => {
                let word = self.clones[index].as_ref().ok_or(Fault::MissingState)?.word;
                self.services.write_parameter_word(&mut self.parameter, self.member.pointer_offset(), word).map_err(Fault::Service)?;
                self.captures.member_store_returned[index] = true; self.phase = Phase::ReloadArgument;
            }
            Phase::ReloadArgument => {
                self.services.reload_argument28(&mut self.frame, &mut self.argument,
                    &mut self.captures.argument_reload_words[index]).map_err(Fault::Service)?;
                if self.captures.argument_reload_words[index].is_none() { return Err(Fault::MissingState); }
                self.next_member();
            }
            Phase::ReadDefault => {
                self.captures.default_bits = Some(self.services.read_default_d0(&mut self.frame).map_err(Fault::Service)?);
                self.phase = Phase::CaptureRange;
            }
            Phase::CaptureRange => {
                self.captures.range_address = Some(self.services.inline_address(&self.parameter, 0x88).map_err(Fault::Service)?);
                self.phase = Phase::StoreRangeSlot;
            }
            Phase::StoreRangeSlot => {
                let word = self.captures.range_address.ok_or(Fault::MissingState)?;
                self.services.write_frame_word(&mut self.frame, 0x20, word).map_err(Fault::Service)?;
                self.captures.range_slot_returned = true; self.phase = Phase::StorePublic;
            }
            Phase::StorePublic => {
                let bits = self.captures.default_bits.ok_or(Fault::MissingState)?;
                self.services.write_parameter_f32(&mut self.parameter, 0x160, bits).map_err(Fault::Service)?;
                self.captures.public_store_returned = true; self.phase = Phase::StoreDefault;
            }
            Phase::StoreDefault => {
                let bits = self.captures.default_bits.ok_or(Fault::MissingState)?;
                self.services.write_parameter_f32(&mut self.parameter, 0x164, bits).map_err(Fault::Service)?;
                self.captures.default_store_returned = true; self.phase = Phase::AttributesRequired;
            }
            Phase::AttributesRequired => {}
        }
        Ok(())
    }
}
