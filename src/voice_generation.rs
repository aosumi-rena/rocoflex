use crate::{
    Result,
    analysis_frontend::AnalysisFrontend,
    ltv_coefficient_math::ScalarUpper,
    generator::Generator,
    pitch_selected_head::IndicatorAccumulation,
    trajectory_pitch::pitch_statistics,
    voice_library::{FactoryPoint, FactoryVoice},
};
use serde_json::Value;

#[path = "voice_generation_stream.rs"]
mod incremental;
pub use incremental::{VoiceGenerationBatch, VoiceGenerationStream, VoiceGenerationStreamCause,
    VoiceGenerationStreamFailure, PreparedVoiceStream, PreparedVoiceStartFailure,
    PreparedVoiceBatch, PreparedVoiceFailure};

pub struct VoiceGenerator {
    analysis: AnalysisFrontend,
    generator: Generator,
}

pub struct GeneratedPoint {
    pub data: [f32; 128],
    pub reduced: [f32; 4],
}

pub struct GeneratedVoice {
    pub data: [f32; 128],
    pub pitch_stats: [f32; 5],
    pub points: Vec<GeneratedPoint>,
}

pub struct SourceLog2VoiceGeneration {
    voice: Option<GeneratedVoice>,
    logarithm: crate::ltv_transform_log::SourcePositiveNormalLog2,
}
impl SourceLog2VoiceGeneration {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn logarithm(&self) -> &crate::ltv_transform_log::SourcePositiveNormalLog2 { &self.logarithm }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::ltv_transform_log::SourcePositiveNormalLog2) {
        (self.voice, self.logarithm)
    }
}

pub enum SourceLog2VoiceGenerationCause {
    Analysis(String),
    Generation(String),
    Statistics(crate::trajectory_pitch::SourcePitchStatisticsFailure),
    Assembly(&'static str),
}

pub struct SourceLog2VoiceGenerationFailure {
    cause: SourceLog2VoiceGenerationCause,
    logarithm: Option<crate::ltv_transform_log::SourcePositiveNormalLog2>,
    analysis: Option<crate::analysis_frontend::AnalysisRows>,
    generated: Option<crate::generator::ComputedGeneration>,
    points: Vec<GeneratedPoint>,
}
impl SourceLog2VoiceGenerationFailure {
    pub fn cause(&self) -> &SourceLog2VoiceGenerationCause { &self.cause }

    pub fn logarithm(&self) -> Option<&crate::ltv_transform_log::SourcePositiveNormalLog2> {
        self.logarithm.as_ref()
    }
    pub fn analysis(&self) -> Option<&crate::analysis_frontend::AnalysisRows> { self.analysis.as_ref() }
    pub fn generated(&self) -> Option<&crate::generator::ComputedGeneration> { self.generated.as_ref() }
    pub fn assembled_points(&self) -> &[GeneratedPoint] { &self.points }
}

pub struct SourceMathVoiceGeneration {
    voice: Option<GeneratedVoice>,
    math: crate::trajectory_pitch::SourcePitchMath,
}
impl SourceMathVoiceGeneration {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn math(&self) -> &crate::trajectory_pitch::SourcePitchMath { &self.math }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::trajectory_pitch::SourcePitchMath) {
        (self.voice, self.math)
    }
}
pub enum SourceMathVoiceGenerationCause {
    Analysis(String),
    Generation(String),
    Statistics(crate::trajectory_pitch::SourceMathPitchStatisticsFailure),
    Assembly(&'static str),
}

pub struct SourceMathVoiceGenerationFailure {
    cause: SourceMathVoiceGenerationCause,
    math: Option<crate::trajectory_pitch::SourcePitchMath>,
    analysis: Option<crate::analysis_frontend::AnalysisRows>,
    generated: Option<crate::generator::ComputedGeneration>,
    points: Vec<GeneratedPoint>,
}
impl SourceMathVoiceGenerationFailure {
    pub fn cause(&self) -> &SourceMathVoiceGenerationCause { &self.cause }

    pub fn math(&self) -> Option<&crate::trajectory_pitch::SourcePitchMath> { self.math.as_ref() }
    pub fn analysis(&self) -> Option<&crate::analysis_frontend::AnalysisRows> { self.analysis.as_ref() }
    pub fn generated(&self) -> Option<&crate::generator::ComputedGeneration> { self.generated.as_ref() }
    pub fn assembled_points(&self) -> &[GeneratedPoint] { &self.points }

