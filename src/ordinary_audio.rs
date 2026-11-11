use crate::{frontend_gain, wav48k};
use crate::frontend_gain_log_rows::SourceGainRowLogServices;
use crate::frontend_gain_statistics::SourceGainStatisticsLogServices;
pub use crate::resample44100::{Error as ResamplerError, Policy as ResamplerPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathPolicy {
    WindowsHostLog10RoundF32AndPowf,
}

#[derive(Debug)]
pub enum Error {
    Decode(wav48k::Error),
    UnsupportedShortInput(usize),
    NonFiniteInput,
    NumericDomain(String),
    Resample(ResamplerError),
}

pub struct NormalizedAudio {
    pub metadata: wav48k::Metadata,
    pub samples: Vec<f32>,
}

pub struct PreparedAudio {
    pub metadata: wav48k::Metadata,
    pub analysis_rate: u32,
    pub samples: Vec<f32>,
}

pub fn normalize_wav48k(bytes: &[u8], policy: MathPolicy) -> Result<NormalizedAudio, Error> {
    let decoded = wav48k::decode(bytes).map_err(Error::Decode)?;
    let samples = normalize_mono48k(&decoded.channel0, policy)?;
    Ok(NormalizedAudio { metadata: decoded.metadata, samples })
}

pub enum SourcePeakAudioCause {
    Decode(wav48k::Error), Preparation(Error), Peak, Output(Error),
}
enum SourcePeakAudioOwner {
    Math(frontend_gain::SourcePeakMath), Peak(frontend_gain::SourcePeakFailure),
}

pub struct SourcePeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, metadata: Option<wav48k::Metadata>,
    input: Option<Vec<f32>>, stages: Option<frontend_gain::FrontendGainStages>,
    output: Option<Vec<f32>>, owner: SourcePeakAudioOwner, cause: SourcePeakAudioCause,
}
impl SourcePeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { self.metadata.as_ref() }
    pub fn input(&self) -> Option<&[f32]> { self.input.as_deref() }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { self.stages.as_ref() }
    pub fn output(&self) -> Option<&[f32]> { self.output.as_deref() }
    pub fn cause(&self) -> &SourcePeakAudioCause { &self.cause }
    pub fn math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner { SourcePeakAudioOwner::Math(math) => math, SourcePeakAudioOwner::Peak(failure) => failure.math() }
    }
    pub fn peak_failure(&self) -> Option<&frontend_gain::SourcePeakFailure> {
        match &self.owner { SourcePeakAudioOwner::Peak(failure) => Some(failure), SourcePeakAudioOwner::Math(_) => None }
    }
}

pub fn normalize_wav48k_with_source_peak(
    bytes: &[u8], policy: MathPolicy, math: frontend_gain::SourcePeakMath,
) -> Result<(NormalizedAudio, frontend_gain::SourcePeakMath), SourcePeakAudioFailure<'_>> {
    let decoded = match wav48k::decode(bytes) {
        Ok(decoded) => decoded,
        Err(error) => return Err(SourcePeakAudioFailure {
            bytes, policy, metadata: None, input: None, stages: None, output: None,
            owner: SourcePeakAudioOwner::Math(math), cause: SourcePeakAudioCause::Decode(error),
        }),
    };
    let prepared = match prepare_mono48k(&decoded.channel0, policy) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourcePeakAudioFailure {
            bytes, policy, metadata: Some(decoded.metadata), input: Some(decoded.channel0),
            stages: failure.stages, output: None, owner: SourcePeakAudioOwner::Math(math),
            cause: SourcePeakAudioCause::Preparation(failure.error),
        }),
    };
    let (samples, math) = match frontend_gain::peak_normalize09_with_source_math(prepared.gained, math) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourcePeakAudioFailure {
            bytes, policy, metadata: Some(decoded.metadata), input: Some(decoded.channel0),
            stages: Some(prepared.stages), output: None, owner: SourcePeakAudioOwner::Peak(failure),
            cause: SourcePeakAudioCause::Peak,
        }),
    };
    if samples.iter().any(|v| !v.is_finite()) {
        return Err(SourcePeakAudioFailure {
            bytes, policy, metadata: Some(decoded.metadata), input: Some(decoded.channel0),
            stages: Some(prepared.stages), output: Some(samples), owner: SourcePeakAudioOwner::Math(math),
            cause: SourcePeakAudioCause::Output(Error::NumericDomain("Nonfinite normalized output.".into())),
        });
    }
    Ok((NormalizedAudio { metadata: decoded.metadata, samples }, math))
}

pub enum SourceFilterAudioCause {
    Decode(wav48k::Error), Admission(Error), Gain, Application(Error), Peak(Error), Output(Error),
}
enum SourceFilterAudioOwner {
    Math(frontend_gain::SourceFilterMath), Gain(frontend_gain::SourceFilterGainFailure),
}
pub struct SourceFilterAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, metadata: Option<wav48k::Metadata>, input: Option<Vec<f32>>,
    unused_outputs: Option<frontend_gain::SourceFilterOutputs>, stages: Option<frontend_gain::FrontendGainStages>,
    gained: Option<Vec<f32>>, output: Option<Vec<f32>>, owner: SourceFilterAudioOwner, cause: SourceFilterAudioCause,
}
impl SourceFilterAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { self.metadata.as_ref() }
    pub fn input(&self) -> Option<&[f32]> { self.input.as_deref() }
    pub fn unused_outputs(&self) -> Option<&frontend_gain::SourceFilterOutputs> { self.unused_outputs.as_ref() }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { self.stages.as_ref() }
    pub fn gained(&self) -> Option<&[f32]> { self.gained.as_deref() }
    pub fn output(&self) -> Option<&[f32]> { self.output.as_deref() }
    pub fn cause(&self) -> &SourceFilterAudioCause { &self.cause }
    pub fn math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner { SourceFilterAudioOwner::Math(math) => math, SourceFilterAudioOwner::Gain(failure) => failure.math() }
    }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceFilterGainFailure> {
        match &self.owner { SourceFilterAudioOwner::Gain(failure) => Some(failure), SourceFilterAudioOwner::Math(_) => None }
    }
}

