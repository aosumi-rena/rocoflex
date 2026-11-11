#[path = "parameter_range_construction.rs"]
pub mod construction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { Start, End, Interval, Skew }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionField { FromNormalized, Snap }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binary { Add, Subtract, Multiply, Divide, Minimum, Maximum }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call { Log5834c0, Exp57f8e0, Floor580388 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareOpcode { Ucomiss, CmpLessScalar, CmpLessEqualScalar }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comparison { pub unordered: bool, pub less: bool, pub equal: bool }

pub trait Services {
    type Parameter;
    type Range;
    type Function;
    type Error;

    fn range_virtual_90(&mut self, parameter: &mut Self::Parameter) -> Result<Self::Range, Self::Error>;

    fn read_field(&mut self, range: &mut Self::Range, field: Field) -> Result<u32, Self::Error>;
    fn symmetric_skew(&mut self, range: &mut Self::Range) -> Result<bool, Self::Error>;
    fn function(&mut self, range: &mut Self::Range, field: FunctionField)
        -> Result<Option<Self::Function>, Self::Error>;

    fn custom(&mut self, pc: u32, function: &mut Self::Function, arguments: &mut [u32; 3])
        -> Result<u32, Self::Error>;

    fn binary(&mut self, pc: u32, op: Binary, left: u32, right: u32) -> Result<u32, Self::Error>;

    fn compare(&mut self, pc: u32, opcode: CompareOpcode, left: u32, right: u32)
        -> Result<Comparison, Self::Error>;

    fn call(&mut self, pc: u32, call: Call, value: u32) -> Result<u32, Self::Error>;

    fn exchange_public_160(&mut self, parameter: &mut Self::Parameter, value: u32)
        -> Result<u32, Self::Error>;
    fn read_public_160(&mut self, parameter: &mut Self::Parameter) -> Result<u32, Self::Error>;

