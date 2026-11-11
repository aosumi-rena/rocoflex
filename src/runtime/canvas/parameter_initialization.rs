#[path = "parameter_base_call.rs"]
pub mod base_call;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub actual_image_base: u64, pub actual_rbp: u64,
    pub saved_rdi_source: u64, pub saved_rsi_source: u64,
}
pub struct Input<P, A> { pub range: super::Input<P, A>, pub binding: Binding }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Increment { pub before: u32, pub after: u32 }
pub trait Services: super::Services {

    fn init_write16(&mut self, pc: u32, parameter: &mut Self::Parameter, offset: u32,
        words: [u32; 4]) -> Result<(), Self::Error>;
    fn init_write_word(&mut self, pc: u32, parameter: &mut Self::Parameter, offset: u32,
        word: u64) -> Result<(), Self::Error>;
    fn init_read_word(&mut self, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn init_read_dword(&mut self, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn init_read_byte(&mut self, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn init_write_dword(&mut self, pc: u32, parameter: &mut Self::Parameter, offset: u32,
        bits: u32) -> Result<(), Self::Error>;
    fn init_write_byte(&mut self, pc: u32, parameter: &mut Self::Parameter, offset: u32,
        byte: u8) -> Result<(), Self::Error>;

    fn init_lock_inc(&mut self, pc: u32, header: u64,
        returned: &mut Option<Increment>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub base_call: Option<base_call::History>,
    pub zero58_returned: bool, pub zero48_returned: bool, pub base_vtable_returned: bool,
    pub strings: [Option<u64>; 3], pub string_store_returned: [bool; 3],
    pub headers: [Option<u64>; 3], pub sentinel_skip: [bool; 3],
    pub increments: [Option<Increment>; 3], pub attributes_word: Option<u64>,
    pub attribute_dword: Option<u32>, pub attribute_dword_store_returned: bool,
    pub flags: [Option<u8>; 3], pub flag_store_returned: [bool; 3],
    pub derived_vtable_returned: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub binding: Binding, pub captures: Captures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Zero58, Zero48, StoreBaseVtable, ReadString, StoreString, RetainString,
    ReadAttributes, ReadAttributeDword, StoreAttributeDword, ReadFlag, StoreFlag,
    StoreDerivedVtable, RangeCopyRequired,
}
pub enum Fault<E> { Service(E), MissingState, IncrementBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, captures: Captures,
    phase: Phase, string: usize, flag: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Initialized<S>), Failed(Stopped<S>) }
pub struct Initialized<S: Services> { parts: Parts<S> }
pub struct Parts<S: Services> {
    services: S, input: Input<S::Parameter, S::Argument>, pub captures: Captures,
    pub phase: Phase, pub string: usize, pub flag: usize,
    pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Parameter, S::Argument> { &self.input }
}
impl<S: Services> Initialized<S> {
    pub fn required_next_pc(&self) -> u32 { 0x2b30f1 }
    pub fn into_parts(self) -> Parts<S> { self.parts }

    pub fn into_range_copy(self) -> super::Prepared<S> {
        let parts = self.parts;
        let mut captures = super::Captures::default();
        captures.initialization = Some(History { binding: parts.input.binding, captures: parts.captures });
        super::Prepared { services: parts.services, input: parts.input.range, captures,
            phase: super::Phase::ReadWords, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_2b305e(services: S, input: Input<S::Parameter, S::Argument>) -> Self {
        Self { services, input, captures: Captures::default(), phase: Phase::Zero58,
            string: 0, flag: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, captures: self.captures,
            phase: self.phase, string: self.string, flag: self.flag, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::RangeCopyRequired && spent < budget {
            self.attempted = Some((self.phase, if matches!(self.phase, Phase::ReadFlag | Phase::StoreFlag) { self.flag } else { self.string }));
            spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::RangeCopyRequired { return Poll::Pending(self); }
        Poll::Ready(Initialized { parts: self.into_parts() })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let p = &mut self.input.range.parameter;
        let b = self.input.binding;
        let c = &mut self.captures;
        match self.phase {
            Phase::Zero58 => {

                self.services.init_write16(0x2b3061, p, 0x58, [0; 4]).map_err(Fault::Service)?;
                c.zero58_returned = true; self.phase = Phase::Zero48;
            }
            Phase::Zero48 => {
                self.services.init_write16(0x2b3066, p, 0x48, [0; 4]).map_err(Fault::Service)?;
                c.zero48_returned = true; self.phase = Phase::StoreBaseVtable;
            }
            Phase::StoreBaseVtable => {
                self.services.init_write_word(0x2b3072, p, 0, b.actual_image_base.wrapping_add(0x6270e8)).map_err(Fault::Service)?;
                c.base_vtable_returned = true; self.phase = Phase::ReadString;
            }
            Phase::ReadString => {
                if self.string >= 3 { return Err(Fault::MissingState); }
                let pc = [0x2b3075, 0x2b3096, 0x2b30a9][self.string];
                let source = match self.string { 0 => b.saved_rdi_source, 1 => b.saved_rsi_source,
                    _ => c.attributes_word.ok_or(Fault::MissingState)? };
                c.strings[self.string] = Some(self.services.init_read_word(pc, source).map_err(Fault::Service)?);
                self.phase = Phase::StoreString;
            }
            Phase::StoreString => {
                let value = c.strings[self.string].ok_or(Fault::MissingState)?;
                self.services.init_write_word([0x2b3078, 0x2b3099, 0x2b30ac][self.string], p,
                    [0x68, 0x70, 0x78][self.string], value).map_err(Fault::Service)?;
                c.string_store_returned[self.string] = true; self.phase = Phase::RetainString;
            }
            Phase::RetainString => {
                let header = c.strings[self.string].ok_or(Fault::MissingState)?.wrapping_sub(0x10);
                c.headers[self.string] = Some(header);
                if header == b.actual_image_base.wrapping_add(0x839cc8) { c.sentinel_skip[self.string] = true; }
                else {
                    self.services.init_lock_inc([0x2b308c, 0x2b30a6, 0x2b30b9][self.string], header,
                        &mut c.increments[self.string]).map_err(Fault::Service)?;
                    let receipt = c.increments[self.string].ok_or(Fault::MissingState)?;
                    if receipt.after != receipt.before.wrapping_add(1) { return Err(Fault::IncrementBinding); }
                }
                self.phase = match self.string { 0 => Phase::ReadAttributes, 1 => Phase::ReadString, _ => Phase::ReadAttributeDword };
                self.string += 1;
            }
            Phase::ReadAttributes => {
                c.attributes_word = Some(self.services.init_read_word(0x2b308f, b.actual_rbp.wrapping_add(0xd8)).map_err(Fault::Service)?);
                self.phase = Phase::ReadString;
            }
            Phase::ReadAttributeDword => {
                let source = c.attributes_word.ok_or(Fault::MissingState)?.wrapping_add(8);
                c.attribute_dword = Some(self.services.init_read_dword(0x2b30bc, source).map_err(Fault::Service)?);
                self.phase = Phase::StoreAttributeDword;
            }
            Phase::StoreAttributeDword => {
                self.services.init_write_dword(0x2b30bf, p, 0x80, c.attribute_dword.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.attribute_dword_store_returned = true; self.phase = Phase::ReadFlag;
            }
            Phase::ReadFlag => {
                if self.flag >= 3 { return Err(Fault::MissingState); }
                let source = c.attributes_word.ok_or(Fault::MissingState)?.wrapping_add(0xc + self.flag as u64);
                c.flags[self.flag] = Some(self.services.init_read_byte([0x2b30c6, 0x2b30d1, 0x2b30dc][self.flag], source).map_err(Fault::Service)?);
                self.phase = Phase::StoreFlag;
            }
            Phase::StoreFlag => {
                self.services.init_write_byte([0x2b30ca, 0x2b30d5, 0x2b30e0][self.flag], p, 0x84 + self.flag as u32,
                    c.flags[self.flag].ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.flag_store_returned[self.flag] = true; self.flag += 1;
                self.phase = if self.flag == 3 { Phase::StoreDerivedVtable } else { Phase::ReadFlag };
            }
            Phase::StoreDerivedVtable => {
                self.services.init_write_word(0x2b30ee, p, 0, b.actual_image_base.wrapping_add(0x627188)).map_err(Fault::Service)?;
                c.derived_vtable_returned = true; self.phase = Phase::RangeCopyRequired;
            }
            Phase::RangeCopyRequired => {}
        }
        Ok(())
    }
}
