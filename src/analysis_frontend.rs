use crate::{
    Result,
    ltv_coefficient_math::ScalarUpper,
    audio_spectrum::AudioSpectrum,
    frontend_producer::FrontendProducer,
    model::ModelTree,
    normalization::NormalizationBridge,
    pitch_audio_spectrum::PitchAudioSpectrum,
    pitch_selected_head::{IndicatorAccumulation, SelectedPitchHead},
    pitch_spectral_encoder::PitchSpectralEncoder,
    pitch_temporal::{PitchTemporal, PitchTemporalState},
    source_pitch_join::{JoinedRows, SourcePitchJoin, SourceJoinMath, SourceJoinMathFailure},
    spectral_encoder::SpectralEncoder,
    timbre::u32_at,
};

#[path = "analysis_frontend_stream.rs"]
mod incremental;
pub use incremental::{AnalysisHistory, AnalysisStartFailure, AnalysisStream, AnalysisStreamBatch,
    AnalysisStreamCause, AnalysisStreamFailure, AnalysisStreamPolicy, AnalysisStreamWork};

pub struct AnalysisFrontend {
    spectrum: AudioSpectrum,
    encoder: SpectralEncoder,
    source: FrontendProducer,
    pitch_spectrum: PitchAudioSpectrum,
    pitch_encoder: PitchSpectralEncoder,
    pitch_temporal: PitchTemporal,
    pitch_head: SelectedPitchHead,
    normalization: NormalizationBridge,
}

pub struct AnalysisRows {
    pub network72: Vec<f32>,
    pub joined: JoinedRows,
    pub source_rows: usize,
    pub pitch_rows: usize,

    pub pending: (usize, usize),
}

#[derive(Clone, Copy)]
pub enum SourcePostlogAnalysisStage { Input, Spectrum, Encoder, Source, PitchEncoder, Temporal, Head, Join }

pub struct SourcePostlogAnalysisPrefix {
    spectrum: Option<Vec<f32>>,
    spectral: Option<crate::spectral_encoder::SpectralEncoderStages>,
    source_hidden384: Option<[f32; 384]>,
    source: Option<crate::frontend_producer::FrontendProducerStages>,
    pitch_spectra: Option<[Vec<f32>; 3]>,
    pitch_encoded: Option<crate::pitch_spectral_encoder::PitchSpectralStages>,
    temporal_state: Option<PitchTemporalState>,
    temporal: Option<crate::pitch_temporal::PitchTemporalStages>,
    head: Option<crate::pitch_selected_head::SelectedPitchHeadStages>,
    join: Option<SourcePitchJoin>,
}
impl SourcePostlogAnalysisPrefix {
    fn empty() -> Self {
        Self { spectrum: None, spectral: None, source_hidden384: None, source: None,
            pitch_spectra: None, pitch_encoded: None, temporal_state: None,
            temporal: None, head: None, join: None }
    }
    pub fn spectrum(&self) -> Option<&[f32]> { self.spectrum.as_deref() }
    pub fn spectral(&self) -> Option<&crate::spectral_encoder::SpectralEncoderStages> { self.spectral.as_ref() }
    pub fn source_hidden384(&self) -> Option<&[f32; 384]> { self.source_hidden384.as_ref() }
    pub fn source(&self) -> Option<&crate::frontend_producer::FrontendProducerStages> { self.source.as_ref() }
    pub fn pitch_spectra(&self) -> Option<&[Vec<f32>; 3]> { self.pitch_spectra.as_ref() }
    pub fn pitch_encoded(&self) -> Option<&crate::pitch_spectral_encoder::PitchSpectralStages> { self.pitch_encoded.as_ref() }
    pub fn temporal_state(&self) -> Option<&PitchTemporalState> { self.temporal_state.as_ref() }
    pub fn temporal(&self) -> Option<&crate::pitch_temporal::PitchTemporalStages> { self.temporal.as_ref() }
    pub fn head(&self) -> Option<&crate::pitch_selected_head::SelectedPitchHeadStages> { self.head.as_ref() }
    pub fn join(&self) -> Option<&SourcePitchJoin> { self.join.as_ref() }
}

