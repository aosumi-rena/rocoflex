#[path = "parameter_range_argument.rs"]
pub mod argument;
#[path = "parameter_range_functions.rs"]
pub mod functions;
#[path = "parameter_initialization.rs"]
pub mod initialization;
#[path = "parameter_constructor_exception.rs"]
pub mod exception;
#[path = "parameter_base_retirement.rs"]
pub mod base_retirement;

pub trait Services {
    type Parameter;
    type Argument;
    type Error;

    fn read_argument16(&mut self, argument: &Self::Argument) -> Result<[u32; 4], Self::Error>;

    fn write_parameter88(&mut self, parameter: &mut Self::Parameter, words: [u32; 4])
        -> Result<(), Self::Error>;

    fn read_argument10(&mut self, argument: &Self::Argument) -> Result<u8, Self::Error>;

    fn write_parameter98(&mut self, parameter: &mut Self::Parameter, byte: u8)
        -> Result<(), Self::Error>;
}

pub struct Input<P, A> { pub parameter: P, pub argument: A }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadWords, StoreWords, ReadSymmetry, StoreSymmetry, FunctionMembersRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub initialization: Option<initialization::History>,
    pub words: Option<[u32; 4]>,
    pub words_store_returned: bool,
    pub symmetry_byte: Option<u8>,
    pub symmetry_store_returned: bool,
}
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S,
    input: Input<S::Parameter, S::Argument>,
    captures: Captures,
    phase: Phase,
    attempted: Option<Phase>,
    stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(RangeWritten<S>), Failed(Stopped<S>) }

pub struct RangeWritten<S: Services> { returned: FunctionMembersRequired<S> }
pub struct FunctionMembersRequired<S: Services> {
    pub services: S,
    pub parameter: S::Parameter,
    pub argument: S::Argument,
    pub captures: Captures,
}

pub struct Parts<S: Services> {
    services: S,
    input: Input<S::Parameter, S::Argument>,
    pub captures: Captures,
    pub phase: Phase,
    pub attempted: Option<Phase>,
    pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Parameter, S::Argument> { &self.input }
}
impl<S: Services> RangeWritten<S> {
    pub fn into_function_members_required(self) -> FunctionMembersRequired<S> { self.returned }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_2b30f1(services: S, input: Input<S::Parameter, S::Argument>) -> Self {
        Self { services, input, captures: Captures::default(), phase: Phase::ReadWords,
            attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn services(&self) -> &S { &self.services }
    pub fn input(&self) -> &Input<S::Parameter, S::Argument> { &self.input }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { services: self.services, input: self.input, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::FunctionMembersRequired && spent < budget {
            self.attempted = Some(self.phase);
            spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::FunctionMembersRequired { return Poll::Pending(self); }
        if !self.captures.words_store_returned || !self.captures.symmetry_store_returned {
            self.stopped = true;
            return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        }
        Poll::Ready(RangeWritten { returned: FunctionMembersRequired { services: self.services,
            parameter: self.input.parameter, argument: self.input.argument, captures: self.captures } })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::ReadWords => {
                self.captures.words = Some(self.services.read_argument16(&self.input.argument).map_err(Fault::Service)?);
                self.phase = Phase::StoreWords;
            }
            Phase::StoreWords => {
                let words = self.captures.words.ok_or(Fault::MissingState)?;
                self.services.write_parameter88(&mut self.input.parameter, words).map_err(Fault::Service)?;
                self.captures.words_store_returned = true; self.phase = Phase::ReadSymmetry;
            }
            Phase::ReadSymmetry => {
                self.captures.symmetry_byte = Some(self.services.read_argument10(&self.input.argument).map_err(Fault::Service)?);
                self.phase = Phase::StoreSymmetry;
            }
            Phase::StoreSymmetry => {
                let byte = self.captures.symmetry_byte.ok_or(Fault::MissingState)?;
                self.services.write_parameter98(&mut self.input.parameter, byte).map_err(Fault::Service)?;
                self.captures.symmetry_store_returned = true; self.phase = Phase::FunctionMembersRequired;
            }
            Phase::FunctionMembersRequired => {}
        }
        Ok(())
    }
}