    fn from_statistics_generation(failure: StatisticsGenerationFailure<crate::trajectory_pitch::SourcePitchMath,
        crate::trajectory_pitch::SourceMathPitchStatisticsFailure>) -> Self
    {
        Self {
            cause: match failure.cause {
                StatisticsGenerationCause::Analysis(cause) => SourceMathVoiceGenerationCause::Analysis(cause),
                StatisticsGenerationCause::Generation(cause) => SourceMathVoiceGenerationCause::Generation(cause),
                StatisticsGenerationCause::Statistics(cause) => SourceMathVoiceGenerationCause::Statistics(cause),
                StatisticsGenerationCause::Assembly(cause) => SourceMathVoiceGenerationCause::Assembly(cause),
            },
            math: failure.math, analysis: failure.analysis, generated: failure.generated, points: failure.points,
        }
    }
}

pub struct SourceAnalysisVoiceGeneration {
    voice: Option<GeneratedVoice>,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    statistics: crate::trajectory_pitch::SourcePitchMath,
}
impl SourceAnalysisVoiceGeneration {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn statistics(&self) -> &crate::trajectory_pitch::SourcePitchMath { &self.statistics }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        crate::trajectory_pitch::SourcePitchMath)
    { (self.voice, self.postlog, self.statistics) }
}
pub enum SourceAnalysisVoiceGenerationCause {
    Analysis(crate::analysis_frontend::SourcePostlogAnalysisFailure),
    AfterAnalysis(SourceMathVoiceGenerationFailure),
}
pub struct SourceAnalysisVoiceGenerationFailure {
    cause: SourceAnalysisVoiceGenerationCause,
    postlog: Option<crate::pitch_audio_spectrum::SourcePitchPostlogMath>,
    statistics: Option<crate::trajectory_pitch::SourcePitchMath>,
}
impl SourceAnalysisVoiceGenerationFailure {
    pub fn cause(&self) -> &SourceAnalysisVoiceGenerationCause { &self.cause }

    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> { self.postlog.as_ref() }

    pub fn statistics(&self) -> Option<&crate::trajectory_pitch::SourcePitchMath> { self.statistics.as_ref() }
}

pub struct SourceJoinedAnalysisVoiceGeneration<U: ScalarUpper> {
    voice: Option<GeneratedVoice>,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    join_math: crate::source_pitch_join::SourceJoinMath<U>,
    statistics: crate::trajectory_pitch::SourcePitchMath,
}
impl<U: ScalarUpper> SourceJoinedAnalysisVoiceGeneration<U> {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &crate::source_pitch_join::SourceJoinMath<U> { &self.join_math }
    pub fn statistics(&self) -> &crate::trajectory_pitch::SourcePitchMath { &self.statistics }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        crate::source_pitch_join::SourceJoinMath<U>, crate::trajectory_pitch::SourcePitchMath)
    { (self.voice, self.postlog, self.join_math, self.statistics) }
}
pub enum SourceJoinedAnalysisVoiceGenerationCause<U: ScalarUpper> {
    Analysis(crate::analysis_frontend::SourcePostlogJoinAnalysisFailure<U>),
    AfterAnalysis(SourceMathVoiceGenerationFailure),
}
pub struct SourceJoinedAnalysisVoiceGenerationFailure<U: ScalarUpper> {
    cause: SourceJoinedAnalysisVoiceGenerationCause<U>,
    postlog: Option<crate::pitch_audio_spectrum::SourcePitchPostlogMath>,
    join_math: Option<crate::source_pitch_join::SourceJoinMath<U>>,
    statistics: Option<crate::trajectory_pitch::SourcePitchMath>,
}
impl<U: ScalarUpper> SourceJoinedAnalysisVoiceGenerationFailure<U> {
    pub fn cause(&self) -> &SourceJoinedAnalysisVoiceGenerationCause<U> { &self.cause }

    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> { self.postlog.as_ref() }
    pub fn join_math(&self) -> Option<&crate::source_pitch_join::SourceJoinMath<U>> { self.join_math.as_ref() }

    pub fn statistics(&self) -> Option<&crate::trajectory_pitch::SourcePitchMath> { self.statistics.as_ref() }
}