pub struct SourcePostlogAnalysis {
    rows: AnalysisRows,
    math: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
}
impl SourcePostlogAnalysis {
    pub fn rows(&self) -> &AnalysisRows { &self.rows }
    pub fn math(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.math }
    pub fn into_parts(self) -> (AnalysisRows, crate::pitch_audio_spectrum::SourcePitchPostlogMath) {
        (self.rows, self.math)
    }
}
pub enum SourcePostlogAnalysisCause {
    Stage { stage: SourcePostlogAnalysisStage, reason: String },
    PitchSpectrum(crate::pitch_audio_spectrum::SourcePitchSpectrumFailure),
}
pub struct SourcePostlogAnalysisFailure {
    cause: SourcePostlogAnalysisCause,
    math: Option<crate::pitch_audio_spectrum::SourcePitchPostlogMath>,
    prefix: SourcePostlogAnalysisPrefix,
}
impl SourcePostlogAnalysisFailure {
    pub fn cause(&self) -> &SourcePostlogAnalysisCause { &self.cause }

    pub fn math(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> { self.math.as_ref() }
    pub fn prefix(&self) -> &SourcePostlogAnalysisPrefix { &self.prefix }
}

pub struct SourcePostlogJoinAnalysis<U: ScalarUpper> {
    rows: AnalysisRows,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    join_math: SourceJoinMath<U>,
}
impl<U: ScalarUpper> SourcePostlogJoinAnalysis<U> {
    pub fn rows(&self) -> &AnalysisRows { &self.rows }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &SourceJoinMath<U> { &self.join_math }
    pub fn into_parts(self) -> (AnalysisRows, crate::pitch_audio_spectrum::SourcePitchPostlogMath, SourceJoinMath<U>) {
        (self.rows, self.postlog, self.join_math)
    }
}
pub enum SourcePostlogJoinAnalysisCause<U: ScalarUpper> {
    Stage { stage: SourcePostlogAnalysisStage, reason: String },
    PitchSpectrum(crate::pitch_audio_spectrum::SourcePitchSpectrumFailure),
    Join(SourceJoinMathFailure<U>),
}
pub struct SourcePostlogJoinAnalysisFailure<U: ScalarUpper> {
    cause: SourcePostlogJoinAnalysisCause<U>,
    postlog: Option<crate::pitch_audio_spectrum::SourcePitchPostlogMath>,
    join_math: Option<SourceJoinMath<U>>,
    prefix: SourcePostlogAnalysisPrefix,
}
impl<U: ScalarUpper> SourcePostlogJoinAnalysisFailure<U> {
    pub fn cause(&self) -> &SourcePostlogJoinAnalysisCause<U> { &self.cause }

    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> { self.postlog.as_ref() }

