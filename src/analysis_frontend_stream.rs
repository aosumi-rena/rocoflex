use super::AnalysisFrontend;
use crate::{
    audio_spectrum::{SpectrumStream, SpectrumStreamFailure},
    frontend_producer::{FrontendStream, FrontendStreamFailure},
    pitch_audio_spectrum::{PitchSpectrumStream, PitchSpectrumStreamFailure},
    pitch_selected_head::{PitchRecord, SelectedPitchStream, SelectedPitchStreamFailure},
    pitch_spectral_encoder::{PitchEncoderStream, PitchEncoderStreamFailure},
    pitch_temporal::{PitchTemporalState, PitchTemporalStream, PitchTemporalStreamFailure},
    source_pitch_join::{JoinedRows, SourcePitchJoin},
};

pub struct AnalysisHistory {
    pub source_hidden384: Vec<f32>,
    pub pitch_hidden: PitchTemporalState,
    pub previous_pitch_position: i32,
}
pub struct AnalysisStartFailure { pub reason: String, pub history: AnalysisHistory }

#[derive(Clone, Copy, Debug)]
pub enum AnalysisStreamPolicy { WindowsHostExistingOperators }

fn check_environment(policy: AnalysisStreamPolicy) -> crate::Result<()> {
    match policy { AnalysisStreamPolicy::WindowsHostExistingOperators => {} }
    #[cfg(all(target_arch="x86_64", target_os="windows"))]
    {
        let mut word = 0u32;
        unsafe { core::arch::asm!("stmxcsr [{address}]", address = in(reg) &mut word,
            options(nostack, preserves_flags)); }
        if word & 0xffc0 != 0x1f80 {
            return Err(format!("Analysis stream requires RNE, masked exceptions and gradual underflow; MXCSR={word:08x}"));
        }
        Ok(())
    }
    #[cfg(not(all(target_arch="x86_64", target_os="windows")))]
    { Err("The selected analysis stream policy requires Windows x86-64".into()) }
}

pub struct AnalysisStream<'a> {
    frontend: &'a AnalysisFrontend,
    policy: AnalysisStreamPolicy,
    spectrum: Option<SpectrumStream<'a>>,
    source: Option<FrontendStream<'a>>,
    pitch_spectrum: Option<PitchSpectrumStream<'a>>,
    pitch_encoder: Option<PitchEncoderStream<'a>>,
    pitch_temporal: Option<PitchTemporalStream<'a>>,
    pitch_head: Option<SelectedPitchStream<'a>>,
    join: SourcePitchJoin,
    sample_limit: usize,
    accepted_samples: usize,
    source_rows: usize,
    pitch_rows: usize,
    joined_rows: usize,
    ended: bool,
    halted: bool,
}

