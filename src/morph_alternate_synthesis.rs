use std::collections::VecDeque;
use crate::{Result,model::ModelTree,prototype_synthesis_networks::AlternateCommon};
use super::{AlternateFilterProgress,SynthesisStartupProgress,SynthesisStatistics,
    alternate_fir::AlternateFilterStream,worker::VocoderExecution};
use super::super::{coefficients::{CoefficientFrame,FeatureSynthesis},
    decoder::alternate_inner::AlternateInnerPair,excitation::{HarmonicExcitation,HarmonicRecord},
    generator::SourceGenerator,vocoder::VocoderStream};

const HOP:usize=240;
const TAPS:usize=512;

pub(crate) struct AlternateWaveformSynthesis {
    coefficients:FeatureSynthesis,
    first_filter:AlternateFilterStream,
    second_filter:AlternateFilterStream,
    vocoder:VocoderExecution,
    excitation:HarmonicExcitation,
    generator:SourceGenerator,
    modulation_offset:f32,
    pitch:VecDeque<(u64,f32,f32)>,
    waveform_conditions:VecDeque<[f32;72]>,
    primary_signals:VecDeque<[f32;HOP]>,
    primary_for_modulation:VecDeque<[f32;HOP]>,
    middle_samples:VecDeque<f32>,
    first_outputs:VecDeque<f32>,
    second_outputs:VecDeque<f32>,
    frames:Vec<CoefficientFrame>,
    samples:Vec<f32>,
    blocks:Vec<[f32;HOP]>,
    accepted:u64,
    published:u64,
    first_published:u64,
    second_published:u64,
    phase:f64,
    failure:Option<String>,
    statistics:SynthesisStatistics,
}

impl AlternateWaveformSynthesis {
    pub(super) fn new(userdata:&[u8],seed:u32)->Result<Self> {
        let tree=ModelTree::parse(userdata)?;
        let root=tree.root.children.get(1).ok_or("Missing alternate synthesis root")?;
        if tree.version!=2 || root.kind!=0x7fca41ff || root.tag!=0xc3b2e0d90e2fdb1b
            || root.data.len()!=16 || root.children.len()!=12 {
            return Err("Alternate synthesis root topology differs".into());
        }
        let band_count=u32::from_le_bytes(root.data[0..4].try_into().unwrap());
        let generator_gain=f32::from_le_bytes(root.data[4..8].try_into().unwrap());
        let secondary_parameter=f32::from_le_bytes(root.data[8..12].try_into().unwrap());
        let modulation_offset=f32::from_le_bytes(root.data[12..16].try_into().unwrap());
        if band_count!=8 || !generator_gain.is_finite() || !secondary_parameter.is_finite()
            || !modulation_offset.is_finite() {
            return Err("Alternate synthesis configuration differs".into());
        }

        let common=AlternateCommon::from_userdata(userdata)?;
        let filters=AlternateInnerPair::from_userdata(userdata)?;
        let AlternateCommon {first,second,subgroup,down8,up8,post,conditioning,projection,frequencies}=common;
        let coefficients=FeatureSynthesis::new_alternate(conditioning,first,second,filters,projection)?;
        let vocoder=VocoderExecution::new(VocoderStream::new(down8,up8,subgroup,post,true,seed)?)?;
        Ok(Self {coefficients,first_filter:AlternateFilterStream::new(),second_filter:AlternateFilterStream::new(),
            vocoder,excitation:HarmonicExcitation::new(frequencies)?,generator:SourceGenerator::new(generator_gain)?,
            modulation_offset,pitch:VecDeque::new(),waveform_conditions:VecDeque::new(),
            primary_signals:VecDeque::new(),primary_for_modulation:VecDeque::new(),
            middle_samples:VecDeque::new(),first_outputs:VecDeque::new(),second_outputs:VecDeque::new(),
            frames:Vec::new(),samples:Vec::new(),blocks:Vec::new(),accepted:0,published:0,
            first_published:0,second_published:0,phase:0.,failure:None,statistics:SynthesisStatistics::default()})
    }

