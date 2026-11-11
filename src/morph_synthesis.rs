use std::collections::VecDeque;
use crate::{Result, continuous_dsp::LtvPair, prototype_synthesis_networks::SynthesisNetworks};
#[path="morph_vocoder_worker.rs"]
mod worker;
use worker::VocoderExecution;
use super::{coefficients::{CoefficientFrame,FeatureSynthesis}, vocoder::VocoderStream,
    generator::SourceGenerator,
    excitation::{HarmonicExcitation,HarmonicRecord}};

#[derive(Clone,Copy,Default)]
pub(crate) struct StageStatistics {
    pub samples:u64,
}
impl StageStatistics {
    fn observe(&mut self,samples:&[f32]) {
        self.samples=self.samples.saturating_add(samples.len() as u64);
    }
}
#[derive(Clone,Copy,Default)]
pub(crate) struct SynthesisStatistics {
    pub source_generator:StageStatistics,
    pub excitation:StageStatistics,
    pub first_filter:StageStatistics,
    pub middle:StageStatistics,
    pub second_filter:StageStatistics,
}

#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct SynthesisStartupProgress {
    pub accepted_features:u64,
    pub published_harmonic_hops:u64,
    pub pending_pitch_rows:usize,
    pub pending_filter_rows:usize,
    pub waveform_condition_rows:usize,
    pub source_signal_hops_pending:usize,
    pub source_modulation_hops_pending:usize,
    pub first_output_samples_pending:usize,
    pub second_output_samples_pending:usize,
    pub second_coefficient_rows:usize,
    pub middle_samples_pending:usize,
    pub excitation_samples:u64,
    pub first_filter_samples:u64,
    pub middle_samples_produced:u64,
    pub second_filter_samples:u64,
    pub failed:bool,
    pub ltv:Option<crate::continuous_dsp::LtvPairStartupProgress>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub alternate_filter:Option<AlternateFilterProgress>,
    pub vocoder:super::vocoder::VocoderStartupProgress,

    pub vocoder_progress_current:bool,
    pub vocoder_execution:&'static str,
    pub vocoder_worker_pending:bool,
}
#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct AlternateFilterProgress {
    pub first_coefficient_rows:usize,pub first_signal_hops:usize,
    pub second_coefficient_rows:usize,pub second_signal_hops:usize,
    pub first_published_hops:u64,pub second_published_hops:u64,
}
struct PendingFilter {
    ordinal:u64, first:Vec<f32>,
}

struct SynthesisScratch {
    frames:Vec<CoefficientFrame>,
    excitation:Vec<f32>,
    primary:Vec<f32>,
    first:Vec<f32>,
    modulated:[f32;240],
    vocoder:Vec<f32>,
    second:[f32;240],
    filtered_second:Vec<f32>,
}

