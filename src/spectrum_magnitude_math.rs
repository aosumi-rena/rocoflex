use std::convert::Infallible;
use crate::fft::{SourceFftMath, SourceFftFailure, SourceFftDirection};
use crate::source_scalar::{ExplicitExceptionState, SourceCvttMxcsrCapabilities,
    sqrtss_with_raw_mxcsr, SourceRawSqrtssOutcome, SourceRawSqrtssError, PowerError};
use crate::source_scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture,
    ScalarF32InstructionSet, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome,
    ScalarF32Mxcsr, ScalarF32Error};
use crate::source_scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy, Debug)]
pub enum SourceMagnitudeStage { RealSquare, ImaginarySquare, Add, SquareRoot }

#[derive(Clone, Copy, Debug)]
pub struct SourceMagnitudeAttempt {
    pub bin: usize, pub real_bits: u32, pub imaginary_bits: u32, pub stage: SourceMagnitudeStage,
    pub real_square_bits: Option<u32>, pub imaginary_square_bits: Option<u32>,
    pub sum_bits: Option<u32>, pub output_bits: Option<u32>,
    pub binary_reported: [Option<ScalarF32Outcome>; 3],
    pub sqrt_reported: Option<Result<SourceRawSqrtssOutcome, SourceRawSqrtssError>>,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub known_advanced_raw_mxcsr: u32, pub reported_state_error: Option<PowerError>,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceWindowCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 }, ReportedScalar(SourceScalarF32Projection),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceWindowAttempt {
    pub index: usize, pub sample_bits: u32, pub coefficient_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub reported: Option<ScalarF32Outcome>, pub output_bits: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceSpectrumMathCause {
    Window(SourceWindowCause), Fft(SourceFftFailure), Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), Sqrt(SourceRawSqrtssError),
    ReportedSqrtState { reported_mxcsr: u32, reason: PowerError },
    TrappedSqrt(SourceRawSqrtssOutcome),
}
pub struct SourceSpectrumMathFailure {
    cause: SourceSpectrumMathCause, magnitude_attempt: Option<SourceMagnitudeAttempt>,
    window_attempt: Option<SourceWindowAttempt>,
}
impl SourceSpectrumMathFailure {
    pub fn cause(&self) -> SourceSpectrumMathCause { self.cause }
    pub fn magnitude_attempt(&self) -> Option<&SourceMagnitudeAttempt> { self.magnitude_attempt.as_ref() }
    pub fn window_attempt(&self) -> Option<&SourceWindowAttempt> { self.window_attempt.as_ref() }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceSpectrumMathBindingCause {
    FftAlreadyFailed(SourceFftFailure), Architecture, Controls,
    Capabilities { mxcsr: u32, supported_bits: u32 }, Scalar(ScalarF32Error),
}
pub struct SourceSpectrumMathBindingFailure {
    fft: SourceFftMath, architecture: ScalarF32Architecture, entry: ExplicitExceptionState,
    capabilities: SourceCvttMxcsrCapabilities, cause: SourceSpectrumMathBindingCause,
}
impl SourceSpectrumMathBindingFailure {
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn capabilities(&self) -> SourceCvttMxcsrCapabilities { self.capabilities }
    pub fn cause(&self) -> SourceSpectrumMathBindingCause { self.cause }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceSpectrumWindowBindingCause {
    Magnitude(SourceSpectrumMathBindingCause), WindowControls, WindowScalar(ScalarF32Error),
}
pub struct SourceSpectrumWindowBindingFailure {
    fft: SourceFftMath, architecture: ScalarF32Architecture,
    window_entry: ExplicitExceptionState, magnitude_entry: ExplicitExceptionState,
    capabilities: SourceCvttMxcsrCapabilities, cause: SourceSpectrumWindowBindingCause,
}
impl SourceSpectrumWindowBindingFailure {
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn window_entry(&self) -> ExplicitExceptionState { self.window_entry }
    pub fn magnitude_entry(&self) -> ExplicitExceptionState { self.magnitude_entry }
    pub fn capabilities(&self) -> SourceCvttMxcsrCapabilities { self.capabilities }
    pub fn cause(&self) -> SourceSpectrumWindowBindingCause { self.cause }
}

pub(crate) fn host_window(_: usize, sample: f32, coefficient: f32) -> Result<f32, Infallible> {
    Ok(sample * coefficient)
}
struct WindowLeaf {
    state: ExplicitExceptionState, scalar: SourceScalarF32, latest: Option<SourceWindowAttempt>,
}
impl WindowLeaf {
    fn apply(&mut self, index: usize, sample: f32, coefficient: f32) -> Result<f32, SourceWindowCause> {
        let mut attempt = SourceWindowAttempt { index, sample_bits: sample.to_bits(), coefficient_bits: coefficient.to_bits(),
            entry: self.state, known_advanced: self.state, reported: None, output_bits: None };
        let result = (|| {
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceWindowCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            let reported = self.scalar.execute(ScalarF32Operation::Mulss {
                left: attempt.sample_bits, right: attempt.coefficient_bits }, before).map_err(SourceWindowCause::Scalar)?;
            attempt.reported = Some(reported);
            let projection = project_scalar_f32_outcome(self.state, reported);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => {
                    self.state = advanced; attempt.known_advanced = advanced; attempt.output_bits = Some(bits);
                    Ok(f32::from_bits(bits))
                }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; attempt.known_advanced = advanced;
                    Err(SourceWindowCause::ReportedScalar(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; attempt.known_advanced = known_advanced;
                    Err(SourceWindowCause::ReportedScalar(projection))
                }
            }
        })();
        self.latest = Some(attempt);
        result
    }
}

#[derive(Clone, Copy)]
pub(crate) enum MagnitudeBinaryStage { RealSquare, ImaginarySquare, Add }
pub(crate) trait MagnitudeOps {
    type Error;
    fn begin(&mut self, bin: usize, re: u32, im: u32) -> Result<(), Self::Error>;
    fn binary(&mut self, stage: MagnitudeBinaryStage, left: u32, right: u32) -> Result<u32, Self::Error>;
    fn square_root(&mut self, sum: u32) -> Result<u32, Self::Error>;
}
pub(crate) fn magnitude_with<O: MagnitudeOps>(ops: &mut O, bin: usize, re: f32, im: f32)
    -> Result<f32, O::Error>
{
    let re = re.to_bits(); let im = im.to_bits();
    ops.begin(bin, re, im)?;
    let re2 = ops.binary(MagnitudeBinaryStage::RealSquare, re, re)?;
    let im2 = ops.binary(MagnitudeBinaryStage::ImaginarySquare, im, im)?;
    let sum = ops.binary(MagnitudeBinaryStage::Add, re2, im2)?;
    ops.square_root(sum).map(f32::from_bits)
}
struct HostMagnitude;
impl MagnitudeOps for HostMagnitude {
    type Error = Infallible;
    fn begin(&mut self, _: usize, _: u32, _: u32) -> Result<(), Infallible> { Ok(()) }
    fn binary(&mut self, stage: MagnitudeBinaryStage, left: u32, right: u32) -> Result<u32, Infallible> {
        let left = f32::from_bits(left); let right = f32::from_bits(right);
        Ok(match stage {
            MagnitudeBinaryStage::Add => (left + right).to_bits(),
            MagnitudeBinaryStage::RealSquare | MagnitudeBinaryStage::ImaginarySquare => (left * right).to_bits(),
        })
    }
    fn square_root(&mut self, sum: u32) -> Result<u32, Infallible> { Ok(f32::from_bits(sum).sqrt().to_bits()) }
}
pub(crate) fn host_magnitude(bin: usize, re: f32, im: f32) -> Result<f32, Infallible> {
    magnitude_with(&mut HostMagnitude, bin, re, im)
}

struct MagnitudeLeaf {
    architecture: ScalarF32Architecture, state: ExplicitExceptionState,
    capabilities: SourceCvttMxcsrCapabilities, scalar: SourceScalarF32,
    latest: Option<SourceMagnitudeAttempt>,
}
impl MagnitudeLeaf {
    fn adopt_raw(&mut self, word: u32) -> Result<(), SourceSpectrumMathCause> {
        let attempt = self.latest.as_mut().expect("magnitude attempt precedes its operations");
        attempt.known_advanced_raw_mxcsr = word;
        match ExplicitExceptionState::from_explicit_parts(word, self.state.errno()) {
            Ok(state) => { self.state = state; attempt.known_advanced = state; Ok(()) }
            Err(reason) => {
                attempt.reported_state_error = Some(reason);
                Err(SourceSpectrumMathCause::ReportedSqrtState { reported_mxcsr: word, reason })
            }
        }
    }
}
impl MagnitudeOps for MagnitudeLeaf {
    type Error = SourceSpectrumMathCause;
    fn begin(&mut self, bin: usize, real_bits: u32, imaginary_bits: u32) -> Result<(), Self::Error> {
        self.latest = Some(SourceMagnitudeAttempt { bin, real_bits, imaginary_bits,
            stage: SourceMagnitudeStage::RealSquare, real_square_bits: None, imaginary_square_bits: None,
            sum_bits: None, output_bits: None, binary_reported: [None; 3], sqrt_reported: None,
            entry: self.state, known_advanced: self.state, known_advanced_raw_mxcsr: self.state.mxcsr(),
            reported_state_error: None });
        Ok(())
    }
    fn binary(&mut self, stage: MagnitudeBinaryStage, left: u32, right: u32) -> Result<u32, Self::Error> {
        let attempt = self.latest.as_mut().expect("magnitude attempt precedes its operations");
        let (index, operation) = match stage {
            MagnitudeBinaryStage::RealSquare => { attempt.stage = SourceMagnitudeStage::RealSquare; (0, ScalarF32Operation::Mulss { left, right }) }
            MagnitudeBinaryStage::ImaginarySquare => { attempt.stage = SourceMagnitudeStage::ImaginarySquare; (1, ScalarF32Operation::Mulss { left, right }) }
            MagnitudeBinaryStage::Add => { attempt.stage = SourceMagnitudeStage::Add; (2, ScalarF32Operation::Addss { left, right }) }
        };
        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(SourceSpectrumMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
        let reported = self.scalar.execute(operation, before).map_err(SourceSpectrumMathCause::Scalar)?;
        attempt.binary_reported[index] = Some(reported);
        let reported_word = match reported {
            ScalarF32Outcome::Completed { state, .. } | ScalarF32Outcome::Trapped { state, .. } => state.word(),
        };
        attempt.known_advanced_raw_mxcsr = reported_word;
        let projection = project_scalar_f32_outcome(self.state, reported);
        let bits = match projection {
            SourceScalarF32Projection::Completed { bits, advanced } => {
                self.state = advanced; attempt.known_advanced = advanced; bits
            }
            SourceScalarF32Projection::Trapped { advanced, .. } => {
                self.state = advanced; attempt.known_advanced = advanced;
                return Err(SourceSpectrumMathCause::ReportedScalar(projection));
            }
            SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                self.state = known_advanced; attempt.known_advanced = known_advanced;
                return Err(SourceSpectrumMathCause::ReportedScalar(projection));
            }
        };
        match stage {
            MagnitudeBinaryStage::RealSquare => attempt.real_square_bits = Some(bits),
            MagnitudeBinaryStage::ImaginarySquare => attempt.imaginary_square_bits = Some(bits),
            MagnitudeBinaryStage::Add => attempt.sum_bits = Some(bits),
        }
        Ok(bits)
    }
    fn square_root(&mut self, sum: u32) -> Result<u32, Self::Error> {
        self.latest.as_mut().expect("magnitude attempt precedes its operations").stage = SourceMagnitudeStage::SquareRoot;
        let entry = self.state.mxcsr();
        let reported = sqrtss_with_raw_mxcsr(sum, entry, self.capabilities, self.architecture);
        self.latest.as_mut().expect("magnitude attempt precedes its operations").sqrt_reported = Some(reported);
        match reported {
            Ok(SourceRawSqrtssOutcome::Completed { bits, state, .. }) => {
                self.adopt_raw(state.word())?;
                self.latest.as_mut().expect("magnitude attempt precedes its operations").output_bits = Some(bits);
                Ok(bits)
            }
            Ok(trapped @ SourceRawSqrtssOutcome::Trapped { state, .. }) => {
                self.adopt_raw(state.word())?;
                Err(SourceSpectrumMathCause::TrappedSqrt(trapped))
            }
            Err(error) => {

                let _admission = self.adopt_raw(error.known_advanced_mxcsr(entry));
                Err(SourceSpectrumMathCause::Sqrt(error))
            }
        }
    }
}

fn bind_magnitude(fft: &SourceFftMath, architecture: ScalarF32Architecture,
    entry: ExplicitExceptionState, capabilities: SourceCvttMxcsrCapabilities)
    -> Result<MagnitudeLeaf, SourceSpectrumMathBindingCause>
{
    if let Some(cause) = fft.failure() { return Err(SourceSpectrumMathBindingCause::FftAlreadyFailed(*cause)); }
    if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceSpectrumMathBindingCause::Architecture); }
    if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceSpectrumMathBindingCause::Controls); }
    if entry.mxcsr() & !capabilities.supported_bits() != 0 {
        return Err(SourceSpectrumMathBindingCause::Capabilities { mxcsr: entry.mxcsr(), supported_bits: capabilities.supported_bits() });
    }
    let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Mulss, ScalarF32Opcode::Addss])
        .map_err(SourceSpectrumMathBindingCause::Scalar)?;
    let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions).map_err(SourceSpectrumMathBindingCause::Scalar)?;
    Ok(MagnitudeLeaf { architecture, state: entry, capabilities, scalar, latest: None })
}