pub fn normalize_wav48k_with_source_filters(bytes: &[u8], policy: MathPolicy,
    math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath), SourceFilterAudioFailure<'_>>
{
    match normalize_filter_import(bytes, policy, math, outputs, (), host_peak_choice) {
        Ok((audio, math, ())) => Ok((audio, math)),
        Err(FilterImportFailure::BeforePeak { failure, peak: () }) => Err(failure),
        Err(FilterImportFailure::Peak { prefix, failure }) =>
            Err(prefix.into_host_failure(Some(failure.gained), None, SourceFilterAudioCause::Peak(failure.error))),
        Err(FilterImportFailure::Output { prefix, completed, error }) =>
            Err(prefix.into_host_failure(completed.gained, Some(completed.samples), SourceFilterAudioCause::Output(error))),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceFilterPeakAudioCause { BeforePeak, Peak, Output }
pub struct SourceFilterPeakAudioFailure<'a> {
    inner: FilterImportFailure<'a, frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>,
}
impl SourceFilterPeakAudioFailure<'_> {
    pub fn cause(&self) -> SourceFilterPeakAudioCause {
        match &self.inner {
            FilterImportFailure::BeforePeak { .. } => SourceFilterPeakAudioCause::BeforePeak,
            FilterImportFailure::Peak { .. } => SourceFilterPeakAudioCause::Peak,
            FilterImportFailure::Output { .. } => SourceFilterPeakAudioCause::Output,
        }
    }
    pub fn early_failure(&self) -> Option<&SourceFilterAudioFailure<'_>> {
        match &self.inner { FilterImportFailure::BeforePeak { failure, .. } => Some(failure), _ => None }
    }
    pub fn peak_failure(&self) -> Option<&frontend_gain::SourcePeakFailure> {
        match &self.inner { FilterImportFailure::Peak { failure, .. } => Some(failure), _ => None }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.math(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => &prefix.math,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.inner {
            FilterImportFailure::BeforePeak { peak, .. } => peak,
            FilterImportFailure::Peak { failure, .. } => failure.math(),
            FilterImportFailure::Output { completed, .. } => &completed.peak,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.bytes(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => prefix.bytes,
        }
    }
    pub fn policy(&self) -> MathPolicy {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.policy(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => prefix.policy,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.metadata(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.input(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.stages(),
            FilterImportFailure::Peak { prefix, .. } | FilterImportFailure::Output { prefix, .. } => Some(&prefix.stages),
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.inner {
            FilterImportFailure::BeforePeak { failure, .. } => failure.output(),
            FilterImportFailure::Peak { failure, .. } => Some(failure.progress().output()),
            FilterImportFailure::Output { completed, .. } => Some(&completed.samples),
        }
    }
    pub fn output_error(&self) -> Option<&Error> {
        match &self.inner { FilterImportFailure::Output { error, .. } => Some(error), _ => None }
    }
}

pub fn normalize_wav48k_with_source_filters_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, frontend_gain::SourcePeakMath), SourceFilterPeakAudioFailure<'_>>
{
    normalize_filter_import(bytes, policy, filter_math, outputs, peak_math, source_peak_choice)
        .map_err(|inner| SourceFilterPeakAudioFailure { inner })
}

struct PeakCompleted<P> { samples: Vec<f32>, peak: P, gained: Option<Vec<f32>> }
struct HostPeakFailure { gained: Vec<f32>, error: Error }
fn host_peak_choice(gained: Vec<f32>, (): ()) -> Result<PeakCompleted<()>, HostPeakFailure> {
    let samples = match frontend_gain::peak_normalize09(&gained) {
        Ok(samples) => samples,
        Err(error) => return Err(HostPeakFailure { gained, error: Error::NumericDomain(error) }),
    };
    Ok(PeakCompleted { samples, peak: (), gained: Some(gained) })
}
fn source_peak_choice(gained: Vec<f32>, peak: frontend_gain::SourcePeakMath)
    -> Result<PeakCompleted<frontend_gain::SourcePeakMath>, frontend_gain::SourcePeakFailure>
{
    frontend_gain::peak_normalize09_with_source_math(gained, peak)
        .map(|(samples, peak)| PeakCompleted { samples, peak, gained: None })
}
fn source_in_place_peak_choice(samples: Vec<f32>, peak: frontend_gain::SourcePeakMath)
    -> Result<PeakCompleted<frontend_gain::SourcePeakMath>, frontend_gain::SourcePeakInPlaceFailure>
{
    frontend_gain::peak_normalize09_in_place_with_source_bits(samples, peak)
        .map(|(samples, peak)| PeakCompleted { samples, peak, gained: None })
}
struct ImportPrefix<'a, O, D = Vec<f32>> {
    bytes: &'a [u8], policy: MathPolicy, metadata: wav48k::Metadata,
    input: D, stages: frontend_gain::FrontendGainStages, math: O,
}
type FilterImportPrefix<'a> = ImportPrefix<'a, frontend_gain::SourceFilterMath>;
impl<'a> ImportPrefix<'a, frontend_gain::SourceFilterMath> {
    fn into_host_failure(self, gained: Option<Vec<f32>>, output: Option<Vec<f32>>, cause: SourceFilterAudioCause)
        -> SourceFilterAudioFailure<'a>
    {
        SourceFilterAudioFailure { bytes: self.bytes, policy: self.policy, metadata: Some(self.metadata),
            input: Some(self.input), unused_outputs: None, stages: Some(self.stages), gained, output,
            owner: SourceFilterAudioOwner::Math(self.math), cause }
    }
}
enum FilterImportFailure<'a, P, E> {
    BeforePeak { failure: SourceFilterAudioFailure<'a>, peak: P },
    Peak { prefix: FilterImportPrefix<'a>, failure: E },
    Output { prefix: FilterImportPrefix<'a>, completed: PeakCompleted<P>, error: Error },
}
fn normalize_filter_import<'a, P, E>(bytes: &'a [u8], policy: MathPolicy,
    math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs, peak: P,
    apply_peak: impl FnOnce(Vec<f32>, P) -> Result<PeakCompleted<P>, E>)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, P), FilterImportFailure<'a, P, E>>
{
    let prepared = match prepare_wav_gain(bytes, policy, (math, outputs), |input, (math, outputs)| {
        frontend_gain::frontend_gain_table_with_source_filters(input, -10.0, math, outputs)
    }) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(FilterImportFailure::BeforePeak {
            failure: failure.into_filter_failure(bytes, policy), peak }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak, apply_peak).map_err(|failure| match failure {
        PeakImportFailure::Peak { prefix, failure } => FilterImportFailure::Peak { prefix, failure },
        PeakImportFailure::Output { prefix, completed, error } => FilterImportFailure::Output { prefix, completed, error },
    })
}

struct PreparedImport<'a, O, D = Vec<f32>> { prefix: ImportPrefix<'a, O, D>, gained: Vec<f32> }
enum GainImportFailure<I, O, E> {
    Decode { initial: I, error: wav48k::Error },
    Admission { initial: I, metadata: wav48k::Metadata, input: Vec<f32>, error: Error },
    Gain { metadata: wav48k::Metadata, failure: E },
    Application { metadata: wav48k::Metadata, input: Vec<f32>, stages: Option<frontend_gain::FrontendGainStages>, owner: O, error: Error },
}
impl GainImportFailure<(frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs),
    frontend_gain::SourceFilterMath, frontend_gain::SourceFilterGainFailure>
{
    fn into_filter_failure(self, bytes: &[u8], policy: MathPolicy) -> SourceFilterAudioFailure<'_> {
        match self {
            Self::Decode { initial: (math, outputs), error } => SourceFilterAudioFailure {
                bytes, policy, metadata: None, input: None, unused_outputs: Some(outputs), stages: None,
                gained: None, output: None, owner: SourceFilterAudioOwner::Math(math), cause: SourceFilterAudioCause::Decode(error),
            },
            Self::Admission { initial: (math, outputs), metadata, input, error } => SourceFilterAudioFailure {
                bytes, policy, metadata: Some(metadata), input: Some(input), unused_outputs: Some(outputs), stages: None,
                gained: None, output: None, owner: SourceFilterAudioOwner::Math(math), cause: SourceFilterAudioCause::Admission(error),
            },
            Self::Gain { metadata, failure } => SourceFilterAudioFailure {
                bytes, policy, metadata: Some(metadata), input: None, unused_outputs: None, stages: None,
                gained: None, output: None, owner: SourceFilterAudioOwner::Gain(failure), cause: SourceFilterAudioCause::Gain,
            },
            Self::Application { metadata, input, stages, owner, error } => SourceFilterAudioFailure {
                bytes, policy, metadata: Some(metadata), input: Some(input), unused_outputs: None, stages,
                gained: None, output: None, owner: SourceFilterAudioOwner::Math(owner), cause: SourceFilterAudioCause::Application(error),
            },
        }
    }
}
fn prepare_wav_gain<'a, I, O, E>(bytes: &'a [u8], policy: MathPolicy, initial: I,
    gain: impl FnOnce(Vec<f32>, I) -> Result<(frontend_gain::FrontendGainStages, O, Vec<f32>), E>)
    -> Result<PreparedImport<'a, O>, GainImportFailure<I, O, E>>
{
    match prepare_wav_gain_with_application(bytes, policy, initial, (), gain, host_import_application) {
        Ok(PreparedImport { prefix, gained }) => {
            let ImportPrefix { bytes, policy, metadata, input, stages, math: (owner, ()) } = prefix;
            Ok(PreparedImport { prefix: ImportPrefix { bytes, policy, metadata, input, stages, math: owner }, gained })
        }
        Err(failure) => Err(match failure {
            GainApplicationImportFailure::Decode { initial, application: (), error } => GainImportFailure::Decode { initial, error },
            GainApplicationImportFailure::Admission { initial, application: (), metadata, input, error } =>
                GainImportFailure::Admission { initial, metadata, input, error },
            GainApplicationImportFailure::Gain { metadata, failure, application: () } => GainImportFailure::Gain { metadata, failure },
            GainApplicationImportFailure::Application { metadata, owner, failure: HostApplicationFailure { input, stages, error } } =>
                GainImportFailure::Application { metadata, input, stages, owner, error },
        }),
    }
}
struct ApplicationImportCompleted<A, D = Vec<f32>> {
    input: D, stages: frontend_gain::FrontendGainStages, gained: Vec<f32>, owner: A,
}
struct HostApplicationFailure { input: Vec<f32>, stages: Option<frontend_gain::FrontendGainStages>, error: Error }
fn host_import_application(input: Vec<f32>, stages: frontend_gain::FrontendGainStages, (): ())
    -> Result<ApplicationImportCompleted<()>, HostApplicationFailure>
{
    match apply_prepared_gain(&input, stages) {
        Ok(prepared) => Ok(ApplicationImportCompleted { input, stages: prepared.stages, gained: prepared.gained, owner: () }),
        Err(failure) => Err(HostApplicationFailure { input, stages: failure.stages, error: failure.error }),
    }
}
enum GainApplicationImportFailure<I, O, E, A, AE> {
    Decode { initial: I, application: A, error: wav48k::Error },
    Admission { initial: I, application: A, metadata: wav48k::Metadata, input: Vec<f32>, error: Error },
    Gain { metadata: wav48k::Metadata, failure: E, application: A },
    Application { metadata: wav48k::Metadata, owner: O, failure: AE },
}
#[derive(Clone, Copy)]
enum WavGainAdmission { Legacy, PositiveSource, SourceFinite }
fn prepare_wav_gain_with_application<'a, I, O, E, A, AO, AE, D>(bytes: &'a [u8], policy: MathPolicy, initial: I, application: A,
    gain: impl FnOnce(Vec<f32>, I) -> Result<(frontend_gain::FrontendGainStages, O, Vec<f32>), E>,
    apply: impl FnOnce(Vec<f32>, frontend_gain::FrontendGainStages, A) -> Result<ApplicationImportCompleted<AO, D>, AE>)
    -> Result<PreparedImport<'a, (O, AO), D>, GainApplicationImportFailure<I, O, E, A, AE>>
{
    prepare_wav_gain_with_application_and_admission(bytes, policy, initial, application, gain, apply, WavGainAdmission::Legacy)
}
fn prepare_wav_gain_with_application_and_admission<'a, I, O, E, A, AO, AE, D>(bytes: &'a [u8], policy: MathPolicy, initial: I, application: A,
    gain: impl FnOnce(Vec<f32>, I) -> Result<(frontend_gain::FrontendGainStages, O, Vec<f32>), E>,
    apply: impl FnOnce(Vec<f32>, frontend_gain::FrontendGainStages, A) -> Result<ApplicationImportCompleted<AO, D>, AE>,
    admission: WavGainAdmission)
    -> Result<PreparedImport<'a, (O, AO), D>, GainApplicationImportFailure<I, O, E, A, AE>>
{
    let decoded = match wav48k::decode(bytes) {
        Ok(decoded) => decoded,
        Err(error) => return Err(GainApplicationImportFailure::Decode { initial, application, error }),
    };
    if let Err(error) = admit_mono48k_with_admission(&decoded.channel0, policy, admission) {
        return Err(GainApplicationImportFailure::Admission { initial, application, metadata: decoded.metadata, input: decoded.channel0, error });
    }
    let (stages, owner, input) = match gain(decoded.channel0, initial) {
        Ok(completed) => completed,
        Err(failure) => return Err(GainApplicationImportFailure::Gain { metadata: decoded.metadata, failure, application }),
    };
    let prepared = match apply(input, stages, application) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(GainApplicationImportFailure::Application { metadata: decoded.metadata, owner, failure }),
    };
    Ok(PreparedImport { prefix: ImportPrefix { bytes, policy, metadata: decoded.metadata, input: prepared.input,
        stages: prepared.stages, math: (owner, prepared.owner) },
        gained: prepared.gained })
}
enum PeakImportFailure<'a, O, P, E, D = Vec<f32>> {
    Peak { prefix: ImportPrefix<'a, O, D>, failure: E },
    Output { prefix: ImportPrefix<'a, O, D>, completed: PeakCompleted<P>, error: Error },
}
enum PeakOutputAdmission { LegacyFinite, SourceBits }
fn finish_import_peak<'a, O, P, E, D>(prefix: ImportPrefix<'a, O, D>, gained: Vec<f32>, peak: P,
    apply_peak: impl FnOnce(Vec<f32>, P) -> Result<PeakCompleted<P>, E>)
    -> Result<(NormalizedAudio, O, P), PeakImportFailure<'a, O, P, E, D>>
{
    finish_import_peak_with_admission(prefix, gained, peak, apply_peak, PeakOutputAdmission::LegacyFinite)
}
fn finish_import_peak_with_admission<'a, O, P, E, D>(prefix: ImportPrefix<'a, O, D>, gained: Vec<f32>, peak: P,
    apply_peak: impl FnOnce(Vec<f32>, P) -> Result<PeakCompleted<P>, E>, admission: PeakOutputAdmission)
    -> Result<(NormalizedAudio, O, P), PeakImportFailure<'a, O, P, E, D>>
{
    let completed = match apply_peak(gained, peak) {
        Ok(completed) => completed,
        Err(failure) => return Err(PeakImportFailure::Peak { prefix, failure }),
    };
    if matches!(admission, PeakOutputAdmission::LegacyFinite) && completed.samples.iter().any(|v| !v.is_finite()) {
        return Err(PeakImportFailure::Output { prefix, completed,
            error: Error::NumericDomain("Nonfinite normalized output.".into()),
        });
    }
    Ok((NormalizedAudio { metadata: prefix.metadata, samples: completed.samples }, prefix.math, completed.peak))
}