    pub(super) fn uses_inner_worker(&self)->bool {self.vocoder.is_worker()}
    pub(super) fn statistics(&self)->SynthesisStatistics {self.statistics}
    pub(super) fn prepared_count_inputs(&self)->super::super::count_plan::Query<u64> {
        use super::super::count_plan as count;
        count::inner_serial(self.vocoder.is_worker())?;
        let p=self.startup_progress();
        count::cold(p.accepted_features!=0 || p.published_harmonic_hops!=0 || p.pending_pitch_rows!=0
            || p.pending_filter_rows!=0 || p.waveform_condition_rows!=0 || p.source_signal_hops_pending!=0
            || p.source_modulation_hops_pending!=0 || p.first_output_samples_pending!=0
            || p.second_output_samples_pending!=0 || p.second_coefficient_rows!=0 || p.middle_samples_pending!=0,
            p.failed || p.vocoder.failed)?;
        let f=p.alternate_filter.ok_or(count::Unavailable::Invalid(count::Invalid::StructuralMetadata))?;
        let d=p.vocoder.down;let u=p.vocoder.up;

        count::publication_policy(count::COUNT_POLICY_VERSION,
            f.first_coefficient_rows==0 && f.first_signal_hops==0 && f.second_coefficient_rows==0
            && f.second_signal_hops==0 && f.first_published_hops==0 && f.second_published_hops==0
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
    pub(super) fn startup_progress(&self)->SynthesisStartupProgress {
        let (first_rows,first_signals)=self.first_filter.queued_counts();
        let (second_rows,second_signals)=self.second_filter.queued_counts();
        SynthesisStartupProgress {
            accepted_features:self.accepted,published_harmonic_hops:self.published,
            pending_pitch_rows:self.pitch.len(),pending_filter_rows:first_rows,
            waveform_condition_rows:self.waveform_conditions.len(),
            source_signal_hops_pending:self.primary_signals.len(),
            source_modulation_hops_pending:self.primary_for_modulation.len(),
            first_output_samples_pending:self.first_outputs.len(),
            second_output_samples_pending:self.second_outputs.len(),
            second_coefficient_rows:second_rows,middle_samples_pending:self.middle_samples.len(),
            excitation_samples:self.statistics.excitation.samples,first_filter_samples:self.statistics.first_filter.samples,
            middle_samples_produced:self.statistics.middle.samples,second_filter_samples:self.statistics.second_filter.samples,
            failed:self.failure.is_some(),ltv:None,
            alternate_filter:Some(AlternateFilterProgress {first_coefficient_rows:first_rows,
                first_signal_hops:first_signals,second_coefficient_rows:second_rows,
                second_signal_hops:second_signals,first_published_hops:self.first_published,
                second_published_hops:self.second_published}),
            vocoder:self.vocoder.startup_progress(),vocoder_progress_current:self.vocoder.progress_current(),
            vocoder_execution:self.vocoder.mode(),vocoder_worker_pending:self.vocoder.has_pending(),
        }
    }
    pub(super) fn process(&mut self,raw:&[f32],feature:&[f32],output:&mut Vec<f32>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(raw,feature,output);
        if let Err(error)=&result {self.failure=Some(error.clone());self.vocoder.cancel_pending();}
        result
    }

    fn advance(&mut self,raw:&[f32],feature:&[f32],output:&mut Vec<f32>)->Result<()> {
        if raw.len()!=72 || feature.len()!=72 || raw.iter().chain(feature).any(|v|!v.is_finite()) || raw[1]<=0. {
            return Err("Alternate synthesis requires finite decoder72 rows and positive pitch".into());
        }
        self.phase+=f64::from(0.005_f32*raw[1]);
        let phase=self.phase as f32;
        if !phase.is_finite() {return Err("Nonfinite alternate harmonic phase".into());}
        self.pitch.push_back((self.accepted,raw[1],phase));
        self.accepted=self.accepted.checked_add(1).ok_or("Alternate feature ordinal overflow")?;
        self.waveform_conditions.push_back(feature.try_into().map_err(|_|"Alternate condition width")?);

        self.samples.clear();
        self.generator.process(raw,&mut self.samples)?;
        self.statistics.source_generator.observe(&self.samples);
        if self.samples.len()%HOP!=0 {return Err("Alternate generator published a partial hop".into());}
        for row in self.samples.chunks_exact(HOP) {
            let row:[f32;HOP]=row.try_into().map_err(|_|"Alternate generator hop width")?;
            self.primary_signals.push_back(row);self.primary_for_modulation.push_back(row);
        }
        if self.vocoder.is_worker() && !self.primary_signals.is_empty() {
            let signal=*self.primary_signals.front().ok_or("Missing alternate generator hop")?;
            let condition=*self.waveform_conditions.front().ok_or("Missing alternate waveform condition")?;
            self.vocoder.prefetch(signal,condition)?;
            self.primary_signals.pop_front();self.waveform_conditions.pop_front();
        }

        self.frames.clear();
        self.coefficients.process(feature,&mut self.frames)?;
        for frame in self.frames.drain(..) {
            let (ordinal,pitch,phase)=self.pitch.pop_front().ok_or("Missing alternate pitch")?;
            if ordinal!=frame.ordinal {return Err("Alternate pitch/coefficient ordinal mismatch".into());}
            let first:[f32;TAPS]=frame.first.try_into().map_err(|_|"Alternate first coefficient width")?;
            let second:[f32;TAPS]=frame.second.try_into().map_err(|_|"Alternate second coefficient width")?;
            self.first_filter.push_coefficients(&[first],false)?;
            self.second_filter.push_coefficients(&[second],false)?;

            self.samples.clear();
            self.excitation.process(HarmonicRecord {scalar0:pitch,scalar4:phase,
                projection10:&frame.projection10},&mut self.samples)?;
            self.statistics.excitation.observe(&self.samples);
            if !self.samples.is_empty() {
                if self.samples.len()!=HOP {return Err("Alternate harmonic producer published a partial hop".into());}
                let primary=self.primary_for_modulation.pop_front().ok_or("Missing alternate modulation hop")?;
                let mut signal=[0.;HOP];
                for ((to,source),harmonic) in signal.iter_mut().zip(primary).zip(&self.samples) {
                    *to=source*(self.modulation_offset+*harmonic);
                    if !to.is_finite() {return Err("Nonfinite alternate harmonic modulation".into());}
                }
                self.first_filter.push_signals(&[signal],false)?;
                self.published=self.published.checked_add(1).ok_or("Alternate harmonic count overflow")?;
            }
        }
        self.blocks.clear();
        self.first_filter.drain(&mut self.blocks)?;
        self.first_published=self.first_published.checked_add(self.blocks.len() as u64).ok_or("Alternate first count overflow")?;
        for block in self.blocks.drain(..) {
            self.statistics.first_filter.observe(&block);self.first_outputs.extend(block);
        }

        while self.vocoder.has_pending() || !self.primary_signals.is_empty() {
            self.samples.clear();
            if self.vocoder.has_pending() {self.vocoder.finish(&mut self.samples)?;} else {
                let signal=self.primary_signals.pop_front().ok_or("Missing alternate waveform hop")?;
                let condition=self.waveform_conditions.pop_front().ok_or("Missing alternate waveform condition")?;
                self.vocoder.process(&signal,&condition,&mut self.samples)?;
            }
            self.statistics.middle.observe(&self.samples);self.middle_samples.extend(self.samples.drain(..));
        }
        while self.middle_samples.len()>=HOP {
            let mut signal=[0.;HOP];
            for (to,value) in signal.iter_mut().zip(self.middle_samples.drain(..HOP)) {*to=value;}
            self.second_filter.push_signals(&[signal],false)?;
        }
        self.blocks.clear();
        self.second_filter.drain(&mut self.blocks)?;
        self.second_published=self.second_published.checked_add(self.blocks.len() as u64).ok_or("Alternate second count overflow")?;
        for block in self.blocks.drain(..) {self.statistics.second_filter.observe(&block);self.second_outputs.extend(block);}
        let paired=self.first_outputs.len().min(self.second_outputs.len());
        output.try_reserve(paired).map_err(|e|e.to_string())?;
        for (first,second) in self.first_outputs.drain(..paired).zip(self.second_outputs.drain(..paired)) {
            let sample=first+second;
            if !sample.is_finite() {return Err("Nonfinite alternate synthesis output".into());}
            output.push(sample);
        }
        Ok(())
    }
}
