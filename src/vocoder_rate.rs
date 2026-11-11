use crate::{root_binding::VocoderConfigV2, signal_rate::{self, SourceRootRateInstructions,
    SourceSignalRootRate1136}, source_scalar::shared_scalar_f32::{SourceScalarF32WithState,
    SourceScalarF32StateError}, userdata};

impl SourceRootRateInstructions for SourceScalarF32WithState {
    type Error = SourceScalarF32StateError;
    fn cvtsi2ss_signed32(&mut self, input: i32) -> Result<u32, Self::Error> {
        self.cvtsi2ss_signed32_after_xorps(input)
    }
    fn mulss(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        SourceScalarF32WithState::mulss(self, left, right)
    }
}

pub struct Prepared {
    configuration: VocoderConfigV2,
    rate: SourceSignalRootRate1136,
    instructions: SourceScalarF32WithState,
}
impl Prepared {
    pub fn configuration(&self) -> VocoderConfigV2 { self.configuration }

    pub fn rate(&self) -> SourceSignalRootRate1136 { self.rate }
    pub fn instructions(&self) -> &SourceScalarF32WithState { &self.instructions }
    pub fn into_parts(self) -> (VocoderConfigV2, SourceSignalRootRate1136, SourceScalarF32WithState) {
        (self.configuration, self.rate, self.instructions)
    }
}
pub struct Failure {
    pub configuration: VocoderConfigV2,
    pub cause: signal_rate::SignalRootRateError<SourceScalarF32StateError>,

    pub instructions: SourceScalarF32WithState,
}

pub fn prepare(configuration: VocoderConfigV2, mut instructions: SourceScalarF32WithState)
    -> Result<Prepared, Failure>
{
    match SourceSignalRootRate1136::from_bound_config(&configuration, &mut instructions) {
        Ok(rate) => Ok(Prepared { configuration, rate, instructions }),
        Err(cause) => Err(Failure { configuration, cause, instructions }),
    }
}
pub fn prepare_userdata(userdata: &userdata::Decoded<'_>, instructions: SourceScalarF32WithState)
    -> Result<Prepared, Failure>
{
    prepare(userdata.primary().vocoder(), instructions)
}
