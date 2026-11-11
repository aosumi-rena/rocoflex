use super::{Captures, Caller, Fault, Phase, Services};
use super::super::super::FunctionCapture;

pub struct Input<S: Services, T> {
    pub services: S, pub argument: S::Argument, pub frame: S::Frame,
    pub actual_saved_rsi: u64, pub retained: T,
}
pub struct Prepared<S: Services, T> {

    pub(super) input: Input<S, T>, pub(super) functions: [Option<FunctionCapture<S::Function>>; 3], pub(super) captures: Captures,
    pub(super) index: usize, pub(super) phase: Phase, pub(super) attempted: Option<(usize, Phase)>, pub(super) stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(NativeReturnRequired<S, T>), Failed(Stopped<S, T>) }
pub struct NativeReturnRequired<S: Services, T> { parts: Parts<S, T> }
pub struct Parts<S: Services, T> {

    pub(in super::super::super::super) input: Input<S, T>,
    pub functions: [Option<FunctionCapture<S::Function>>; 3], pub captures: Captures,
    pub index: usize, pub phase: Phase, pub attempted: Option<(usize, Phase)>, pub stopped: bool,
}
impl<S: Services, T> Parts<S, T> {
    pub fn input(&self) -> &Input<S, T> { &self.input }
}
impl<S: Services, T> NativeReturnRequired<S, T> {
    pub fn required_next_pc(&self) -> u32 { 0x2b3a4a }
    pub fn into_parts(self) -> Parts<S, T> { self.parts }
}
impl<S: Services, T> Prepared<S, T> {

    pub fn from_actual_2b39d8(input: Input<S, T>) -> Self {
        Self { input, functions: [None, None, None], captures: Captures::default(), index: 0,
            phase: Phase::ReadSource, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { input: self.input, functions: self.functions, captures: self.captures,
            index: self.index, phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::ReturnRequired && spent < budget {
            self.attempted = Some((self.index, self.phase)); spent += 1;
            if let Err(fault) = super::step_arguments(&mut self.input.services, &mut self.input.argument,
                &mut self.input.frame, &mut self.functions, &mut self.captures, &mut self.index,
                &mut self.phase, Caller::Standalone39d8) {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::ReturnRequired { return Poll::Pending(self); }
        Poll::Ready(NativeReturnRequired { parts: self.into_parts() })
    }
}
