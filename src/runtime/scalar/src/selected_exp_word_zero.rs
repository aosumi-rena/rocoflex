use super::finite_power::{ExplicitExceptionState, MathErrorServiceError, SourceMathErrorCallback};
use super::selected_exp::{SourceExpEntry, SourceExpPlan, SourceExpWithState,
    source_entry_plan, resolve_source_exp_range_with_callback};
use super::serial_exp::{self, SerialExpError, SerialExpSites};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::VorbisScalarError;

pub use super::serial_exp::SOURCE_RADIAL_EXP_UPPER_F64;

const SITES: SerialExpSites = SerialExpSites([
    "57f948/word0 count split MULPD.low", "57f959/word0 remainder SUBPD.low",
    "57f96e/word0 polynomial MULPD.low", "57f972/word0 square MULPD.low",
    "57f976/word0 polynomial ADDPD.low", "57f97e/word0 correction MULPD.low",
    "57f982/word0 reduced ADDPD.low", "57f997/word0 table MULPD.low",
    "57f99b/word0 table ADDPD.low", "57f9a4/word0 binary scale MULPD.low",
]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceWordZeroExpError {
    WordZeroRequired { supplied: u32 },
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    UpperLaneContinuationRequired { bits: u64, entry: SourceExpEntry },
    CallbackRequiredFromEntry { entry: SourceExpEntry },
    Arithmetic(VorbisScalarError),
    MathError(MathErrorServiceError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceWordZeroExpCallKind { ValueOnly, WithExplicitCallbackService }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWordZeroExpCallReceipt {
    pub kind: SourceWordZeroExpCallKind,
    pub input_bits: u32,
    pub original_upper_f64_bits: u64,
    pub entry: ExplicitExceptionState,
    pub source_entry: SourceExpEntry,
    pub advanced: ExplicitExceptionState,
    pub result_bits: Option<u32>,
    pub error: Option<SourceWordZeroExpError>,
}

impl SourceWordZeroExpError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::WordZeroRequired { .. } => "57f8e0 packed SSE body requires explicitly supplied word zero",
            Self::ArchitectureNotClosed => "word-zero exp arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "word-zero exp state is not RNE/all-masked/gradual",
            Self::UpperLaneContinuationRequired { .. } => "word-zero exp original upper F64 is outside signed-zero/actual-radial admission",
            Self::CallbackRequiredFromEntry { .. } => "57f8e0 range path requires explicit597c90 callback service",
            Self::Arithmetic(error) => error.reason(),
            Self::MathError(_) => "57f8e0 explicit597c90 numeric callback continuation required",
        }
    }
    pub fn known_advanced_state(&self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::UpperLaneContinuationRequired { entry, .. } | Self::CallbackRequiredFromEntry { entry } => entry.exception_state,
            Self::Arithmetic(error) => error.known_advanced_state(entry),
            Self::MathError(MathErrorServiceError::PathRequired { advanced }) => *advanced,
            Self::MathError(MathErrorServiceError::Callback(failure)) => failure.advanced,
            Self::MathError(MathErrorServiceError::Conversion(required)) => required.advanced,
            _ => entry,
        }
    }
}

pub struct SourceScalarExpWordZero {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_call: Option<SourceWordZeroExpCallReceipt>,
    first_failure: Option<SourceWordZeroExpCallReceipt>,
}