pub struct SourceFftAnalysisVoiceGeneration<U: ScalarUpper> {
    voice: Option<GeneratedVoice>,
    fft: crate::fft::SourceFftMath,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    join_math: crate::source_pitch_join::SourceJoinMath<U>,
    statistics: crate::trajectory_pitch::SourcePitchMath,
}
impl<U: ScalarUpper> SourceFftAnalysisVoiceGeneration<U> {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn fft(&self) -> &crate::fft::SourceFftMath { &self.fft }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &crate::source_pitch_join::SourceJoinMath<U> { &self.join_math }
    pub fn statistics(&self) -> &crate::trajectory_pitch::SourcePitchMath { &self.statistics }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::fft::SourceFftMath,
        crate::pitch_audio_spectrum::SourcePitchPostlogMath, crate::source_pitch_join::SourceJoinMath<U>,
        crate::trajectory_pitch::SourcePitchMath)
    { (self.voice, self.fft, self.postlog, self.join_math, self.statistics) }
}
pub enum SourceFftAnalysisVoiceGenerationCause<U: ScalarUpper> {
    Analysis(crate::analysis_frontend::SourceFftJoinAnalysisFailure<U>),
    AfterAnalysis(SourceMathVoiceGenerationFailure),
}
pub struct SourceFftAnalysisVoiceGenerationFailure<U: ScalarUpper> {
    cause: SourceFftAnalysisVoiceGenerationCause<U>,
    analysis_math: Option<(crate::fft::SourceFftMath, crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        crate::source_pitch_join::SourceJoinMath<U>)>,
    statistics: Option<crate::trajectory_pitch::SourcePitchMath>,
}
impl<U: ScalarUpper> SourceFftAnalysisVoiceGenerationFailure<U> {
    pub fn cause(&self) -> &SourceFftAnalysisVoiceGenerationCause<U> { &self.cause }

    pub fn fft(&self) -> Option<&crate::fft::SourceFftMath> { self.analysis_math.as_ref().map(|math| &math.0) }
    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> {
        self.analysis_math.as_ref().map(|math| &math.1)
    }
    pub fn join_math(&self) -> Option<&crate::source_pitch_join::SourceJoinMath<U>> {
        self.analysis_math.as_ref().map(|math| &math.2)
    }

    pub fn statistics(&self) -> Option<&crate::trajectory_pitch::SourcePitchMath> { self.statistics.as_ref() }
}

pub struct SourceSpectrumVoiceGeneration<U: ScalarUpper> {
    voice: Option<GeneratedVoice>, math: crate::audio_spectrum::SourceSpectrumMath,
    postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath,
}
impl<U: ScalarUpper> SourceSpectrumVoiceGeneration<U> {
    pub fn voice(&self) -> Option<&GeneratedVoice> { self.voice.as_ref() }
    pub fn math(&self) -> &crate::audio_spectrum::SourceSpectrumMath { &self.math }
    pub fn postlog(&self) -> &crate::pitch_audio_spectrum::SourcePitchPostlogMath { &self.postlog }
    pub fn join_math(&self) -> &crate::source_pitch_join::SourceJoinMath<U> { &self.join_math }
    pub fn statistics(&self) -> &crate::trajectory_pitch::SourcePitchMath { &self.statistics }
    pub fn into_parts(self) -> (Option<GeneratedVoice>, crate::audio_spectrum::SourceSpectrumMath,
        crate::pitch_audio_spectrum::SourcePitchPostlogMath, crate::source_pitch_join::SourceJoinMath<U>,
        crate::trajectory_pitch::SourcePitchMath)
    { (self.voice, self.math, self.postlog, self.join_math, self.statistics) }
}
pub enum SourceSpectrumVoiceGenerationCause<U: ScalarUpper> {
    Analysis(crate::analysis_frontend::SourceMathJoinAnalysisFailure<U>),
    AfterAnalysis(SourceMathVoiceGenerationFailure),
}
pub struct SourceSpectrumVoiceGenerationFailure<U: ScalarUpper> {
    cause: SourceSpectrumVoiceGenerationCause<U>,
    analysis_math: Option<(crate::audio_spectrum::SourceSpectrumMath, crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        crate::source_pitch_join::SourceJoinMath<U>)>,
    statistics: Option<crate::trajectory_pitch::SourcePitchMath>,
}
impl<U: ScalarUpper> SourceSpectrumVoiceGenerationFailure<U> {
    pub fn cause(&self) -> &SourceSpectrumVoiceGenerationCause<U> { &self.cause }

