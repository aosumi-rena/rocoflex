use std::sync::OnceLock;

use crate::source_scalar as literal;
use literal::{ExplicitExceptionState, SourceF64Exp, ScalarF32Architecture};
use super::empty_background::literal::public_numeric as camera_values;

impl camera_values::Math for literal::SourceF64ValueMath {
    type Error = literal::SourceF64ValueError;
    fn operation(&mut self, op: camera_values::Operation) -> Result<u64, Self::Error> {
        match op {
            camera_values::Operation::WidenF32 { bits } => self.widen_f32_bits(bits),
            camera_values::Operation::Add { left, right } => self.add_bits(left, right),
            camera_values::Operation::Sub { left, right } => self.subtract_bits(left, right),
            camera_values::Operation::Mul { left, right } => self.multiply_bits(left, right),
            camera_values::Operation::Div { left, right } => self.divide_bits(left, right),
            camera_values::Operation::Min { first, second } => self.minimum_bits(first, second),
        }
    }
    fn ordered_less(&mut self, left: u64, right: u64) -> Result<bool, Self::Error> {
        self.ordered_less_bits(left, right)
    }
}
use literal::scalar_cpu_initializer::{
    SourceScalarCpuInitializer, SourceScalarCpuInitializerError, SourceScalarCpuRequest,
    SuppliedCpuidRecord, SuppliedScalarCpuRecord, SuppliedXgetbvRecord,
};

enum StartupIssue {
    UnsupportedArchitecture,
    Record(SourceScalarCpuInitializerError),
    Incomplete,
}

struct StartupFailure {

    cursor: SourceScalarCpuInitializer,
    issue: StartupIssue,
}

static STARTUP: OnceLock<Result<SourceScalarCpuInitializer, StartupFailure>> = OnceLock::new();

pub(super) enum Failure {
    StartupNotRun,
    Startup,
    FloatingPoint,
    State,
    Math {
        error: literal::SourceF64ExpError,
        advanced: ExplicitExceptionState,
        receipt: Option<literal::SourceF64ExpCallReceipt>,
    },
}

impl Failure {
    pub(super) fn message(&self) -> &'static str {
        match self {
            Self::StartupNotRun | Self::Startup => "Probe sizing could not initialize on this processor.",
            Self::FloatingPoint | Self::State => "Probe sizing requires supported floating-point settings.",
            Self::Math { .. } => "The source probe-size calculation could not complete.",
        }
    }
}

pub(super) fn initialize_current_process() {
    let _ = STARTUP.get_or_init(observe_initializer);
}

fn observe_initializer() -> Result<SourceScalarCpuInitializer, StartupFailure> {
    let mut cursor = SourceScalarCpuInitializer::begin_serial();
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    return Err(StartupFailure { cursor, issue: StartupIssue::UnsupportedArchitecture });
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{__cpuid_count, _xgetbv};
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{__cpuid_count, _xgetbv};

        loop {
            let record = match cursor.request() {
                Some(SourceScalarCpuRequest::Cpuid { eax, ecx }) => {
                    let output = __cpuid_count(eax, ecx);
                    SuppliedScalarCpuRecord::Cpuid(SuppliedCpuidRecord {
                        input_eax: eax, input_ecx: ecx,
                        eax: output.eax, ebx: output.ebx, ecx: output.ecx, edx: output.edx,
                    })
                }
                Some(SourceScalarCpuRequest::Xgetbv { ecx }) => {

                    let output = unsafe { _xgetbv(ecx) };
                    SuppliedScalarCpuRecord::Xgetbv(SuppliedXgetbvRecord {
                        input_ecx: ecx, eax: output as u32, edx: (output >> 32) as u32,
                    })
                }
                None => {
                    return if cursor.completion().is_some() { Ok(cursor) }
                    else { Err(StartupFailure { cursor, issue: StartupIssue::Incomplete }) };
                }
            };
            if let Err(error) = cursor.supply(record) {
                return Err(StartupFailure { cursor, issue: StartupIssue::Record(error) });
            }
        }
    }
}

pub(super) fn wheel_exp(exponent: f64) -> Result<f64, Failure> {
    let initializer = STARTUP.get().ok_or(Failure::StartupNotRun)?
        .as_ref().map_err(|_| Failure::Startup)?;
    let source_word = initializer.completion().ok_or(Failure::Startup)?.word_861cb8;
    let observed = super::layout::context::provider::check_current_thread()
        .map_err(|_| Failure::FloatingPoint)?;
    let entry = ExplicitExceptionState::from_explicit_parts(observed, 0)
        .map_err(|_| Failure::State)?;
    let mut service = SourceF64Exp::bind(source_word, ScalarF32Architecture::IntelSdm, entry)
        .map_err(|error| Failure::Math { error, advanced: error.known_advanced_state(entry), receipt: None })?;
    service.evaluate(exponent).map_err(|error| Failure::Math {
        error,
        advanced: error.known_advanced_state(entry),
        receipt: service.last_call().copied(),
    })
}