pub(crate) struct WaveformSynthesis {
    coefficients: FeatureSynthesis,
    vocoder: VocoderExecution,
    filters: LtvPair,
    excitation: HarmonicExcitation,
    source_generator:SourceGenerator,
    scratch:SynthesisScratch,
    pitch_hz: VecDeque<(u64,f32,f32)>,
    pending_filters: VecDeque<PendingFilter>,
    accepted:u64, published:u64, phase_cycles:f64,
    waveform_conditions: VecDeque<[f32;72]>,
    primary_signals:VecDeque<[f32;240]>,
    primary_for_modulation:VecDeque<[f32;240]>,
    modulation_offset:f32,
    first_outputs:VecDeque<f32>,
    second_outputs:VecDeque<f32>,
    second_coefficients: VecDeque<Vec<f32>>,
    middle_samples: VecDeque<f32>,
    failure: Option<String>,
    statistics:SynthesisStatistics,
}
impl WaveformSynthesis {
    pub(crate) fn new(networks:SynthesisNetworks, noise_enabled:bool, random_seed:u32,
        generator_log_gain:f32,modulation_offset:f32) -> Result<Self> {
        if !modulation_offset.is_finite() {return Err("Nonfinite harmonic modulation offset".into());}
        let SynthesisNetworks {first,second,subgroup,cepstral,down8,up8,post,conditioning,projection,frequencies}=networks;
        Ok(Self {
            coefficients:FeatureSynthesis::new(conditioning,first,second,cepstral,projection)?,
            vocoder:VocoderExecution::new(VocoderStream::new(down8,up8,subgroup,post,noise_enabled,random_seed)?)?,
            filters:LtvPair::new()?, excitation:HarmonicExcitation::new(frequencies)?,
            source_generator:SourceGenerator::new(generator_log_gain)?,
            scratch:SynthesisScratch {frames:Vec::new(),excitation:Vec::new(),
                primary:Vec::new(),first:Vec::new(),modulated:[0.;240],vocoder:Vec::new(),second:[0.;240],
                filtered_second:Vec::new()},
            pitch_hz:VecDeque::new(),pending_filters:VecDeque::new(),
            accepted:0,published:0,phase_cycles:0.,waveform_conditions:VecDeque::new(),
            primary_signals:VecDeque::new(),first_outputs:VecDeque::new(),second_outputs:VecDeque::new(),
            primary_for_modulation:VecDeque::new(),modulation_offset,
            second_coefficients:VecDeque::new(), middle_samples:VecDeque::new(), failure:None,
            statistics:SynthesisStatistics::default(),
        })
    }
    pub(crate) fn prepared_count_inputs(&self)->super::count_plan::Query<u64> {
        use super::count_plan as count;

        count::inner_serial(self.vocoder.is_worker())?;
        let p=self.startup_progress();
        let ltv=p.ltv.ok_or(count::Unavailable::Unsupported(count::Unsupported::UnknownTopology))?;
        count::cold(p.accepted_features!=0 || p.published_harmonic_hops!=0
            || p.pending_pitch_rows!=0 || p.pending_filter_rows!=0 || p.waveform_condition_rows!=0
            || p.source_signal_hops_pending!=0 || p.source_modulation_hops_pending!=0
            || p.first_output_samples_pending!=0 || p.second_output_samples_pending!=0
            || p.second_coefficient_rows!=0 || p.middle_samples_pending!=0,
            p.failed || ltv.failed || p.vocoder.failed)?;
        let ltv_initial=[ltv.first,ltv.second].iter().all(|s|
            s.startup_remaining==512 && s.publication_samples==0 && s.completed_hops==0 && s.published_hops==0);
        let d=p.vocoder.down;let u=p.vocoder.up;

        count::publication_policy(count::COUNT_POLICY_VERSION,ltv_initial
            && p.excitation_samples==0 && p.first_filter_samples==0 && p.middle_samples_produced==0
            && p.second_filter_samples==0 && p.vocoder_progress_current && !p.vocoder_worker_pending
            && d.input_samples==0 && d.padding_samples==0 && d.output_rows==0
            && d.ring_position==0 && d.counter==32 && !d.finished
            && u.input_rows==0 && u.suppressed_ticks==0 && u.output_samples==0 && u.eos_samples==0
            && u.ring_position==0 && u.counter==0 && !u.finished
            && p.vocoder.trim_remaining==3 && p.vocoder.pending_feature_rows==0
            && p.vocoder.pending_condition_rows==0 && !p.vocoder.has_previous_condition)?;
        self.coefficients.prepared_right_rows()
    }
    pub(crate) fn statistics(&self)->SynthesisStatistics {self.statistics}
    pub(crate) fn startup_progress(&self)->SynthesisStartupProgress {
        SynthesisStartupProgress {
            accepted_features:self.accepted,published_harmonic_hops:self.published,
            pending_pitch_rows:self.pitch_hz.len(),pending_filter_rows:self.pending_filters.len(),
            waveform_condition_rows:self.waveform_conditions.len(),
            source_signal_hops_pending:self.primary_signals.len(),
            source_modulation_hops_pending:self.primary_for_modulation.len(),
            first_output_samples_pending:self.first_outputs.len(),
            second_output_samples_pending:self.second_outputs.len(),
            second_coefficient_rows:self.second_coefficients.len(),
            middle_samples_pending:self.middle_samples.len(),
            excitation_samples:self.statistics.excitation.samples,
            first_filter_samples:self.statistics.first_filter.samples,
            middle_samples_produced:self.statistics.middle.samples,
            second_filter_samples:self.statistics.second_filter.samples,
            failed:self.failure.is_some(),ltv:Some(self.filters.startup_progress()),alternate_filter:None,
            vocoder:self.vocoder.startup_progress(),
            vocoder_progress_current:self.vocoder.progress_current(),
            vocoder_execution:self.vocoder.mode(),
            vocoder_worker_pending:self.vocoder.has_pending(),
        }
    }

