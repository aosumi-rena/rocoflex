use crate::source_scalar::{self as scalar, ExplicitExceptionState, PowerError,
    SourceCvttMxcsrCapabilities, SourceUcomissFlags, SourceRawUcomissOutcome,
    SourceRawUcomissError, SourceRawMaxssOutcome, SourceRawMaxssError,
    SourceRawMinssOutcome, SourceRawMinssError};
use scalar::selected_log::{LogArithmeticError, SourceLogWithState};
use scalar::shared_scalar_f32::{ScalarF32Architecture, ScalarF32InstructionSet,
    SourceScalarF32, ScalarF32Operation, ScalarF32Outcome, ScalarF32Error, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug)]
pub struct SourceInterpolationRemapConfiguration {
    pub upper_clamp_bits: u32,
    pub architecture: ScalarF32Architecture,
    pub instructions: ScalarF32InstructionSet,
    pub capabilities: SourceCvttMxcsrCapabilities,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInterpolationRemapBindingCause { Architecture, Controls, Backend(ScalarF32Error) }
pub struct SourceInterpolationRemapBindingFailure {
    configuration: SourceInterpolationRemapConfiguration,
    entry: ExplicitExceptionState,
    cause: SourceInterpolationRemapBindingCause,
}
impl SourceInterpolationRemapBindingFailure {
    pub fn configuration(&self) -> SourceInterpolationRemapConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceInterpolationRemapBindingCause { self.cause }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceInterpolationRemapTag { pub query: usize, pub index: i64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceInterpolationRemapCurrent {
    Wanted { query: usize }, Scan(SourceInterpolationRemapTag), Interpolation(SourceInterpolationRemapTag),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceInterpolationRemapStage { Begin, Operation { slot: usize, pc: u32 }, Complete }
#[derive(Clone, Copy, Debug)]
pub enum SourceInterpolationRemapOperands {
    Scalar(ScalarF32Operation), Pair { left: u32, right: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInterpolationRemapReport {
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    Divide(Result<SourceLogWithState, LogArithmeticError>),
    Compare(Result<SourceRawUcomissOutcome, SourceRawUcomissError>),
    Maximum(Result<SourceRawMaxssOutcome, SourceRawMaxssError>),
    Minimum(Result<SourceRawMinssOutcome, SourceRawMinssError>),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceInterpolationRemapAttempt {
    pub pc: u32,
    pub entry: ExplicitExceptionState,
    pub operands: SourceInterpolationRemapOperands,
    pub reported: SourceInterpolationRemapReport,

    pub raw_admission: Option<Result<ExplicitExceptionState, PowerError>>,
    pub known_advanced: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceInterpolationRemapWanted {
    pub query: usize,
    pub query_bits: u32,
    pub total_bits: u32,
    pub attempts: [Option<SourceInterpolationRemapAttempt>; 1],
    pub completed_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceInterpolationRemapScan {
    pub tag: SourceInterpolationRemapTag,
    pub wanted_bits: u32,
    pub next_cumulative_bits: u32,
    pub attempts: [Option<SourceInterpolationRemapAttempt>; 1],
    pub completed_flags: Option<SourceUcomissFlags>,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceInterpolationRemapInterpolation {
    pub tag: SourceInterpolationRemapTag,
    pub wanted_bits: u32,
    pub current_cumulative_bits: u32,
    pub next_cumulative_bits: u32,
    pub position_bits: u32,
    pub next_position_bits: u32,
    pub attempts: [Option<SourceInterpolationRemapAttempt>; 8],
    pub shifted: Option<u32>,
    pub divided: Option<u32>,
    pub fraction: Option<u32>,
    pub max_result: Option<u32>,
    pub clamped_fraction: Option<u32>,
    pub delta: Option<u32>,
    pub product: Option<u32>,
    pub completed_bits: Option<u32>,
}
pub struct SourceInterpolationRemapProgress {
    current: SourceInterpolationRemapCurrent,
    stage: SourceInterpolationRemapStage,
    wanted: Option<SourceInterpolationRemapWanted>,
    scan: Option<SourceInterpolationRemapScan>,
    interpolation: Option<SourceInterpolationRemapInterpolation>,
    wanted_reached: bool, scan_reached: bool, interpolation_reached: bool,
}
impl SourceInterpolationRemapProgress {
    pub fn current(&self) -> SourceInterpolationRemapCurrent { self.current }
    pub fn stage(&self) -> SourceInterpolationRemapStage { self.stage }

    pub fn wanted(&self) -> Option<&SourceInterpolationRemapWanted> { self.wanted.as_ref() }
    pub fn scan(&self) -> Option<&SourceInterpolationRemapScan> { self.scan.as_ref() }
    pub fn interpolation(&self) -> Option<&SourceInterpolationRemapInterpolation> { self.interpolation.as_ref() }
    pub fn wanted_reached(&self) -> bool { self.wanted_reached }
    pub fn scan_reached(&self) -> bool { self.scan_reached }
    pub fn interpolation_reached(&self) -> bool { self.interpolation_reached }
}
enum Packet {
    Wanted(SourceInterpolationRemapWanted), Scan(SourceInterpolationRemapScan),
    Interpolation(SourceInterpolationRemapInterpolation),
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInterpolationRemapCause {
    ScalarInputStateRequired { mxcsr: u32 }, Scalar(ScalarF32Error), ScalarTrapped,
    Divide(LogArithmeticError), Compare(SourceRawUcomissError), CompareTrapped,
    Maximum(SourceRawMaxssError), MaximumTrapped, Minimum(SourceRawMinssError), MinimumTrapped,
    RawStateAdmission(PowerError),
}
pub struct SourceInterpolationRemapFailure {
    current: SourceInterpolationRemapCurrent, slot: usize, pc: u32, cause: SourceInterpolationRemapCause,
}
impl SourceInterpolationRemapFailure {
    pub fn current(&self) -> SourceInterpolationRemapCurrent { self.current }
    pub fn slot(&self) -> usize { self.slot }
    pub fn pc(&self) -> u32 { self.pc }
    pub fn cause(&self) -> SourceInterpolationRemapCause { self.cause }
}
pub struct SourceInterpolationRemapStopped { _private: () }
pub struct SourceInterpolationRemapMath {
    configuration: SourceInterpolationRemapConfiguration,
    backend: SourceScalarF32,
    state: ExplicitExceptionState,
    progress: Option<SourceInterpolationRemapProgress>,
    failure: Option<SourceInterpolationRemapFailure>,
    active: bool,
}
impl SourceInterpolationRemapMath {
    pub fn from_explicit_parts(configuration: SourceInterpolationRemapConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceInterpolationRemapBindingFailure>
    {
        let cause = if configuration.architecture != ScalarF32Architecture::IntelSdm {
            Some(SourceInterpolationRemapBindingCause::Architecture)
        } else if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            Some(SourceInterpolationRemapBindingCause::Controls)
        } else { None };
        if let Some(cause) = cause {
            return Err(SourceInterpolationRemapBindingFailure { configuration, entry, cause });
        }
        let backend = match SourceScalarF32::from_explicit_architecture(configuration.architecture, configuration.instructions) {
            Ok(backend) => backend,
            Err(error) => return Err(SourceInterpolationRemapBindingFailure {
                configuration, entry, cause: SourceInterpolationRemapBindingCause::Backend(error) }),
        };
        Ok(Self { configuration, backend, state: entry, progress: None, failure: None, active: false })
    }
    pub fn configuration(&self) -> SourceInterpolationRemapConfiguration { self.configuration }

    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn backend(&self) -> &SourceScalarF32 { &self.backend }
    pub fn progress(&self) -> Option<&SourceInterpolationRemapProgress> { self.progress.as_ref() }
    pub fn failure(&self) -> Option<&SourceInterpolationRemapFailure> { self.failure.as_ref() }
    pub fn is_active(&self) -> bool { self.active }

    fn marker() -> SourceInterpolationRemapStopped { SourceInterpolationRemapStopped { _private: () } }
    fn begin(&mut self, packet: Packet) {
        let current = match &packet {
            Packet::Wanted(value) => SourceInterpolationRemapCurrent::Wanted { query: value.query },
            Packet::Scan(value) => SourceInterpolationRemapCurrent::Scan(value.tag),
            Packet::Interpolation(value) => SourceInterpolationRemapCurrent::Interpolation(value.tag),
        };
        let (wanted, scan, interpolation) = match self.progress.as_ref() {
            Some(prior) => (prior.wanted, prior.scan, prior.interpolation),
            None => (None, None, None),
        };
        let mut progress = SourceInterpolationRemapProgress { current, stage: SourceInterpolationRemapStage::Begin,
            wanted, scan, interpolation, wanted_reached: false, scan_reached: false, interpolation_reached: false };
        match packet {
            Packet::Wanted(value) => { progress.wanted = Some(value); progress.wanted_reached = true; }
            Packet::Scan(value) => { progress.scan = Some(value); progress.scan_reached = true; }
            Packet::Interpolation(value) => { progress.interpolation = Some(value); progress.interpolation_reached = true; }
        }
        self.progress = Some(progress);
        self.active = true;
    }
    fn stop(&mut self, slot: usize, pc: u32, cause: SourceInterpolationRemapCause) -> SourceInterpolationRemapStopped {
        if self.failure.is_none() {
            self.failure = Some(SourceInterpolationRemapFailure {
                current: self.progress.as_ref().unwrap().current, slot, pc, cause });
        }
        self.active = false;
        Self::marker()
    }
    fn step(&mut self, slot: usize, pc: u32) {
        self.progress.as_mut().unwrap().stage = SourceInterpolationRemapStage::Operation { slot, pc };
    }
    fn attempt_mut(&mut self, slot: usize) -> &mut SourceInterpolationRemapAttempt {
        let progress = self.progress.as_mut().unwrap();
        match progress.current {
            SourceInterpolationRemapCurrent::Wanted { .. } => progress.wanted.as_mut().unwrap().attempts[slot].as_mut().unwrap(),
            SourceInterpolationRemapCurrent::Scan(_) => progress.scan.as_mut().unwrap().attempts[slot].as_mut().unwrap(),
            SourceInterpolationRemapCurrent::Interpolation(_) => progress.interpolation.as_mut().unwrap().attempts[slot].as_mut().unwrap(),
        }
    }
    fn save(&mut self, slot: usize, pc: u32, operands: SourceInterpolationRemapOperands, reported: SourceInterpolationRemapReport) {
        let attempt = SourceInterpolationRemapAttempt { pc, entry: self.state, operands, reported,
            raw_admission: None, known_advanced: self.state };
        let progress = self.progress.as_mut().unwrap();
        match progress.current {
            SourceInterpolationRemapCurrent::Wanted { .. } => progress.wanted.as_mut().unwrap().attempts[slot] = Some(attempt),
            SourceInterpolationRemapCurrent::Scan(_) => progress.scan.as_mut().unwrap().attempts[slot] = Some(attempt),
            SourceInterpolationRemapCurrent::Interpolation(_) => progress.interpolation.as_mut().unwrap().attempts[slot] = Some(attempt),
        }
    }
    fn adopt_raw(&mut self, word: u32, slot: usize, pc: u32) -> Result<(), SourceInterpolationRemapStopped> {
        let admission = ExplicitExceptionState::from_explicit_parts(word, self.state.errno());
        self.attempt_mut(slot).raw_admission = Some(admission);
        match admission {
            Ok(advanced) => { self.state = advanced; self.attempt_mut(slot).known_advanced = advanced; Ok(()) }
            Err(reason) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::RawStateAdmission(reason))),
        }
    }
    fn complete(&mut self) {
        self.progress.as_mut().unwrap().stage = SourceInterpolationRemapStage::Complete;
        self.active = false;
    }
    fn scalar(&mut self, slot: usize, pc: u32, operation: ScalarF32Operation) -> Result<u32, SourceInterpolationRemapStopped> {
        self.step(slot, pc);
        let before = match ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()) {
            Some(before) => before,
            None => return Err(self.stop(slot, pc, SourceInterpolationRemapCause::ScalarInputStateRequired { mxcsr: self.state.mxcsr() })),
        };
        let reported = self.backend.execute(operation, before);
        self.save(slot, pc, SourceInterpolationRemapOperands::Scalar(operation), SourceInterpolationRemapReport::Scalar(reported));
        let word = match reported {
            Ok(ScalarF32Outcome::Completed { state, .. }) | Ok(ScalarF32Outcome::Trapped { state, .. }) => state.word(),

            Err(error) => return Err(self.stop(slot, pc, SourceInterpolationRemapCause::Scalar(error))),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(ScalarF32Outcome::Completed { bits, .. }) => Ok(bits),
            Ok(ScalarF32Outcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::ScalarTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::Scalar(error))),
        }
    }
    fn divide(&mut self, slot: usize, pc: u32, left: u32, right: u32) -> Result<u32, SourceInterpolationRemapStopped> {
        self.step(slot, pc);
        let reported = scalar::raw_divss_with_explicit_state(left, right, self.state, self.configuration.architecture);
        self.save(slot, pc, SourceInterpolationRemapOperands::Pair { left, right }, SourceInterpolationRemapReport::Divide(reported));
        let advanced = match reported {
            Ok(value) => value.exception_state,
            Err(error) => division_advanced(error, self.state),
        };
        self.state = advanced;
        self.attempt_mut(slot).known_advanced = advanced;
        match reported {
            Ok(value) => Ok(value.bits),
            Err(error) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::Divide(error))),
        }
    }
    fn compare_raw(&mut self, slot: usize, pc: u32, left: u32, right: u32)
        -> Result<SourceUcomissFlags, SourceInterpolationRemapStopped>
    {
        self.step(slot, pc);
        let reported = scalar::ucomiss_f32_with_raw_mxcsr(left, right, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceInterpolationRemapOperands::Pair { left, right }, SourceInterpolationRemapReport::Compare(reported));
        let word = match reported {
            Ok(SourceRawUcomissOutcome::Completed { state, .. }) | Ok(SourceRawUcomissOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawUcomissOutcome::Completed { flags, .. }) => Ok(flags),
            Ok(SourceRawUcomissOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::CompareTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::Compare(error))),
        }
    }
    fn maximum(&mut self, slot: usize, pc: u32, left: u32, right: u32) -> Result<u32, SourceInterpolationRemapStopped> {
        self.step(slot, pc);
        let reported = scalar::maxss_with_raw_mxcsr(left, right, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceInterpolationRemapOperands::Pair { left, right }, SourceInterpolationRemapReport::Maximum(reported));
        let word = match reported {
            Ok(SourceRawMaxssOutcome::Completed { state, .. }) | Ok(SourceRawMaxssOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawMaxssOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawMaxssOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::MaximumTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::Maximum(error))),
        }
    }
    fn minimum(&mut self, slot: usize, pc: u32, left: u32, right: u32) -> Result<u32, SourceInterpolationRemapStopped> {
        self.step(slot, pc);
        let reported = scalar::minss_with_raw_mxcsr(left, right, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceInterpolationRemapOperands::Pair { left, right }, SourceInterpolationRemapReport::Minimum(reported));
        let word = match reported {
            Ok(SourceRawMinssOutcome::Completed { state, .. }) | Ok(SourceRawMinssOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawMinssOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawMinssOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::MinimumTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceInterpolationRemapCause::Minimum(error))),
        }
    }

    pub(crate) fn wanted(&mut self, query: usize, query_bits: u32, total_bits: u32)
        -> Result<u32, SourceInterpolationRemapStopped>
    {
        if self.failure.is_some() || self.active { return Err(Self::marker()); }
        self.begin(Packet::Wanted(SourceInterpolationRemapWanted { query, query_bits, total_bits,
            attempts: [None; 1], completed_bits: None }));
        let bits = self.scalar(0, 0x3e7ba0, ScalarF32Operation::Mulss { left: query_bits, right: total_bits })?;
        self.progress.as_mut().unwrap().wanted.as_mut().unwrap().completed_bits = Some(bits);
        self.complete();
        Ok(bits)
    }

    pub(crate) fn compare(&mut self, tag: SourceInterpolationRemapTag, wanted_bits: u32, next_cumulative_bits: u32)
        -> Result<SourceUcomissFlags, SourceInterpolationRemapStopped>
    {
        if self.failure.is_some() || self.active { return Err(Self::marker()); }
        self.begin(Packet::Scan(SourceInterpolationRemapScan { tag, wanted_bits, next_cumulative_bits,
            attempts: [None; 1], completed_flags: None }));
        let flags = self.compare_raw(0, 0x3e7bc8, wanted_bits, next_cumulative_bits)?;
        self.progress.as_mut().unwrap().scan.as_mut().unwrap().completed_flags = Some(flags);
        self.complete();
        Ok(flags)
    }

    pub(crate) fn interpolate(&mut self, tag: SourceInterpolationRemapTag, wanted_bits: u32,
        current_cumulative_bits: u32, next_cumulative_bits: u32, position_bits: u32, next_position_bits: u32)
        -> Result<u32, SourceInterpolationRemapStopped>
    {
        if self.failure.is_some() || self.active { return Err(Self::marker()); }
        self.begin(Packet::Interpolation(SourceInterpolationRemapInterpolation { tag, wanted_bits,
            current_cumulative_bits, next_cumulative_bits, position_bits, next_position_bits,
            attempts: [None; 8], shifted: None, divided: None, fraction: None, max_result: None,
            clamped_fraction: None, delta: None, product: None, completed_bits: None }));
        let shifted = self.scalar(0, 0x3e7b5c, ScalarF32Operation::Subss { left: wanted_bits, right: current_cumulative_bits })?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().shifted = Some(shifted);
        let divided = self.divide(1, 0x3e7b60, shifted, next_cumulative_bits)?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().divided = Some(divided);
        let fraction = self.scalar(2, 0x3e7b67, ScalarF32Operation::Subss { left: divided, right: current_cumulative_bits })?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().fraction = Some(fraction);

        let maximum = self.maximum(3, 0x3e7b6b, fraction, 0x0000_0000)?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().max_result = Some(maximum);
        let clamped = self.minimum(4, 0x3e7b6f, maximum, self.configuration.upper_clamp_bits)?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().clamped_fraction = Some(clamped);
        let delta = self.scalar(5, 0x3e7b80, ScalarF32Operation::Subss { left: next_position_bits, right: position_bits })?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().delta = Some(delta);
        let product = self.scalar(6, 0x3e7b84, ScalarF32Operation::Mulss { left: delta, right: clamped })?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().product = Some(product);
        let bits = self.scalar(7, 0x3e7b88, ScalarF32Operation::Addss { left: product, right: position_bits })?;
        self.progress.as_mut().unwrap().interpolation.as_mut().unwrap().completed_bits = Some(bits);
        self.complete();
        Ok(bits)
    }
}

fn division_advanced(error: LogArithmeticError, entry: ExplicitExceptionState) -> ExplicitExceptionState {
    match error {
        LogArithmeticError::ArithmeticRequired { advanced, .. }
        | LogArithmeticError::ScalarServiceRequired { advanced, .. }
        | LogArithmeticError::UnexpectedScalarState { advanced, .. }
        | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
        LogArithmeticError::Dispatch(scalar::selected_log::Error::CallbackRequiredFromEntry { entry }) => entry.exception_state,
        LogArithmeticError::Dispatch(_) => entry,
    }
}
