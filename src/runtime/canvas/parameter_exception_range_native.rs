use super::{ActualRangeBody, ActualRangeReturn, BindLoadedRange, ChildBag};
use super::super::{metadata::Action, Phase as ParentPhase, Prepared as Parent};
pub use super::super::native_entry::{Push, Subtract};
pub use super::super::native_return::{Pop, StackAdjustment};

pub trait Services: super::super::Services {

    fn range_entry_push_rsi(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;

    fn range_entry_sub20(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<Subtract>) -> Result<(), Self::Error>;

    fn range_entry_mov_rsi_rcx(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn range_return_add20(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn range_return_pop_rsi(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;

    fn range_actual_ret(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<ActualRet>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryInput { pub entry_pc: u32, pub entry_rsp: u64, pub rcx: u64, pub original_rsi: u64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct EntryCaptures { pub push: Option<Push>, pub subtract: Option<Subtract>, pub rsi: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct EntryHistory { pub input: EntryInput, pub captures: EntryCaptures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryPhase { PushRsi, Subtract, SetRsi, BodyRequired }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RegisterBinding, ReturnBinding, AlreadyStopped }
pub enum EntryAdmission { NotPendingRangeCall, MissingReceiver, ActualBinding }
pub struct RejectedEntry<S: Services, T> { pub parent: Parent<S, T>, pub input: EntryInput, pub fault: EntryAdmission }
pub struct EntryPrepared<S: Services, T> {
    parent: Parent<S, T>, input: EntryInput, captures: EntryCaptures,
    phase: EntryPhase, attempted: Option<EntryPhase>, stopped: bool,
}
pub struct EntryParts<S: Services, T> {

    pub parent: Parent<S, T>, pub input: EntryInput, pub captures: EntryCaptures,
    pub phase: EntryPhase, pub attempted: Option<EntryPhase>, pub stopped: bool,
}
pub struct EntryStopped<S: Services, T> { pub pending: EntryPrepared<S, T>, pub fault: Fault<S::Error> }
pub enum EntryPoll<S: Services, T> { Pending(EntryPrepared<S, T>), Ready(Entered<S, T>), Failed(EntryStopped<S, T>) }
pub struct Entered<S: Services, T> { pending: EntryPrepared<S, T> }
impl<S: Services, T> Entered<S, T> {
    pub fn history(&self) -> EntryHistory { EntryHistory { input: self.pending.input, captures: self.pending.captures } }
    pub fn into_parts(self) -> EntryParts<S, T> { self.pending.into_parts() }

    pub fn bind_body<B: BindLoadedRange<S, T>>(self, binder: &mut B)
        -> Result<super::Bound<S, T>, super::RejectedBind<S, T>> {
        let history = self.history();
        let actual = ActualRangeBody { body_pc: 0x2b39d8, receiver_rcx: self.pending.input.rcx,
            saved_rsi: self.pending.captures.rsi.unwrap(), original_caller_rsi: self.pending.input.original_rsi,
            work_rsp: self.pending.captures.subtract.unwrap().after_rsp };
        self.pending.parent.bind_range_body(actual, binder, Some(history))
    }
}
impl<S: Services, T> EntryPrepared<S, T> {

    pub fn from_actual_entry(mut parent: Parent<S, T>, input: EntryInput) -> Result<Self, RejectedEntry<S, T>> {
        let already_stopped = parent.stopped;
        parent.stopped = true;
        let action = parent.stage.profile.actions.get(parent.action).copied().flatten();
        if already_stopped || parent.phase != ParentPhase::CallDirect
            || !matches!(action, Some(Action::Direct { target: 0x2b39d0, .. })) {
            return Err(RejectedEntry { parent, input, fault: EntryAdmission::NotPendingRangeCall });
        }
        let Some(receiver) = parent.stage.captures.direct_receivers[parent.action] else {
            return Err(RejectedEntry { parent, input, fault: EntryAdmission::MissingReceiver });
        };
        if input.entry_pc != 0x2b39d0 || input.rcx != receiver
            || input.entry_rsp != parent.stage.actual.funclet_work_rsp.wrapping_sub(8) {
            return Err(RejectedEntry { parent, input, fault: EntryAdmission::ActualBinding });
        }
        Ok(Self { parent, input, captures: EntryCaptures::default(), phase: EntryPhase::PushRsi,
            attempted: None, stopped: false })
    }
    pub fn phase(&self) -> EntryPhase { self.phase }
    pub fn captures(&self) -> &EntryCaptures { &self.captures }
    pub fn into_parts(self) -> EntryParts<S, T> {
        EntryParts { parent: self.parent, input: self.input, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> EntryPoll<S, T> {
        if self.stopped { return EntryPoll::Failed(EntryStopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != EntryPhase::BodyRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return EntryPoll::Failed(EntryStopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != EntryPhase::BodyRequired { return EntryPoll::Pending(self); }
        EntryPoll::Ready(Entered { pending: self })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let r = &mut self.parent.resources;
        let c = &mut self.captures;
        match self.phase {
            EntryPhase::PushRsi => {
                r.services.range_entry_push_rsi(&mut r.frame, &mut c.push).map_err(Fault::Service)?;
                let actual = c.push.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != self.input.entry_rsp || actual.after_rsp != self.input.entry_rsp.wrapping_sub(8)
                    || actual.stored != self.input.original_rsi { return Err(Fault::StackBinding); }
                self.phase = EntryPhase::Subtract;
            }
            EntryPhase::Subtract => {
                r.services.range_entry_sub20(&mut r.frame, &mut c.subtract).map_err(Fault::Service)?;
                let actual = c.subtract.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != self.input.entry_rsp.wrapping_sub(8)
                    || actual.after_rsp != self.input.entry_rsp.wrapping_sub(0x28) { return Err(Fault::StackBinding); }
                self.phase = EntryPhase::SetRsi;
            }
            EntryPhase::SetRsi => {
                r.services.range_entry_mov_rsi_rcx(&mut r.frame, &mut c.rsi).map_err(Fault::Service)?;
                if c.rsi.ok_or(Fault::MissingReceipt)? != self.input.rcx { return Err(Fault::RegisterBinding); }
                self.phase = EntryPhase::BodyRequired;
            }
            EntryPhase::BodyRequired => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualRet { pub binding: ActualRangeReturn, pub target: u64, pub raw_rax: u64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct ReturnCaptures {
    pub adjustment: Option<StackAdjustment>, pub pop: Option<Pop>, pub actual_return: Option<ActualRet>,
}
#[derive(Clone, Copy, Debug)]
pub struct ReturnHistory { pub captures: ReturnCaptures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnPhase { AdjustStack, PopRsi, Return, Done }
pub struct ReturnPrepared<S: Services, T> {
    body: super::range::Parts<S, ChildBag<S, T>>, captures: ReturnCaptures,
    phase: ReturnPhase, attempted: Option<ReturnPhase>, stopped: bool,
}
pub struct ReturnParts<S: Services, T> {
    pub body: super::range::Parts<S, ChildBag<S, T>>, pub captures: ReturnCaptures,
    pub phase: ReturnPhase, pub attempted: Option<ReturnPhase>, pub stopped: bool,
}
pub struct ReturnStopped<S: Services, T> { pub pending: ReturnPrepared<S, T>, pub fault: Fault<S::Error> }
pub enum ReturnPoll<S: Services, T> { Pending(ReturnPrepared<S, T>), Ready(Returned<S, T>), Failed(ReturnStopped<S, T>) }
pub struct Returned<S: Services, T> { pending: ReturnPrepared<S, T> }
impl<S: Services, T> Returned<S, T> {
    pub fn captures(&self) -> &ReturnCaptures { &self.pending.captures }
    pub fn into_parts(self) -> ReturnParts<S, T> { self.pending.into_parts() }

    pub fn into_parent(self) -> Result<Parent<S, super::ReturnedRange<T, S::Argument, S::Function>>, super::RejectedReturn<S, T>> {
        let actual = self.pending.captures.actual_return.unwrap();
        super::resume_after_range_return(self.pending.body, actual.binding,
            Some(ReturnHistory { captures: self.pending.captures }))
    }
}
impl<S: Services, T> ReturnPrepared<S, T> {
    pub fn from_completed_body(body: super::ReturnRequired<S, T>) -> Self {
        Self { body: body.into_parts(), captures: ReturnCaptures::default(), phase: ReturnPhase::AdjustStack,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> ReturnPhase { self.phase }
    pub fn captures(&self) -> &ReturnCaptures { &self.captures }
    pub fn into_parts(self) -> ReturnParts<S, T> {
        ReturnParts { body: self.body, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> ReturnPoll<S, T> {
        if self.stopped { return ReturnPoll::Failed(ReturnStopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != ReturnPhase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return ReturnPoll::Failed(ReturnStopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != ReturnPhase::Done { return ReturnPoll::Pending(self); }
        ReturnPoll::Ready(Returned { pending: self })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let work = self.body.input.retained.actual.work_rsp;
        let r = &mut self.body.input;
        let c = &mut self.captures;
        match self.phase {
            ReturnPhase::AdjustStack => {
                r.services.range_return_add20(&mut r.frame, &mut c.adjustment).map_err(Fault::Service)?;
                let actual = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if actual.before != work || actual.after != work.wrapping_add(0x20) { return Err(Fault::StackBinding); }
                self.phase = ReturnPhase::PopRsi;
            }
            ReturnPhase::PopRsi => {
                r.services.range_return_pop_rsi(&mut r.frame, &mut c.pop).map_err(Fault::Service)?;
                let actual = c.pop.ok_or(Fault::MissingReceipt)?;
                if actual.before != work.wrapping_add(0x20) || actual.after != work.wrapping_add(0x28) { return Err(Fault::StackBinding); }
                self.phase = ReturnPhase::Return;
            }
            ReturnPhase::Return => {
                r.services.range_actual_ret(&mut r.frame, &mut c.actual_return).map_err(Fault::Service)?;
                let actual = c.actual_return.ok_or(Fault::MissingReceipt)?.binding;
                if actual.ret_pc != 0x2b3a4f || actual.before_ret_rsp != work.wrapping_add(0x28)
                    || actual.after_ret_rsp != work.wrapping_add(0x30)
                    || actual.restored_rsi != c.pop.ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); }
                self.phase = ReturnPhase::Done;
            }
            ReturnPhase::Done => {}
        }
        Ok(())
    }
}
