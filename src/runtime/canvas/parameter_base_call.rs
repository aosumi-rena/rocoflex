use super::{Binding, Input as InitializationInput};

#[path = "parameter_constructor_entry.rs"]
pub mod entry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words { pub actual_r14: u64, pub actual_rbx: u64 }
pub struct Input<P, A> { pub initialization: InitializationInput<P, A>, pub words: Words }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Push { pub before_rsp: u64, pub after_rsp: u64, pub stored: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallRequest {
    pub pc: u32, pub target: u64, pub rcx: u64, pub rax: u64,
    pub source_pushed_rsp: u64, pub required_call_entry_rsp: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualContinuation {
    pub pc: u32, pub rsp: u64, pub rbp: u64, pub r14: u64, pub rbx: u64,
    pub rdi: u64, pub rsi: u64,
}
pub trait Services: super::Services {

    type BaseContinuationOwner;

    fn base_push_rax(&mut self, pc: u32, value: u64, returned: &mut Option<Push>) -> Result<(), Self::Error>;

    fn adopt_actual_base_continuation(&mut self, call: &CallRequest,
        actual: &ActualContinuation, held: &mut Option<Self::BaseContinuationOwner>)
        -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub entry: Option<entry::History>,
    pub source_dword: Option<u32>, pub stores: [bool; 4], pub push: Option<Push>,
}
#[derive(Clone, Copy, Debug)]
pub struct History {
    pub binding: Binding, pub words: Words, pub captures: Captures,
    pub call: CallRequest, pub actual_continuation: ActualContinuation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadDword, StoreVtable, StoreZero8, StoreMinusOne10, StoreDword14, PushRax, CallRequired }
pub enum Fault<E> { Service(E), MissingState, PushBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(ProtectedCallRequired<S>), Failed(Stopped<S>) }
pub struct ProtectedCallRequired<S: Services> { parts: Parts<S>, call: CallRequest }
pub enum ContinuationFault<E> { Binding, Service(E), OwnerNotAdopted }
pub struct RejectedContinuation<S: Services> {
    pub parts: Parts<S>, pub expected_call: CallRequest, pub actual: ActualContinuation,
    pub owner: Option<S::BaseContinuationOwner>, pub fault: ContinuationFault<S::Error>,
}
pub struct Parts<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Parameter, S::Argument> { &self.input }
}
impl<S: Services> ProtectedCallRequired<S> {
    pub fn request(&self) -> &CallRequest { &self.call }
    pub fn into_parts(self) -> Parts<S> { self.parts }

    pub fn after_actual_continuation(mut self, actual: ActualContinuation,
        owner: S::BaseContinuationOwner)
        -> Result<super::Prepared<S>, RejectedContinuation<S>> {
        let mut held = Some(owner);
        let b = self.parts.input.initialization.binding;
        let words = self.parts.input.words;
        if actual.pc != 0x2b305e || actual.rbp != b.actual_rbp
            || actual.r14 != words.actual_r14 || actual.rbx != words.actual_rbx
            || actual.rdi != b.saved_rdi_source || actual.rsi != b.saved_rsi_source {
            return Err(RejectedContinuation { parts: self.parts, expected_call: self.call, actual,
                owner: held, fault: ContinuationFault::Binding });
        }
        if let Err(error) = self.parts.services.adopt_actual_base_continuation(&self.call, &actual, &mut held) {
            return Err(RejectedContinuation { parts: self.parts, expected_call: self.call, actual,
                owner: held, fault: ContinuationFault::Service(error) });
        }
        if held.is_some() {
            return Err(RejectedContinuation { parts: self.parts, expected_call: self.call, actual,
                owner: held, fault: ContinuationFault::OwnerNotAdopted });
        }
        let history = History { binding: b, words, captures: self.parts.captures,
            call: self.call, actual_continuation: actual };
        let mut next = super::Prepared::from_actual_2b305e(self.parts.services, self.parts.input.initialization);
        next.captures.base_call = Some(history);
        Ok(next)
    }
}
impl<S: Services> Prepared<S> {

    pub fn from_actual_2b3033(services: S, input: Input<S::Parameter, S::Argument>) -> Self {
        Self { services, input, captures: Captures::default(), phase: Phase::ReadDword,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::CallRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::CallRequired { return Poll::Pending(self); }
        let Some(push) = self.captures.push else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let call = CallRequest { pc: 0x2b3059,
            target: self.input.initialization.binding.actual_image_base.wrapping_add(0xcbd54d),
            rcx: self.input.words.actual_r14.wrapping_add(0x18), rax: push.stored,
            source_pushed_rsp: push.after_rsp, required_call_entry_rsp: push.after_rsp.wrapping_sub(8) };
        Poll::Ready(ProtectedCallRequired { parts: self.into_parts(), call })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let p = &mut self.input.initialization.range.parameter;
        let b = self.input.initialization.binding;
        let c = &mut self.captures;
        match self.phase {
            Phase::ReadDword => {
                c.source_dword = Some(self.services.init_read_dword(0x2b3033,
                    b.saved_rdi_source.wrapping_add(8)).map_err(Fault::Service)?);
                self.phase = Phase::StoreVtable;
            }
            Phase::StoreVtable => {
                self.services.init_write_word(0x2b303d, p, 0, b.actual_image_base.wrapping_add(0x60ef98)).map_err(Fault::Service)?;
                c.stores[0] = true; self.phase = Phase::StoreZero8;
            }
            Phase::StoreZero8 => {
                self.services.init_write_word(0x2b3040, p, 8, 0).map_err(Fault::Service)?;
                c.stores[1] = true; self.phase = Phase::StoreMinusOne10;
            }
            Phase::StoreMinusOne10 => {
                self.services.init_write_dword(0x2b3048, p, 0x10, u32::MAX).map_err(Fault::Service)?;
                c.stores[2] = true; self.phase = Phase::StoreDword14;
            }
            Phase::StoreDword14 => {
                self.services.init_write_dword(0x2b3050, p, 0x14, c.source_dword.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.stores[3] = true; self.phase = Phase::PushRax;
            }
            Phase::PushRax => {
                let value = c.source_dword.ok_or(Fault::MissingState)? as u64;
                self.services.base_push_rax(0x2b3058, value, &mut c.push).map_err(Fault::Service)?;
                let actual = c.push.ok_or(Fault::MissingState)?;
                if actual.before_rsp != b.actual_rbp.wrapping_sub(0x80)
                    || actual.after_rsp != actual.before_rsp.wrapping_sub(8) || actual.stored != value {
                    return Err(Fault::PushBinding);
                }
                self.phase = Phase::CallRequired;
            }
            Phase::CallRequired => {}
        }
        Ok(())
    }
}
