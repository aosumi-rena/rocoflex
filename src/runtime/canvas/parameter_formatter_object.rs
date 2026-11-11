use super::{PrecisionSelected, Captures as PrecisionCaptures};
use super::super::{Retained, BranchSelected, Next};
use super::super::super::{FunctionCapture, CloneReturn};

#[path = "parameter_formatter_function.rs"]
pub mod function;
#[path = "parameter_formatter_cleanup.rs"]
pub mod selected_cleanup;
#[path = "parameter_formatter_move.rs"]
pub mod selected_move;

pub struct Input { pub actual_image_base: u64, pub actual_saved_rsi: u64 }
pub trait Services: super::Services + super::super::retirement::Services {
    fn write_frame_dword(&mut self, frame: &mut Self::Frame, offset: i32, bits: u32) -> Result<(), Self::Error>;
    fn frame_address(&mut self, frame: &Self::Frame, offset: i32) -> Result<u64, Self::Error>;
    fn read_parameter_function(&mut self, parameter: &Self::Parameter, offset: u32,
        held: &mut Option<FunctionCapture<Self::Function>>) -> Result<(), Self::Error>;
    fn read_frame_function(&mut self, frame: &mut Self::Frame, offset: i32,
        held: &mut Option<FunctionCapture<Self::Function>>) -> Result<(), Self::Error>;