pub(super) enum BackgroundMathFailure {
    Controls,
    Startup,
    State { mxcsr: u32 },
    Source { error: literal::SourceAtan2F32Error, advanced: ExplicitExceptionState },
    RingOperand { bits: u32 },
    RingZero { error: literal::SourceRingTrigError, advanced: ExplicitExceptionState },
    RingNonzero { error: literal::ScalarTrigError, entry: ExplicitExceptionState },
    ProjectionOperand { bits: u32 },
    ProjectionZero { error: literal::SourceRingTrigError, advanced: ExplicitExceptionState },
    Norm {
        error: literal::SourceNormF32Error,
        advanced: ExplicitExceptionState,
        receipt: Option<literal::SourceNormCallReceipt>,
    },
    SquareRoot { error: literal::SourceSqrtF64Error, advanced: ExplicitExceptionState },
    DisplaySquareRoot { operand: u64 },
}

pub(super) struct BackgroundMath {
    service: literal::SourceAtan2F32,
    source_word: u32,
    failure: Option<BackgroundMathFailure>,
}

impl BackgroundMath {
    pub(super) fn source_word(&self) -> u32 { self.source_word }
    pub(super) fn exception_state(&self) -> ExplicitExceptionState { self.service.exception_state() }
    pub(super) fn failure(&self) -> Option<&BackgroundMathFailure> { self.failure.as_ref() }

    pub fn bind(mxcsr: u32) -> Result<Self, BackgroundMathFailure> {
        let source_word = STARTUP.get().and_then(|value| value.as_ref().ok())
            .and_then(|initializer| initializer.completion()).map(|value| value.word_861cb8)
            .ok_or(BackgroundMathFailure::Startup)?;

        let entry = ExplicitExceptionState::from_explicit_parts(mxcsr, 0)
            .map_err(|_| BackgroundMathFailure::State { mxcsr })?;
        let service = literal::SourceAtan2F32::bind(ScalarF32Architecture::IntelSdm, entry)
            .map_err(|error| BackgroundMathFailure::Source {
                error, advanced: error.known_advanced_state(entry),
            })?;
        Ok(Self { service, source_word, failure: None })
    }