    pub(crate) fn process(&mut self, raw:&[f32], feature:&[f32],
        output:&mut Vec<f32>) -> Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(raw,feature,output);
        if let Err(error)=&result {
            self.failure=Some(error.clone());
            self.vocoder.cancel_pending();
        }
        result
    }

    fn advance(&mut self, raw:&[f32], feature:&[f32],output:&mut Vec<f32>) -> Result<()> {
        if raw.len()!=72 || feature.len()!=72 {return Err("Synthesis feature width differs".into());}
        let raw_pitch_hz=raw[1];
        if !raw_pitch_hz.is_finite() || raw_pitch_hz<=0. {
            return Err("Synthesis requires the actual positive raw decoder pitch in Hz".into());
        }

        self.phase_cycles+=f64::from(0.005_f32*raw_pitch_hz);
        let phase=self.phase_cycles as f32;
        if !phase.is_finite() {return Err("Nonfinite harmonic phase accumulator".into());}
        self.pitch_hz.push_back((self.accepted,raw_pitch_hz,phase));
        self.accepted=self.accepted.checked_add(1).ok_or("Waveform feature ordinal overflow")?;
        self.waveform_conditions.push_back(feature.try_into().map_err(|_|"Waveform condition width")?);
        self.scratch.primary.clear();
        let result=self.source_generator.process(raw,&mut self.scratch.primary);
        result?;
        self.statistics.source_generator.observe(&self.scratch.primary);
        if self.scratch.primary.len()%240!=0 {return Err("Source generator published a partial hop".into());}
        for signal in self.scratch.primary.chunks_exact(240) {
            let row:[f32;240]=signal.try_into().map_err(|_|"Source signal width")?;
            self.primary_signals.push_back(row);
            self.primary_for_modulation.push_back(row);
        }

        if self.vocoder.is_worker() && !self.primary_signals.is_empty() {
            let signal=*self.primary_signals.front().ok_or("Missing primary hop")?;
            let condition=*self.waveform_conditions.front().ok_or("Missing condition for source-generator hop")?;
            self.vocoder.prefetch(signal,condition)?;
            self.primary_signals.pop_front();
            self.waveform_conditions.pop_front();
        }
        self.scratch.frames.clear();
        let result=self.coefficients.process(feature,&mut self.scratch.frames);
        if let Err(error)=result {
            self.scratch.frames.clear();return Err(error);
        }

        for frame in self.scratch.frames.drain(..) {
            let (ordinal,pitch_hz,scalar4)=self.pitch_hz.pop_front().ok_or("Missing raw pitch for learned harmonic record")?;
            if ordinal!=frame.ordinal {return Err("Harmonic pitch/coefficient ordinal mismatch".into());}
            self.pending_filters.push_back(PendingFilter {ordinal,first:frame.first});
            self.second_coefficients.push_back(frame.second);
            self.scratch.excitation.clear();
            let result=self.excitation.process(HarmonicRecord {scalar0:pitch_hz,scalar4,
                projection10:&frame.projection10},&mut self.scratch.excitation);
            if let Err(error)=result {
                self.scratch.excitation.clear();return Err(error);
            }
            self.statistics.excitation.observe(&self.scratch.excitation);
            if self.scratch.excitation.is_empty() {continue;}
            if self.scratch.excitation.len()!=240 {return Err("Harmonic producer published a partial signal hop".into());}
            let pending=self.pending_filters.pop_front().ok_or("Missing filters for published harmonic hop")?;
            if pending.ordinal!=self.published {return Err("Harmonic/filter publication ordinal mismatch".into());}
            self.published=self.published.checked_add(1).ok_or("Harmonic publication ordinal overflow")?;
            let primary=self.primary_for_modulation.pop_front()
                .ok_or("Missing primary source hop for harmonic modulation")?;

            for ((destination,source),subharmonic) in self.scratch.modulated.iter_mut()
                .zip(primary).zip(&self.scratch.excitation) {
                let envelope=self.modulation_offset+*subharmonic;
                let sample=source*envelope;
                if !sample.is_finite() {return Err("Nonfinite primary/subharmonic modulation".into());}
                *destination=sample;
            }
            self.scratch.first.clear();
            let result=self.filters.process_first(&self.scratch.modulated,&pending.first,&mut self.scratch.first);
            if let Err(error)=result {
                self.scratch.first.clear();return Err(error);
            }
            self.statistics.first_filter.observe(&self.scratch.first);
            if self.scratch.first.len()%240!=0 {return Err("First filter published a partial model frame".into());}
            self.first_outputs.extend(self.scratch.first.drain(..));
        }

        while self.vocoder.has_pending() || !self.primary_signals.is_empty() {
            self.scratch.vocoder.clear();
            let result=if self.vocoder.has_pending() {
                self.vocoder.finish(&mut self.scratch.vocoder)
            } else {
                let signal=self.primary_signals.pop_front().ok_or("Missing primary hop")?;
                let condition=self.waveform_conditions.pop_front().ok_or("Missing condition for source-generator hop")?;
                self.vocoder.process(&signal,&condition,&mut self.scratch.vocoder)
            };
            if let Err(error)=result {
                self.scratch.vocoder.clear();return Err(error);
            }
            self.statistics.middle.observe(&self.scratch.vocoder);
            self.middle_samples.extend(self.scratch.vocoder.drain(..));
        }
        while self.middle_samples.len()>=240 && !self.second_coefficients.is_empty() {
            let coefficients=self.second_coefficients.pop_front().ok_or("Missing second-filter coefficients")?;
            for (slot,sample) in self.scratch.second.iter_mut().zip(self.middle_samples.drain(..240)) {
                *slot=sample;
            }
            self.scratch.filtered_second.clear();
            let result=self.filters.process_second(&self.scratch.second,&coefficients,&mut self.scratch.filtered_second);
            result?;
            if self.scratch.filtered_second.len()%240!=0 {return Err("Second filter published a partial model frame".into());}
            self.statistics.second_filter.observe(&self.scratch.filtered_second);
            self.second_outputs.extend(self.scratch.filtered_second.drain(..));
        }
        let paired=self.first_outputs.len().min(self.second_outputs.len());
        output.try_reserve(paired).map_err(|e|e.to_string())?;
        for (first,second) in self.first_outputs.drain(..paired).zip(self.second_outputs.drain(..paired)) {
            let sample=second+first;
            if !sample.is_finite() {return Err("Nonfinite paired synthesis output".into());}
            output.push(sample);
        }
        Ok(())
    }
}