    fn capture_function_owner(&mut self, frame: &mut Self::Frame, word: u64,
        held: &mut Option<FunctionCapture<Self::Function>>) -> Result<(), Self::Error>;
    fn read_move_target8(&mut self, saved_vtable: u64) -> Result<u64, Self::Error>;
    fn move_virtual8(&mut self, pc: u32, function: &mut Self::Function, source_word: u64,
        target: u64, destination: u64, frame: &mut Self::Frame,
        held: &mut Option<CloneReturn<Self::CloneOwner>>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup { Old, Temporary, Holder }
impl Cleanup {
    fn index(self) -> usize { match self { Self::Old => 0, Self::Temporary => 1, Self::Holder => 2 } }
    fn function(self) -> usize { match self { Self::Old => 1, Self::Temporary => 3, Self::Holder => 4 } }
    fn pc(self) -> u32 { match self { Self::Old => 0x2b3596, Self::Temporary => 0x2b35e3, Self::Holder => 0x2b35fc } }
    fn after_call(self) -> Phase { match self { Self::Old => Phase::ClearMember,
        Self::Temporary => Phase::ReadHolder, Self::Holder => Phase::ReadFinalSecond } }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub init_returned: [bool; 6], pub initial_member_read: bool,
    pub first_frame38: Option<u64>, pub old_move_vtable: Option<u64>,
    pub move_destinations: [Option<u64>; 2], pub move_targets: [Option<u64>; 2],
    pub move_returned: [bool; 2], pub old_holder_store_returned: bool,
    pub selected_moves: [Option<selected_move::History>; 2],
    pub old_current_read: bool, pub old_cleanup_frame38: Option<u64>,
    pub member_clear_returned: bool, pub temp_read_returned: bool,
    pub direct_transfer_returned: bool, pub new_move_vtable: Option<u64>,
    pub new_member_store_returned: bool, pub temp_cleanup_read: bool,
    pub holder_read_returned: bool, pub holder_inline: Option<u64>,
    pub cleanup_delete: [Option<bool>; 3], pub cleanup_vtables: [Option<u64>; 3],
    pub cleanup_targets: [Option<u64>; 3], pub cleanup_returned: [bool; 3],
    pub selected_cleanup: [Option<selected_cleanup::History>; 3],
    pub final_second_member: Option<u64>, pub next: Option<Next>,
}
pub struct History<F, C> {
    pub actual_image_base: u64, pub saved_rsi: u64, pub template_vtable: u64,
    pub digits: u32, pub initial_branch: Next, pub captures: Captures,

    pub functions: [Option<FunctionCapture<F>>; 6],
    pub moves: [Option<CloneReturn<C>>; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Initialize, ReadInitialMember, BranchInitial, ReadFirstFrame38, BranchOld,
    StoreOldPointer, ReadOldInlineVtable, CaptureOldDestination, ReadOldMoveTarget,
    MoveOld, StoreOldMoveResult, ReadOldCurrent, BranchOldCurrent,
    ReadOldCleanupFrame38, ClearMember, ReadTemporary, BranchTemporary,
    StoreDirectTemporary, ReadNewVtable, ReadNewDestination, CaptureNewFunction,
    ReadNewMoveTarget, MoveNew, StoreNewMember, ReadTempCleanup, BranchTempCleanup,
    ReadHolder, BranchHolder, CaptureHolderInline, CleanupVtable, CleanupTarget,
    CleanupCall, ReadFinalSecond, Done,
}
pub enum Fault<E> { Service(E), MissingState, FunctionBinding { expected: u64, received: u64 }, AlreadyStopped }
pub struct Prepared<S: Services> {
    retained: Retained<S>, precision: PrecisionCaptures,
    history: History<S::Function, S::CloneOwner>, init: usize,
    cleanup: Option<Cleanup>, phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(FormatterWritten<S>), Failed(Stopped<S>) }
pub struct FormatterWritten<S: Services> {
    retained: Retained<S>, precision: PrecisionCaptures,
    history: History<S::Function, S::CloneOwner>,
}
pub struct Parts<S: Services> {
    retained: Retained<S>, pub precision: PrecisionCaptures,
    pub history: History<S::Function, S::CloneOwner>, pub init: usize,
    pub cleanup: Option<Cleanup>, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn retained(&self) -> &Retained<S> { &self.retained }
}
impl<S: Services> FormatterWritten<S> {

    pub fn into_selected_branch(self) -> BranchSelected<S> {
        let mut retained = self.retained;
        retained.captures.next = self.history.captures.next;
        retained.precision = Some(self.precision);
        retained.formatter = Some(self.history);
        BranchSelected { retained }
    }
}
impl<S: Services> Prepared<S> {

    pub fn from_precision_selected(selected: PrecisionSelected<S>, input: Input) -> Self {
        let digits = selected.digits();
        let returned = selected.into_formatter_object_required();
        Self { retained: returned.retained, precision: returned.captures,
            history: History { actual_image_base: input.actual_image_base,
                saved_rsi: input.actual_saved_rsi,
                template_vtable: input.actual_image_base.wrapping_add(0x627ac8),
                digits, initial_branch: Next::Formatting2b3307, captures: Captures::default(),
                functions: [None, None, None, None, None, None], moves: [None, None] },
            init: 0, cleanup: None, phase: Phase::Initialize, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn history(&self) -> &History<S::Function, S::CloneOwner> { &self.history }
    pub fn into_parts(self) -> Parts<S> {
        Parts { retained: self.retained, precision: self.precision, history: self.history,
            init: self.init, cleanup: self.cleanup, phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }
        if self.history.captures.next.is_none() {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        }
        Poll::Ready(FormatterWritten { retained: self.retained, precision: self.precision, history: self.history })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let p = &mut self.retained.parent;
        let h = &mut self.history;
        let c = &mut h.captures;
        match self.phase {
            Phase::Initialize => {
                match self.init {
                    0 => p.services.write_frame_word(&mut p.frame, -0x60, h.template_vtable),
                    1 => p.services.write_frame_dword(&mut p.frame, -0x58, h.digits),
                    2 => p.services.write_frame_word(&mut p.frame, -0x20, h.template_vtable),
                    3 => p.services.write_frame_dword(&mut p.frame, -0x18, h.digits),
                    4 => p.services.write_frame_word(&mut p.frame, 0x18, h.saved_rsi),
                    5 => p.services.write_frame_word(&mut p.frame, -0x28, 0),
                    _ => return Err(Fault::MissingState),
                }.map_err(Fault::Service)?;
                c.init_returned[self.init] = true; self.init += 1;
                if self.init == 6 { self.phase = Phase::ReadInitialMember; }
            }
            Phase::ReadInitialMember => {
                p.services.read_parameter_function(&p.parameter, 0x1a0, &mut h.functions[0]).map_err(Fault::Service)?;
                c.initial_member_read = true; self.phase = Phase::BranchInitial;
            }
            Phase::BranchInitial => {
                if h.functions[0].as_ref().is_some_and(|f| f.word != 0) { self.phase = Phase::ReadFirstFrame38; }
                else { c.new_move_vtable = Some(h.template_vtable); self.phase = Phase::ReadNewDestination; }
            }
            Phase::ReadFirstFrame38 => {
                c.first_frame38 = Some(p.services.read_frame_word(&mut p.frame, 0x38).map_err(Fault::Service)?);
                self.phase = Phase::BranchOld;
            }
            Phase::BranchOld => {
                let old = h.functions[0].as_ref().ok_or(Fault::MissingState)?.word;
                self.phase = if old == c.first_frame38.ok_or(Fault::MissingState)? { Phase::ReadOldInlineVtable } else { Phase::StoreOldPointer };
            }
            Phase::StoreOldPointer => {
                let word = h.functions[0].as_ref().ok_or(Fault::MissingState)?.word;
                p.services.write_frame_word(&mut p.frame, -0x28, word).map_err(Fault::Service)?;
                c.old_holder_store_returned = true; self.phase = Phase::ClearMember;
            }
            Phase::ReadOldInlineVtable => {

                c.old_move_vtable = Some(p.services.read_parameter_word(&p.parameter, 0x168).map_err(Fault::Service)?);
                self.phase = Phase::CaptureOldDestination;
            }
            Phase::CaptureOldDestination => {
                c.move_destinations[0] = Some(p.services.frame_address(&p.frame, -0x60).map_err(Fault::Service)?);
                self.phase = Phase::ReadOldMoveTarget;
            }
            Phase::ReadOldMoveTarget => {
                c.move_targets[0] = Some(p.services.read_move_target8(c.old_move_vtable.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::MoveOld;
            }
            Phase::MoveOld => {
                let f = h.functions[0].as_mut().ok_or(Fault::MissingState)?;
                p.services.move_virtual8(0x2b3579, &mut f.owner, f.word, c.move_targets[0].ok_or(Fault::MissingState)?,
                    c.move_destinations[0].ok_or(Fault::MissingState)?, &mut p.frame, &mut h.moves[0]).map_err(Fault::Service)?;
                if h.moves[0].is_none() { return Err(Fault::MissingState); }
                c.move_returned[0] = true; self.phase = Phase::StoreOldMoveResult;
            }
            Phase::StoreOldMoveResult => {
                let word = h.moves[0].as_ref().ok_or(Fault::MissingState)?.word;
                p.services.write_frame_word(&mut p.frame, -0x28, word).map_err(Fault::Service)?;
                c.old_holder_store_returned = true; self.phase = Phase::ReadOldCurrent;
            }
            Phase::ReadOldCurrent => {
                p.services.read_parameter_function(&p.parameter, 0x1a0, &mut h.functions[1]).map_err(Fault::Service)?;
                c.old_current_read = true; self.phase = Phase::BranchOldCurrent;
            }
            Phase::BranchOldCurrent => {
                self.phase = if h.functions[1].as_ref().is_some_and(|f| f.word != 0) { Phase::ReadOldCleanupFrame38 } else { Phase::ReadTemporary };
            }
            Phase::ReadOldCleanupFrame38 => {
                let inline = p.services.read_frame_word(&mut p.frame, 0x38).map_err(Fault::Service)?;
                c.old_cleanup_frame38 = Some(inline);
                c.cleanup_delete[0] = Some(h.functions[1].as_ref().ok_or(Fault::MissingState)?.word != inline);
                self.cleanup = Some(Cleanup::Old); self.phase = Phase::CleanupVtable;
            }
            Phase::ClearMember => {
                p.services.write_parameter_word(&mut p.parameter, 0x1a0, 0).map_err(Fault::Service)?;
                c.member_clear_returned = true; self.phase = Phase::ReadTemporary;
            }
            Phase::ReadTemporary => {
                p.services.read_frame_function(&mut p.frame, 0x18, &mut h.functions[2]).map_err(Fault::Service)?;
                c.temp_read_returned = true; self.phase = Phase::BranchTemporary;
            }
            Phase::BranchTemporary => {
                let word = h.functions[2].as_ref().map_or(0, |f| f.word);
                if word == 0 { self.phase = Phase::ReadHolder; }
                else if word == h.saved_rsi { self.phase = Phase::ReadNewVtable; }
                else { self.phase = Phase::StoreDirectTemporary; }
            }
            Phase::StoreDirectTemporary => {
                let word = h.functions[2].as_ref().ok_or(Fault::MissingState)?.word;
                p.services.write_parameter_word(&mut p.parameter, 0x1a0, word).map_err(Fault::Service)?;
                c.direct_transfer_returned = true; self.phase = Phase::ReadHolder;
            }
            Phase::ReadNewVtable => {
                c.new_move_vtable = Some(p.services.read_frame_word(&mut p.frame, -0x20).map_err(Fault::Service)?);
                self.phase = Phase::ReadNewDestination;
            }
            Phase::ReadNewDestination => {
                c.move_destinations[1] = Some(p.services.read_frame_word(&mut p.frame, 0x38).map_err(Fault::Service)?);
                self.phase = Phase::CaptureNewFunction;
            }
            Phase::CaptureNewFunction => {
                p.services.capture_function_owner(&mut p.frame, h.saved_rsi, &mut h.functions[5]).map_err(Fault::Service)?;
                let received = h.functions[5].as_ref().ok_or(Fault::MissingState)?.word;
                if received != h.saved_rsi { return Err(Fault::FunctionBinding { expected: h.saved_rsi, received }); }
                self.phase = Phase::ReadNewMoveTarget;
            }
            Phase::ReadNewMoveTarget => {
                c.move_targets[1] = Some(p.services.read_move_target8(c.new_move_vtable.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::MoveNew;
            }
            Phase::MoveNew => {
                let f = h.functions[5].as_mut().ok_or(Fault::MissingState)?;
                p.services.move_virtual8(0x2b35c6, &mut f.owner, h.saved_rsi, c.move_targets[1].ok_or(Fault::MissingState)?,
                    c.move_destinations[1].ok_or(Fault::MissingState)?, &mut p.frame, &mut h.moves[1]).map_err(Fault::Service)?;
                if h.moves[1].is_none() { return Err(Fault::MissingState); }
                c.move_returned[1] = true; self.phase = Phase::StoreNewMember;
            }
            Phase::StoreNewMember => {
                let word = h.moves[1].as_ref().ok_or(Fault::MissingState)?.word;
                p.services.write_parameter_word(&mut p.parameter, 0x1a0, word).map_err(Fault::Service)?;
                c.new_member_store_returned = true; self.phase = Phase::ReadTempCleanup;
            }
            Phase::ReadTempCleanup => {
                p.services.read_frame_function(&mut p.frame, 0x18, &mut h.functions[3]).map_err(Fault::Service)?;
                c.temp_cleanup_read = true; self.phase = Phase::BranchTempCleanup;
            }
            Phase::BranchTempCleanup => {
                let word = h.functions[3].as_ref().map_or(0, |f| f.word);
                if word == 0 { self.phase = Phase::ReadHolder; }
                else { c.cleanup_delete[1] = Some(word != h.saved_rsi); self.cleanup = Some(Cleanup::Temporary); self.phase = Phase::CleanupVtable; }
            }
            Phase::ReadHolder => {
                p.services.read_frame_function(&mut p.frame, -0x28, &mut h.functions[4]).map_err(Fault::Service)?;
                c.holder_read_returned = true; self.phase = Phase::BranchHolder;
            }
            Phase::BranchHolder => self.phase = if h.functions[4].as_ref().is_some_and(|f| f.word != 0) { Phase::CaptureHolderInline } else { Phase::ReadFinalSecond },
            Phase::CaptureHolderInline => {
                let inline = p.services.frame_address(&p.frame, -0x60).map_err(Fault::Service)?;
                c.holder_inline = Some(inline);
                c.cleanup_delete[2] = Some(h.functions[4].as_ref().ok_or(Fault::MissingState)?.word != inline);
                self.cleanup = Some(Cleanup::Holder); self.phase = Phase::CleanupVtable;
            }
            Phase::CleanupVtable => {
                let kind = self.cleanup.ok_or(Fault::MissingState)?;
                let f = h.functions[kind.function()].as_mut().ok_or(Fault::MissingState)?;
                c.cleanup_vtables[kind.index()] = Some(p.services.read_clone_vtable(&mut f.owner).map_err(Fault::Service)?);
                self.phase = Phase::CleanupTarget;
            }
            Phase::CleanupTarget => {
                let index = self.cleanup.ok_or(Fault::MissingState)?.index();
                c.cleanup_targets[index] = Some(p.services.read_retirement_target20(c.cleanup_vtables[index].ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::CleanupCall;
            }
            Phase::CleanupCall => {
                let kind = self.cleanup.ok_or(Fault::MissingState)?;
                let index = kind.index(); let f = h.functions[kind.function()].as_mut().ok_or(Fault::MissingState)?;
                p.services.retire_virtual20(kind.pc(), &mut f.owner, f.word,
                    c.cleanup_targets[index].ok_or(Fault::MissingState)?, c.cleanup_delete[index].ok_or(Fault::MissingState)?,
                    &mut p.frame).map_err(Fault::Service)?;
                c.cleanup_returned[index] = true; self.cleanup = None;
                self.phase = kind.after_call();
            }
            Phase::ReadFinalSecond => {
                let word = p.services.read_parameter_word(&p.parameter, 0x1e0).map_err(Fault::Service)?;
                c.final_second_member = Some(word);
                c.next = Some(if word == 0 { Next::DefaultCallback2b360f } else { Next::ArgumentRetirement2b326f });
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