    fn tail_virtual_98(&mut self, parameter: &mut Self::Parameter, value: u32) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Range, Clamp, Forward, Symmetric, Ordinary, Snap, Done }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt { Range, Field(Field), Symmetric, Function(FunctionField),
    Binary(u32, Binary), Compare(u32, CompareOpcode), Call(u32, Call), Custom(u32),
    ExchangePublic, ReadPublic, TailVirtual98 }

pub struct Prepared<S: Services> {
    services: S,
    parameter: S::Parameter,
    range: Option<S::Range>,
    function: Option<S::Function>,
    arguments: Option<[u32; 3]>,
    lanes: [Option<u32>; 8],
    input: u32,
    phase: Phase,
    attempted: Option<Attempt>,
    halted: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Fault<E> { Service(E), UnwrittenLane(usize), RequiredRange, RequiredFunction,
    RequiredArguments, AlreadyStopped }
#[must_use = "conversion is not the setter; retain original service/parameter/range owners"]
pub struct Converted<S: Services> { owner: Prepared<S>, value: u32 }

pub struct Parts<S: Services> {
    pub services: S, pub parameter: S::Parameter, pub range: Option<S::Range>,
    pub function: Option<S::Function>, pub arguments: Option<[u32; 3]>,
    pub scalar_temporaries: [Option<u32>; 8], pub input: u32,
    pub phase: Phase, pub attempted: Option<Attempt>, pub halted: bool,
}

impl<S: Services> Prepared<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn parameter(&self) -> &S::Parameter { &self.parameter }
    pub fn range(&self) -> Option<&S::Range> { self.range.as_ref() }
    pub fn arguments(&self) -> Option<&[u32; 3]> { self.arguments.as_ref() }
    pub fn scalar_temporaries(&self) -> &[Option<u32>; 8] { &self.lanes }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Attempt> { self.attempted }
    pub fn into_owned_prefix(self) -> Parts<S> {
        Parts { services: self.services, parameter: self.parameter, range: self.range,
            function: self.function, arguments: self.arguments, scalar_temporaries: self.lanes,
            input: self.input, phase: self.phase, attempted: self.attempted, halted: self.halted }
    }
    pub fn at_2b2da0(services: S, parameter: S::Parameter, input: u32) -> Self {
        let mut lanes = [None; 8]; lanes[6] = Some(input);
        Self { services, parameter, range: None, function: None, arguments: None,
            lanes, input, phase: Phase::Range, attempted: None, halted: false }
    }
    pub fn run(mut self) -> Result<Converted<S>, Stopped<S>> {
        if self.halted { return Err(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        while self.phase != Phase::Done {
            if let Err(fault) = self.step() {
                self.halted = true; return Err(Stopped { pending: self, fault });
            }
        }
        match self.lane(6) {
            Ok(value) => Ok(Converted { owner: self, value }),
            Err(fault) => { self.halted = true; Err(Stopped { pending: self, fault }) }
        }
    }
    fn lane(&self, n: usize) -> Result<u32, Fault<S::Error>> {
        self.lanes[n].ok_or(Fault::UnwrittenLane(n))
    }
    fn field(&mut self, destination: usize, field: Field) -> Result<(), Fault<S::Error>> {
        self.attempted = Some(Attempt::Field(field));
        let range = self.range.as_mut().ok_or(Fault::RequiredRange)?;
        let value = self.services.read_field(range, field).map_err(Fault::Service)?;
        self.lanes[destination] = Some(value); Ok(())
    }
    fn binary(&mut self, pc: u32, destination: usize, op: Binary, left: u32, right: u32)
        -> Result<(), Fault<S::Error>> {
        self.attempted = Some(Attempt::Binary(pc, op));
        let value = self.services.binary(pc, op, left, right).map_err(Fault::Service)?;
        self.lanes[destination] = Some(value); Ok(())
    }
    fn compare(&mut self, pc: u32, opcode: CompareOpcode, left: u32, right: u32)
        -> Result<Comparison, Fault<S::Error>> {
        self.attempted = Some(Attempt::Compare(pc, opcode));
        self.services.compare(pc, opcode, left, right).map_err(Fault::Service)
    }
    fn call(&mut self, pc: u32, kind: Call) -> Result<(), Fault<S::Error>> {
        let value = self.lane(0)?;
        self.attempted = Some(Attempt::Call(pc, kind));
        let value = self.services.call(pc, kind, value).map_err(Fault::Service)?;
        self.lanes[0] = Some(value); Ok(())
    }
    fn function(&mut self, field: FunctionField) -> Result<bool, Fault<S::Error>> {
        self.attempted = Some(Attempt::Function(field));
        let range = self.range.as_mut().ok_or(Fault::RequiredRange)?;
        self.function = self.services.function(range, field).map_err(Fault::Service)?;
        Ok(self.function.is_some())
    }
    fn custom(&mut self, pc: u32, value: u32) -> Result<(), Fault<S::Error>> {

        self.field(1, Field::Start)?; self.field(2, Field::End)?;
        self.arguments = Some([self.lane(1)?, self.lane(2)?, value]);
        self.attempted = Some(Attempt::Custom(pc));
        let value = self.services.custom(pc, self.function.as_mut().ok_or(Fault::RequiredFunction)?,
            self.arguments.as_mut().ok_or(Fault::RequiredArguments)?).map_err(Fault::Service)?;
        self.lanes[0] = Some(value); Ok(())
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        const ONE: u32 = 0x3f80_0000;
        const HALF: u32 = 0x3f00_0000;
        match self.phase {
            Phase::Range => {
                self.attempted = Some(Attempt::Range);
                self.range = Some(self.services.range_virtual_90(&mut self.parameter).map_err(Fault::Service)?);
                self.phase = Phase::Clamp;
            }
            Phase::Clamp => {
                self.binary(0x2b2dd2, 0, Binary::Minimum, ONE, self.lane(6)?)?;
                let below = self.compare(0x2b2dd9, CompareOpcode::CmpLessScalar, self.lane(6)?, 0)?;
                self.lanes[6] = Some(if !below.unordered && below.less { 0 } else { self.lane(0)? });
                self.binary(0x2b2de1, 1, Binary::Maximum, 0, self.lane(6)?)?;
                self.phase = Phase::Forward;
            }
            Phase::Forward => {
                if self.function(FunctionField::FromNormalized)? {
                    self.custom(0x2b2e15, self.lane(1)?)?;
                    self.phase = Phase::Snap;
                } else {
                    self.attempted = Some(Attempt::Symmetric);
                    let range = self.range.as_mut().ok_or(Fault::RequiredRange)?;
                    self.phase = if self.services.symmetric_skew(range).map_err(Fault::Service)? {
                        Phase::Symmetric
                    } else { Phase::Ordinary };
                }
            }
            Phase::Symmetric => {
                self.binary(0x2b2e27, 1, Binary::Add, self.lane(1)?, self.lane(1)?)?;
                self.binary(0x2b2e2e, 0, Binary::Add, self.lane(1)?, 0xbf80_0000)?;
                let zero = self.compare(0x2b2e39, CompareOpcode::Ucomiss, self.lane(0)?, 0)?;
                let mut direct = !zero.unordered && zero.equal;
                if !direct {
                    self.field(1, Field::Skew)?;
                    let unit = self.compare(0x2b2e45, CompareOpcode::Ucomiss, self.lane(1)?, ONE)?;
                    direct = !unit.unordered && unit.equal;
                }
                if !direct {
                    self.lanes[6] = self.lanes[0];
                    self.lanes[0] = Some(self.lane(0)? & 0x7fff_ffff);
                    self.call(0x2b2e60, Call::Log5834c0)?;
                    self.field(1, Field::Skew)?;
                    self.binary(0x2b2e65, 0, Binary::Divide, self.lane(0)?, self.lane(1)?)?;
                    self.call(0x2b2e6a, Call::Exp57f8e0)?;
                    self.lanes[1] = self.lanes[0];
                    let sign = self.compare(0x2b2e82, CompareOpcode::CmpLessScalar, self.lane(6)?, 0)?;
                    self.lanes[0] = Some(self.lane(1)? ^ if !sign.unordered && sign.less { 0x8000_0000 } else { 0 });
                }
                self.field(1, Field::Start)?; self.field(2, Field::End)?;
                self.binary(0x2b2e9c, 2, Binary::Subtract, self.lane(2)?, self.lane(1)?)?;
                self.binary(0x2b2ea0, 2, Binary::Multiply, self.lane(2)?, HALF)?;
                self.binary(0x2b2ea8, 0, Binary::Add, self.lane(0)?, ONE)?;
                self.binary(0x2b2eb0, 0, Binary::Multiply, self.lane(0)?, self.lane(2)?)?;
                self.binary(0x2b2eb4, 0, Binary::Add, self.lane(0)?, self.lane(1)?)?;
                self.phase = Phase::Snap;
            }
            Phase::Ordinary => {
                let positive = self.compare(0x2b2ebd, CompareOpcode::Ucomiss, self.lane(1)?, 0)?;
                if !positive.unordered && !positive.less && !positive.equal {
                    self.field(0, Field::Skew)?;
                    let unit = self.compare(0x2b2ec7, CompareOpcode::Ucomiss, self.lane(0)?, ONE)?;
                    if unit.unordered || !unit.equal {
                        self.lanes[0] = self.lanes[1];
                        self.call(0x2b2ed5, Call::Log5834c0)?;
                        self.field(2, Field::Skew)?;
                        self.binary(0x2b2eda, 0, Binary::Divide, self.lane(0)?, self.lane(2)?)?;
                        self.call(0x2b2edf, Call::Exp57f8e0)?;
                        self.lanes[1] = self.lanes[0];
                    }
                }
                self.field(2, Field::Start)?; self.field(0, Field::End)?;
                self.binary(0x2b2ef0, 0, Binary::Subtract, self.lane(0)?, self.lane(2)?)?;
                self.binary(0x2b2ef4, 0, Binary::Multiply, self.lane(0)?, self.lane(1)?)?;
                self.binary(0x2b2ef8, 0, Binary::Add, self.lane(0)?, self.lane(2)?)?;
                self.phase = Phase::Snap;
            }
            Phase::Snap => {
                if self.function(FunctionField::Snap)? {
                    let value = self.lane(0)?;
                    self.custom(0x2b2f2f, value)?;
                    self.lanes[6] = self.lanes[0];
                } else {
                    self.field(6, Field::Start)?; self.field(7, Field::Interval)?;
                    let positive = self.compare(0x2b2f43, CompareOpcode::Ucomiss, self.lane(7)?, 0)?;
                    if !positive.unordered && !positive.less && !positive.equal {
                        self.binary(0x2b2f48, 0, Binary::Subtract, self.lane(0)?, self.lane(6)?)?;
                        self.binary(0x2b2f4c, 0, Binary::Divide, self.lane(0)?, self.lane(7)?)?;
                        self.binary(0x2b2f50, 0, Binary::Add, self.lane(0)?, HALF)?;
                        self.call(0x2b2f58, Call::Floor580388)?;
                        self.binary(0x2b2f5d, 0, Binary::Multiply, self.lane(0)?, self.lane(7)?)?;
                        self.binary(0x2b2f61, 0, Binary::Add, self.lane(0)?, self.lane(6)?)?;
                    }
                    let lower = self.compare(0x2b2f65, CompareOpcode::Ucomiss, self.lane(6)?, self.lane(0)?)?;
                    if lower.unordered || lower.less {
                        self.field(1, Field::End)?;
                        let bounds = self.compare(0x2b2f6f, CompareOpcode::Ucomiss, self.lane(6)?, self.lane(1)?)?;
                        if bounds.unordered || bounds.less {
                            let upper = self.compare(0x2b2f77, CompareOpcode::CmpLessEqualScalar, self.lane(1)?, self.lane(0)?)?;
                            self.lanes[6] = Some(if !upper.unordered && (upper.less || upper.equal) {
                                self.lane(1)?
                            } else { self.lane(0)? });
                        }
                    }
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}

pub struct SetterStopped<S: Services> {
    pub converted: Converted<S>, pub fault: Fault<S::Error>, pub attempted: Attempt,
    pub exchanged_old: Option<u32>, pub reread: Option<u32>,
}
pub struct SetterReturned<S: Services> {
    pub converted: Converted<S>, pub exchanged_old: u32, pub reread: u32,
}
impl<S: Services> Converted<S> {
    pub fn value(&self) -> u32 { self.value }
    pub fn owner(&self) -> &Prepared<S> { &self.owner }
    pub fn into_owned_prefix(self) -> Parts<S> { self.owner.into_owned_prefix() }

    pub fn continue_actual_setter(mut self) -> Result<SetterReturned<S>, SetterStopped<S>> {
        if self.owner.halted {
            return Err(SetterStopped { converted: self, fault: Fault::AlreadyStopped,
                attempted: Attempt::ExchangePublic, exchanged_old: None, reread: None });
        }

        self.owner.halted = true;
        let old = match self.owner.services.exchange_public_160(&mut self.owner.parameter, self.value) {
            Ok(old) => old,
            Err(error) => return Err(SetterStopped { converted: self, fault: Fault::Service(error),
                attempted: Attempt::ExchangePublic, exchanged_old: None, reread: None }),
        };
        let reread = match self.owner.services.read_public_160(&mut self.owner.parameter) {
            Ok(value) => value,
            Err(error) => return Err(SetterStopped { converted: self, fault: Fault::Service(error),
                attempted: Attempt::ReadPublic, exchanged_old: Some(old), reread: None }),
        };
        if let Err(error) = self.owner.services.tail_virtual_98(&mut self.owner.parameter, reread) {
            return Err(SetterStopped { converted: self, fault: Fault::Service(error), attempted: Attempt::TailVirtual98,
                exchanged_old: Some(old), reread: Some(reread) });
        }
        Ok(SetterReturned { converted: self, exchanged_old: old, reread })
    }
}