    pub fn join_math(&self) -> Option<&SourceJoinMath<U>> { self.join_math.as_ref() }
    pub fn prefix(&self) -> &SourcePostlogAnalysisPrefix { &self.prefix }
}

pub struct SourceFftJoinAnalysis<U: ScalarUpper> {
    rows: AnalysisRows,
    fft: crate::fft::SourceFftMath,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    join_math: SourceJoinMath<U>,
}
impl<U: ScalarUpper> SourceFftJoinAnalysis<U> {
    pub fn rows(&self) -> &AnalysisRows { &self.rows }
    pub fn fft(&self) -> &crate::fft::SourceFftMath { &self.fft }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &SourceJoinMath<U> { &self.join_math }
    pub fn into_parts(self) -> (AnalysisRows, crate::fft::SourceFftMath,
        crate::pitch_audio_spectrum::SourcePitchPostlogMath, SourceJoinMath<U>)
    { (self.rows, self.fft, self.postlog, self.join_math) }
}
pub enum SourceFftJoinAnalysisCause<U: ScalarUpper> {
    Stage { stage: SourcePostlogAnalysisStage, reason: String },
    Spectrum(SourceFftAnalysisSpectrumFailure),
    PitchSpectrum(crate::pitch_audio_spectrum::SourceFftPitchSpectrumFailure),
    Join(SourceJoinMathFailure<U>),
}

pub struct SourceFftAnalysisSpectrumFailure {
    spectrum: crate::audio_spectrum::SourceFftSpectrumFailure,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
}
impl SourceFftAnalysisSpectrumFailure {
    pub fn spectrum(&self) -> &crate::audio_spectrum::SourceFftSpectrumFailure { &self.spectrum }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
}
pub struct SourceFftJoinAnalysisFailure<U: ScalarUpper> {
    cause: SourceFftJoinAnalysisCause<U>,
    pitch_math: Option<(crate::fft::SourceFftMath, crate::pitch_audio_spectrum::SourcePitchPostlogMath)>,
    join_math: Option<SourceJoinMath<U>>,
    prefix: SourcePostlogAnalysisPrefix,
}
impl<U: ScalarUpper> SourceFftJoinAnalysisFailure<U> {
    pub fn cause(&self) -> &SourceFftJoinAnalysisCause<U> { &self.cause }

    pub fn fft(&self) -> Option<&crate::fft::SourceFftMath> { self.pitch_math.as_ref().map(|math| &math.0) }
    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> {
        self.pitch_math.as_ref().map(|math| &math.1)
    }

    pub fn join_math(&self) -> Option<&SourceJoinMath<U>> { self.join_math.as_ref() }
    pub fn prefix(&self) -> &SourcePostlogAnalysisPrefix { &self.prefix }
}

pub struct SourceMathJoinAnalysis<U: ScalarUpper> {
    rows: AnalysisRows, math: crate::audio_spectrum::SourceSpectrumMath,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath, join_math: SourceJoinMath<U>,
}
impl<U: ScalarUpper> SourceMathJoinAnalysis<U> {
    pub fn rows(&self) -> &AnalysisRows { &self.rows }
    pub fn math(&self) -> &crate::audio_spectrum::SourceSpectrumMath { &self.math }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &SourceJoinMath<U> { &self.join_math }
    pub fn into_parts(self) -> (AnalysisRows, crate::audio_spectrum::SourceSpectrumMath,
        crate::pitch_audio_spectrum::SourcePitchPostlogMath, SourceJoinMath<U>)
    { (self.rows, self.math, self.postlog, self.join_math) }
}
pub struct SourceMathAnalysisSpectrumFailure {
    spectrum: crate::audio_spectrum::SourceMathSpectrumFailure,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
}
impl SourceMathAnalysisSpectrumFailure {
    pub fn spectrum(&self) -> &crate::audio_spectrum::SourceMathSpectrumFailure { &self.spectrum }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
}
pub enum SourceMathJoinAnalysisCause<U: ScalarUpper> {
    Stage { stage: SourcePostlogAnalysisStage, reason: String },
    Spectrum(SourceMathAnalysisSpectrumFailure),
    PitchSpectrum(crate::pitch_audio_spectrum::SourceMathPitchSpectrumFailure),
    Join(SourceJoinMathFailure<U>),
}
pub struct SourceMathJoinAnalysisFailure<U: ScalarUpper> {
    cause: SourceMathJoinAnalysisCause<U>,
    spectrum_math: Option<(crate::audio_spectrum::SourceSpectrumMath, crate::pitch_audio_spectrum::SourcePitchPostlogMath)>,
    join_math: Option<SourceJoinMath<U>>, prefix: SourcePostlogAnalysisPrefix,
}
impl<U: ScalarUpper> SourceMathJoinAnalysisFailure<U> {
    pub fn cause(&self) -> &SourceMathJoinAnalysisCause<U> { &self.cause }