#[path="morph_alternate_fir.rs"]
mod alternate_fir;
#[path="morph_alternate_synthesis.rs"]
mod alternate;

pub(crate) enum SynthesisBackend {
    Primary(Box<WaveformSynthesis>),
    Alternate(Box<alternate::AlternateWaveformSynthesis>),
}
impl SynthesisBackend {
    pub(crate) fn primary(networks:SynthesisNetworks,noise:bool,seed:u32,
        gain:f32,offset:f32)->Result<Self> {
        Ok(Self::Primary(Box::new(WaveformSynthesis::new(networks,noise,seed,gain,offset)?)))
    }
    pub(crate) fn alternate(userdata:&[u8],seed:u32)->Result<Self> {
        Ok(Self::Alternate(Box::new(alternate::AlternateWaveformSynthesis::new(userdata,seed)?)))
    }
    pub(crate) fn prepared_count_inputs(&self)->super::count_plan::Query<(u64,u32)> {
        match self {
            Self::Primary(owner)=>owner.prepared_count_inputs().map(|rows|
                (rows,super::count_plan::COUNT_POLICY_VERSION)),
            Self::Alternate(owner)=>owner.prepared_count_inputs().map(|rows|
                (rows,super::count_plan::ALTERNATE_COUNT_POLICY_VERSION)),
        }
    }
    pub(crate) fn process(&mut self,raw:&[f32],feature:&[f32],out:&mut Vec<f32>)->Result<()> {
        match self {Self::Primary(owner)=>owner.process(raw,feature,out),
            Self::Alternate(owner)=>owner.process(raw,feature,out)}
    }
    pub(crate) fn uses_inner_worker(&self)->bool {
        match self {Self::Primary(owner)=>owner.vocoder.is_worker(),Self::Alternate(owner)=>owner.uses_inner_worker()}
    }
    pub(crate) fn statistics(&self)->SynthesisStatistics {
        match self {Self::Primary(owner)=>owner.statistics(),Self::Alternate(owner)=>owner.statistics()}
    }
    pub(crate) fn startup_progress(&self)->SynthesisStartupProgress {
        match self {Self::Primary(owner)=>owner.startup_progress(),Self::Alternate(owner)=>owner.startup_progress()}
    }
}
