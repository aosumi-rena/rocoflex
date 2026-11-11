use super::{GeneratedVoice, VoiceGenerator};
use crate::{
    analysis_frontend::{AnalysisHistory, AnalysisStartFailure, AnalysisStream, AnalysisStreamFailure,
        AnalysisStreamPolicy},
    pitch_selected_head::PitchRecord,
};

pub struct VoiceGenerationStream<'a> {
    generator: &'a VoiceGenerator,
    analysis: Option<AnalysisStream<'a>>,
    normalized72: Vec<f32>,
    records: Vec<PitchRecord>,
    energies: Vec<f32>,
    ended: bool,
    halted: bool,
}

pub enum VoiceGenerationStreamCause<'a> {
    Admission(String),
    Analysis(AnalysisStreamFailure<'a>),
    Generation(String),
}

pub struct VoiceGenerationStreamFailure<'a> {
    pub cause: VoiceGenerationStreamCause<'a>,
    pub advanced: VoiceGenerationStream<'a>,
}

pub struct VoiceGenerationBatch<'a> {
    pub newly_joined_rows: usize,
    pub pending_rows: (usize, usize),

    pub voice: Option<GeneratedVoice>,
    pub final_input: bool,
    pub state: VoiceGenerationStream<'a>,
}

impl<'a> VoiceGenerationStream<'a> {
    pub(super) fn new(generator: &'a VoiceGenerator, history: AnalysisHistory, maximum_samples: usize,
        policy: AnalysisStreamPolicy)
        -> Result<Self, AnalysisStartFailure>
    {
        let analysis = generator.analysis.stream_from_explicit_history(history, maximum_samples, policy)?;
        Ok(Self { generator, analysis: Some(analysis), normalized72: Vec::new(),
            records: Vec::new(), energies: Vec::new(), ended: false, halted: false })
    }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn collected_rows(&self) -> usize { self.records.len() }
    pub fn normalized_rows(&self) -> &[f32] { &self.normalized72 }
    pub fn records(&self) -> &[PitchRecord] { &self.records }
    pub fn energies(&self) -> &[f32] { &self.energies }
    pub fn analysis_state(&self) -> Option<&AnalysisStream<'a>> { self.analysis.as_ref() }

    fn failure(self, cause: VoiceGenerationStreamCause<'a>) -> VoiceGenerationStreamFailure<'a> {
        VoiceGenerationStreamFailure { cause, advanced: self }
    }

    pub fn push(mut self, samples: &[f32], final_input: bool)
        -> Result<VoiceGenerationBatch<'a>, VoiceGenerationStreamFailure<'a>>
    {
        if self.ended || self.halted || self.analysis.is_none() {
            return Err(self.failure(VoiceGenerationStreamCause::Admission(
                "Voice generation stream is already ended or halted".into())));
        }
        let analysis = self.analysis.take().expect("active analysis owner");
        let batch = match analysis.push(samples, final_input) {
            Ok(batch) => batch,
            Err(failure) => {
                self.halted = true;
                return Err(self.failure(VoiceGenerationStreamCause::Analysis(failure)));
            }
        };
        let pending_rows = batch.pending;
        let input_samples = batch.state.accepted_samples();
        self.analysis = Some(batch.state);
        let newly_joined_rows = batch.joined.records.len();
        self.normalized72.extend(batch.joined.normalized72);
        self.records.extend(batch.joined.records);
        self.energies.extend(batch.joined.energies);
        let voice = if final_input {
            match self.generator.generate_from_joined_rows(&self.normalized72,
                &self.records, &self.energies, input_samples)
            {
                Ok(voice) => voice,
                Err(reason) => {
                    self.halted = true;
                    return Err(self.failure(VoiceGenerationStreamCause::Generation(reason)));
                }
            }
        } else { None };
        self.ended = final_input;
        Ok(VoiceGenerationBatch { newly_joined_rows, pending_rows, voice, final_input, state: self })
    }
}

pub struct PreparedVoiceStream<'a> {
    audio: crate::ordinary_audio::PreparedAudio,
    stream: Option<VoiceGenerationStream<'a>>,
    next_sample: usize,
    ended: bool,
    halted: bool,
}