pub struct SourceSpectrumMath {
    fft: SourceFftMath, magnitude: MagnitudeLeaf, window: Option<WindowLeaf>, failure: Option<SourceSpectrumMathFailure>,
}
impl SourceSpectrumMath {
    pub fn from_explicit_parts(fft: SourceFftMath, architecture: ScalarF32Architecture,
        logical_magnitude_entry: ExplicitExceptionState, capabilities: SourceCvttMxcsrCapabilities)
        -> Result<Self, SourceSpectrumMathBindingFailure>
    {
        let magnitude = match bind_magnitude(&fft, architecture, logical_magnitude_entry, capabilities) {
            Ok(magnitude) => magnitude,
            Err(cause) => return Err(SourceSpectrumMathBindingFailure { fft, architecture, entry: logical_magnitude_entry, capabilities, cause }),
        };
        Ok(Self { fft, magnitude, window: None, failure: None })
    }

    pub fn from_explicit_window_parts(fft: SourceFftMath, architecture: ScalarF32Architecture,
        logical_window_entry: ExplicitExceptionState, logical_magnitude_entry: ExplicitExceptionState,
        capabilities: SourceCvttMxcsrCapabilities) -> Result<Self, SourceSpectrumWindowBindingFailure>
    {
        let bind = || {
            let magnitude = bind_magnitude(&fft, architecture, logical_magnitude_entry, capabilities)
                .map_err(SourceSpectrumWindowBindingCause::Magnitude)?;
            if logical_window_entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceSpectrumWindowBindingCause::WindowControls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Mulss])
                .map_err(SourceSpectrumWindowBindingCause::WindowScalar)?;
            let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
                .map_err(SourceSpectrumWindowBindingCause::WindowScalar)?;
            Ok((magnitude, WindowLeaf { state: logical_window_entry, scalar, latest: None }))
        };
        let (magnitude, window) = match bind() {
            Ok(parts) => parts,
            Err(cause) => return Err(SourceSpectrumWindowBindingFailure { fft, architecture,
                window_entry: logical_window_entry, magnitude_entry: logical_magnitude_entry, capabilities, cause }),
        };
        Ok(Self { fft, magnitude, window: Some(window), failure: None })
    }
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn architecture(&self) -> ScalarF32Architecture { self.magnitude.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.magnitude.state }
    pub fn capabilities(&self) -> SourceCvttMxcsrCapabilities { self.magnitude.capabilities }
    pub fn window_exception_state(&self) -> Option<ExplicitExceptionState> { self.window.as_ref().map(|window| window.state) }

    pub fn latest_window_attempt(&self) -> Option<&SourceWindowAttempt> { self.window.as_ref().and_then(|window| window.latest.as_ref()) }

    pub fn latest_attempt(&self) -> Option<&SourceMagnitudeAttempt> { self.magnitude.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourceSpectrumMathFailure> { self.failure.as_ref() }
    pub(crate) fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> Result<f32, SourceSpectrumMathCause> {
        if let Some(failure) = &self.failure { return Err(failure.cause); }
        let Some(window) = self.window.as_mut() else {
            return match host_window(index, sample, coefficient) { Ok(value) => Ok(value), Err(never) => match never {} };
        };
        match window.apply(index, sample, coefficient) {
            Ok(value) => Ok(value),
            Err(error) => {
                let cause = SourceSpectrumMathCause::Window(error);
                self.failure = Some(SourceSpectrumMathFailure { cause, magnitude_attempt: None, window_attempt: window.latest });
                Err(cause)
            }
        }
    }
    pub(crate) fn transform_forward(&mut self, values: &mut [f32]) -> Result<(), SourceSpectrumMathCause> {
        if let Some(failure) = &self.failure { return Err(failure.cause); }
        if let Err(error) = self.fft.transform_in_place(SourceFftDirection::ForwardPlus1, values) {
            let cause = SourceSpectrumMathCause::Fft(error);
            self.failure = Some(SourceSpectrumMathFailure { cause, magnitude_attempt: None, window_attempt: None });
            return Err(cause);
        }
        Ok(())
    }
    pub(crate) fn magnitude(&mut self, bin: usize, re: f32, im: f32) -> Result<f32, SourceSpectrumMathCause> {
        if let Some(failure) = &self.failure { return Err(failure.cause); }
        match magnitude_with(&mut self.magnitude, bin, re, im) {
            Ok(value) => Ok(value),
            Err(cause) => {
                self.failure = Some(SourceSpectrumMathFailure { cause, magnitude_attempt: self.magnitude.latest, window_attempt: None });
                Err(cause)
            }
        }
    }
}