    pub fn math(&self) -> Option<&crate::audio_spectrum::SourceSpectrumMath> { self.spectrum_math.as_ref().map(|math| &math.0) }
    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> {
        self.spectrum_math.as_ref().map(|math| &math.1)
    }
    pub fn join_math(&self) -> Option<&SourceJoinMath<U>> { self.join_math.as_ref() }
    pub fn prefix(&self) -> &SourcePostlogAnalysisPrefix { &self.prefix }
}

enum FinalSpectrumFailure<M, S> { Source { reason: String, math: M }, Selected(S) }
enum FinalAnalysisCause<S, E, F> {
    Stage { stage: SourcePostlogAnalysisStage, reason: String }, Spectrum(S), PitchSpectrum(E), Join(F),
}
struct FinalAnalysisFailure<M, J, S, E, F> {
    cause: FinalAnalysisCause<S, E, F>, math: Option<M>, join_math: Option<J>, prefix: SourcePostlogAnalysisPrefix,
}
impl<M, J, S, E, F> FinalAnalysisFailure<M, J, S, E, F> {
    fn stage(stage: SourcePostlogAnalysisStage, reason: String, math: M, join_math: J, prefix: SourcePostlogAnalysisPrefix) -> Self {
        Self { cause: FinalAnalysisCause::Stage { stage, reason }, math: Some(math), join_math: Some(join_math), prefix }
    }
}

impl AnalysisFrontend {

    pub fn stream_from_explicit_history(&self, history: AnalysisHistory, maximum_samples: usize,
        policy: AnalysisStreamPolicy)
        -> std::result::Result<AnalysisStream<'_>, AnalysisStartFailure>
    {
        AnalysisStream::new(self, history, maximum_samples, policy)
    }

    pub fn from_dnni(model: &[u8], policy: IndicatorAccumulation) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        let root = &tree.root;
        if tree.version != 2
            || root.kind != 0x7fca40ff
            || root.tag != 5416372951164482264_u64
            || root.data.len() != 18
            || root.children.len() != 9
            || u32_at(root.data, 0)? != 1
            || u32_at(root.data, 4)? != 48000
            || u32_at(root.data, 8)? != 1024
            || root.data[12..] != [240, 0, 72, 0, 2, 0]
            || root.children[0].children.len() != 14
        {
            return Err("unsupported default analysis model/configuration".into());
        }
        Ok(Self {
            spectrum: AudioSpectrum::from_dnni(model)?,
            encoder: SpectralEncoder::from_dnni(model)?,
            source: FrontendProducer::from_dnni(model)?,
            pitch_spectrum: PitchAudioSpectrum::from_dnni(model)?,
            pitch_encoder: PitchSpectralEncoder::from_dnni(model)?,
            pitch_temporal: PitchTemporal::from_dnni(model)?,
            pitch_head: SelectedPitchHead::from_dnni(model, policy)?,
            normalization: NormalizationBridge::from_dnni(model)?,
        })
    }

    pub fn forward_final_normalized48k(&self, samples: &[f32]) -> Result<AnalysisRows> {
        self.forward_final_with_pitch_spectrum(samples, (), |spectrum, samples, ()| {
            spectrum.forward_final(samples).map(|spectra| (spectra, ()))
        }).map(|(rows, ())| rows).map_err(|failure| match failure.cause {
            FinalAnalysisCause::Stage { reason, .. } | FinalAnalysisCause::PitchSpectrum(reason)
                | FinalAnalysisCause::Join(reason) => reason,
            FinalAnalysisCause::Spectrum(never) => match never {},
        })
    }