pub enum AnalysisStreamCause<'a> {
    Admission(String),
    Spectrum(SpectrumStreamFailure<'a>),
    Source(FrontendStreamFailure<'a>),
    PitchSpectrum(PitchSpectrumStreamFailure<'a>),
    PitchEncoder(PitchEncoderStreamFailure<'a>),
    PitchTemporal(PitchTemporalStreamFailure<'a>),
    PitchHead(SelectedPitchStreamFailure<'a>),
    Arithmetic { stage: &'static str, reason: String },
}

pub struct AnalysisStreamWork {
    pub postlog513: Vec<f32>,
    pub spectral256: Vec<f32>,
    pub network72: Vec<f32>,
    pub pitch_spectra: [Vec<f32>; 3],
    pub pitch_encoded384: Vec<f32>,
    pub pitch_temporal384: Vec<f32>,
    pub records: Vec<PitchRecord>,
}
impl AnalysisStreamWork {
    fn empty() -> Self {
        Self { postlog513: Vec::new(), spectral256: Vec::new(), network72: Vec::new(),
            pitch_spectra: std::array::from_fn(|_| Vec::new()),
            pitch_encoded384: Vec::new(), pitch_temporal384: Vec::new(), records: Vec::new() }
    }
}
pub struct AnalysisStreamFailure<'a> {
    pub cause: AnalysisStreamCause<'a>,
    pub work: AnalysisStreamWork,
    pub advanced: AnalysisStream<'a>,
}
pub struct AnalysisStreamBatch<'a> {
    pub first_joined_row: usize,
    pub network72: Vec<f32>,
    pub joined: JoinedRows,
    pub source_rows: usize,
    pub pitch_rows: usize,
    pub pending: (usize, usize),
    pub final_input: bool,
    pub state: AnalysisStream<'a>,
}

impl<'a> AnalysisStream<'a> {
    pub(super) fn new(frontend: &'a AnalysisFrontend, history: AnalysisHistory, sample_limit: usize,
        policy: AnalysisStreamPolicy)
        -> Result<Self, AnalysisStartFailure>
    {
        if sample_limit == 0 || sample_limit > i32::MAX as usize
            || history.source_hidden384.len() != 384
            || history.pitch_hidden.first.len() != 1024 || history.pitch_hidden.second.len() != 1024
            || history.source_hidden384.iter().chain(&history.pitch_hidden.first)
                .chain(&history.pitch_hidden.second).any(|v| !v.is_finite())
            || !(-1..128).contains(&history.previous_pitch_position)
        {
            return Err(AnalysisStartFailure {
                reason: "Analysis stream requires finite explicit recurrent history, pitch position and a positive i32 sample limit".into(),
                history,
            });
        }

        let row_limit = (sample_limit + 2048).div_ceil(240);
        let attempt = (|| -> crate::Result<Self> {
            Ok(Self {
                frontend, policy,
                spectrum: Some(frontend.spectrum.stream_with_limit(sample_limit)?),
                source: Some(frontend.source.stream_from_explicit_state(
                    history.source_hidden384.clone(), row_limit).map_err(|e| e.reason)?),
                pitch_spectrum: Some(frontend.pitch_spectrum.stream_with_limit(sample_limit)?),
                pitch_encoder: Some(frontend.pitch_encoder.stream_with_limit(row_limit)?),
                pitch_temporal: Some(frontend.pitch_temporal.stream_from_explicit_state(
                    PitchTemporalState { first: history.pitch_hidden.first.clone(),
                        second: history.pitch_hidden.second.clone() }, row_limit).map_err(|e| e.reason)?),
                pitch_head: Some(frontend.pitch_head.stream_from_explicit_position(
                    history.previous_pitch_position, row_limit)?),
                join: SourcePitchJoin::new(frontend.normalization.clone()), sample_limit,
                accepted_samples: 0, source_rows: 0, pitch_rows: 0, joined_rows: 0,
                ended: false, halted: false,
            })
        })();
        attempt.map_err(|reason| AnalysisStartFailure { reason, history })
    }

    pub fn accepted_samples(&self) -> usize { self.accepted_samples }
    pub fn math_policy(&self) -> AnalysisStreamPolicy { self.policy }
    pub fn emitted_counts(&self) -> (usize, usize, usize) {
        (self.source_rows, self.pitch_rows, self.joined_rows)
    }
    pub fn pending_counts(&self) -> (usize, usize) { self.join.pending_counts() }
    pub fn unmatched_spectral_rows(&self) -> Option<[usize; 3]> {
        self.pitch_encoder.as_ref().map(PitchEncoderStream::retained_row_counts)
    }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, cause: AnalysisStreamCause<'a>, work: AnalysisStreamWork)
        -> AnalysisStreamFailure<'a>
    {
        AnalysisStreamFailure { cause, work, advanced: self }
    }

    pub fn push(mut self, input: &[f32], final_input: bool)
        -> Result<AnalysisStreamBatch<'a>, AnalysisStreamFailure<'a>>
    {
        if self.ended || self.halted {
            return Err(self.failure(AnalysisStreamCause::Admission(
                "Analysis stream is already ended or halted".into()), AnalysisStreamWork::empty()));
        }
        if let Err(reason) = check_environment(self.policy) {
            return Err(self.failure(AnalysisStreamCause::Admission(reason), AnalysisStreamWork::empty()));
        }
        let admitted = self.accepted_samples.checked_add(input.len())
            .filter(|&n| n <= self.sample_limit);
        if input.iter().any(|v| !v.is_finite()) || admitted.is_none()
            || (final_input && admitted == Some(0))
        {
            return Err(self.failure(AnalysisStreamCause::Admission(
                "Analysis stream requires finite input within its sample limit and nonempty final input".into()),
                AnalysisStreamWork::empty()));
        }
        self.accepted_samples = admitted.expect("admitted sample count");
        let mut work = AnalysisStreamWork::empty();
        let joined = match self.advance(input, final_input, &mut work) {
            Ok(joined) => joined,
            Err(cause) => { self.halted = true; return Err(self.failure(cause, work)); }
        };
        let first_joined_row = self.joined_rows;
        self.joined_rows += joined.records.len();
        let source_rows = work.network72.len() / 72;
        let pitch_rows = work.records.len();
        self.ended = final_input;
        let pending = self.join.pending_counts();
        Ok(AnalysisStreamBatch { first_joined_row, network72: work.network72, joined,
            source_rows, pitch_rows, pending, final_input, state: self })
    }

    fn advance(&mut self, input: &[f32], final_input: bool, work: &mut AnalysisStreamWork)
        -> Result<JoinedRows, AnalysisStreamCause<'a>>
    {
        let spectrum = self.spectrum.take().expect("active source spectral owner")
            .push(input, final_input).map_err(AnalysisStreamCause::Spectrum)?;
        self.spectrum = Some(spectrum.state);
        work.postlog513 = spectrum.postlog513;
        work.spectral256 = if work.postlog513.is_empty() { Vec::new() } else {
            self.frontend.encoder.forward513(&work.postlog513)
                .map_err(|reason| AnalysisStreamCause::Arithmetic { stage: "spectral encoder", reason })?
                .spectral256
        };
        let source = self.source.take().expect("active source producer owner")
            .push(&work.spectral256, final_input).map_err(AnalysisStreamCause::Source)?;
        self.source = Some(source.state);
        work.network72 = source.projection.map_or_else(Vec::new, |p| p.features72);
        self.source_rows += work.network72.len() / 72;

        let spectra = self.pitch_spectrum.take().expect("active pitch spectral owner")
            .push(input, final_input).map_err(AnalysisStreamCause::PitchSpectrum)?;
        self.pitch_spectrum = Some(spectra.state);
        work.pitch_spectra = spectra.spectra;
        let encoded = self.pitch_encoder.take().expect("active pitch encoder owner")
            .push([&work.pitch_spectra[0], &work.pitch_spectra[1], &work.pitch_spectra[2]], final_input)
            .map_err(AnalysisStreamCause::PitchEncoder)?;
        self.pitch_encoder = Some(encoded.state);
        work.pitch_encoded384 = encoded.stages.map_or_else(Vec::new, |s| s.summed384);
        let temporal = self.pitch_temporal.take().expect("active pitch recurrent owner")
            .push(&work.pitch_encoded384, final_input).map_err(AnalysisStreamCause::PitchTemporal)?;
        self.pitch_temporal = Some(temporal.state);
        work.pitch_temporal384 = temporal.stages.map_or_else(Vec::new, |s| s.output384);
        let head = self.pitch_head.take().expect("active selected pitch head owner")
            .push(&work.pitch_temporal384, final_input).map_err(AnalysisStreamCause::PitchHead)?;
        self.pitch_head = Some(head.state);
        work.records = head.rows.map_or_else(Vec::new, |r| r.records);
        self.pitch_rows += work.records.len();
        self.join.append(&work.network72, &work.records)
            .map_err(|reason| AnalysisStreamCause::Arithmetic { stage: "source/pitch join", reason })
    }
}