enum SlewAudioOwner<'a> {
    Gain(GainImportFailure<frontend_gain::SourceSlewMath, frontend_gain::SourceSlewMath, frontend_gain::SourceSlewGainFailure>),
    Peak(PeakImportFailure<'a, frontend_gain::SourceSlewMath, (), HostPeakFailure>),
}
pub enum SourceSlewAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceSlewGainFailure),
    Application(&'a Error), Peak(&'a Error), Output(&'a Error),
}
pub struct SourceSlewAudioFailure<'a> { bytes: &'a [u8], policy: MathPolicy, owner: SlewAudioOwner<'a> }
impl SourceSlewAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceSlewAudioError<'_> {
        match &self.owner {
            SlewAudioOwner::Gain(GainImportFailure::Decode { error, .. }) => SourceSlewAudioError::Decode(error),
            SlewAudioOwner::Gain(GainImportFailure::Admission { error, .. }) => SourceSlewAudioError::Admission(error),
            SlewAudioOwner::Gain(GainImportFailure::Gain { failure, .. }) => SourceSlewAudioError::Gain(failure),
            SlewAudioOwner::Gain(GainImportFailure::Application { error, .. }) => SourceSlewAudioError::Application(error),
            SlewAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceSlewAudioError::Peak(&failure.error),
            SlewAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceSlewAudioError::Output(error),
        }
    }
    pub fn math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            SlewAudioOwner::Gain(GainImportFailure::Decode { initial, .. })
            | SlewAudioOwner::Gain(GainImportFailure::Admission { initial, .. }) => initial,
            SlewAudioOwner::Gain(GainImportFailure::Gain { failure, .. }) => failure.math(),
            SlewAudioOwner::Gain(GainImportFailure::Application { owner, .. }) => owner,
            SlewAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SlewAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            SlewAudioOwner::Gain(GainImportFailure::Decode { .. }) => None,
            SlewAudioOwner::Gain(GainImportFailure::Admission { metadata, .. })
            | SlewAudioOwner::Gain(GainImportFailure::Gain { metadata, .. })
            | SlewAudioOwner::Gain(GainImportFailure::Application { metadata, .. }) => Some(metadata),
            SlewAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SlewAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            SlewAudioOwner::Gain(GainImportFailure::Decode { .. }) => None,
            SlewAudioOwner::Gain(GainImportFailure::Admission { input, .. })
            | SlewAudioOwner::Gain(GainImportFailure::Application { input, .. }) => Some(input),
            SlewAudioOwner::Gain(GainImportFailure::Gain { failure, .. }) => Some(failure.input()),
            SlewAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SlewAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            SlewAudioOwner::Gain(GainImportFailure::Application { stages, .. }) => stages.as_ref(),
            SlewAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SlewAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            SlewAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(&failure.gained),
            SlewAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner { SlewAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None }
    }
}

pub fn normalize_wav48k_with_source_slew(bytes: &[u8], policy: MathPolicy, math: frontend_gain::SourceSlewMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceSlewMath), SourceSlewAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, math, |input, math| {
        frontend_gain::frontend_gain_table_with_source_slew(input, -10.0, math)
    }).map_err(|failure| SourceSlewAudioFailure { bytes, policy, owner: SlewAudioOwner::Gain(failure) })?;
    finish_import_peak(prepared.prefix, prepared.gained, (), host_peak_choice).map(|(audio, math, ())| (audio, math))
        .map_err(|failure| SourceSlewAudioFailure { bytes, policy, owner: SlewAudioOwner::Peak(failure) })
}

type FilterSlewGainImportFailure = GainImportFailure<
    (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs, frontend_gain::SourceSlewMath),
    (frontend_gain::SourceFilterMath, frontend_gain::SourceSlewMath), frontend_gain::SourceFilterSlewGainFailure>;
enum FilterSlewPeakAudioOwner<'a> {
    Gain { failure: FilterSlewGainImportFailure, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceFilterMath, frontend_gain::SourceSlewMath),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceFilterSlewPeakAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceFilterSlewGainFailure),
    Application(&'a Error), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceFilterSlewPeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, owner: FilterSlewPeakAudioOwner<'a>,
}
impl SourceFilterSlewPeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceFilterSlewPeakAudioError<'_> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { error, .. }, .. } => SourceFilterSlewPeakAudioError::Decode(error),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { error, .. }, .. } => SourceFilterSlewPeakAudioError::Admission(error),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => SourceFilterSlewPeakAudioError::Gain(failure),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { error, .. }, .. } => SourceFilterSlewPeakAudioError::Application(error),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceFilterSlewPeakAudioError::Peak(failure),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceFilterSlewPeakAudioError::Output(error),
        }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (math, _, _), .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (math, _, _), .. }, .. } => math,
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.filter_math(),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.0,
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
        }
    }
    pub fn slew_math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, math), .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, math), .. }, .. } => math,
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.slew_math(),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.1,
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.1,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { peak, .. } => peak,
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn unused_outputs(&self) -> Option<&frontend_gain::SourceFilterOutputs> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, outputs, _), .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, outputs, _), .. }, .. } => Some(outputs),
            _ => None,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { metadata, .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { metadata, .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { input, .. }, .. }
            | FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { input, .. }, .. } => Some(input),
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { stages, .. }, .. } => stages.as_ref(),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            FilterSlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples),
            _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_filters_slew_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    slew_math: frontend_gain::SourceSlewMath, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, frontend_gain::SourceSlewMath, frontend_gain::SourcePeakMath),
        SourceFilterSlewPeakAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, (filter_math, outputs, slew_math), |input, (filter, outputs, slew)| {
        frontend_gain::frontend_gain_table_with_source_filters_and_slew(input, -10.0, filter, outputs, slew)
            .map(|(stages, filter, slew, input)| (stages, (filter, slew), input))
    });
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterSlewPeakAudioFailure { bytes, policy,
            owner: FilterSlewPeakAudioOwner::Gain { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, (filter, slew), peak)| (audio, filter, slew, peak))
        .map_err(|failure| SourceFilterSlewPeakAudioFailure { bytes, policy, owner: FilterSlewPeakAudioOwner::Peak(failure) })
}

type FilterEnergySlewGainImportFailure = GainImportFailure<
    (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs, frontend_gain::SourceEnergyMath,
        frontend_gain::SourceEnergyWindows, frontend_gain::SourceSlewMath),
    (frontend_gain::SourceFilterMath, frontend_gain::SourceEnergyRowsCompleted, frontend_gain::SourceSlewMath),
    frontend_gain::SourceFilterEnergySlewGainFailure>;
enum FilterEnergySlewPeakAudioOwner<'a> {
    Gain { failure: FilterEnergySlewGainImportFailure, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceFilterMath, frontend_gain::SourceEnergyRowsCompleted, frontend_gain::SourceSlewMath),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceFilterEnergySlewPeakAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceFilterEnergySlewGainFailure),
    Application(&'a Error), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceFilterEnergySlewPeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, owner: FilterEnergySlewPeakAudioOwner<'a>,
}
impl SourceFilterEnergySlewPeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceFilterEnergySlewPeakAudioError<'_> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { error, .. }, .. } => SourceFilterEnergySlewPeakAudioError::Decode(error),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { error, .. }, .. } => SourceFilterEnergySlewPeakAudioError::Admission(error),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => SourceFilterEnergySlewPeakAudioError::Gain(failure),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { error, .. }, .. } => SourceFilterEnergySlewPeakAudioError::Application(error),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceFilterEnergySlewPeakAudioError::Peak(failure),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceFilterEnergySlewPeakAudioError::Output(error),
        }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (math, _, _, _, _), .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (math, _, _, _, _), .. }, .. } => math,
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.filter_math(),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.0,
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
        }
    }
    pub fn energy_math(&self) -> &frontend_gain::SourceEnergyMath {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, math, _, _), .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, math, _, _), .. }, .. } => math,
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.energy_math(),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.1.math(),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.1.math(),
        }
    }
    pub fn slew_math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, math), .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, math), .. }, .. } => math,
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.slew_math(),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.2,
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.2,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { peak, .. } => peak,
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn unused_energy_windows(&self) -> Option<&frontend_gain::SourceEnergyWindows> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, windows, _), .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, windows, _), .. }, .. } => Some(windows),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.unused_energy_windows(),
            _ => None,
        }
    }
    pub fn energy_rows(&self) -> Option<&frontend_gain::SourceEnergyRowsCompleted> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.energy_rows(),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => Some(&owner.1),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1),
            _ => None,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { metadata, .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { metadata, .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { input, .. }, .. }
            | FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { input, .. }, .. } => Some(input),
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { stages, .. }, .. } => stages.as_ref(),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            FilterEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples),
            _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_filters_energy_slew_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    energy_math: frontend_gain::SourceEnergyMath, windows: frontend_gain::SourceEnergyWindows,
    slew_math: frontend_gain::SourceSlewMath, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, frontend_gain::SourceEnergyRowsCompleted,
        frontend_gain::SourceSlewMath, frontend_gain::SourcePeakMath), SourceFilterEnergySlewPeakAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, (filter_math, outputs, energy_math, windows, slew_math),
        |input, (filter, outputs, energy, windows, slew)| {
            frontend_gain::frontend_gain_table_with_source_filters_energy_and_slew(input, -10.0, filter, outputs, energy, windows, slew)
                .map(|(stages, filter, rows, slew, input)| (stages, (filter, rows, slew), input))
        });
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterEnergySlewPeakAudioFailure { bytes, policy,
            owner: FilterEnergySlewPeakAudioOwner::Gain { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, (filter, rows, slew), peak)| (audio, filter, rows, slew, peak))
        .map_err(|failure| SourceFilterEnergySlewPeakAudioFailure { bytes, policy, owner: FilterEnergySlewPeakAudioOwner::Peak(failure) })
}

type FilterWindowEnergySlewGainImportFailure = GainImportFailure<
    (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs, crate::frontend_energy_window_math::SourceWindowMath,
        frontend_gain::SourceEnergyMath, frontend_gain::SourceEnergyWindows, frontend_gain::SourceSlewMath),
    (frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath),
    frontend_gain::SourceFilterWindowEnergySlewGainFailure>;
enum FilterWindowEnergySlewPeakAudioOwner<'a> {
    Gain { failure: FilterWindowEnergySlewGainImportFailure, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceFilterWindowEnergySlewPeakAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceFilterWindowEnergySlewGainFailure),
    Application(&'a Error), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceFilterWindowEnergySlewPeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, owner: FilterWindowEnergySlewPeakAudioOwner<'a>,
}
impl SourceFilterWindowEnergySlewPeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceFilterWindowEnergySlewPeakAudioError<'_> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { error, .. }, .. } => SourceFilterWindowEnergySlewPeakAudioError::Decode(error),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { error, .. }, .. } => SourceFilterWindowEnergySlewPeakAudioError::Admission(error),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => SourceFilterWindowEnergySlewPeakAudioError::Gain(failure),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { error, .. }, .. } => SourceFilterWindowEnergySlewPeakAudioError::Application(error),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceFilterWindowEnergySlewPeakAudioError::Peak(failure),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceFilterWindowEnergySlewPeakAudioError::Output(error),
        }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (math, _, _, _, _, _), .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (math, _, _, _, _, _), .. }, .. } => math,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.filter_math(),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.0,
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
        }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, math, _, _, _), .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, math, _, _, _), .. }, .. } => math,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_math(),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.1.window_math(),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.1.window_math(),
        }
    }
    pub fn energy_math(&self) -> &frontend_gain::SourceEnergyMath {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, math, _, _), .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, math, _, _), .. }, .. } => math,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.energy_math(),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.1.math(),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.1.math(),
        }
    }
    pub fn slew_math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, math), .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, math), .. }, .. } => math,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.slew_math(),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.2,
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.2,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { peak, .. } => peak,
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn unused_window_storage(&self) -> Option<&frontend_gain::SourceEnergyWindows> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, storage, _), .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, storage, _), .. }, .. } => Some(storage),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.unused_window_storage(),
            _ => None,
        }
    }
    pub fn window_energy_rows(&self) -> Option<&frontend_gain::SourceWindowEnergyRowsCompleted> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_energy_rows(),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => Some(&owner.1),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1),
            _ => None,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { metadata, .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { metadata, .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Admission { input, .. }, .. }
            | FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { input, .. }, .. } => Some(input),
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Gain { failure: GainImportFailure::Application { stages, .. }, .. } => stages.as_ref(),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            FilterWindowEnergySlewPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples),
            _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_filters_window_energy_slew_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: frontend_gain::SourceEnergyMath,
    storage: frontend_gain::SourceEnergyWindows, slew_math: frontend_gain::SourceSlewMath, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted,
        frontend_gain::SourceSlewMath, frontend_gain::SourcePeakMath), SourceFilterWindowEnergySlewPeakAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, (filter_math, outputs, window_math, energy_math, storage, slew_math),
        |input, (filter, outputs, window, energy, storage, slew)| {
            frontend_gain::frontend_gain_table_with_source_filters_window_energy_and_slew(input, -10.0, filter, outputs, window, energy, storage, slew)
                .map(|(stages, filter, rows, slew, input)| (stages, (filter, rows, slew), input))
        });
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterWindowEnergySlewPeakAudioFailure { bytes, policy,
            owner: FilterWindowEnergySlewPeakAudioOwner::Gain { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, (filter, rows, slew), peak)| (audio, filter, rows, slew, peak))
        .map_err(|failure| SourceFilterWindowEnergySlewPeakAudioFailure { bytes, policy,
            owner: FilterWindowEnergySlewPeakAudioOwner::Peak(failure) })
}