    pub fn forward_final_normalized48k_with_source_pitch_postlog(&self, samples: &[f32],
        math: crate::pitch_audio_spectrum::SourcePitchPostlogMath)
        -> std::result::Result<SourcePostlogAnalysis, SourcePostlogAnalysisFailure>
    {
        self.forward_final_with_pitch_spectrum(samples, math, |spectrum, samples, math| {
            spectrum.forward_final_with_source_postlog(samples, math).map(|complete| complete.into_parts())
        }).map(|(rows, math)| SourcePostlogAnalysis { rows, math })
            .map_err(|failure| SourcePostlogAnalysisFailure {
                cause: match failure.cause {
                    FinalAnalysisCause::Stage { stage, reason } => SourcePostlogAnalysisCause::Stage { stage, reason },
                    FinalAnalysisCause::Spectrum(never) => match never {},
                    FinalAnalysisCause::PitchSpectrum(cause) => SourcePostlogAnalysisCause::PitchSpectrum(cause),
                    FinalAnalysisCause::Join(reason) => SourcePostlogAnalysisCause::Stage { stage: SourcePostlogAnalysisStage::Join, reason },
                },
                math: failure.math, prefix: failure.prefix,
            })
    }

    pub fn forward_final_normalized48k_with_source_postlog_and_join<U: ScalarUpper>(&self, samples: &[f32],
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath, join_math: SourceJoinMath<U>)
        -> std::result::Result<SourcePostlogJoinAnalysis<U>, SourcePostlogJoinAnalysisFailure<U>>
    {
        self.forward_final_with_choices(samples, postlog, join_math, |spectrum, samples, math| {
            spectrum.forward_final_with_source_postlog(samples, math).map(|complete| complete.into_parts())
        }, |join, network, records, math| join.append_with_owned_source_math(network, records, math))
            .map(|(rows, postlog, join_math)| SourcePostlogJoinAnalysis { rows, postlog, join_math })
            .map_err(|failure| SourcePostlogJoinAnalysisFailure {
                cause: match failure.cause {
                    FinalAnalysisCause::Stage { stage, reason } => SourcePostlogJoinAnalysisCause::Stage { stage, reason },
                    FinalAnalysisCause::Spectrum(never) => match never {},
                    FinalAnalysisCause::PitchSpectrum(cause) => SourcePostlogJoinAnalysisCause::PitchSpectrum(cause),
                    FinalAnalysisCause::Join(cause) => SourcePostlogJoinAnalysisCause::Join(cause),
                },
                postlog: failure.math, join_math: failure.join_math, prefix: failure.prefix,
            })
    }

    pub fn forward_final_normalized48k_with_source_fft_postlog_and_join<U: ScalarUpper>(&self, samples: &[f32],
        fft: crate::fft::SourceFftMath, postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: SourceJoinMath<U>)
        -> std::result::Result<SourceFftJoinAnalysis<U>, SourceFftJoinAnalysisFailure<U>>
    {
        self.forward_final_with_choices(samples, (fft, postlog), join_math, |spectrum, samples, (fft, postlog)| {
            spectrum.forward_final_with_source_fft_and_postlog(samples, fft, postlog).map(|complete| {
                let (spectra, fft, postlog) = complete.into_parts();
                (spectra, (fft, postlog))
            })
        }, |join, network, records, math| join.append_with_owned_source_math(network, records, math))
            .map(|(rows, (fft, postlog), join_math)| SourceFftJoinAnalysis { rows, fft, postlog, join_math })
            .map_err(|failure| SourceFftJoinAnalysisFailure {
                cause: match failure.cause {
                    FinalAnalysisCause::Stage { stage, reason } => SourceFftJoinAnalysisCause::Stage { stage, reason },
                    FinalAnalysisCause::Spectrum(never) => match never {},
                    FinalAnalysisCause::PitchSpectrum(cause) => SourceFftJoinAnalysisCause::PitchSpectrum(cause),
                    FinalAnalysisCause::Join(cause) => SourceFftJoinAnalysisCause::Join(cause),
                },
                pitch_math: failure.math, join_math: failure.join_math, prefix: failure.prefix,
            })
    }

