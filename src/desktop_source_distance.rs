use super::{layout, source_math};
use crate::source_scalar::{ExplicitExceptionState, ScalarF32Architecture, SourceNorm579824, SourceNormF32Error};

pub(super) enum Cause {
    Controls(layout::context::provider::Error),
    State { mxcsr: u32 },
    Norm { error: SourceNormF32Error, advanced: ExplicitExceptionState },
    BindingRequired,
}

pub(super) struct Operands {
    pub sample: [f32; 2], pub center: [f32; 2], pub difference: Option<[f32; 2]>,
    pub value: Option<f32>,
}

pub(super) struct Norm {
    service: Option<SourceNorm579824>,
    pub cause: Option<Cause>,
    pub operands: Option<Operands>,
}

#[must_use = "retain the norm call's operands, pending callback request and logical state"]
pub(super) struct Failure { pub pending: Norm }

impl Failure {
    pub fn message(&self) -> &'static str {
        "The source screen-distance calculation could not complete."
    }
}

impl Norm {
    pub fn bind() -> Result<Self, Failure> {
        let mut pending = Self { service: None, cause: None, operands: None };
        let result = (|| -> Result<SourceNorm579824, Cause> {
            let observed = layout::context::provider::check_current_thread().map_err(Cause::Controls)?;

            let entry = ExplicitExceptionState::from_explicit_parts(observed, 0)
                .map_err(|_| Cause::State { mxcsr: observed })?;
            SourceNorm579824::bind(ScalarF32Architecture::IntelSdm, entry)
                .map_err(|error| Cause::Norm { error, advanced: error.known_advanced_state(entry) })
        })();
        match result {
            Ok(service) => { pending.service = Some(service); Ok(pending) }
            Err(cause) => { pending.cause = Some(cause); Err(Failure { pending }) }
        }
    }

    pub fn distance(&mut self, sample: [f32; 2], center: [f32; 2]) -> Result<f32, ()> {
        if self.cause.is_some() { return Err(()); }
        self.operands = Some(Operands { sample, center, difference: None, value: None });
        self.check_controls()?;
        let dy = sample[1] - center[1];
        let dx = sample[0] - center[0];
        if let Some(operands) = &mut self.operands { operands.difference = Some([dx, dy]); }
        let Some(service) = &mut self.service else {
            self.cause = Some(Cause::BindingRequired); return Err(());
        };
        let entry = service.exception_state();
        let value = match service.norm_f32(dx, dy) {
            Ok(value) => value,
            Err(error) => {
                self.cause = Some(Cause::Norm { error, advanced: error.known_advanced_state(entry) });
                return Err(());
            }
        };
        if let Some(operands) = &mut self.operands { operands.value = Some(value); }
        self.check_controls()?;
        Ok(value)
    }

    pub fn check_controls(&mut self) -> Result<(), ()> {
        if self.cause.is_some() { return Err(()); }
        match layout::context::provider::check_current_thread() {
            Ok(_) => Ok(()),
            Err(error) => { self.cause = Some(Cause::Controls(error)); Err(()) }
        }
    }

    pub fn is_failed(&self) -> bool { self.cause.is_some() }
    pub fn into_failure(self) -> Failure { Failure { pending: self } }
}
