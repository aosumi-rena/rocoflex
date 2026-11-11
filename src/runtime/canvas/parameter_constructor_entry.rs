#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { Rbp, R15, R14, Rsi, Rdi, Rbx }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source { R9, R8, Rdx, Rcx }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Xmm { Xmm7, Xmm6 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualEntry {
    pub rva: u32, pub source_image_base: u64, pub rsp: u64,
    pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64,

    pub saved: [u64; 6], pub xmm6: [u32; 4], pub xmm7: [u32; 4],
}
pub struct Input<P, A> { pub range: super::super::super::Input<P, A>, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Push { pub before_rsp: u64, pub after_rsp: u64, pub stored: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subtract { pub before_rsp: u64, pub after_rsp: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XmmStore { pub address: u64, pub words: [u32; 4] }
pub trait Services: super::Services {

    fn ctor_entry_push(&mut self, pc: u32, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn ctor_entry_sub_f8(&mut self, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
    fn ctor_entry_lea_rbp80(&mut self, returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn ctor_entry_save_xmm(&mut self, pc: u32, source: Xmm, actual_rbp: u64, offset: u32,
        returned: &mut Option<XmmStore>) -> Result<(), Self::Error>;
    fn ctor_entry_store_frame48(&mut self, actual_rbp: u64, value: u64) -> Result<(), Self::Error>;
    fn ctor_entry_move_register(&mut self, pc: u32, destination: Register, source: Source,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub pushes: [Option<Push>; 6], pub subtract: Option<Subtract>, pub rbp: Option<u64>,
    pub xmm7: Option<XmmStore>, pub xmm6: Option<XmmStore>,
    pub frame48_store_returned: bool, pub moves: [Option<u64>; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Push, Subtract, SetRbp, SaveXmm7, SaveXmm6, StoreFrame48, BindRegister, BaseCallRequired }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RegisterBinding, XmmBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, captures: Captures,
    phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Rejected<S: Services> { pub services: S, pub input: Input<S::Parameter, S::Argument> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Entered<S: Services> { parts: Parts<S> }
pub struct Parts<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, pub captures: Captures,
    pub phase: Phase, pub index: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Parameter, S::Argument> { &self.input }
}
impl<S: Services> Entered<S> {
    pub fn required_next_pc(&self) -> u32 { 0x2b3033 }
    pub fn history(&self) -> History { History { actual: self.parts.input.actual, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }

    pub fn into_base_call(self) -> super::Prepared<S> {
        let history = self.history();
        let actual = self.parts.input.actual;
        let rbp = self.parts.captures.rbp.unwrap();
        let mut next = super::Prepared::from_actual_2b3033(self.parts.services,
            super::Input { initialization: super::super::Input { range: self.parts.input.range,
                binding: super::super::Binding { actual_image_base: actual.source_image_base,
                    actual_rbp: rbp, saved_rdi_source: actual.rdx, saved_rsi_source: actual.r8 } },
                words: super::Words { actual_r14: actual.rcx, actual_rbx: actual.r9 } });
        next.captures.entry = Some(history);
        next
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(services: S, input: Input<S::Parameter, S::Argument>) -> Result<Self, Rejected<S>> {
        if input.actual.rva != 0x2b3000 { return Err(Rejected { services, input }); }
        Ok(Self { services, input, captures: Captures::default(), phase: Phase::Push,
            index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, captures: self.captures,
            phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::BaseCallRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::BaseCallRequired { return Poll::Pending(self); }
        Poll::Ready(Entered { parts: self.into_parts() })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let a = self.input.actual;
        let c = &mut self.captures;
        match self.phase {
            Phase::Push => {
                let registers = [Register::Rbp, Register::R15, Register::R14, Register::Rsi, Register::Rdi, Register::Rbx];
                let pcs = [0x2b3000, 0x2b3001, 0x2b3003, 0x2b3005, 0x2b3006, 0x2b3007];
                if self.index >= 6 { return Err(Fault::MissingReceipt); }
                self.services.ctor_entry_push(pcs[self.index], registers[self.index], &mut c.pushes[self.index]).map_err(Fault::Service)?;
                let actual = c.pushes[self.index].ok_or(Fault::MissingReceipt)?;
                let expected = a.rsp.wrapping_sub(8 * self.index as u64);
                if actual.before_rsp != expected || actual.after_rsp != expected.wrapping_sub(8)
                    || actual.stored != a.saved[self.index] { return Err(Fault::StackBinding); }
                self.index += 1;
                if self.index == 6 { self.phase = Phase::Subtract; }
            }
            Phase::Subtract => {
                self.services.ctor_entry_sub_f8(&mut c.subtract).map_err(Fault::Service)?;
                let actual = c.subtract.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != a.rsp.wrapping_sub(0x30)
                    || actual.after_rsp != actual.before_rsp.wrapping_sub(0xf8) { return Err(Fault::StackBinding); }
                self.phase = Phase::SetRbp;
            }
            Phase::SetRbp => {
                self.services.ctor_entry_lea_rbp80(&mut c.rbp).map_err(Fault::Service)?;
                let value = c.rbp.ok_or(Fault::MissingReceipt)?;
                if value != c.subtract.ok_or(Fault::MissingReceipt)?.after_rsp.wrapping_add(0x80) { return Err(Fault::RegisterBinding); }
                self.phase = Phase::SaveXmm7;
            }
            Phase::SaveXmm7 => {
                let rbp = c.rbp.ok_or(Fault::MissingReceipt)?;
                self.services.ctor_entry_save_xmm(0x2b3017, Xmm::Xmm7, rbp, 0x60, &mut c.xmm7).map_err(Fault::Service)?;
                let actual = c.xmm7.ok_or(Fault::MissingReceipt)?;
                if actual.address != rbp.wrapping_add(0x60) || actual.words != a.xmm7 { return Err(Fault::XmmBinding); }
                self.phase = Phase::SaveXmm6;
            }
            Phase::SaveXmm6 => {
                let rbp = c.rbp.ok_or(Fault::MissingReceipt)?;
                self.services.ctor_entry_save_xmm(0x2b301b, Xmm::Xmm6, rbp, 0x50, &mut c.xmm6).map_err(Fault::Service)?;
                let actual = c.xmm6.ok_or(Fault::MissingReceipt)?;
                if actual.address != rbp.wrapping_add(0x50) || actual.words != a.xmm6 { return Err(Fault::XmmBinding); }
                self.phase = Phase::StoreFrame48;
            }
            Phase::StoreFrame48 => {
                self.services.ctor_entry_store_frame48(c.rbp.ok_or(Fault::MissingReceipt)?, u64::MAX - 1).map_err(Fault::Service)?;
                c.frame48_store_returned = true; self.index = 0; self.phase = Phase::BindRegister;
            }
            Phase::BindRegister => {
                let destinations = [Register::Rbx, Register::Rsi, Register::Rdi, Register::R14];
                let sources = [Source::R9, Source::R8, Source::Rdx, Source::Rcx];
                let pcs = [0x2b3027, 0x2b302a, 0x2b302d, 0x2b3030];
                let values = [a.r9, a.r8, a.rdx, a.rcx];
                if self.index >= 4 { return Err(Fault::MissingReceipt); }
                self.services.ctor_entry_move_register(pcs[self.index], destinations[self.index], sources[self.index],
                    &mut c.moves[self.index]).map_err(Fault::Service)?;
                if c.moves[self.index].ok_or(Fault::MissingReceipt)? != values[self.index] { return Err(Fault::RegisterBinding); }
                self.index += 1;
                if self.index == 4 { self.phase = Phase::BaseCallRequired; }
            }
            Phase::BaseCallRequired => {}
        }
        Ok(())
    }
}