    pub fn forward_final_normalized48k_with_source_spectra_postlog_and_join<U: ScalarUpper>(&self, samples: &[f32],
        fft: crate::fft::SourceFftMath, postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: SourceJoinMath<U>)
        -> std::result::Result<SourceFftJoinAnalysis<U>, SourceFftJoinAnalysisFailure<U>>
    {
        self.forward_final_with_all_choices(samples, (fft, postlog), join_math,
            |spectrum, samples, (fft, postlog)| {
                match spectrum.from_stream48k_with_source_fft(samples, fft) {
                    Ok(complete) => {
                        let (values, fft) = complete.into_parts();
                        Ok((values, (fft, postlog)))
                    }
                    Err(spectrum) => Err(FinalSpectrumFailure::Selected(SourceFftAnalysisSpectrumFailure { spectrum, postlog })),
                }
            }, |spectrum, samples, (fft, postlog)| {
                spectrum.forward_final_with_source_fft_and_postlog(samples, fft, postlog).map(|complete| {
                    let (spectra, fft, postlog) = complete.into_parts();
                    (spectra, (fft, postlog))
                })
            }, |join, network, records, math| join.append_with_owned_source_math(network, records, math))
            .map(|(rows, (fft, postlog), join_math)| SourceFftJoinAnalysis { rows, fft, postlog, join_math })
            .map_err(|failure| SourceFftJoinAnalysisFailure {
                cause: match failure.cause {
                    FinalAnalysisCause::Stage { stage, reason } => SourceFftJoinAnalysisCause::Stage { stage, reason },
                    FinalAnalysisCause::Spectrum(cause) => SourceFftJoinAnalysisCause::Spectrum(cause),
                    FinalAnalysisCause::PitchSpectrum(cause) => SourceFftJoinAnalysisCause::PitchSpectrum(cause),
                    FinalAnalysisCause::Join(cause) => SourceFftJoinAnalysisCause::Join(cause),
                },
                pitch_math: failure.math, join_math: failure.join_math, prefix: failure.prefix,
            })
    }

    pub fn forward_final_normalized48k_with_source_spectrum_math_postlog_and_join<U: ScalarUpper>(&self, samples: &[f32],
        math: crate::audio_spectrum::SourceSpectrumMath, postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: SourceJoinMath<U>)
        -> std::result::Result<SourceMathJoinAnalysis<U>, SourceMathJoinAnalysisFailure<U>>
    {
        self.forward_final_with_source_spectrum_math_postlog_join_choice(samples, math, postlog, join_math,
            |join, network, records, math| join.append_with_owned_source_math(network, records, math))
    }

    pub fn forward_final_normalized48k_with_source_spectrum_math_postlog_and_record_join<U: ScalarUpper>(
        &self, samples: &[f32], math: crate::audio_spectrum::SourceSpectrumMath,
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath, join_math: SourceJoinMath<U>)
        -> std::result::Result<SourceMathJoinAnalysis<U>, SourceMathJoinAnalysisFailure<U>>
    {
        self.forward_final_with_source_spectrum_math_postlog_join_choice(samples, math, postlog, join_math,
            |join, network, records, math| join.append_with_owned_source_record_math(network, records, math))
    }