    pub fn math(&self) -> Option<&crate::audio_spectrum::SourceSpectrumMath> { self.analysis_math.as_ref().map(|math| &math.0) }
    pub fn postlog(&self) -> Option<&crate::pitch_audio_spectrum::SourcePitchPostlogMath> {
        self.analysis_math.as_ref().map(|math| &math.1)
    }
    pub fn join_math(&self) -> Option<&crate::source_pitch_join::SourceJoinMath<U>> {
        self.analysis_math.as_ref().map(|math| &math.2)
    }

    pub fn statistics(&self) -> Option<&crate::trajectory_pitch::SourcePitchMath> { self.statistics.as_ref() }
}

enum StatisticsGenerationCause<E> { Analysis(String), Generation(String), Statistics(E), Assembly(&'static str) }
struct StatisticsGenerationFailure<M, E> {
    cause: StatisticsGenerationCause<E>, math: Option<M>,
    analysis: Option<crate::analysis_frontend::AnalysisRows>,
    generated: Option<crate::generator::ComputedGeneration>, points: Vec<GeneratedPoint>,
}

struct VoiceAssemblyFailure {
    cause: &'static str,
    generated: crate::generator::ComputedGeneration,
    points: Vec<GeneratedPoint>,
}

fn assemble_voice(generated: crate::generator::ComputedGeneration, pitch_stats: [f32; 5])
    -> std::result::Result<GeneratedVoice, VoiceAssemblyFailure>
{
    let rows = &generated.rows;
    if rows.points128.len() / 128 != rows.points4.len() / 4
        || rows.points128.len() % 128 != 0 || rows.points4.len() % 4 != 0
    {
        return Err(VoiceAssemblyFailure { cause: "generated voice point dimensions differ",
            generated, points: Vec::new() });
    }
    let points = rows.points128.chunks_exact(128).zip(rows.points4.chunks_exact(4))
        .map(|(data, reduced)| GeneratedPoint {
            data: data.try_into().expect("128-lane chunk"),
            reduced: reduced.try_into().expect("four-lane chunk"),
        }).collect();
    let data = match rows.trajectory128.as_slice().try_into() {
        Ok(data) => data,
        Err(_) => return Err(VoiceAssemblyFailure { cause: "generated voice summary width",
            generated, points }),
    };
    Ok(GeneratedVoice { data, pitch_stats, points })
}

impl VoiceGenerator {

    pub fn prepared_stream_from_explicit_history(&self, audio: crate::ordinary_audio::PreparedAudio,
        history: crate::analysis_frontend::AnalysisHistory,
        policy: crate::analysis_frontend::AnalysisStreamPolicy)
        -> std::result::Result<PreparedVoiceStream<'_>, PreparedVoiceStartFailure>
    {
        PreparedVoiceStream::new(self, audio, history, policy)
    }

    pub fn stream_from_explicit_history(&self, history: crate::analysis_frontend::AnalysisHistory,
        maximum_samples: usize, policy: crate::analysis_frontend::AnalysisStreamPolicy)
        -> std::result::Result<VoiceGenerationStream<'_>, crate::analysis_frontend::AnalysisStartFailure>
    {
        VoiceGenerationStream::new(self, history, maximum_samples, policy)
    }

    pub fn from_dnni(model: &[u8], indicator: IndicatorAccumulation) -> Result<Self> {
        Ok(Self {
            analysis: AnalysisFrontend::from_dnni(model, indicator)?,
            generator: Generator::from_dnni(model)?,
        })
    }

    pub fn generate_final_normalized48k(&self, input: &[f32]) -> Result<Option<GeneratedVoice>> {
        let analysis = self.analysis.forward_final_normalized48k(input)?;
        self.generate_from_joined_rows(&analysis.joined.normalized72, &analysis.joined.records,
            &analysis.joined.energies, input.len())
    }

    pub fn generate_final_normalized48k_with_source_log2(&self, input: &[f32],
        logarithm: crate::ltv_transform_log::SourcePositiveNormalLog2)
        -> std::result::Result<SourceLog2VoiceGeneration, SourceLog2VoiceGenerationFailure>
    {
        self.generate_final_with_statistics(input, logarithm, |records, energies, logarithm| {
            crate::trajectory_pitch::pitch_statistics_with_source_log2(records, energies, logarithm)
                .map(|statistics| statistics.into_parts())
        }).map(|(voice, logarithm)| SourceLog2VoiceGeneration { voice, logarithm })
            .map_err(|failure| SourceLog2VoiceGenerationFailure {
                cause: match failure.cause {
                    StatisticsGenerationCause::Analysis(cause) => SourceLog2VoiceGenerationCause::Analysis(cause),
                    StatisticsGenerationCause::Generation(cause) => SourceLog2VoiceGenerationCause::Generation(cause),
                    StatisticsGenerationCause::Statistics(cause) => SourceLog2VoiceGenerationCause::Statistics(cause),
                    StatisticsGenerationCause::Assembly(cause) => SourceLog2VoiceGenerationCause::Assembly(cause),
                },
                logarithm: failure.math, analysis: failure.analysis, generated: failure.generated, points: failure.points,
            })
    }

    pub fn generate_final_normalized48k_with_source_math(&self, input: &[f32],
        math: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceMathVoiceGeneration, SourceMathVoiceGenerationFailure>
    {
        self.generate_final_with_statistics(input, math, |records, energies, math| {
            crate::trajectory_pitch::pitch_statistics_with_source_math(records, energies, math)
                .map(|statistics| statistics.into_parts())
        }).map(|(voice, math)| SourceMathVoiceGeneration { voice, math })
            .map_err(SourceMathVoiceGenerationFailure::from_statistics_generation)
    }

    pub fn generate_final_normalized48k_with_source_pitch_postlog_and_statistics(&self, input: &[f32],
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        statistics: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceAnalysisVoiceGeneration, SourceAnalysisVoiceGenerationFailure>
    {
        let analysis = match self.analysis.forward_final_normalized48k_with_source_pitch_postlog(input, postlog) {
            Ok(analysis) => analysis,
            Err(cause) => return Err(SourceAnalysisVoiceGenerationFailure {
                cause: SourceAnalysisVoiceGenerationCause::Analysis(cause), postlog: None, statistics: Some(statistics),
            }),
        };
        let (analysis, postlog) = analysis.into_parts();
        match self.generate_from_analysis_with_statistics(analysis, input.len(), statistics, |records, energies, math| {
            crate::trajectory_pitch::pitch_statistics_with_source_math(records, energies, math)
                .map(|complete| complete.into_parts())
        }) {
            Ok((voice, statistics)) => Ok(SourceAnalysisVoiceGeneration { voice, postlog, statistics }),
            Err(failure) => Err(SourceAnalysisVoiceGenerationFailure {
                cause: SourceAnalysisVoiceGenerationCause::AfterAnalysis(
                    SourceMathVoiceGenerationFailure::from_statistics_generation(failure)),
                postlog: Some(postlog), statistics: None,
            }),
        }
    }

    pub fn generate_final_normalized48k_with_source_pitch_postlog_join_and_statistics<U: ScalarUpper>(&self,
        input: &[f32], postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceJoinedAnalysisVoiceGeneration<U>, SourceJoinedAnalysisVoiceGenerationFailure<U>>
    {
        let analysis = match self.analysis.forward_final_normalized48k_with_source_postlog_and_join(input, postlog, join_math) {
            Ok(analysis) => analysis,
            Err(cause) => return Err(SourceJoinedAnalysisVoiceGenerationFailure {
                cause: SourceJoinedAnalysisVoiceGenerationCause::Analysis(cause),
                postlog: None, join_math: None, statistics: Some(statistics),
            }),
        };
        let (analysis, postlog, join_math) = analysis.into_parts();
        match self.generate_from_analysis_with_statistics(analysis, input.len(), statistics, |records, energies, math| {
            crate::trajectory_pitch::pitch_statistics_with_source_math(records, energies, math)
                .map(|complete| complete.into_parts())
        }) {
            Ok((voice, statistics)) => Ok(SourceJoinedAnalysisVoiceGeneration { voice, postlog, join_math, statistics }),
            Err(failure) => Err(SourceJoinedAnalysisVoiceGenerationFailure {
                cause: SourceJoinedAnalysisVoiceGenerationCause::AfterAnalysis(
                    SourceMathVoiceGenerationFailure::from_statistics_generation(failure)),
                postlog: Some(postlog), join_math: Some(join_math), statistics: None,
            }),
        }
    }

    pub fn generate_final_normalized48k_with_source_fft_postlog_join_and_statistics<U: ScalarUpper>(&self,
        input: &[f32], fft: crate::fft::SourceFftMath,
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceFftAnalysisVoiceGeneration<U>, SourceFftAnalysisVoiceGenerationFailure<U>>
    {
        self.generate_final_with_fft_analysis(input, fft, postlog, join_math, statistics,
            |analysis, input, fft, postlog, join_math| {
                analysis.forward_final_normalized48k_with_source_fft_postlog_and_join(input, fft, postlog, join_math)
            })
    }

    pub fn generate_final_normalized48k_with_source_spectra_postlog_join_and_statistics<U: ScalarUpper>(&self,
        input: &[f32], fft: crate::fft::SourceFftMath,
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceFftAnalysisVoiceGeneration<U>, SourceFftAnalysisVoiceGenerationFailure<U>>
    {
        self.generate_final_with_fft_analysis(input, fft, postlog, join_math, statistics,
            |analysis, input, fft, postlog, join_math| {
                analysis.forward_final_normalized48k_with_source_spectra_postlog_and_join(input, fft, postlog, join_math)
            })
    }

    pub fn generate_final_normalized48k_with_source_spectrum_math_postlog_join_and_statistics<U: ScalarUpper>(&self,
        input: &[f32], math: crate::audio_spectrum::SourceSpectrumMath,
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath)
        -> std::result::Result<SourceSpectrumVoiceGeneration<U>, SourceSpectrumVoiceGenerationFailure<U>>
    {
        let analysis = match self.analysis.forward_final_normalized48k_with_source_spectrum_math_postlog_and_join(input, math, postlog, join_math) {
            Ok(analysis) => analysis,
            Err(cause) => return Err(SourceSpectrumVoiceGenerationFailure {
                cause: SourceSpectrumVoiceGenerationCause::Analysis(cause),
                analysis_math: None, statistics: Some(statistics),
            }),
        };
        let (analysis, math, postlog, join_math) = analysis.into_parts();
        match self.generate_from_analysis_with_statistics(analysis, input.len(), statistics, |records, energies, math| {
            crate::trajectory_pitch::pitch_statistics_with_source_math(records, energies, math)
                .map(|complete| complete.into_parts())
        }) {
            Ok((voice, statistics)) => Ok(SourceSpectrumVoiceGeneration { voice, math, postlog, join_math, statistics }),
            Err(failure) => Err(SourceSpectrumVoiceGenerationFailure {
                cause: SourceSpectrumVoiceGenerationCause::AfterAnalysis(
                    SourceMathVoiceGenerationFailure::from_statistics_generation(failure)),
                analysis_math: Some((math, postlog, join_math)), statistics: None,
            }),
        }
    }

    fn generate_final_with_fft_analysis<U: ScalarUpper>(&self, input: &[f32], fft: crate::fft::SourceFftMath,
        postlog: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
        join_math: crate::source_pitch_join::SourceJoinMath<U>, statistics: crate::trajectory_pitch::SourcePitchMath,
        analysis: impl FnOnce(&AnalysisFrontend, &[f32], crate::fft::SourceFftMath,
            crate::pitch_audio_spectrum::SourcePitchPostlogMath, crate::source_pitch_join::SourceJoinMath<U>)
            -> std::result::Result<crate::analysis_frontend::SourceFftJoinAnalysis<U>, crate::analysis_frontend::SourceFftJoinAnalysisFailure<U>>)
        -> std::result::Result<SourceFftAnalysisVoiceGeneration<U>, SourceFftAnalysisVoiceGenerationFailure<U>>
    {
        let analysis = match analysis(&self.analysis, input, fft, postlog, join_math) {
            Ok(analysis) => analysis,
            Err(cause) => return Err(SourceFftAnalysisVoiceGenerationFailure {
                cause: SourceFftAnalysisVoiceGenerationCause::Analysis(cause),
                analysis_math: None, statistics: Some(statistics),
            }),
        };
        let (analysis, fft, postlog, join_math) = analysis.into_parts();
        match self.generate_from_analysis_with_statistics(analysis, input.len(), statistics, |records, energies, math| {
            crate::trajectory_pitch::pitch_statistics_with_source_math(records, energies, math)
                .map(|complete| complete.into_parts())
        }) {
            Ok((voice, statistics)) => Ok(SourceFftAnalysisVoiceGeneration { voice, fft, postlog, join_math, statistics }),
            Err(failure) => Err(SourceFftAnalysisVoiceGenerationFailure {
                cause: SourceFftAnalysisVoiceGenerationCause::AfterAnalysis(
                    SourceMathVoiceGenerationFailure::from_statistics_generation(failure)),
                analysis_math: Some((fft, postlog, join_math)), statistics: None,
            }),
        }
    }

    fn generate_final_with_statistics<M, E>(&self, input: &[f32], math: M,
        statistics: impl FnOnce(&[crate::pitch_selected_head::PitchRecord], &[f32], M)
            -> std::result::Result<(Option<[f32; 5]>, M), E>)
        -> std::result::Result<(Option<GeneratedVoice>, M), StatisticsGenerationFailure<M, E>>
    {
        let analysis = match self.analysis.forward_final_normalized48k(input) {
            Ok(analysis) => analysis,
            Err(cause) => return Err(StatisticsGenerationFailure {
                cause: StatisticsGenerationCause::Analysis(cause), math: Some(math),
                analysis: None, generated: None, points: Vec::new(),
            }),
        };
        self.generate_from_analysis_with_statistics(analysis, input.len(), math, statistics)
    }

    fn generate_from_analysis_with_statistics<M, E>(&self, analysis: crate::analysis_frontend::AnalysisRows,
        input_samples: usize, math: M,
        statistics: impl FnOnce(&[crate::pitch_selected_head::PitchRecord], &[f32], M)
            -> std::result::Result<(Option<[f32; 5]>, M), E>)
        -> std::result::Result<(Option<GeneratedVoice>, M), StatisticsGenerationFailure<M, E>>
    {
        let generated = match self.prepare_generation(&analysis.joined.normalized72,
            &analysis.joined.records, &analysis.joined.energies, input_samples)
        {
            Ok(Some(generated)) => generated,
            Ok(None) => return Ok((None, math)),
            Err(cause) => return Err(StatisticsGenerationFailure {
                cause: StatisticsGenerationCause::Generation(cause), math: Some(math),
                analysis: Some(analysis), generated: None, points: Vec::new(),
            }),
        };
        let (statistics, math) = match statistics(
            &analysis.joined.records, &analysis.joined.energies, math)
        {
            Ok(statistics) => statistics,
            Err(cause) => return Err(StatisticsGenerationFailure {
                cause: StatisticsGenerationCause::Statistics(cause), math: None,
                analysis: Some(analysis), generated: Some(generated), points: Vec::new(),
            }),
        };
        let Some(statistics) = statistics else { return Ok((None, math)); };
        match assemble_voice(generated, statistics) {
            Ok(voice) => Ok((Some(voice), math)),
            Err(failure) => Err(StatisticsGenerationFailure {
                cause: StatisticsGenerationCause::Assembly(failure.cause), math: Some(math),
                analysis: Some(analysis), generated: Some(failure.generated), points: failure.points,
            }),
        }
    }

    fn generate_from_joined_rows(&self, normalized72: &[f32], records: &[crate::pitch_selected_head::PitchRecord],
        energies: &[f32], input_samples: usize) -> Result<Option<GeneratedVoice>>
    {
        let Some(generated) = self.prepare_generation(normalized72, records, energies, input_samples)?
        else { return Ok(None); };
        let Some(pitch_stats) = pitch_statistics(records, energies)? else { return Ok(None); };
        assemble_voice(generated, pitch_stats).map(Some).map_err(|failure| failure.cause.into())
    }

    fn prepare_generation(&self, normalized72: &[f32], records: &[crate::pitch_selected_head::PitchRecord],
        energies: &[f32], input_samples: usize) -> Result<Option<crate::generator::ComputedGeneration>>
    {
        if normalized72.len() % 72 != 0 || normalized72.len() / 72 != records.len()
            || records.len() != energies.len()
        {
            return Err("generated voice analysis row dimensions differ".into());
        }
        self.generator.try_from_vec90_with_schedule(normalized72, input_samples)
    }
}

impl GeneratedVoice {

    pub fn source_document(&self, name: String) -> Result<Value> {
        FactoryVoice {
            name,
            data: self.data,
            pitch_stats: self.pitch_stats,
            points: self
                .points
                .iter()
                .map(|p| FactoryPoint {
                    data: p.data,
                    reduced: p.reduced,
                })
                .collect(),
        }
        .source_document()
    }
}
