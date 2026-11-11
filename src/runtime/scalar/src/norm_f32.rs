use super::finite_power::{ExplicitExceptionState, MathErrorCallbackCompletion};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::square_root_f64::{SourceRawSquareRootF64, SourceSqrtF64Error};
use super::vorbis_power::{Arithmetic, VorbisScalarError};

pub(super) struct SourceNormWideningPrefix {
    pub(super) classification_first: u64,
    pub(super) classification_second: u64,
    pub(super) arithmetic_first: u64,
    pub(super) arithmetic_second: u64,
    pub(super) state: ExplicitExceptionState,
}

pub(super) fn source_norm_widening_prefix(first: u32, second: u32, state: ExplicitExceptionState) -> SourceNormWideningPrefix {
    let (classification_first, state) = state.widen_f32_for_math_error(first);
    let (classification_second, state) = state.widen_f32_for_math_error(second);
    let (arithmetic_first, state) = state.widen_f32_for_math_error(first);
    let (arithmetic_second, state) = state.widen_f32_for_math_error(second);
    SourceNormWideningPrefix { classification_first, classification_second, arithmetic_first, arithmetic_second, state }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNormOverflowRequest {
    pub argument1_f32_bits: u32,
    pub argument2_f32_bits: u32,
    pub proposed_f32_bits: u32,
    pub edx: u32,
    pub r9d: u32,
    pub stack_arg5: u32,
    pub stack_arg6: u32,
    pub stack_arg9: u32,
    pub root_f64_bits: u64,
    pub advanced: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormF32Error {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    NonfinitePairRequired { argument1_bits: u32, argument2_bits: u32 },
    OverflowCallbackRequired(SourceNormOverflowRequest),
    Arithmetic(VorbisScalarError),
    SquareRoot { error: SourceSqrtF64Error, advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormStage { SecondSquare, FirstSquare, Sum, SquareRoot, Narrowing }
impl SourceNormStage {
    fn receipt_index(self) -> usize {
        match self { Self::SecondSquare => 0, Self::FirstSquare => 1, Self::Sum => 2,
            Self::SquareRoot => 3, Self::Narrowing => 4 }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormStepOperands { BinaryF64 { left: u64, right: u64 }, F64 { source: u64 } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormStepResult { F64Bits(u64), F32Bits(u32) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNormArithmeticAttempt {
    pub stage: SourceNormStage,
    pub operands: SourceNormStepOperands,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub result: Option<SourceNormStepResult>,
    pub error: Option<SourceNormF32Error>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNormWideningReceipt {
    pub classification_first: u64,
    pub classification_second: u64,
    pub arithmetic_first: u64,
    pub arithmetic_second: u64,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNormCallReceipt {
    pub argument1_bits: u32,
    pub argument2_bits: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub widening: Option<SourceNormWideningReceipt>,

    pub attempts: [Option<SourceNormArithmeticAttempt>; 5],
    pub last_stage: Option<SourceNormStage>,
    pub result_bits: Option<u32>,
    pub error: Option<SourceNormF32Error>,
}
impl SourceNormF32Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "579824 explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "579824 state is not RNE/all-masked/gradual",
            Self::NonfinitePairRequired { .. } => "579824 nonfinite pair requires its separate source path",
            Self::OverflowCallbackRequired(_) => "579824 root exceeds F32 maximum; actual596150 continuation required",
            Self::Arithmetic(error) => error.reason(),
            Self::SquareRoot { error, .. } => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::OverflowCallbackRequired(request) => request.advanced,
            Self::Arithmetic(error) => error.known_advanced_state(entry),
            Self::SquareRoot { advanced, .. } => advanced,
            _ => entry,
        }
    }
}

pub struct SourceNorm579824 {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_call: Option<SourceNormCallReceipt>,
    first_failure: Option<SourceNormCallReceipt>,
}
impl SourceNorm579824 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState) -> Result<Self, SourceNormF32Error> {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceNormF32Error::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceNormF32Error::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source579824/two-f32/finite/separate" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceNormCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceNormCallReceipt> { self.first_failure.as_ref() }
    pub fn norm_f32(&mut self, first: f32, second: f32) -> Result<f32, SourceNormF32Error> {
        self.norm_bits(first.to_bits(), second.to_bits()).map(f32::from_bits)
    }
    pub fn norm_bits(&mut self, first: u32, second: u32) -> Result<u32, SourceNormF32Error> {
        self.last_call = Some(SourceNormCallReceipt { argument1_bits: first, argument2_bits: second,
            entry: self.state, advanced: self.state, widening: None, attempts: [None; 5], last_stage: None,
            result_bits: None, error: None });
        let result = self.evaluate_source_bits(first, second);
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.advanced = self.state;
            match result {
                Ok(bits) => receipt.result_bits = Some(bits),
                Err(error) => {
                    receipt.error = Some(error);
                    if self.first_failure.is_none() { self.first_failure = Some(*receipt); }
                }
            }
        }
        result
    }

    fn evaluate_source_bits(&mut self, first: u32, second: u32) -> Result<u32, SourceNormF32Error> {
        if first & 0x7fff_ffff >= 0x7f80_0000 || second & 0x7fff_ffff >= 0x7f80_0000 {
            return Err(SourceNormF32Error::NonfinitePairRequired { argument1_bits: first, argument2_bits: second });
        }
        let entry = self.state;
        match self.graph(first, second) {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    fn begin_step(&mut self, stage: SourceNormStage, operands: SourceNormStepOperands, entry: ExplicitExceptionState) {
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.last_stage = Some(stage);
            receipt.attempts[stage.receipt_index()] = Some(SourceNormArithmeticAttempt { stage, operands, entry,
                advanced: entry, result: None, error: None });
        }
    }

    fn finish_step(&mut self, result: Result<SourceNormStepResult, SourceNormF32Error>, advanced: ExplicitExceptionState) {
        if let Some(step) = self.last_call.as_mut().and_then(|receipt| {
            let stage = receipt.last_stage?;
            receipt.attempts[stage.receipt_index()].as_mut()
        }) {
            match result {
                Ok(result) => { step.result = Some(result); step.advanced = advanced; }
                Err(error) => { step.error = Some(error); step.advanced = error.known_advanced_state(step.entry); }
            }
        }
    }

    fn graph(&mut self, first: u32, second: u32) -> Result<(u32, ExplicitExceptionState), SourceNormF32Error> {

        let prefix = source_norm_widening_prefix(first, second, self.state);
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.widening = Some(SourceNormWideningReceipt {
                classification_first: prefix.classification_first, classification_second: prefix.classification_second,
                arithmetic_first: prefix.arithmetic_first, arithmetic_second: prefix.arithmetic_second,
                entry: self.state, advanced: prefix.state });
        }
        let x = prefix.arithmetic_first;
        let y = prefix.arithmetic_second;
        let mut a = Arithmetic { state: prefix.state };
        self.begin_step(SourceNormStage::SecondSquare, SourceNormStepOperands::BinaryF64 { left: y, right: y }, a.state);
        let yy = a.multiply("579898/second square MULSD", y, y).map_err(SourceNormF32Error::Arithmetic);
        self.finish_step(yy.map(SourceNormStepResult::F64Bits), a.state);
        let yy = yy?;
        self.begin_step(SourceNormStage::FirstSquare, SourceNormStepOperands::BinaryF64 { left: x, right: x }, a.state);
        let xx = a.multiply("57989c/first square MULSD", x, x).map_err(SourceNormF32Error::Arithmetic);
        self.finish_step(xx.map(SourceNormStepResult::F64Bits), a.state);
        let xx = xx?;
        self.begin_step(SourceNormStage::Sum, SourceNormStepOperands::BinaryF64 { left: yy, right: xx }, a.state);
        let sum = a.add("5798a0/squares ADDSD", yy, xx).map_err(SourceNormF32Error::Arithmetic);
        self.finish_step(sum.map(SourceNormStepResult::F64Bits), a.state);
        let sum = sum?;
        self.begin_step(SourceNormStage::SquareRoot, SourceNormStepOperands::F64 { source: sum }, a.state);
        let mut square_root = match SourceRawSquareRootF64::bind(self.architecture, a.state)
            .map_err(|error| SourceNormF32Error::SquareRoot { error, advanced: a.state }) {
                Ok(service) => service,
                Err(error) => { self.finish_step(Err(error), a.state); return Err(error); }
            };
        let root = square_root.evaluate_bits(sum)
            .map_err(|error| SourceNormF32Error::SquareRoot { error, advanced: square_root.exception_state() });
        self.finish_step(root.map(SourceNormStepResult::F64Bits), square_root.exception_state());
        let root = root?;
        a.state = square_root.exception_state();

        if root > 0x47ef_ffff_e000_0000 {
            return Err(SourceNormF32Error::OverflowCallbackRequired(SourceNormOverflowRequest {
                argument1_f32_bits: first, argument2_f32_bits: second, proposed_f32_bits: 0x7f80_0000,
                edx: 0x18, r9d: 3, stack_arg5: 0x11, stack_arg6: 0x22, stack_arg9: 2,
                root_f64_bits: root, advanced: a.state,
            }));
        }

        self.begin_step(SourceNormStage::Narrowing, SourceNormStepOperands::F64 { source: root }, a.state);
        let narrowed = MathErrorCallbackCompletion { result_f64_bits: root, advanced: a.state }
            .into_rne_f32().map_err(|required| SourceNormF32Error::Arithmetic(VorbisScalarError::ConversionRequired(required)));
        match narrowed {
            Ok(narrowed) => self.finish_step(Ok(SourceNormStepResult::F32Bits(narrowed.bits)), narrowed.advanced),
            Err(error) => self.finish_step(Err(error), error.known_advanced_state(a.state)),
        }
        let narrowed = narrowed?;
        Ok((narrowed.bits, narrowed.advanced))
    }
}