    fn forward_final_with_source_spectrum_math_postlog_join_choice<U: ScalarUpper>(&self, samples: &[f32],
        math: crate::audio_spectrum::SourceSpectrumMath, postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: SourceJoinMath<U>,
        join: impl FnOnce(&mut SourcePitchJoin, &[f32], &[crate::pitch_selected_head::PitchRecord], SourceJoinMath<U>)
            -> std::result::Result<(JoinedRows, SourceJoinMath<U>), SourceJoinMathFailure<U>>)
        -> std::result::Result<SourceMathJoinAnalysis<U>, SourceMathJoinAnalysisFailure<U>>
    {
        self.forward_final_with_all_choices(samples, (math, postlog), join_math,
            |spectrum, samples, (math, postlog)| {
                match spectrum.from_stream48k_with_source_math(samples, math) {
                    Ok(complete) => {
                        let (values, math) = complete.into_parts();
                        Ok((values, (math, postlog)))
                    }
                    Err(spectrum) => Err(FinalSpectrumFailure::Selected(SourceMathAnalysisSpectrumFailure { spectrum, postlog })),
                }
            }, |spectrum, samples, (math, postlog)| {
                spectrum.forward_final_with_source_math_and_postlog(samples, math, postlog).map(|complete| {
                    let (spectra, math, postlog) = complete.into_parts();
                    (spectra, (math, postlog))
                })
            }, join)
            .map(|(rows, (math, postlog), join_math)| SourceMathJoinAnalysis { rows, math, postlog, join_math })
            .map_err(|failure| SourceMathJoinAnalysisFailure {
                cause: match failure.cause {
                    FinalAnalysisCause::Stage { stage, reason } => SourceMathJoinAnalysisCause::Stage { stage, reason },
                    FinalAnalysisCause::Spectrum(cause) => SourceMathJoinAnalysisCause::Spectrum(cause),
                    FinalAnalysisCause::PitchSpectrum(cause) => SourceMathJoinAnalysisCause::PitchSpectrum(cause),
                    FinalAnalysisCause::Join(cause) => SourceMathJoinAnalysisCause::Join(cause),
                },
                spectrum_math: failure.math, join_math: failure.join_math, prefix: failure.prefix,
            })
    }

    fn forward_final_with_pitch_spectrum<M, E>(&self, samples: &[f32], math: M,
        pitch_spectrum: impl FnOnce(&PitchAudioSpectrum, &[f32], M)
            -> std::result::Result<([Vec<f32>; 3], M), E>)
        -> std::result::Result<(AnalysisRows, M), FinalAnalysisFailure<M, (), std::convert::Infallible, E, String>>
    {
        self.forward_final_with_choices(samples, math, (), pitch_spectrum, |join, network, records, ()| {
            join.append(network, records).map(|rows| (rows, ()))
        }).map(|(rows, math, ())| (rows, math))
    }

    fn forward_final_with_choices<M, J, E, F>(&self, samples: &[f32], math: M, join_math: J,
        pitch_spectrum: impl FnOnce(&PitchAudioSpectrum, &[f32], M)
            -> std::result::Result<([Vec<f32>; 3], M), E>,
        join: impl FnOnce(&mut SourcePitchJoin, &[f32], &[crate::pitch_selected_head::PitchRecord], J)
            -> std::result::Result<(JoinedRows, J), F>)
        -> std::result::Result<(AnalysisRows, M, J), FinalAnalysisFailure<M, J, std::convert::Infallible, E, F>>
    {
        self.forward_final_with_all_choices(samples, math, join_math, |spectrum, samples, math| {
            match spectrum.from_stream48k(samples) {
                Ok(values) => Ok((values, math)),
                Err(reason) => Err(FinalSpectrumFailure::Source { reason, math }),
            }
        }, pitch_spectrum, join)
    }

