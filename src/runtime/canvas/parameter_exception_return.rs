use super::{BodyComplete, Parts as ParentParts};
pub use super::super::functions::attributes::retirement::native_return::{
    Register, Xmm, StackAdjustment, Pop, ActualReturn,
};

pub trait Services: super::Services {

    fn eh_restore_movaps(&mut self, pc: u32, frame: &mut Self::Frame,
        actual_work_rsp: u64, offset: u32, destination: Xmm,
        returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn eh_add_rsp48(&mut self, pc: u32, frame: &mut Self::Frame,
        returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn eh_pop_saved(&mut self, pc: u32, frame: &mut Self::Frame, destination: Register,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;

    fn eh_actual_ret(&mut self, pc: u32, frame: &mut Self::Frame,
        returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { RestoreXmm6, RestoreXmm7, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub xmm6: Option<[u32; 4]>, pub xmm7: Option<[u32; 4]>,
    pub adjustment: Option<StackAdjustment>, pub pops: [Option<Pop>; 6],
    pub actual_return: Option<ActualReturn>,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub epilogue_pc: u32, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services, T> {
    parent: ParentParts<S, T>, captures: Captures, phase: Phase,
    pop: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Parts<S: Services, T> {
    pub parent: ParentParts<S, T>, pub captures: Captures, pub phase: Phase,
    pub pop: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(Completed<S, T>), Failed(Stopped<S, T>) }
pub struct Completed<S: Services, T> { parts: Parts<S, T> }
impl<S: Services, T> Completed<S, T> {
    pub fn captures(&self) -> &Captures { &self.parts.captures }
    pub fn into_parts(self) -> Parts<S, T> { self.parts }

    pub fn into_returned(mut self) -> Result<super::Returned<S, T>, super::RejectedReturn<S, T>> {
        let actual = self.parts.captures.actual_return.unwrap();
        let stage = &mut self.parts.parent.stage;
        stage.native_return = Some(History { epilogue_pc: stage.profile.epilogue_pc, captures: self.parts.captures });
        let receipt = super::ActualReturn { selected_funclet: stage.profile.funclet,
            ret_pc: stage.profile.ret_pc, work_rsp: stage.actual.funclet_work_rsp,
            before_rsp: actual.before_rsp, after_rsp: actual.after_rsp, target: actual.target };
        BodyComplete { parts: self.parts.parent }.after_actual_return(receipt)
    }
}
impl<S: Services, T> Prepared<S, T> {
    pub fn from_completed_body(body: BodyComplete<S, T>) -> Self {
        Self { parent: body.into_parts(), captures: Captures::default(), phase: Phase::RestoreXmm6,
            pop: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { parent: self.parent, captures: self.captures, phase: self.phase, pop: self.pop,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.pop)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }
        Poll::Ready(Completed { parts: self.into_parts() })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let profile = self.parent.stage.profile;
        let work = self.parent.stage.actual.funclet_work_rsp;
        let epilogue = profile.epilogue_pc;
        let r = &mut self.parent.resources;
        let c = &mut self.captures;
        match self.phase {
            Phase::RestoreXmm6 => {
                r.services.eh_restore_movaps(epilogue, &mut r.frame, work, 0x30, Xmm::Xmm6, &mut c.xmm6).map_err(Fault::Service)?;
                if c.xmm6.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::RestoreXmm7;
            }
            Phase::RestoreXmm7 => {
                r.services.eh_restore_movaps(epilogue + 5, &mut r.frame, work, 0x20, Xmm::Xmm7, &mut c.xmm7).map_err(Fault::Service)?;
                if c.xmm7.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                r.services.eh_add_rsp48(epilogue + 10, &mut r.frame, &mut c.adjustment).map_err(Fault::Service)?;
                let actual = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if actual.before != work || actual.after != work.wrapping_add(0x48) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi, Register::R14, Register::R15, Register::Rbp];
                let offsets = [14, 15, 16, 17, 19, 21];
                if self.pop >= 6 { return Err(Fault::MissingReceipt); }
                r.services.eh_pop_saved(epilogue + offsets[self.pop], &mut r.frame,
                    registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?;
                let actual = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let expected = work.wrapping_add(0x48 + 8 * self.pop as u64);
                if actual.before != expected || actual.after != expected.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1;
                if self.pop == 6 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                r.services.eh_actual_ret(profile.ret_pc, &mut r.frame, &mut c.actual_return).map_err(Fault::Service)?;
                let actual = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != work.wrapping_add(0x78) || actual.after_rsp != work.wrapping_add(0x80)
                    || actual.xmm6 != c.xmm6.ok_or(Fault::MissingReceipt)?
                    || actual.xmm7 != c.xmm7.ok_or(Fault::MissingReceipt)? { return Err(Fault::ReturnBinding); }
                for i in 0..6 {
                    if actual.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); }
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