pub struct PreparedVoiceStartFailure {
    pub reason: String,
    pub audio: crate::ordinary_audio::PreparedAudio,
    pub history: AnalysisHistory,
}

pub struct PreparedVoiceFailure<'a> {
    pub reason: String,

    pub attempted_samples: Option<std::ops::Range<usize>>,
    pub pipeline_failure: Option<VoiceGenerationStreamFailure<'a>>,
    pub advanced: PreparedVoiceStream<'a>,
}

pub struct PreparedVoiceBatch<'a> {
    pub accepted_samples: std::ops::Range<usize>,
    pub newly_joined_rows: usize,
    pub pending_rows: (usize, usize),

    pub voice: Option<GeneratedVoice>,
    pub complete: bool,
    pub state: PreparedVoiceStream<'a>,
}

impl<'a> PreparedVoiceStream<'a> {
    pub(super) fn new(generator: &'a VoiceGenerator,
        audio: crate::ordinary_audio::PreparedAudio, history: AnalysisHistory,
        policy: AnalysisStreamPolicy)
        -> Result<Self, PreparedVoiceStartFailure>
    {
        if audio.analysis_rate != 48_000 || audio.samples.is_empty()
            || audio.samples.len() > i32::MAX as usize
            || audio.samples.iter().any(|v| !v.is_finite())
        {
            return Err(PreparedVoiceStartFailure {
                reason: "Prepared voice input requires finite normalized mono 48 kHz samples within the i32 domain".into(),
                audio, history,
            });
        }
        let stream = match generator.stream_from_explicit_history(history, audio.samples.len(), policy) {
            Ok(stream) => stream,
            Err(failure) => return Err(PreparedVoiceStartFailure {
                reason: failure.reason, audio, history: failure.history,
            }),
        };
        Ok(Self { audio, stream: Some(stream), next_sample: 0, ended: false, halted: false })
    }

    pub fn metadata(&self) -> &crate::wav48k::Metadata { &self.audio.metadata }
    pub fn prepared_audio(&self) -> &crate::ordinary_audio::PreparedAudio { &self.audio }
    pub fn accepted_samples(&self) -> usize { self.next_sample }
    pub fn remaining_samples(&self) -> usize { self.audio.samples.len() - self.next_sample }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn pipeline(&self) -> Option<&VoiceGenerationStream<'a>> { self.stream.as_ref() }

    pub fn advance(mut self, maximum_new_samples: usize)
        -> Result<PreparedVoiceBatch<'a>, PreparedVoiceFailure<'a>>
    {
        if self.ended || self.halted || self.stream.is_none() || maximum_new_samples == 0 {
            return Err(PreparedVoiceFailure {
                reason: "Prepared voice stream requires an active owner and a positive sample budget".into(),
                attempted_samples: None, pipeline_failure: None, advanced: self,
            });
        }
        let begin = self.next_sample;
        let end = begin + maximum_new_samples.min(self.audio.samples.len() - begin);
        let complete = end == self.audio.samples.len();
        let stream = self.stream.take().expect("active prepared voice pipeline");
        let batch = match stream.push(&self.audio.samples[begin..end], complete) {
            Ok(batch) => batch,
            Err(failure) => {
                self.halted = true;
                return Err(PreparedVoiceFailure {
                    reason: "Prepared voice analysis or generation failed; advanced state is retained".into(),
                    attempted_samples: Some(begin..end), pipeline_failure: Some(failure), advanced: self,
                });
            }
        };
        self.next_sample = end;
        self.ended = complete;
        self.stream = Some(batch.state);
        Ok(PreparedVoiceBatch { accepted_samples: begin..end,
            newly_joined_rows: batch.newly_joined_rows, pending_rows: batch.pending_rows,
            voice: batch.voice, complete, state: self })
    }
}