type FilterWindowEnergySlewFinalGainImportFailure = GainImportFailure<
    (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs, crate::frontend_energy_window_math::SourceWindowMath,
        frontend_gain::SourceEnergyMath, frontend_gain::SourceEnergyWindows, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath),
    (frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath),
    frontend_gain::SourceFilterWindowEnergySlewFinalGainFailure>;
enum FilterWindowEnergySlewFinalGainPeakAudioOwner<'a> {
    Gain { failure: FilterWindowEnergySlewFinalGainImportFailure, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted,
        frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath), frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceFilterWindowEnergySlewFinalGainPeakAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceFilterWindowEnergySlewFinalGainFailure),
    Application(&'a Error), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceFilterWindowEnergySlewFinalGainPeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, owner: FilterWindowEnergySlewFinalGainPeakAudioOwner<'a>,
}
impl SourceFilterWindowEnergySlewFinalGainPeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceFilterWindowEnergySlewFinalGainPeakAudioError<'_> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { error, .. }, .. } => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Decode(error),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { error, .. }, .. } => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Admission(error),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Gain(failure),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { error, .. }, .. } => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Application(error),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Peak(failure),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceFilterWindowEnergySlewFinalGainPeakAudioError::Output(error),
        }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (math, _, _, _, _, _, _), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (math, _, _, _, _, _, _), .. }, .. } => math,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.filter_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.0,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
        }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, math, _, _, _, _), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, math, _, _, _, _), .. }, .. } => math,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.1.window_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.1.window_math(),
        }
    }
    pub fn energy_math(&self) -> &frontend_gain::SourceEnergyMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, math, _, _, _), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, math, _, _, _), .. }, .. } => math,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.energy_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.1.math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.1.math(),
        }
    }
    pub fn slew_math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, math, _), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, math, _), .. }, .. } => math,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.slew_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.2,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.2,
        }
    }
    pub fn final_gain_math(&self) -> &frontend_gain::SourceFinalGainMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, _, math), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, _, math), .. }, .. } => math,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.final_gain_math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.3,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.3,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { peak, .. } => peak,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn unused_window_storage(&self) -> Option<&frontend_gain::SourceEnergyWindows> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, storage, _, _), .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, storage, _, _), .. }, .. } => Some(storage),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.unused_window_storage(),
            _ => None,
        }
    }
    pub fn window_energy_rows(&self) -> Option<&frontend_gain::SourceWindowEnergyRowsCompleted> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_energy_rows(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => Some(&owner.1),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1),
            _ => None,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { metadata, .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { metadata, .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { input, .. }, .. }
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { input, .. }, .. } => Some(input),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { stages, .. }, .. } => stages.as_ref(),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples),
            _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_filters_window_energy_slew_final_gain_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: frontend_gain::SourceEnergyMath,
    storage: frontend_gain::SourceEnergyWindows, slew_math: frontend_gain::SourceSlewMath,
    final_math: frontend_gain::SourceFinalGainMath, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, frontend_gain::SourceWindowEnergyRowsCompleted,
        frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath, frontend_gain::SourcePeakMath),
        SourceFilterWindowEnergySlewFinalGainPeakAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, (filter_math, outputs, window_math, energy_math, storage, slew_math, final_math),
        |input, (filter, outputs, window, energy, storage, slew, final_math)| {
            frontend_gain::frontend_gain_table_with_source_filters_window_energy_slew_and_final_gain(
                input, -10.0, filter, outputs, window, energy, storage, slew, final_math)
                .map(|(stages, filter, rows, slew, final_math, input)| (stages, (filter, rows, slew, final_math), input))
        });
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterWindowEnergySlewFinalGainPeakAudioFailure { bytes, policy,
            owner: FilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, (filter, rows, slew, final_math), peak)| (audio, filter, rows, slew, final_math, peak))
        .map_err(|failure| SourceFilterWindowEnergySlewFinalGainPeakAudioFailure { bytes, policy,
            owner: FilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(failure) })
}

type CountFilterWindowEnergySlewFinalGainImportFailure = GainImportFailure<
    (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs, crate::frontend_gain_count_math::SourceGainCountMath,
        crate::frontend_energy_window_math::SourceWindowMath, frontend_gain::SourceEnergyMath, frontend_gain::SourceEnergyWindows,
        frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath),
    (frontend_gain::SourceFilterMath, crate::frontend_gain_count_math::SourceGainCountParts,
        frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath),
    frontend_gain::SourceCountFilterWindowEnergySlewFinalGainFailure>;
enum CountFilterWindowEnergySlewFinalGainPeakAudioOwner<'a> {
    Gain { failure: CountFilterWindowEnergySlewFinalGainImportFailure, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceFilterMath, crate::frontend_gain_count_math::SourceGainCountParts,
        frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceCountFilterWindowEnergySlewFinalGainPeakAudioError<'a> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceCountFilterWindowEnergySlewFinalGainFailure),
    Application(&'a Error), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceCountFilterWindowEnergySlewFinalGainPeakAudioFailure<'a> {
    bytes: &'a [u8], policy: MathPolicy, owner: CountFilterWindowEnergySlewFinalGainPeakAudioOwner<'a>,
}
impl SourceCountFilterWindowEnergySlewFinalGainPeakAudioFailure<'_> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceCountFilterWindowEnergySlewFinalGainPeakAudioError<'_> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { error, .. }, .. } => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Decode(error),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { error, .. }, .. } => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Admission(error),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Gain(failure),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { error, .. }, .. } => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Application(error),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Peak(failure),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceCountFilterWindowEnergySlewFinalGainPeakAudioError::Output(error),
        }
    }
    pub fn filter_math(&self) -> &frontend_gain::SourceFilterMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (math, _, _, _, _, _, _, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (math, _, _, _, _, _, _, _), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.filter_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.0,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
        }
    }
    pub fn count_math(&self) -> &crate::frontend_gain_count_math::SourceGainCountMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, math, _, _, _, _, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, math, _, _, _, _, _), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.count_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.1.math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.1.math,
        }
    }
    pub fn count_parts(&self) -> Option<&crate::frontend_gain_count_math::SourceGainCountParts> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.count_parts(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => Some(&owner.1),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1),
            _ => None,
        }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, math, _, _, _, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, math, _, _, _, _), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.2.window_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.2.window_math(),
        }
    }
    pub fn energy_math(&self) -> &frontend_gain::SourceEnergyMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, math, _, _, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, math, _, _, _), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.energy_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => owner.2.math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => prefix.math.2.math(),
        }
    }
    pub fn slew_math(&self) -> &frontend_gain::SourceSlewMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, _, math, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, _, math, _), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.slew_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.3,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.3,
        }
    }
    pub fn final_gain_math(&self) -> &frontend_gain::SourceFinalGainMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, _, _, math), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, _, _, math), .. }, .. } => math,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.final_gain_math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => &owner.4,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.4,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { peak, .. } => peak,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn unused_window_storage(&self) -> Option<&frontend_gain::SourceEnergyWindows> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { initial: (_, _, _, _, _, storage, _, _), .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { initial: (_, _, _, _, _, storage, _, _), .. }, .. } => Some(storage),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.unused_window_storage(),
            _ => None,
        }
    }
    pub fn window_energy_rows(&self) -> Option<&frontend_gain::SourceWindowEnergyRowsCompleted> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => failure.window_energy_rows(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { owner, .. }, .. } => Some(&owner.2),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.2),
            _ => None,
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { metadata, .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { metadata, .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Decode { .. }, .. } => None,
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Admission { input, .. }, .. }
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { input, .. }, .. } => Some(input),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure: GainImportFailure::Application { stages, .. }, .. } => stages.as_ref(),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages),
            _ => None,
        }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(),
            _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples),
            _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_filters_count_window_energy_slew_final_gain_and_peak(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    count: crate::frontend_gain_count_math::SourceGainCountMath, window_math: crate::frontend_energy_window_math::SourceWindowMath,
    energy_math: frontend_gain::SourceEnergyMath, storage: frontend_gain::SourceEnergyWindows,
    slew_math: frontend_gain::SourceSlewMath, final_math: frontend_gain::SourceFinalGainMath, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, crate::frontend_gain_count_math::SourceGainCountParts,
        frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath, frontend_gain::SourcePeakMath),
        SourceCountFilterWindowEnergySlewFinalGainPeakAudioFailure<'_>>
{
    let prepared = prepare_wav_gain(bytes, policy, (filter_math, outputs, count, window_math, energy_math, storage, slew_math, final_math),
        |input, (filter, outputs, count, window, energy, storage, slew, final_math)| {
            frontend_gain::frontend_gain_table_with_source_filters_count_window_energy_slew_and_final_gain(
                input, -10.0, filter, outputs, count, window, energy, storage, slew, final_math)
                .map(|(stages, filter, count, rows, slew, final_math, input)| (stages, (filter, count, rows, slew, final_math), input))
        });
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceCountFilterWindowEnergySlewFinalGainPeakAudioFailure { bytes, policy,
            owner: CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Gain { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, (filter, count, rows, slew, final_math), peak)| (audio, filter, count, rows, slew, final_math, peak))
        .map_err(|failure| SourceCountFilterWindowEnergySlewFinalGainPeakAudioFailure { bytes, policy,
            owner: CountFilterWindowEnergySlewFinalGainPeakAudioOwner::Peak(failure) })
}

pub struct SourceGainPartialStages {
    first_filter: Vec<f32>, second_filter: Vec<f32>, energy: Vec<f32>, metric_log: Vec<f32>, slew_rounds: Vec<Vec<f32>>,
}
impl SourceGainPartialStages {
    pub fn first_filter(&self) -> &[f32] { &self.first_filter }
    pub fn second_filter(&self) -> &[f32] { &self.second_filter }
    pub fn energy(&self) -> &[f32] { &self.energy }
    pub fn metric_log(&self) -> &[f32] { &self.metric_log }
    pub fn slew_rounds(&self) -> &[Vec<f32>] { &self.slew_rounds }
    fn with_gain(self, gain_db: Vec<f32>) -> frontend_gain::FrontendGainStages {
        frontend_gain::FrontendGainStages { first_filter: self.first_filter, second_filter: self.second_filter,
            energy: self.energy, metric_log: self.metric_log, slew_rounds: self.slew_rounds, gain_db }
    }
}
struct ApplicationStageFailure<F> {
    stages: SourceGainPartialStages, failure: F,
}
type SourceApplicationStageFailure<P: frontend_gain::SourceGainPowerServices> = ApplicationStageFailure<frontend_gain::SourceGainApplicationFailure<P>>;
type SourceInPlaceApplicationStageFailure<P: frontend_gain::SourceGainPowerServices> = ApplicationStageFailure<frontend_gain::SourceGainInPlaceApplicationFailure<P>>;

