use super::finite_power::ExplicitExceptionState;
use super::selected_exp::{SourceExpEntry, SourceExpPlan, source_entry_plan};
use super::serial_exp::{self, SerialExpError, SerialExpSites};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::VorbisScalarError;

const SITES: SerialExpSites = SerialExpSites([
    "5b05e8/serial count split MULPD.low", "5b05f9/serial remainder SUBPD.low",
    "5b060e/serial polynomial MULPD.low", "5b0612/serial square MULPD.low",
    "5b0616/serial polynomial ADDPD.low", "5b061e/serial correction MULPD.low",
    "5b0622/serial reduced ADDPD.low", "5b0637/serial table MULPD.low",
    "5b063b/serial table ADDPD.low", "5b0644/serial binary scale MULPD.low",
]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceUiSerialExpRequired {
    NonzeroUiSelector { supplied_8a5ed4: u32 },
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    OutsideEasingDomain { input_bits: u32 },
    SpecialPath { input_bits: u32 },
    UpperOperand { bits: u64, prefix: SourceExpEntry },
    OrdinaryPrefix { prefix: SourceExpEntry },
    Arithmetic(VorbisScalarError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceUiSerialExpError {
    pub required: SourceUiSerialExpRequired,

    pub input_bits: Option<u32>,
    pub original_upper_f64_bits: Option<u64>,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
}

impl SourceUiSerialExpError {
    pub fn reason(&self) -> &'static str {
        match &self.required {
            SourceUiSerialExpRequired::NonzeroUiSelector { .. } =>
                "5b0580 nonzero UI selector requires the distinct AVX/FMA body",
            SourceUiSerialExpRequired::ArchitectureNotClosed =>
                "5b0580 explicit arithmetic architecture is not closed",
            SourceUiSerialExpRequired::UnsupportedEnvironment { .. } =>
                "5b0580 serial state must be RNE/all-masked/gradual",
            SourceUiSerialExpRequired::OutsideEasingDomain { .. } =>
                "5b0580 value is outside the admitted finite [-4,0] easing domain",
            SourceUiSerialExpRequired::SpecialPath { .. } =>
                "5b0580 nonfinite source path is outside this ordinary value service",
            SourceUiSerialExpRequired::UpperOperand { .. } =>
                "5b0580 original upper F64 requires a further packed-lane continuation",
            SourceUiSerialExpRequired::OrdinaryPrefix { .. } =>
                "5b0580 ordinary source prefix is required",
            SourceUiSerialExpRequired::Arithmetic(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(&self, _entry: ExplicitExceptionState) -> ExplicitExceptionState {
        self.advanced
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceUiSerialExpCompletion {
    pub input_bits: u32,
    pub original_upper_f64_bits: u64,
    pub bits: u32,
    pub entry: ExplicitExceptionState,
    pub exception_state: ExplicitExceptionState,
}

pub struct SourceUiSerialExp5b0580 {
    explicit_ui_word_8a5ed4: u32,
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    failure: Option<SourceUiSerialExpError>,
    last_completion: Option<SourceUiSerialExpCompletion>,
}

pub struct SourceUiSerialExpParts {
    pub explicit_ui_word_8a5ed4: u32,
    pub architecture: ScalarF32Architecture,
    pub exception_state: ExplicitExceptionState,
    pub failure: Option<SourceUiSerialExpError>,
    pub last_completion: Option<SourceUiSerialExpCompletion>,
}

impl SourceUiSerialExp5b0580 {
    pub fn bind(explicit_ui_word_8a5ed4: u32, architecture: ScalarF32Architecture,
        state: ExplicitExceptionState) -> Result<Self, SourceUiSerialExpError>
    {
        let required = if explicit_ui_word_8a5ed4 != 0 {
            Some(SourceUiSerialExpRequired::NonzeroUiSelector {
                supplied_8a5ed4: explicit_ui_word_8a5ed4,
            })
        } else if architecture != ScalarF32Architecture::IntelSdm {
            Some(SourceUiSerialExpRequired::ArchitectureNotClosed)
        } else if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            Some(SourceUiSerialExpRequired::UnsupportedEnvironment { mxcsr: state.mxcsr() })
        } else { None };
        if let Some(required) = required {
            return Err(SourceUiSerialExpError { required, input_bits: None,
                original_upper_f64_bits: None, entry: state, advanced: state });
        }
        Ok(Self { explicit_ui_word_8a5ed4, architecture, state,
            failure: None, last_completion: None })
    }

    pub fn name(&self) -> &'static str { "source-ui5b0580/serial/easing/explicit-upper" }
    pub fn explicit_ui_word_8a5ed4(&self) -> u32 { self.explicit_ui_word_8a5ed4 }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn failure(&self) -> Option<&SourceUiSerialExpError> { self.failure.as_ref() }
    pub fn last_completion(&self) -> Option<&SourceUiSerialExpCompletion> { self.last_completion.as_ref() }

    pub fn evaluate_bits(&mut self, input_bits: u32, original_upper_f64_bits: u64)
        -> Result<u32, SourceUiSerialExpError>
    {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let entry = self.state;
        let magnitude = input_bits & 0x7fff_ffff;
        let result = if magnitude >= 0x7f80_0000 {
            Err((SourceUiSerialExpRequired::SpecialPath { input_bits }, entry))
        } else if magnitude > 0x4080_0000 || (input_bits >> 31 == 0 && magnitude != 0) {
            Err((SourceUiSerialExpRequired::OutsideEasingDomain { input_bits }, entry))
        } else {

            let (plan, flags) = source_entry_plan(input_bits);
            let prefix = SourceExpEntry { plan, exception_state: entry.with_source_status(flags) };
            match prefix.plan {
                SourceExpPlan::Ordinary { .. } =>
                    serial_exp::ordinary(prefix, original_upper_f64_bits, &SITES)
                        .map_err(|error| match error {
                            SerialExpError::UpperRequired { bits, entry: prefix } =>
                                (SourceUiSerialExpRequired::UpperOperand { bits, prefix }, prefix.exception_state),
                            SerialExpError::OrdinaryRequired { entry: prefix } =>
                                (SourceUiSerialExpRequired::OrdinaryPrefix { prefix }, prefix.exception_state),
                            SerialExpError::Arithmetic(error) => {
                                let advanced = error.known_advanced_state(prefix.exception_state);
                                (SourceUiSerialExpRequired::Arithmetic(error), advanced)
                            }
                        }),
                _ => Err((SourceUiSerialExpRequired::OrdinaryPrefix { prefix }, prefix.exception_state)),
            }
        };
        match result {
            Ok(completed) => {
                self.state = completed.exception_state;
                self.last_completion = Some(SourceUiSerialExpCompletion { input_bits,
                    original_upper_f64_bits, bits: completed.bits, entry,
                    exception_state: completed.exception_state });
                Ok(completed.bits)
            }
            Err((required, advanced)) => {
                self.state = advanced;
                let error = SourceUiSerialExpError { required, input_bits: Some(input_bits),
                    original_upper_f64_bits: Some(original_upper_f64_bits), entry, advanced };
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn into_parts(self) -> SourceUiSerialExpParts {
        SourceUiSerialExpParts { explicit_ui_word_8a5ed4: self.explicit_ui_word_8a5ed4,
            architecture: self.architecture, exception_state: self.state,
            failure: self.failure, last_completion: self.last_completion }
    }
}