    fn forward_final_with_all_choices<M, J, S, E, F>(&self, samples: &[f32], math: M, join_math: J,
        spectrum: impl FnOnce(&AudioSpectrum, &[f32], M)
            -> std::result::Result<(Vec<f32>, M), FinalSpectrumFailure<M, S>>,
        pitch_spectrum: impl FnOnce(&PitchAudioSpectrum, &[f32], M)
            -> std::result::Result<([Vec<f32>; 3], M), E>,
        join: impl FnOnce(&mut SourcePitchJoin, &[f32], &[crate::pitch_selected_head::PitchRecord], J)
            -> std::result::Result<(JoinedRows, J), F>)
        -> std::result::Result<(AnalysisRows, M, J), FinalAnalysisFailure<M, J, S, E, F>>
    {
        let mut prefix = SourcePostlogAnalysisPrefix::empty();
        if samples.is_empty()
            || samples.len() > i32::MAX as usize
            || samples.iter().any(|v| !v.is_finite())
        {
            return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::Input,
                "analysis requires a finite normalized 48 kHz mono stream".into(), math, join_math, prefix));
        }
        let (values, math) = match spectrum(&self.spectrum, samples, math) {
            Ok(value) => value,
            Err(FinalSpectrumFailure::Source { reason, math }) => return Err(FinalAnalysisFailure::stage(
                SourcePostlogAnalysisStage::Spectrum, reason, math, join_math, prefix)),
            Err(FinalSpectrumFailure::Selected(cause)) => return Err(FinalAnalysisFailure {
                cause: FinalAnalysisCause::Spectrum(cause), math: None, join_math: Some(join_math), prefix,
            }),
        };
        prefix.spectrum = Some(values);
        match self.encoder.forward513(prefix.spectrum.as_ref().expect("completed spectrum")) {
            Ok(value) => prefix.spectral = Some(value),
            Err(reason) => return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::Encoder, reason, math, join_math, prefix)),
        }

        prefix.source_hidden384 = Some([0.0; 384]);
        match self.source.forward_spectral256(&prefix.spectral.as_ref().expect("completed encoder").spectral256,
            prefix.source_hidden384.as_mut().expect("fresh source hidden"))
        {
            Ok(value) => prefix.source = Some(value),
            Err(reason) => return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::Source, reason, math, join_math, prefix)),
        }
        let (spectra, math) = match pitch_spectrum(&self.pitch_spectrum, samples, math) {
            Ok(value) => value,
            Err(cause) => return Err(FinalAnalysisFailure {
                cause: FinalAnalysisCause::PitchSpectrum(cause), math: None, join_math: Some(join_math), prefix,
            }),
        };
        prefix.pitch_spectra = Some(spectra);
        let spectra = prefix.pitch_spectra.as_ref().expect("completed pitch spectra");
        match self.pitch_encoder.forward([&spectra[0], &spectra[1], &spectra[2]]) {
            Ok(value) => prefix.pitch_encoded = Some(value),
            Err(reason) => return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::PitchEncoder, reason, math, join_math, prefix)),
        }

        prefix.temporal_state = Some(PitchTemporalState::default());
        match self.pitch_temporal.forward384(&prefix.pitch_encoded.as_ref().expect("completed pitch encoder").summed384,
            prefix.temporal_state.as_mut().expect("fresh temporal state"))
        {
            Ok(value) => prefix.temporal = Some(value),
            Err(reason) => return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::Temporal, reason, math, join_math, prefix)),
        }
        match self.pitch_head.forward384_final(&prefix.temporal.as_ref().expect("completed temporal stage").output384) {
            Ok(value) => prefix.head = Some(value),
            Err(reason) => return Err(FinalAnalysisFailure::stage(SourcePostlogAnalysisStage::Head, reason, math, join_math, prefix)),
        }
        let source_rows = prefix.source.as_ref().expect("completed source stage").features_72.len() / 72;
        let pitch_rows = prefix.head.as_ref().expect("completed head").records.len();
        prefix.join = Some(SourcePitchJoin::new(self.normalization.clone()));
        let (joined, join_math) = match join(prefix.join.as_mut().expect("fresh join"),
            &prefix.source.as_ref().expect("completed source stage").features_72,
            &prefix.head.as_ref().expect("completed head").records, join_math)
        {
            Ok(value) => value,
            Err(cause) => return Err(FinalAnalysisFailure {
                cause: FinalAnalysisCause::Join(cause), math: Some(math), join_math: None, prefix,
            }),
        };
        let pending = prefix.join.as_ref().expect("completed join").pending_counts();
        prefix.join.as_mut().expect("completed join").clear_queues();
        let source = prefix.source.take().expect("completed source stage");
        Ok((AnalysisRows {
            network72: source.features_72,
            joined,
            source_rows,
            pitch_rows,
            pending,
        }, math, join_math))
    }
}