struct ConsumedApplicationInput;
fn source_import_application<P: frontend_gain::SourceGainPowerServices>(input: Vec<f32>, stages: frontend_gain::FrontendGainStages,
    request: frontend_gain::SourceGainApplicationRequest<P>)
    -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>>, SourceApplicationStageFailure<P>>
{
    let frontend_gain::FrontendGainStages { first_filter, second_filter, energy, metric_log, slew_rounds, gain_db } = stages;
    let remaining = SourceGainPartialStages { first_filter, second_filter, energy, metric_log, slew_rounds };
    match frontend_gain::apply_gain_windows_with_source_services(input, gain_db, request) {
        Ok(completed) => {
            let frontend_gain::SourceGainApplicationParts { input, gain, output, context } = completed.into_parts();
            Ok(ApplicationImportCompleted { input, stages: remaining.with_gain(gain), gained: output, owner: context })
        }
        Err(failure) => Err(SourceApplicationStageFailure { stages: remaining, failure }),
    }
}
fn source_import_in_place_application<P: frontend_gain::SourceGainPowerServices>(input: Vec<f32>, stages: frontend_gain::FrontendGainStages,
    request: frontend_gain::SourceGainInPlaceApplicationRequest<P>)
    -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>, ConsumedApplicationInput>, SourceInPlaceApplicationStageFailure<P>>
{
    source_import_in_place_application_with(input, stages, request, frontend_gain::apply_gain_windows_in_place_with_source_services)
}
fn source_import_in_place_application_bits<P: frontend_gain::SourceGainPowerServices>(input: Vec<f32>, stages: frontend_gain::FrontendGainStages,
    request: frontend_gain::SourceGainInPlaceApplicationRequest<P>)
    -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>, ConsumedApplicationInput>, SourceInPlaceApplicationStageFailure<P>>
{
    source_import_in_place_application_with(input, stages, request, frontend_gain::apply_gain_windows_in_place_with_source_bits)
}
fn source_import_in_place_application_with<P: frontend_gain::SourceGainPowerServices>(input: Vec<f32>, stages: frontend_gain::FrontendGainStages,
    request: frontend_gain::SourceGainInPlaceApplicationRequest<P>,
    apply: impl FnOnce(Vec<f32>, Vec<f32>, frontend_gain::SourceGainInPlaceApplicationRequest<P>)
        -> Result<frontend_gain::SourceGainInPlaceApplicationCompleted<P>, frontend_gain::SourceGainInPlaceApplicationFailure<P>>)
    -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>, ConsumedApplicationInput>, SourceInPlaceApplicationStageFailure<P>>
{
    let frontend_gain::FrontendGainStages { first_filter, second_filter, energy, metric_log, slew_rounds, gain_db } = stages;
    let remaining = SourceGainPartialStages { first_filter, second_filter, energy, metric_log, slew_rounds };
    match apply(input, gain_db, request) {
        Ok(completed) => {
            let frontend_gain::SourceGainInPlaceApplicationParts { samples, gain, context } = completed.into_parts();
            Ok(ApplicationImportCompleted { input: ConsumedApplicationInput, stages: remaining.with_gain(gain), gained: samples, owner: context })
        }
        Err(failure) => Err(SourceInPlaceApplicationStageFailure { stages: remaining, failure }),
    }
}
type ApplicationGainInitial = (frontend_gain::SourceFilterMath, frontend_gain::SourceFilterOutputs,
    crate::frontend_gain_count_math::SourceGainCountMath, crate::frontend_energy_window_math::SourceWindowMath,
    frontend_gain::SourceEnergyMath, frontend_gain::SourceEnergyWindows, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath);
type ApplicationGainContext = (frontend_gain::SourceFilterMath, crate::frontend_gain_count_math::SourceGainCountParts,
    frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath);
type SourceApplicationPreparationFailure<P: frontend_gain::SourceGainPowerServices> = GainApplicationImportFailure<ApplicationGainInitial, ApplicationGainContext,
    frontend_gain::SourceCountFilterWindowEnergySlewFinalGainFailure, frontend_gain::SourceGainApplicationRequest<P>, SourceApplicationStageFailure<P>>;
enum SourceGainApplicationAudioOwner<'a, P: frontend_gain::SourceGainPowerServices> {
    BeforePeak { failure: SourceApplicationPreparationFailure<P>, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (ApplicationGainContext, frontend_gain::SourceGainApplicationContext<P>),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakFailure>),
}
pub enum SourceGainApplicationAudioError<'a, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceCountFilterWindowEnergySlewFinalGainFailure),
    Application(&'a frontend_gain::SourceGainApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceGainApplicationInitialRef<'a> {
    pub filter_math: &'a frontend_gain::SourceFilterMath, pub outputs: &'a frontend_gain::SourceFilterOutputs,
    pub count_math: &'a crate::frontend_gain_count_math::SourceGainCountMath,
    pub window_math: &'a crate::frontend_energy_window_math::SourceWindowMath, pub energy_math: &'a frontend_gain::SourceEnergyMath,
    pub windows: &'a frontend_gain::SourceEnergyWindows, pub slew_math: &'a frontend_gain::SourceSlewMath, pub final_gain_math: &'a frontend_gain::SourceFinalGainMath,
}
pub struct SourceGainApplicationGainContextRef<'a> {
    pub filter_math: &'a frontend_gain::SourceFilterMath, pub count: &'a crate::frontend_gain_count_math::SourceGainCountParts,
    pub rows: &'a frontend_gain::SourceWindowEnergyRowsCompleted, pub slew_math: &'a frontend_gain::SourceSlewMath,
    pub final_gain_math: &'a frontend_gain::SourceFinalGainMath,
}
pub struct SourceGainApplicationAudioFailure<'a, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy, owner: SourceGainApplicationAudioOwner<'a, P>,
}
impl<P: frontend_gain::SourceGainPowerServices> SourceGainApplicationAudioFailure<'_, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceGainApplicationAudioError<'_, P> {
        match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { error, .. }, .. } => SourceGainApplicationAudioError::Decode(error),
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { error, .. }, .. } => SourceGainApplicationAudioError::Admission(error),
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => SourceGainApplicationAudioError::Gain(failure),
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => SourceGainApplicationAudioError::Application(&failure.failure),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceGainApplicationAudioError::Peak(failure),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceGainApplicationAudioError::Output(error),
        }
    }
    pub fn initial_gain(&self) -> Option<SourceGainApplicationInitialRef<'_>> {
        let initial = match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { initial, .. }, .. }
            | SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { initial, .. }, .. } => initial,
            _ => return None,
        };
        Some(SourceGainApplicationInitialRef { filter_math: &initial.0, outputs: &initial.1, count_math: &initial.2,
            window_math: &initial.3, energy_math: &initial.4, windows: &initial.5, slew_math: &initial.6, final_gain_math: &initial.7 })
    }
    pub fn gain_context(&self) -> Option<SourceGainApplicationGainContextRef<'_>> {
        let owner = match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { owner, .. }, .. } => owner,
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => &prefix.math.0,
            _ => return None,
        };
        Some(SourceGainApplicationGainContextRef { filter_math: &owner.0, count: &owner.1,
            rows: &owner.2, slew_math: &owner.3, final_gain_math: &owner.4 })
    }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainApplicationRequest<P>> {
        match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { application, .. }, .. }
            | SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { application, .. }, .. }
            | SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { application, .. }, .. } => Some(application),
            _ => None,
        }
    }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainApplicationFailure<P>> {
        match &self.owner { SourceGainApplicationAudioOwner::BeforePeak {
            failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.failure), _ => None }
    }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> {
        match &self.owner { SourceGainApplicationAudioOwner::BeforePeak {
            failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.stages), _ => None }
    }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> {
        match &self.owner {
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1), _ => None,
        }
    }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath {
        match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { peak, .. } => peak,
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
        }
    }
    pub fn input(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { input, .. }, .. } => Some(input),
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure.input()),
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(failure.failure.input()),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
        }
    }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> {
        match &self.owner {
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
            SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { metadata, .. }, .. }
            | SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { metadata, .. }, .. }
            | SourceGainApplicationAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { metadata, .. }, .. } => Some(metadata),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
        }
    }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> {
        match &self.owner { SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
            | SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages), _ => None }
    }
    pub fn gained(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(), _ => None,
        }
    }
    pub fn output(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
            SourceGainApplicationAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None,
        }
    }
}

pub fn normalize_wav48k_with_source_count_gain_application_and_peak<P: frontend_gain::SourceGainPowerServices>(bytes: &[u8], policy: MathPolicy,
    filter_math: frontend_gain::SourceFilterMath, outputs: frontend_gain::SourceFilterOutputs,
    count: crate::frontend_gain_count_math::SourceGainCountMath, window_math: crate::frontend_energy_window_math::SourceWindowMath,
    energy_math: frontend_gain::SourceEnergyMath, windows: frontend_gain::SourceEnergyWindows,
    slew_math: frontend_gain::SourceSlewMath, final_math: frontend_gain::SourceFinalGainMath,
    application: frontend_gain::SourceGainApplicationRequest<P>, peak_math: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceFilterMath, crate::frontend_gain_count_math::SourceGainCountParts,
        frontend_gain::SourceWindowEnergyRowsCompleted, frontend_gain::SourceSlewMath, frontend_gain::SourceFinalGainMath,
        frontend_gain::SourceGainApplicationContext<P>, frontend_gain::SourcePeakMath), SourceGainApplicationAudioFailure<'_, P>>
{
    let prepared = prepare_wav_gain_with_application(bytes, policy,
        (filter_math, outputs, count, window_math, energy_math, windows, slew_math, final_math), application,
        |input, (filter, outputs, count, window, energy, windows, slew, final_math)| {
            frontend_gain::frontend_gain_table_with_source_filters_count_window_energy_slew_and_final_gain(
                input, -10.0, filter, outputs, count, window, energy, windows, slew, final_math)
                .map(|(stages, filter, count, rows, slew, final_math, input)| (stages, (filter, count, rows, slew, final_math), input))
        }, source_import_application);
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceGainApplicationAudioFailure { bytes, policy, owner: SourceGainApplicationAudioOwner::BeforePeak { failure, peak: peak_math } }),
    };
    finish_import_peak(prepared.prefix, prepared.gained, peak_math, source_peak_choice)
        .map(|(audio, ((filter, count, rows, slew, final_math), application), peak)| (audio, filter, count, rows, slew, final_math, application, peak))
        .map_err(|failure| SourceGainApplicationAudioFailure { bytes, policy, owner: SourceGainApplicationAudioOwner::Peak(failure) })
}

pub struct SourceLoggedAudioRequest<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    pub gain: frontend_gain::SourceLoggedGainRequest<L>,
    pub application: frontend_gain::SourceGainApplicationRequest<P>, pub peak: frontend_gain::SourcePeakMath,
}
type LoggedApplicationPreparationFailure<L, I, G, A, AF> = GainApplicationImportFailure<I,
    frontend_gain::SourceLoggedGainContext<L>, G, A, AF>;