    pub fn calculate(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()> {
        if self.failure.is_some() { return Err(()); }
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls);
            return Err(());
        }
        let entry = self.service.exception_state();
        let value = match self.service.atan2_f32(numerator, denominator) {
            Ok(value) => value,
            Err(error) => {
                self.failure = Some(BackgroundMathFailure::Source {
                    error, advanced: error.known_advanced_state(entry),
                });
                return Err(());
            }
        };
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls);
            return Err(());
        }
        Ok(value)
    }

    fn ring_pair(&self, theta: f32, entry: ExplicitExceptionState)
        -> Result<([f32; 2], ExplicitExceptionState), BackgroundMathFailure>
    {
        let bits = theta.to_bits();
        if bits != 0 && !(0x3e00_0000..0x4100_0000).contains(&bits) {
            return Err(BackgroundMathFailure::RingOperand { bits });
        }
        if self.source_word == 0 {
            let mut service = literal::SourceRingTrigWordZero::bind(
                self.source_word, ScalarF32Architecture::IntelSdm, entry,
            ).map_err(|error| BackgroundMathFailure::RingZero {
                error, advanced: error.known_advanced_state(entry),
            })?;
            let sin = service.sinf(theta).map_err(|error| BackgroundMathFailure::RingZero {
                error, advanced: error.known_advanced_state(entry),
            })?;
            let before_cos = service.exception_state();
            let cos = service.cosf(theta).map_err(|error| BackgroundMathFailure::RingZero {
                error, advanced: error.known_advanced_state(before_cos),
            })?;
            Ok(([sin, cos], service.exception_state()))
        } else {
            let service = literal::SourceCoefficientTrigNonzero::bind(self.source_word, entry.mxcsr())
                .map_err(|error| BackgroundMathFailure::RingNonzero { error, entry })?;
            let sin = service.sinf_with_explicit_state(bits, entry)
                .map_err(|error| BackgroundMathFailure::RingNonzero { error, entry })?;
            let before_cos = sin.exception_state;
            let cos = service.cosf_with_explicit_state(bits, before_cos)
                .map_err(|error| BackgroundMathFailure::RingNonzero { error, entry: before_cos })?;
            Ok(([f32::from_bits(sin.bits), f32::from_bits(cos.bits)], cos.exception_state))
        }
    }

    fn projection_value(&self, angle: f32, cosine: bool, entry: ExplicitExceptionState)
        -> Result<(f32, ExplicitExceptionState), BackgroundMathFailure>
    {
        let bits = angle.to_bits();
        let magnitude = bits & 0x7fff_ffff;
        if magnitude != 0 && !(0x3400_0000..0x4100_0000).contains(&magnitude) {
            return Err(BackgroundMathFailure::ProjectionOperand { bits });
        }
        if self.source_word == 0 {
            let mut service = literal::SourceProjectionTrigWordZero::bind(
                self.source_word, ScalarF32Architecture::IntelSdm, entry,
            ).map_err(|error| BackgroundMathFailure::ProjectionZero {
                error, advanced: error.known_advanced_state(entry),
            })?;
            let value = if cosine { service.cosf(angle) } else { service.sinf(angle) }
                .map_err(|error| BackgroundMathFailure::ProjectionZero {
                    error, advanced: error.known_advanced_state(entry),
                })?;
            Ok((value, service.exception_state()))
        } else {
            let service = literal::SourceCoefficientTrigNonzero::bind(self.source_word, entry.mxcsr())
                .map_err(|error| BackgroundMathFailure::RingNonzero { error, entry })?;
            let value = if cosine { service.cosf_with_explicit_state(bits, entry) }
                else { service.sinf_with_explicit_state(bits, entry) }
                .map_err(|error| BackgroundMathFailure::RingNonzero { error, entry })?;
            Ok((f32::from_bits(value.bits), value.exception_state))
        }
    }

    pub(super) fn install_advanced(&mut self, advanced: ExplicitExceptionState) -> Result<(), ()> {
        if self.failure.is_some() { return Err(()); }
        self.service = match literal::SourceAtan2F32::bind(ScalarF32Architecture::IntelSdm, advanced) {
            Ok(service) => service,
            Err(error) => {
                self.failure = Some(BackgroundMathFailure::Source {
                    error, advanced: error.known_advanced_state(advanced),
                }); return Err(());
            }
        };
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls); return Err(());
        }
        Ok(())
    }

    fn projection(&mut self, angle: f32, cosine: bool) -> Result<f32, ()> {
        if self.failure.is_some() { return Err(()); }
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls); return Err(());
        }
        let (value, advanced) = match self.projection_value(angle, cosine, self.service.exception_state()) {
            Ok(result) => result,
            Err(error) => { self.failure = Some(error); return Err(()); }
        };
        self.install_advanced(advanced)?;
        Ok(value)
    }

    fn norm_pair(&mut self, first: f32, second: f32) -> Result<f32, ()> {
        if self.failure.is_some() { return Err(()); }
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls); return Err(());
        }
        let entry = self.service.exception_state();
        let mut service = match literal::SourceNorm579824::bind(ScalarF32Architecture::IntelSdm, entry) {
            Ok(service) => service,
            Err(error) => {
                self.failure = Some(BackgroundMathFailure::Norm {
                    error, advanced: error.known_advanced_state(entry), receipt: None,
                }); return Err(());
            }
        };
        let value = match service.norm_f32(first, second) {
            Ok(value) => value,
            Err(error) => {

                self.failure = Some(BackgroundMathFailure::Norm {
                    error, advanced: error.known_advanced_state(entry),
                    receipt: service.last_call().copied(),
                }); return Err(());
            }
        };
        self.install_advanced(service.exception_state())?;
        Ok(value)
    }

    fn raw_square_root(&mut self, operand: u64) -> Result<u64, ()> {
        if self.failure.is_some() { return Err(()); }
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls); return Err(());
        }
        let entry = self.service.exception_state();
        let mut service = match literal::SourceRawSquareRootF64::bind(ScalarF32Architecture::IntelSdm, entry) {
            Ok(service) => service,
            Err(error) => {
                self.failure = Some(BackgroundMathFailure::SquareRoot {
                    error, advanced: error.known_advanced_state(entry),
                }); return Err(());
            }
        };
        let bits = match service.evaluate_bits(operand) {
            Ok(bits) => bits,
            Err(error) => {
                self.failure = Some(BackgroundMathFailure::SquareRoot {
                    error, advanced: error.known_advanced_state(entry),
                }); return Err(());
            }
        };
        self.install_advanced(service.exception_state())?;
        Ok(bits)
    }

    fn display_square_root(&mut self, operand: u64) -> Result<u64, ()> {
        if self.failure.is_some() { return Err(()); }
        let observed = match super::layout::context::provider::check_current_thread() {
            Ok(observed) => observed,
            Err(_) => { self.failure = Some(BackgroundMathFailure::Controls); return Err(()); }
        };
        let (bits, raised) = match super::background_sqrt::evaluate(operand, observed) {
            Ok(result) => result,
            Err(()) => { self.failure = Some(BackgroundMathFailure::DisplaySquareRoot { operand }); return Err(()); }
        };
        let entry = self.service.exception_state();
        let advanced = match ExplicitExceptionState::from_explicit_parts(entry.mxcsr() | raised, entry.errno()) {
            Ok(advanced) => advanced,
            Err(_) => { self.failure = Some(BackgroundMathFailure::State { mxcsr: entry.mxcsr() | raised }); return Err(()); }
        };
        self.install_advanced(advanced)?;
        Ok(bits)
    }

    fn ring(&mut self, theta: f32) -> Result<[f32; 2], ()> {
        if self.failure.is_some() { return Err(()); }
        if super::layout::context::provider::check_current_thread().is_err() {
            self.failure = Some(BackgroundMathFailure::Controls); return Err(());
        }
        let (pair, advanced) = match self.ring_pair(theta, self.service.exception_state()) {
            Ok(result) => result,
            Err(error) => { self.failure = Some(error); return Err(()); }
        };
        self.install_advanced(advanced)?;
        Ok(pair)
    }
}

