use super::{BranchSelected, Next, Retained};
use super::super::FunctionCapture;

#[path = "parameter_constructor_return.rs"]
pub mod native_return;
#[path = "parameter_argument_cleanup.rs"]
pub mod standalone;
#[path = "parameter_argument_callable.rs"]
pub mod selected_callable;

pub trait Services: super::Services {

    fn argument_inline_address(&mut self, argument: &Self::Argument, offset: u32) -> Result<u64, Self::Error>;

    fn read_retirement_target20(&mut self, saved_vtable: u64) -> Result<u64, Self::Error>;

    fn retire_virtual20(&mut self, pc: u32, function: &mut Self::Function, source_word: u64,
        target: u64, delete_storage: bool, frame: &mut Self::Frame) -> Result<(), Self::Error>;

    fn write_argument_word(&mut self, argument: &mut Self::Argument, offset: u32, word: u64)
        -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadSource, BranchSource, CaptureInline, ReadVtable, ReadTarget, Call, ClearSlot, ReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub source_read_returned: [bool; 3], pub inline: [Option<u64>; 3],
    pub delete_storage: [Option<bool>; 3], pub vtables: [Option<u64>; 3],
    pub targets: [Option<u64>; 3], pub call_returned: [bool; 3],
    pub slot_clear_returned: [bool; 3],
    pub selected_callable: [Option<selected_callable::History>; 3],
}
pub struct Owner<S: Services> {
    pub constructor: Retained<S>,
    pub default_callback: Option<super::default_callback::History>,
    pub current_functions: [Option<FunctionCapture<S::Function>>; 3],
    pub captures: Captures,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    owner: Owner<S>, index: usize, phase: Phase,
    attempted: Option<(usize, Phase)>, stopped: bool,
}
pub struct Rejected<S: Services> { pub selected: BranchSelected<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(ConstructorReturnRequired<S>), Failed(Stopped<S>) }

pub struct ConstructorReturnRequired<S: Services> { owner: Owner<S> }
pub struct Parts<S: Services> {
    owner: Owner<S>, pub index: usize, pub phase: Phase,
    pub attempted: Option<(usize, Phase)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn owner(&self) -> &Owner<S> { &self.owner }
}
impl<S: Services> ConstructorReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x2b32e1 }
    pub fn into_required_native_return(self) -> Owner<S> { self.owner }
}
impl<S: Services> Prepared<S> {
    pub fn from_selected_argument_retirement(selected: BranchSelected<S>) -> Result<Self, Rejected<S>> {
        if selected.next() != Next::ArgumentRetirement2b326f { return Err(Rejected { selected }); }
        Ok(Self { owner: Owner { constructor: selected.into_required_continuation(),
            default_callback: None, current_functions: [None, None, None], captures: Captures::default() },
            index: 0, phase: Phase::ReadSource, attempted: None, stopped: false })
    }

    pub fn from_default_written(written: super::default_callback::DefaultWritten<S>) -> Self {
        let returned = written.into_required_retirement();
        Self { owner: Owner { constructor: returned.retained, default_callback: Some(returned.history),
            current_functions: [None, None, None], captures: Captures::default() },
            index: 0, phase: Phase::ReadSource, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<(usize, Phase)> { self.attempted }
    pub fn owner(&self) -> &Owner<S> { &self.owner }
    pub fn into_parts(self) -> Parts<S> {
        Parts { owner: self.owner, index: self.index, phase: self.phase,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::ReturnRequired && spent < budget {
            self.attempted = Some((self.index, self.phase)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::ReturnRequired { return Poll::Pending(self); }
        Poll::Ready(ConstructorReturnRequired { owner: self.owner })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let o = &mut self.owner;
        let p = &mut o.constructor.parent;
        step_arguments(&mut p.services, &mut p.argument, &mut p.frame, &mut o.current_functions,
            &mut o.captures, &mut self.index, &mut self.phase, Caller::Constructor326f)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Caller { Constructor326f, Standalone39d8 }
impl Caller {
    fn call_pc(self, index: usize) -> Option<u32> {
        match self {
            Self::Constructor326f => [0x2b328b, 0x2b32b2, 0x2b32d6].get(index).copied(),
            Self::Standalone39d8 => [0x2b39f4, 0x2b3a1b, 0x2b3a3f].get(index).copied(),
        }
    }
}
fn advance_arguments(index: &mut usize, phase: &mut Phase) {
    *index += 1;
    *phase = if *index == 3 { Phase::ReturnRequired } else { Phase::ReadSource };
}

fn step_arguments<S: Services>(services: &mut S, argument: &mut S::Argument, frame: &mut S::Frame,
    functions: &mut [Option<FunctionCapture<S::Function>>; 3], c: &mut Captures,
    index: &mut usize, phase: &mut Phase, caller: Caller) -> Result<(), Fault<S::Error>> {
    let i = *index;
    if i > 2 { return Err(Fault::MissingState); }
    let pointer_offset = [0xd0, 0x90, 0x50][i];
    let inline_offset = [0x98, 0x58, 0x18][i];
    let pc = caller.call_pc(i).ok_or(Fault::MissingState)?;
    match *phase {
        Phase::ReadSource => {

            services.read_source_function(argument, pointer_offset, &mut functions[i]).map_err(Fault::Service)?;
            c.source_read_returned[i] = true; *phase = Phase::BranchSource;
        }
        Phase::BranchSource => {
            if functions[i].as_ref().is_some_and(|function| function.word != 0) { *phase = Phase::CaptureInline; }
            else { advance_arguments(index, phase); }
        }
        Phase::CaptureInline => {
            let inline = services.argument_inline_address(argument, inline_offset).map_err(Fault::Service)?;
            c.inline[i] = Some(inline);
            c.delete_storage[i] = Some(functions[i].as_ref().ok_or(Fault::MissingState)?.word != inline);
            *phase = Phase::ReadVtable;
        }
        Phase::ReadVtable => {
            let function = functions[i].as_mut().ok_or(Fault::MissingState)?;
            c.vtables[i] = Some(services.read_clone_vtable(&mut function.owner).map_err(Fault::Service)?);
            *phase = Phase::ReadTarget;
        }
        Phase::ReadTarget => {
            c.targets[i] = Some(services.read_retirement_target20(c.vtables[i].ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
            *phase = Phase::Call;
        }
        Phase::Call => {
            let function = functions[i].as_mut().ok_or(Fault::MissingState)?;
            services.retire_virtual20(pc, &mut function.owner, function.word, c.targets[i].ok_or(Fault::MissingState)?,
                c.delete_storage[i].ok_or(Fault::MissingState)?, frame).map_err(Fault::Service)?;
            c.call_returned[i] = true; *phase = Phase::ClearSlot;
        }
        Phase::ClearSlot => {
            services.write_argument_word(argument, pointer_offset, 0).map_err(Fault::Service)?;
            c.slot_clear_returned[i] = true; advance_arguments(index, phase);
        }
        Phase::ReturnRequired => {}
    }
    Ok(())
}