enum SourceLoggedAudioOwner<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices,
    I = frontend_gain::SourceLoggedGainRequest<L>, G = frontend_gain::SourceLoggedGainFailure<L>,
    A = frontend_gain::SourceGainApplicationRequest<P>, AF = SourceApplicationStageFailure<P>, D = Vec<f32>, PE = frontend_gain::SourcePeakFailure> {
    BeforePeak { failure: LoggedApplicationPreparationFailure<L, I, G, A, AF>, peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>),
        frontend_gain::SourcePeakMath, PE, D>),
}
enum LoggedAudioErrorRef<'a, G, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a G),
    Application(&'a frontend_gain::SourceGainApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub enum SourceLoggedAudioError<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceLoggedGainFailure<L>),
    Application(&'a frontend_gain::SourceGainApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceLoggedAudioFailure<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy, owner: SourceLoggedAudioOwner<'a, L, P>,
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> SourceLoggedAudioFailure<'_, L, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceLoggedAudioError<'_, L, P> { match self.owner.error() {
        LoggedAudioErrorRef::Decode(error) => SourceLoggedAudioError::Decode(error),
        LoggedAudioErrorRef::Admission(error) => SourceLoggedAudioError::Admission(error),
        LoggedAudioErrorRef::Gain(failure) => SourceLoggedAudioError::Gain(failure),
        LoggedAudioErrorRef::Application(failure) => SourceLoggedAudioError::Application(failure),
        LoggedAudioErrorRef::Peak(failure) => SourceLoggedAudioError::Peak(failure),
        LoggedAudioErrorRef::Output(error) => SourceLoggedAudioError::Output(error),
    } }
    pub fn initial_gain(&self) -> Option<&frontend_gain::SourceLoggedGainRequest<L>> { self.owner.initial_gain() }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceLoggedGainFailure<L>> { self.owner.gain_failure() }
    pub fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { self.owner.gain_context() }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainApplicationRequest<P>> { self.owner.unattempted_application() }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainApplicationFailure<P>> { self.owner.application_failure() }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> { self.owner.partial_stages() }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { self.owner.application_context() }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath { self.owner.peak_math() }
    pub fn input(&self) -> Option<&[f32]> { self.owner.input_with(|failure| failure.input()) }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { self.owner.metadata() }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { self.owner.stages() }
    pub fn gained(&self) -> Option<&[f32]> { self.owner.gained() }
    pub fn output(&self) -> Option<&[f32]> { self.owner.output() }
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices, I, G>
    SourceLoggedAudioOwner<'_, L, P, I, G>
{
    fn error(&self) -> LoggedAudioErrorRef<'_, G, P> { match self {
        Self::BeforePeak { failure: GainApplicationImportFailure::Decode { error, .. }, .. } => LoggedAudioErrorRef::Decode(error),
        Self::BeforePeak { failure: GainApplicationImportFailure::Admission { error, .. }, .. } => LoggedAudioErrorRef::Admission(error),
        Self::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => LoggedAudioErrorRef::Gain(failure),
        Self::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => LoggedAudioErrorRef::Application(&failure.failure),
        Self::Peak(PeakImportFailure::Peak { failure, .. }) => LoggedAudioErrorRef::Peak(failure),
        Self::Peak(PeakImportFailure::Output { error, .. }) => LoggedAudioErrorRef::Output(error),
    } }
    fn initial_gain(&self) -> Option<&I> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { initial, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { initial, .. }, .. } => Some(initial), _ => None,
    } }
    fn gain_failure(&self) -> Option<&G> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure), _ => None,
    } }
    fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { owner, .. }, .. } => Some(owner),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.0), _ => None,
    } }
    fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainApplicationRequest<P>> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { application, .. }, .. } => Some(application), _ => None,
    } }
    fn application_failure(&self) -> Option<&frontend_gain::SourceGainApplicationFailure<P>> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.failure), _ => None,
    } }
    fn partial_stages(&self) -> Option<&SourceGainPartialStages> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.stages), _ => None,
    } }
    fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { match self {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1), _ => None,
    } }
    fn peak_math(&self) -> &frontend_gain::SourcePeakMath { match self {
        SourceLoggedAudioOwner::BeforePeak { peak, .. } => peak,
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
    } }
    fn input_with<'b>(&'b self, from_gain: impl FnOnce(&'b G) -> &'b [f32]) -> Option<&'b [f32]> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { input, .. }, .. } => Some(input),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(from_gain(failure)),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(failure.failure.input()),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.input),
    } }
    fn metadata(&self) -> Option<&wav48k::Metadata> { match self {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { metadata, .. }, .. } => Some(metadata),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
    } }
    fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { match self {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages), _ => None,
    } }
    fn gained(&self) -> Option<&[f32]> { match self {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => completed.gained.as_deref(), _ => None,
    } }
    fn output(&self) -> Option<&[f32]> { match self {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None,
    } }
}

pub fn normalize_wav48k_with_source_logs<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    bytes: &'a [u8], policy: MathPolicy, request: SourceLoggedAudioRequest<L, P>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceLoggedAudioFailure<'a, L, P>>
{
    let SourceLoggedAudioRequest { gain, application, peak } = request;
    normalize_logged_wav(bytes, policy, gain, application, peak,
        |input, request| frontend_gain::frontend_gain_with_source_logs(input, -10.0, request))
        .map_err(|owner| SourceLoggedAudioFailure { bytes, policy, owner })
}
pub struct SourceInitializedLoggedAudioRequest<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    pub gain: frontend_gain::SourceInitializedLoggedGainRequest<L>,
    pub application: frontend_gain::SourceGainApplicationRequest<P>, pub peak: frontend_gain::SourcePeakMath,
}
pub enum SourceInitializedLoggedAudioError<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceInitializedLoggedGainFailure<L>),
    Application(&'a frontend_gain::SourceGainApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceInitializedLoggedAudioFailure<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy,
    owner: SourceLoggedAudioOwner<'a, L, P, frontend_gain::SourceInitializedLoggedGainRequest<L>, frontend_gain::SourceInitializedLoggedGainFailure<L>>,
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> SourceInitializedLoggedAudioFailure<'_, L, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceInitializedLoggedAudioError<'_, L, P> { match self.owner.error() {
        LoggedAudioErrorRef::Decode(error) => SourceInitializedLoggedAudioError::Decode(error),
        LoggedAudioErrorRef::Admission(error) => SourceInitializedLoggedAudioError::Admission(error),
        LoggedAudioErrorRef::Gain(failure) => SourceInitializedLoggedAudioError::Gain(failure),
        LoggedAudioErrorRef::Application(failure) => SourceInitializedLoggedAudioError::Application(failure),
        LoggedAudioErrorRef::Peak(failure) => SourceInitializedLoggedAudioError::Peak(failure),
        LoggedAudioErrorRef::Output(error) => SourceInitializedLoggedAudioError::Output(error),
    } }
    pub fn initial_gain(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainRequest<L>> { self.owner.initial_gain() }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainFailure<L>> { self.owner.gain_failure() }
    pub fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { self.owner.gain_context() }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainApplicationRequest<P>> { self.owner.unattempted_application() }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainApplicationFailure<P>> { self.owner.application_failure() }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> { self.owner.partial_stages() }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { self.owner.application_context() }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath { self.owner.peak_math() }
    pub fn input(&self) -> Option<&[f32]> { self.owner.input_with(|failure| failure.input()) }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { self.owner.metadata() }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { self.owner.stages() }
    pub fn gained(&self) -> Option<&[f32]> { self.owner.gained() }
    pub fn output(&self) -> Option<&[f32]> { self.owner.output() }
}

pub fn normalize_wav48k_with_initialized_source_logs<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    bytes: &'a [u8], policy: MathPolicy, request: SourceInitializedLoggedAudioRequest<L, P>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceInitializedLoggedAudioFailure<'a, L, P>>
{
    let SourceInitializedLoggedAudioRequest { gain, application, peak } = request;
    normalize_logged_wav_with_application_and_peak(bytes, policy, gain, application, peak,
        |input, request| frontend_gain::frontend_gain_with_initialized_source_logs(input, -10.0, request),
        source_import_application, source_peak_choice, WavGainAdmission::PositiveSource, PeakOutputAdmission::LegacyFinite)
        .map_err(|owner| SourceInitializedLoggedAudioFailure { bytes, policy, owner })
}
pub struct SourceInitializedInPlaceLoggedAudioRequest<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    pub gain: frontend_gain::SourceInitializedLoggedGainRequest<L>,
    pub application: frontend_gain::SourceGainInPlaceApplicationRequest<P>, pub peak: frontend_gain::SourcePeakMath,
}
pub enum SourceInitializedInPlaceLoggedAudioError<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceInitializedLoggedGainFailure<L>),
    Application(&'a frontend_gain::SourceGainInPlaceApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakFailure), Output(&'a Error),
}
pub struct SourceInitializedInPlaceLoggedAudioFailure<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy,
    owner: SourceLoggedAudioOwner<'a, L, P, frontend_gain::SourceInitializedLoggedGainRequest<L>, frontend_gain::SourceInitializedLoggedGainFailure<L>,
        frontend_gain::SourceGainInPlaceApplicationRequest<P>, SourceInPlaceApplicationStageFailure<P>, ConsumedApplicationInput>,
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> SourceInitializedInPlaceLoggedAudioFailure<'_, L, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceInitializedInPlaceLoggedAudioError<'_, L, P> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { error, .. }, .. } => SourceInitializedInPlaceLoggedAudioError::Decode(error),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { error, .. }, .. } => SourceInitializedInPlaceLoggedAudioError::Admission(error),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => SourceInitializedInPlaceLoggedAudioError::Gain(failure),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => SourceInitializedInPlaceLoggedAudioError::Application(&failure.failure),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceInitializedInPlaceLoggedAudioError::Peak(failure),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceInitializedInPlaceLoggedAudioError::Output(error),
    } }
    pub fn initial_gain(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainRequest<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { initial, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { initial, .. }, .. } => Some(initial), _ => None,
    } }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainFailure<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure), _ => None,
    } }
    pub fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { owner, .. }, .. } => Some(owner),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.0), _ => None,
    } }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationRequest<P>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { application, .. }, .. } => Some(application), _ => None,
    } }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationFailure<P>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.failure), _ => None,
    } }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.stages), _ => None,
    } }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1), _ => None,
    } }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { peak, .. } => peak,
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
    } }

    pub fn original_input(&self) -> Option<&[f32]> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { input, .. }, .. } => Some(input),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure.input()), _ => None,
    } }

    pub fn application_samples(&self) -> Option<&[f32]> { self.application_failure().map(|failure| failure.samples()) }

    pub fn peak_input(&self) -> Option<&[f32]> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.input()), _ => None,
    } }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { metadata, .. }, .. } => Some(metadata),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
    } }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages), _ => None,
    } }
    pub fn output(&self) -> Option<&[f32]> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure.progress().output()),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None,
    } }
}