impl SourceScalarExpWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceWordZeroExpError>
    {
        if word != 0 { return Err(SourceWordZeroExpError::WordZeroRequired { supplied: word }); }
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceWordZeroExpError::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceWordZeroExpError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source57f8e0/word0/packed-separate/explicit-upper" }
    pub fn explicit_source_word(&self) -> u32 { 0 }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceWordZeroExpCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceWordZeroExpCallReceipt> { self.first_failure.as_ref() }
    pub fn source_entry(&self, bits: u32) -> SourceExpEntry {

        let (plan, flags) = source_entry_plan(bits);
        SourceExpEntry { plan, exception_state: self.state.with_source_status(flags) }
    }
    pub fn evaluate(&mut self, value: f32, original_upper_f64_bits: u64) -> Result<f32, SourceWordZeroExpError> {
        self.evaluate_bits(value.to_bits(), original_upper_f64_bits).map(f32::from_bits)
    }
    pub fn evaluate_bits(&mut self, bits: u32, original_upper_f64_bits: u64) -> Result<u32, SourceWordZeroExpError> {
        let call_entry = self.state;
        let entry = self.source_entry(bits);
        let result = match entry.plan {
            SourceExpPlan::CompleteBits { bits } => Ok(SourceExpWithState { bits, exception_state: entry.exception_state }),
            SourceExpPlan::Ordinary { .. } => self.ordinary(entry, original_upper_f64_bits),
            SourceExpPlan::MaskedRangeError { .. } => Err(SourceWordZeroExpError::CallbackRequiredFromEntry { entry }),
        };
        self.finish_call(SourceWordZeroExpCallKind::ValueOnly, bits, original_upper_f64_bits, call_entry, entry, result)
    }
    pub fn with_callback(&mut self, value: f32, original_upper_f64_bits: u64,
        callback: &mut impl SourceMathErrorCallback) -> Result<f32, SourceWordZeroExpError>
    {
        self.with_callback_bits(value.to_bits(), original_upper_f64_bits, callback).map(f32::from_bits)
    }
    pub fn with_callback_bits(&mut self, bits: u32, original_upper_f64_bits: u64,
        callback: &mut impl SourceMathErrorCallback) -> Result<u32, SourceWordZeroExpError>
    {
        let call_entry = self.state;
        let entry = self.source_entry(bits);
        let result = match entry.plan {
            SourceExpPlan::CompleteBits { bits } => Ok(SourceExpWithState { bits, exception_state: entry.exception_state }),
            SourceExpPlan::Ordinary { .. } => self.ordinary(entry, original_upper_f64_bits),
            SourceExpPlan::MaskedRangeError { .. } => {
                resolve_source_exp_range_with_callback(entry.plan, entry.exception_state, callback)
                    .map(|completed| SourceExpWithState { bits: completed.bits, exception_state: completed.advanced })
                    .map_err(SourceWordZeroExpError::MathError)
            }
        };
        self.finish_call(SourceWordZeroExpCallKind::WithExplicitCallbackService, bits,
            original_upper_f64_bits, call_entry, entry, result)
    }
    fn finish_call(&mut self, kind: SourceWordZeroExpCallKind, input_bits: u32,
        original_upper_f64_bits: u64, entry: ExplicitExceptionState, source_entry: SourceExpEntry,
        reported: Result<SourceExpWithState, SourceWordZeroExpError>) -> Result<u32, SourceWordZeroExpError> {
        let result = self.retain(reported);
        let receipt = SourceWordZeroExpCallReceipt { kind, input_bits, original_upper_f64_bits,
            entry, source_entry, advanced: self.state, result_bits: result.as_ref().ok().copied(),
            error: result.as_ref().err().cloned() };
        if result.is_err() && self.first_failure.is_none() { self.first_failure = Some(receipt.clone()); }
        self.last_call = Some(receipt);
        result
    }
    fn retain(&mut self, result: Result<SourceExpWithState, SourceWordZeroExpError>) -> Result<u32, SourceWordZeroExpError> {
        match result {
            Ok(completed) => { self.state = completed.exception_state; Ok(completed.bits) }
            Err(error) => { self.state = error.known_advanced_state(self.state); Err(error) }
        }
    }
    fn ordinary(&self, entry: SourceExpEntry, upper: u64) -> Result<SourceExpWithState, SourceWordZeroExpError> {
        serial_exp::ordinary(entry, upper, &SITES).map_err(|error| match error {
            SerialExpError::UpperRequired { bits, entry } =>
                SourceWordZeroExpError::UpperLaneContinuationRequired { bits, entry },
            SerialExpError::OrdinaryRequired { entry } =>
                SourceWordZeroExpError::CallbackRequiredFromEntry { entry },
            SerialExpError::Arithmetic(error) => SourceWordZeroExpError::Arithmetic(error),
        })
    }
}