impl super::empty_background::literal::radial::SqrtServices for BackgroundMath {
    fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()> { self.raw_square_root(operand) }
}
impl super::empty_background::literal::geometry::MathServices for BackgroundMath {
    fn atan2(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()> {
        self.calculate(numerator, denominator)
    }
    fn ring_sin_cos(&mut self, theta: f32) -> Result<[f32; 2], ()> { self.ring(theta) }
    fn projection_sin(&mut self, angle: f32) -> Result<f32, ()> { self.projection(angle, false) }
    fn projection_cos(&mut self, angle: f32) -> Result<f32, ()> { self.projection(angle, true) }
    fn norm(&mut self, first: f32, second: f32) -> Result<f32, ()> { self.norm_pair(first, second) }
}

pub(super) struct HostRadialDisplay<'a> { math: &'a mut BackgroundMath }
impl<'a> HostRadialDisplay<'a> {
    pub fn new(math: &'a mut BackgroundMath) -> Self { Self { math } }
}
impl super::empty_background::literal::prefix::ExpServices for HostRadialDisplay<'_> {
    fn radial_exp(&mut self, request: super::empty_background::literal::prefix::ExpRequest)
        -> Result<f32, ()> {
        use super::empty_background::literal::prefix::ExpServices;
        if self.math.failure().is_some() { return Err(()); }
        super::empty_background::literal::prefix::HostRadialExp.radial_exp(request)
    }
}
impl super::empty_background::literal::radial::SqrtServices for HostRadialDisplay<'_> {
    fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()> { self.math.display_square_root(operand) }
}
impl super::empty_background::literal::geometry::MathServices for HostRadialDisplay<'_> {
    fn atan2(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()> {
        self.math.calculate(numerator, denominator)
    }
    fn ring_sin_cos(&mut self, theta: f32) -> Result<[f32; 2], ()> { self.math.ring(theta) }
    fn projection_sin(&mut self, angle: f32) -> Result<f32, ()> { self.math.projection(angle, false) }
    fn projection_cos(&mut self, angle: f32) -> Result<f32, ()> { self.math.projection(angle, true) }
    fn norm(&mut self, first: f32, second: f32) -> Result<f32, ()> { self.math.norm_pair(first, second) }
}

pub(super) struct ModeledEmptyRing {
    pub rgb: [u8; 3],

    pub state: ExplicitExceptionState,
}

pub(super) enum EmptyRingFailure {
    State { mxcsr: u32, errno: i32 },
    Divide { error: literal::LogArithmeticError, entry: ExplicitExceptionState },
    UnexpectedConversion { bits: u32, state: ExplicitExceptionState },
}

pub(super) fn masked_empty_ring_rgb(mxcsr: u32) -> Result<ModeledEmptyRing, EmptyRingFailure> {
    let mut state = ExplicitExceptionState::from_explicit_parts(mxcsr, 0)
        .map_err(|_| EmptyRingFailure::State { mxcsr, errno: 0 })?;
    let mut rgb = [0; 3];
    for channel in [2, 1, 0] {
        let quotient = literal::raw_divss_with_explicit_state(
            0, 0, state, ScalarF32Architecture::IntelSdm,
        ).map_err(|error| EmptyRingFailure::Divide { error, entry: state })?;

        if quotient.bits != 0xffc0_0000 {
            return Err(EmptyRingFailure::UnexpectedConversion {
                bits: quotient.bits, state: quotient.exception_state,
            });
        }

        let integer = i32::MIN;
        let next_mxcsr = quotient.exception_state.mxcsr() | 1;
        let errno = quotient.exception_state.errno();
        state = ExplicitExceptionState::from_explicit_parts(next_mxcsr, errno)
            .map_err(|_| EmptyRingFailure::State { mxcsr: next_mxcsr, errno })?;
        rgb[channel] = integer as u8;
    }
    Ok(ModeledEmptyRing { rgb, state })
}