pub fn normalize_wav48k_with_initialized_in_place_source_logs<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    bytes: &'a [u8], policy: MathPolicy, request: SourceInitializedInPlaceLoggedAudioRequest<L, P>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceInitializedInPlaceLoggedAudioFailure<'a, L, P>>
{
    let SourceInitializedInPlaceLoggedAudioRequest { gain, application, peak } = request;
    normalize_logged_wav_with_application_and_peak(bytes, policy, gain, application, peak,
        |input, request| frontend_gain::frontend_gain_with_initialized_source_logs(input, -10.0, request),
        source_import_in_place_application, source_peak_choice, WavGainAdmission::PositiveSource, PeakOutputAdmission::LegacyFinite)
        .map_err(|owner| SourceInitializedInPlaceLoggedAudioFailure { bytes, policy, owner })
}
pub enum SourceInitializedFullyInPlaceLoggedAudioError<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceInitializedLoggedGainFailure<L>),
    Application(&'a frontend_gain::SourceGainInPlaceApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakInPlaceFailure), Output(&'a Error),
}
pub struct SourceInitializedFullyInPlaceLoggedAudioFailure<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy,
    owner: SourceLoggedAudioOwner<'a, L, P, frontend_gain::SourceInitializedLoggedGainRequest<L>, frontend_gain::SourceInitializedLoggedGainFailure<L>,
        frontend_gain::SourceGainInPlaceApplicationRequest<P>, SourceInPlaceApplicationStageFailure<P>, ConsumedApplicationInput, frontend_gain::SourcePeakInPlaceFailure>,
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> SourceInitializedFullyInPlaceLoggedAudioFailure<'_, L, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceInitializedFullyInPlaceLoggedAudioError<'_, L, P> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { error, .. }, .. } => SourceInitializedFullyInPlaceLoggedAudioError::Decode(error),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { error, .. }, .. } => SourceInitializedFullyInPlaceLoggedAudioError::Admission(error),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => SourceInitializedFullyInPlaceLoggedAudioError::Gain(failure),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => SourceInitializedFullyInPlaceLoggedAudioError::Application(&failure.failure),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceInitializedFullyInPlaceLoggedAudioError::Peak(failure),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceInitializedFullyInPlaceLoggedAudioError::Output(error),
    } }
    pub fn initial_gain(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainRequest<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { initial, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { initial, .. }, .. } => Some(initial), _ => None,
    } }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainFailure<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure), _ => None,
    } }
    pub fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { owner, .. }, .. } => Some(owner),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.0), _ => None,
    } }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationRequest<P>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { application, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { application, .. }, .. } => Some(application), _ => None,
    } }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationFailure<P>> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.failure), _ => None,
    } }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.stages), _ => None,
    } }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.1), _ => None,
    } }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { peak, .. } => peak,
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
    } }

    pub fn original_input(&self) -> Option<&[f32]> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { input, .. }, .. } => Some(input),
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure.input()), _ => None,
    } }

    pub fn application_samples(&self) -> Option<&[f32]> { self.application_failure().map(|failure| failure.samples()) }
    pub fn peak_failure(&self) -> Option<&frontend_gain::SourcePeakInPlaceFailure> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure), _ => None,
    } }

    pub fn peak_samples(&self) -> Option<&[f32]> { self.peak_failure().map(|failure| failure.samples()) }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { match &self.owner {
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
        SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { metadata, .. }, .. }
        | SourceLoggedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { metadata, .. }, .. } => Some(metadata),
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
    } }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages), _ => None,
    } }

    pub fn output(&self) -> Option<&[f32]> { match &self.owner {
        SourceLoggedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None,
    } }
}

pub fn normalize_wav48k_with_initialized_fully_in_place_source_logs<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    bytes: &'a [u8], policy: MathPolicy, request: SourceInitializedInPlaceLoggedAudioRequest<L, P>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceInitializedFullyInPlaceLoggedAudioFailure<'a, L, P>>
{
    let SourceInitializedInPlaceLoggedAudioRequest { gain, application, peak } = request;
    normalize_logged_wav_with_application_and_peak(bytes, policy, gain, application, peak,
        |input, request| frontend_gain::frontend_gain_with_initialized_source_logs(input, -10.0, request),
        source_import_in_place_application_bits, source_in_place_peak_choice, WavGainAdmission::PositiveSource, PeakOutputAdmission::SourceBits)
        .map_err(|owner| SourceInitializedFullyInPlaceLoggedAudioFailure { bytes, policy, owner })
}
pub enum SourceEmptyAwareApplicationDisposition<P: frontend_gain::SourceGainPowerServices> {
    Unused(frontend_gain::SourceGainInPlaceApplicationRequest<P>),
    Applied(frontend_gain::SourceGainApplicationContext<P>),
}
pub enum SourceEmptyAwarePeakDisposition {
    Unused(frontend_gain::SourcePeakMath), Applied(frontend_gain::SourcePeakMath),
}
enum EmptyApplicationInput { SkippedEmpty, Applied(ConsumedApplicationInput) }
fn source_import_empty_aware_application<P: frontend_gain::SourceGainPowerServices>(input: Vec<f32>, stages: frontend_gain::FrontendGainStages,
    request: frontend_gain::SourceGainInPlaceApplicationRequest<P>)
    -> Result<ApplicationImportCompleted<SourceEmptyAwareApplicationDisposition<P>, EmptyApplicationInput>, SourceInPlaceApplicationStageFailure<P>>
{
    if input.is_empty() {
        return Ok(ApplicationImportCompleted { input: EmptyApplicationInput::SkippedEmpty, stages, gained: input,
            owner: SourceEmptyAwareApplicationDisposition::Unused(request) });
    }
    source_import_in_place_application_bits(input, stages, request).map(|ApplicationImportCompleted { input, stages, gained, owner }|
        ApplicationImportCompleted { input: EmptyApplicationInput::Applied(input), stages, gained,
            owner: SourceEmptyAwareApplicationDisposition::Applied(owner) })
}
enum EmptyAwareInitializedAudioOwner<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    BeforePeak { failure: GainApplicationImportFailure<frontend_gain::SourceInitializedLoggedGainRequest<L>, frontend_gain::SourceLoggedGainContext<L>,
        frontend_gain::SourceInitializedLoggedGainFailure<L>, frontend_gain::SourceGainInPlaceApplicationRequest<P>, SourceInPlaceApplicationStageFailure<P>>,
        peak: frontend_gain::SourcePeakMath },
    Peak(PeakImportFailure<'a, (frontend_gain::SourceLoggedGainContext<L>, SourceEmptyAwareApplicationDisposition<P>),
        frontend_gain::SourcePeakMath, frontend_gain::SourcePeakInPlaceFailure, EmptyApplicationInput>),
}
pub enum SourceEmptyAwareInitializedLoggedAudioError<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    Decode(&'a wav48k::Error), Admission(&'a Error), Gain(&'a frontend_gain::SourceInitializedLoggedGainFailure<L>),
    Application(&'a frontend_gain::SourceGainInPlaceApplicationFailure<P>), Peak(&'a frontend_gain::SourcePeakInPlaceFailure), Output(&'a Error),
}
pub struct SourceEmptyAwareInitializedLoggedAudioFailure<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> {
    bytes: &'a [u8], policy: MathPolicy, owner: EmptyAwareInitializedAudioOwner<'a, L, P>,
}
impl<L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices> SourceEmptyAwareInitializedLoggedAudioFailure<'_, L, P> {
    pub fn bytes(&self) -> &[u8] { self.bytes }
    pub fn policy(&self) -> MathPolicy { self.policy }
    pub fn error(&self) -> SourceEmptyAwareInitializedLoggedAudioError<'_, L, P> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { error, .. }, .. } => SourceEmptyAwareInitializedLoggedAudioError::Decode(error),
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { error, .. }, .. } => SourceEmptyAwareInitializedLoggedAudioError::Admission(error),
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => SourceEmptyAwareInitializedLoggedAudioError::Gain(failure),
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => SourceEmptyAwareInitializedLoggedAudioError::Application(&failure.failure),
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => SourceEmptyAwareInitializedLoggedAudioError::Peak(failure),
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { error, .. }) => SourceEmptyAwareInitializedLoggedAudioError::Output(error),
    } }
    pub fn initial_gain(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainRequest<L>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { initial, .. }, .. }
        | EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { initial, .. }, .. } => Some(initial), _ => None,
    } }
    pub fn gain_failure(&self) -> Option<&frontend_gain::SourceInitializedLoggedGainFailure<L>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure), _ => None,
    } }
    pub fn gain_context(&self) -> Option<&frontend_gain::SourceLoggedGainContext<L>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { owner, .. }, .. } => Some(owner),
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.math.0), _ => None,
    } }
    pub fn unattempted_application(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationRequest<P>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { application, .. }, .. }
        | EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { application, .. }, .. }
        | EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { application, .. }, .. } => Some(application), _ => None,
    } }
    pub fn application_failure(&self) -> Option<&frontend_gain::SourceGainInPlaceApplicationFailure<P>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.failure), _ => None,
    } }
    pub fn partial_stages(&self) -> Option<&SourceGainPartialStages> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { failure, .. }, .. } => Some(&failure.stages), _ => None,
    } }
    pub fn application_context(&self) -> Option<&frontend_gain::SourceGainApplicationContext<P>> { match &self.owner {
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => match &prefix.math.1 {
            SourceEmptyAwareApplicationDisposition::Applied(context) => Some(context), _ => None,
        }, _ => None,
    } }
    pub fn peak_math(&self) -> &frontend_gain::SourcePeakMath { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { peak, .. } => peak,
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => failure.math(),
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => &completed.peak,
    } }

    pub fn original_input(&self) -> Option<&[f32]> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { input, .. }, .. } => Some(input),
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { failure, .. }, .. } => Some(failure.input()), _ => None,
    } }
    pub fn application_samples(&self) -> Option<&[f32]> { self.application_failure().map(|failure| failure.samples()) }
    pub fn peak_failure(&self) -> Option<&frontend_gain::SourcePeakInPlaceFailure> { match &self.owner {
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { failure, .. }) => Some(failure), _ => None,
    } }
    pub fn peak_samples(&self) -> Option<&[f32]> { self.peak_failure().map(|failure| failure.samples()) }
    pub fn metadata(&self) -> Option<&wav48k::Metadata> { match &self.owner {
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Decode { .. }, .. } => None,
        EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Admission { metadata, .. }, .. }
        | EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Gain { metadata, .. }, .. }
        | EmptyAwareInitializedAudioOwner::BeforePeak { failure: GainApplicationImportFailure::Application { metadata, .. }, .. } => Some(metadata),
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.metadata),
    } }
    pub fn stages(&self) -> Option<&frontend_gain::FrontendGainStages> { match &self.owner {
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Peak { prefix, .. })
        | EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { prefix, .. }) => Some(&prefix.stages), _ => None,
    } }
    pub fn output(&self) -> Option<&[f32]> { match &self.owner {
        EmptyAwareInitializedAudioOwner::Peak(PeakImportFailure::Output { completed, .. }) => Some(&completed.samples), _ => None,
    } }
}
fn finish_empty_aware_import_peak<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    prepared: PreparedImport<'a, (frontend_gain::SourceLoggedGainContext<L>, SourceEmptyAwareApplicationDisposition<P>), EmptyApplicationInput>,
    peak: frontend_gain::SourcePeakMath)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, SourceEmptyAwareApplicationDisposition<P>, SourceEmptyAwarePeakDisposition),
        PeakImportFailure<'a, (frontend_gain::SourceLoggedGainContext<L>, SourceEmptyAwareApplicationDisposition<P>),
            frontend_gain::SourcePeakMath, frontend_gain::SourcePeakInPlaceFailure, EmptyApplicationInput>>
{
    let PreparedImport { prefix, gained } = prepared;

    if matches!(&prefix.input, EmptyApplicationInput::SkippedEmpty) {
        let ImportPrefix { metadata, math: (gain, application), .. } = prefix;
        return Ok((NormalizedAudio { metadata, samples: gained }, gain, application, SourceEmptyAwarePeakDisposition::Unused(peak)));
    }
    finish_import_peak_with_admission(prefix, gained, peak, source_in_place_peak_choice, PeakOutputAdmission::SourceBits)
        .map(|(audio, (gain, application), peak)| (audio, gain, application, SourceEmptyAwarePeakDisposition::Applied(peak)))
}

pub fn normalize_wav48k_with_empty_aware_initialized_in_place_source_logs<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices>(
    bytes: &'a [u8], policy: MathPolicy, request: SourceInitializedInPlaceLoggedAudioRequest<L, P>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, SourceEmptyAwareApplicationDisposition<P>, SourceEmptyAwarePeakDisposition),
        SourceEmptyAwareInitializedLoggedAudioFailure<'a, L, P>>
{
    let SourceInitializedInPlaceLoggedAudioRequest { gain, application, peak } = request;
    let prepared = match prepare_logged_wav_with_application(bytes, policy, gain, application,
        |input, request| frontend_gain::frontend_gain_with_initialized_source_logs(input, -10.0, request),
        source_import_empty_aware_application, WavGainAdmission::SourceFinite) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceEmptyAwareInitializedLoggedAudioFailure { bytes, policy,
            owner: EmptyAwareInitializedAudioOwner::BeforePeak { failure, peak } }),
    };
    finish_empty_aware_import_peak(prepared, peak).map_err(|failure| SourceEmptyAwareInitializedLoggedAudioFailure {
        bytes, policy, owner: EmptyAwareInitializedAudioOwner::Peak(failure) })
}
fn normalize_logged_wav<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices, I, G>(
    bytes: &'a [u8], policy: MathPolicy, initial: I, application: frontend_gain::SourceGainApplicationRequest<P>, peak: frontend_gain::SourcePeakMath,
    gain: impl FnOnce(Vec<f32>, I) -> Result<frontend_gain::SourceLoggedGainCompleted<L>, G>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceLoggedAudioOwner<'a, L, P, I, G>>
{
    normalize_logged_wav_with_application(bytes, policy, initial, application, peak, gain, source_import_application)
}
fn normalize_logged_wav_with_application<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices,
    I, G, A, AF, D>(bytes: &'a [u8], policy: MathPolicy, initial: I, application: A, peak: frontend_gain::SourcePeakMath,
    gain: impl FnOnce(Vec<f32>, I) -> Result<frontend_gain::SourceLoggedGainCompleted<L>, G>,
    apply: impl FnOnce(Vec<f32>, frontend_gain::FrontendGainStages, A)
        -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>, D>, AF>)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceLoggedAudioOwner<'a, L, P, I, G, A, AF, D>>
{
    normalize_logged_wav_with_application_and_peak(bytes, policy, initial, application, peak, gain, apply, source_peak_choice,
        WavGainAdmission::Legacy, PeakOutputAdmission::LegacyFinite)
}
fn normalize_logged_wav_with_application_and_peak<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices, P: frontend_gain::SourceGainPowerServices,
    I, G, A, AF, D, PE>(bytes: &'a [u8], policy: MathPolicy, initial: I, application: A, peak: frontend_gain::SourcePeakMath,
    gain: impl FnOnce(Vec<f32>, I) -> Result<frontend_gain::SourceLoggedGainCompleted<L>, G>,
    apply: impl FnOnce(Vec<f32>, frontend_gain::FrontendGainStages, A)
        -> Result<ApplicationImportCompleted<frontend_gain::SourceGainApplicationContext<P>, D>, AF>,
    peak_choice: impl FnOnce(Vec<f32>, frontend_gain::SourcePeakMath) -> Result<PeakCompleted<frontend_gain::SourcePeakMath>, PE>,
    admission: WavGainAdmission, output_admission: PeakOutputAdmission)
    -> Result<(NormalizedAudio, frontend_gain::SourceLoggedGainContext<L>, frontend_gain::SourceGainApplicationContext<P>,
        frontend_gain::SourcePeakMath), SourceLoggedAudioOwner<'a, L, P, I, G, A, AF, D, PE>>
{
    let prepared = match prepare_logged_wav_with_application(bytes, policy, initial, application, gain, apply, admission) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceLoggedAudioOwner::BeforePeak { failure, peak }),
    };
    finish_import_peak_with_admission(prepared.prefix, prepared.gained, peak, peak_choice, output_admission)
        .map(|(audio, (gain, application), peak)| (audio, gain, application, peak))
        .map_err(SourceLoggedAudioOwner::Peak)
}
fn prepare_logged_wav_with_application<'a, L: SourceGainRowLogServices + SourceGainStatisticsLogServices,
    I, G, A, AO, AF, D>(bytes: &'a [u8], policy: MathPolicy, initial: I, application: A,
    gain: impl FnOnce(Vec<f32>, I) -> Result<frontend_gain::SourceLoggedGainCompleted<L>, G>,
    apply: impl FnOnce(Vec<f32>, frontend_gain::FrontendGainStages, A) -> Result<ApplicationImportCompleted<AO, D>, AF>,
    admission: WavGainAdmission)
    -> Result<PreparedImport<'a, (frontend_gain::SourceLoggedGainContext<L>, AO), D>,
        GainApplicationImportFailure<I, frontend_gain::SourceLoggedGainContext<L>, G, A, AF>>
{
    prepare_wav_gain_with_application_and_admission(bytes, policy, initial, application,
        |input, request| gain(input, request).map(|completed| {
            let frontend_gain::SourceLoggedGainParts { input, stages, context } = completed.into_parts();
            (stages, context, input)
        }), apply, admission)
}

pub fn normalize_supported_wav(
    bytes: &[u8],
    gain_policy: MathPolicy,
    resampler_policy: ResamplerPolicy,
) -> Result<PreparedAudio, Error> {
    let metadata = wav48k::inspect_supported(bytes).map_err(Error::Decode)?;
    if matches!(metadata.sample_rate,
        4_000 | 6_000 | 8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000
            | 44_100 | 64_000 | 72_000 | 88_200 | 96_000 | 144_000
            | 176_400 | 192_000 | 288_000 | 352_800 | 384_000)
    {
        crate::resample44100::validate_environment(resampler_policy).map_err(Error::Resample)?;
    }
    let analysis_count = match metadata.sample_rate {
        44_100 if metadata.frames != 0 =>
            crate::resample44100::output_count(metadata.frames).map_err(Error::Resample)?,
        rate @ (4_000 | 6_000 | 8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000
            | 64_000 | 72_000 | 88_200 | 96_000 | 144_000 | 176_400 | 192_000
            | 288_000 | 352_800 | 384_000)
            if metadata.frames != 0 =>
            crate::resample44100::single_fir_output_count(metadata.frames, rate)
                .map_err(Error::Resample)?,
        4_000 | 6_000 | 8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
            | 64_000 | 72_000 | 88_200 | 96_000 | 144_000 | 176_400 | 192_000
            | 288_000 | 352_800 | 384_000 =>
            metadata.frames,
        rate => return Err(Error::Decode(wav48k::Error::UnsupportedSampleRate(rate))),
    };
    if analysis_count <= 1920 {
        return Err(Error::UnsupportedShortInput(analysis_count));
    }
    let decoded = wav48k::decode_supported(bytes).map_err(Error::Decode)?;
    if decoded.channel0.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFiniteInput);
    }
    let input = match decoded.metadata.sample_rate {
        44_100 => crate::resample44100::convert_channel0(&decoded.channel0, resampler_policy)
            .map_err(Error::Resample)?,
        rate @ (4_000 | 6_000 | 8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000
            | 64_000 | 72_000 | 88_200 | 96_000 | 144_000 | 176_400 | 192_000
            | 288_000 | 352_800 | 384_000) =>
            crate::resample44100::convert_single_fir_channel0(
            &decoded.channel0, rate, resampler_policy,
        ).map_err(Error::Resample)?,
        48_000 => decoded.channel0,
        rate => return Err(Error::Decode(wav48k::Error::UnsupportedSampleRate(rate))),
    };
    let samples = normalize_mono48k(&input, gain_policy)?;
    Ok(PreparedAudio { metadata: decoded.metadata, analysis_rate: wav48k::SAMPLE_RATE, samples })
}

pub fn normalize_source_plan_wav(
    bytes: &[u8],
    expected_source_rate: u32,
    gain_policy: MathPolicy,
    resampler_policy: ResamplerPolicy,
) -> Result<PreparedAudio, Error> {
    let cursor = wav48k::Channel0Cursor::from_source_plan(bytes, expected_source_rate)
        .map_err(Error::Decode)?;
    let metadata = cursor.metadata();
    if metadata.sample_rate != wav48k::SAMPLE_RATE {
        crate::resample44100::validate_environment(resampler_policy).map_err(Error::Resample)?;
    }
    let analysis_count = if metadata.frames == 0 { 0 } else {
        crate::resample44100::source_plan_output_count(metadata.frames, metadata.sample_rate)
            .map_err(Error::Resample)?
    };
    if analysis_count <= 1920 { return Err(Error::UnsupportedShortInput(analysis_count)); }

    if cursor.first_nonfinite_frame().is_some() { return Err(Error::NonFiniteInput); }
    let (metadata, input) = crate::resample44100::convert_borrowed_wav_channel0(cursor, resampler_policy)
        .map_err(Error::Resample)?;
    let samples = normalize_mono48k(&input, gain_policy)?;
    Ok(PreparedAudio { metadata, analysis_rate: wav48k::SAMPLE_RATE, samples })
}

struct PreparedGain { stages: frontend_gain::FrontendGainStages, gained: Vec<f32> }
struct GainPreparationFailure { error: Error, stages: Option<frontend_gain::FrontendGainStages> }
fn prepare_mono48k(input: &[f32], policy: MathPolicy) -> Result<PreparedGain, GainPreparationFailure> {
    admit_mono48k(input, policy).map_err(|error| GainPreparationFailure { error, stages: None })?;
    let stages = frontend_gain::frontend_gain_table(input, -10.0_f32)
        .map_err(|error| GainPreparationFailure { error: Error::NumericDomain(error), stages: None })?;
    apply_prepared_gain(input, stages)
}
fn admit_mono48k(input: &[f32], policy: MathPolicy) -> Result<(), Error> {
    admit_mono48k_with_admission(input, policy, WavGainAdmission::Legacy)
}
fn admit_mono48k_with_admission(input: &[f32], policy: MathPolicy, admission: WavGainAdmission) -> Result<(), Error> {
    if match admission { WavGainAdmission::Legacy => input.len() <= 1920,
        WavGainAdmission::PositiveSource => input.is_empty(), WavGainAdmission::SourceFinite => false } {
        return Err(Error::UnsupportedShortInput(input.len()));
    }
    if input.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFiniteInput);
    }
    match policy {
        MathPolicy::WindowsHostLog10RoundF32AndPowf => {}
    }
    Ok(())
}
fn apply_prepared_gain(input: &[f32], stages: frontend_gain::FrontendGainStages) -> Result<PreparedGain, GainPreparationFailure> {
    match frontend_gain::apply_gain_windows(input, &stages.gain_db) {
        Ok(gained) => Ok(PreparedGain { stages, gained }),
        Err(error) => Err(GainPreparationFailure { error: Error::NumericDomain(error), stages: Some(stages) }),
    }
}

pub(crate) fn normalize_mono48k(input: &[f32], policy: MathPolicy) -> Result<Vec<f32>, Error> {
    let prepared = prepare_mono48k(input, policy).map_err(|failure| failure.error)?;
    let samples = frontend_gain::peak_normalize09(&prepared.gained).map_err(Error::NumericDomain)?;
    if samples.iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericDomain("Nonfinite normalized output.".into()));
    }
    Ok(samples)
}

