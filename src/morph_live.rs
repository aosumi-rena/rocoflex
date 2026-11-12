use std::{collections::VecDeque,sync::Arc};
use crate::{Result,current_condition::{ConditionReader,CurrentCondition,ConditionTarget},
    continuous_dsp::FixedHopStream,tone::ToneGenerator};
use super::{source::{SourceProducer,SourceFrame},engine::{MorphEngine,SourceFeatures}};

#[path="morph_audio_levels.rs"]
mod levels;
use levels::AudioLevels;
use super::input_gain::InputGain;
#[path="morph_input_filters.rs"]
mod input_filters;
use input_filters::SourceInputFilters;

struct ToneConditionProvider {
    generator:ToneGenerator,
    cached:Option<([u32;3],[f32;128])>,
}
impl ToneConditionProvider {
    fn resolve(&mut self,target:&ConditionTarget)->Result<[f32;128]> {
        match target {
            ConditionTarget::Canvas128 {embedding,..}=>Ok(*embedding),
            ConditionTarget::TonePreview {controls}=>{
                let key=(*controls).map(f32::to_bits);
                if let Some((prior,value))=&self.cached {
                    if *prior==key {return Ok(*value);}
                }

                let value=self.generator.generate_embedding(*controls)?;
                if value.iter().any(|value|!value.is_finite()) {
                    return Err("Tone preview produced nonfinite conditioning".into());
                }
                self.cached=Some((key,value));
                Ok(value)
            }
        }
    }
}

struct InputHop {ordinal:u64,condition:Arc<CurrentCondition>,target128:[f32;128],source_tag:[u64;3]}
pub(crate) struct LiveGraph {
    framing:FixedHopStream,source:SourceProducer,engine:MorphEngine,
    tone:ToneConditionProvider,
    input_filters:SourceInputFilters,
    levels:AudioLevels,
    source_frames:Vec<SourceFrame>,
    pitch_observation:crate::pitch_display::Source,
    reader:ConditionReader,pending:VecDeque<InputHop>,next_ordinal:u64,
}
impl LiveGraph {
    pub(crate) fn new(analysis:&[u8],userdata:&[u8],reader:ConditionReader)->Result<Self> {
        Self::with_profile(analysis,userdata,reader,crate::prepare_quality::PrepareProfile::Highest)
    }
    pub(crate) fn with_profile(analysis:&[u8],userdata:&[u8],reader:ConditionReader,
        profile:crate::prepare_quality::PrepareProfile)->Result<Self> {
        let selection=super::engine::SynthesisSelection::default_auto()?;
        Self::with_selection(analysis,userdata,reader,profile,selection)
    }
    pub(crate) fn with_instance_policy(analysis:&[u8],userdata:&[u8],reader:ConditionReader,
        profile:crate::prepare_quality::PrepareProfile,multithread:bool)->Result<Self> {
        let selection=super::engine::SynthesisSelection::for_instance(multithread)?;
        Self::with_selection(analysis,userdata,reader,profile,selection)
    }
    fn with_selection(analysis:&[u8],userdata:&[u8],reader:ConditionReader,
        profile:crate::prepare_quality::PrepareProfile,selection:super::engine::SynthesisSelection)->Result<Self> {
        Ok(Self {framing:FixedHopStream::new(240)?,source:SourceProducer::with_head(analysis,profile.pitch_head())?,

            engine:MorphEngine::new(analysis,userdata,profile,1,selection)?,reader,
            tone:ToneConditionProvider {generator:ToneGenerator::from_dnni(analysis)?,cached:None},
            input_filters:SourceInputFilters::new_reference_48k(),
            levels:AudioLevels::new_source_defaults(),source_frames:Vec::new(),
            pending:VecDeque::new(),next_ordinal:0,pitch_observation:Default::default()})
    }

    pub(crate) fn prepared_count_plan(&self)->super::count_plan::Query<super::count_plan::PreparedCountPlan> {
        use super::count_plan::{self as count,Unavailable,Unsupported,PreparedCountPlan};
        if self.framing.hop()!=count::COUNT_HOP as usize {
            return Err(Unavailable::Unsupported(Unsupported::UnknownTopology));
        }
        let p=self.framing.progress();
        count::cold(p.accepted_input_frames!=0 || p.model_hop_attempts!=0 || p.model_hop_completed!=0
            || p.emitted_output_frames!=0 || self.framing.buffered_input_frames()!=0
            || self.next_ordinal!=0 || !self.pending.is_empty() || !self.source_frames.is_empty(),
            self.framing.failure().is_some())?;
        let head=self.source.cold_count_inputs()?;
        PreparedCountPlan::from_prepared_head(head,self.engine.prepared_count_inputs()?)
    }
    pub(crate) fn pitch_sample(&self)->crate::pitch_display::Observation {self.pitch_observation.sample()}
    pub(crate) fn set_pitch_source(&mut self,epoch:u64,channel:u32,bus:u64) {
        self.pitch_observation.change(epoch,channel,bus,self.framing.buffered_input_frames()!=0);
    }
    pub(crate) fn process(&mut self,input:&[f32],output:&mut Vec<f32>,gain:&mut InputGain)->Result<()> {
        let Self {framing,source,engine,tone,input_filters,levels,source_frames,reader,pending,next_ordinal,pitch_observation}=self;
        let result:Result<()>=(|| {
        let pipelined=engine.pipeline_active();
        if pipelined {engine.pipeline_idle()?;}
        let mut remaining=if pipelined {pipeline_complete_hops(framing.buffered_input_frames(),input.len())?} else {0};

        let input_db=if input.is_empty() {0.} else {
            reader.poll_shared().ok_or(crate::current_condition::PROCESSING_REQUIRES_CONDITION)?.pregain_db
        };
        framing.process_with_input(input,output,|part,buffer| {
            gain.append(part,buffer,input_db)
        },|hop,rendered| {
            let condition=reader.poll_shared().ok_or(crate::current_condition::PROCESSING_REQUIRES_CONDITION)?;
            let mut signal:[f32;240]=std::array::from_fn(|i|hop[i]);

            let target=tone.resolve(&condition.target);
            let target128=target?;
            let prepared:Result<()>=(|| {input_filters.process(&mut signal)?;levels.input_block(&mut signal)?;Ok(())})();
            prepared?;
            pending.push_back(InputHop {ordinal:*next_ordinal,condition,target128,
                source_tag:pitch_observation.begin_hop()});
            *next_ordinal=next_ordinal.checked_add(1).ok_or("Live source ordinal overflow")?;
            source_frames.clear();
            let produced=source.process(&signal,source_frames);
            if let Err(error)=produced {
                source_frames.clear();return Err(error);
            }

            for frame in source_frames.drain(..) {
                let input=pending.front().ok_or("Missing original input for live source features")?;
                if frame.ordinal!=input.ordinal {return Err("Live input/source-feature ordinal mismatch".into());}
                let pitch=frame.pitch;
                let observed=levels.observe_pitch(pitch.voiced);
                observed?;
                let pitch=adjust_pitch(pitch,input.condition.pitch_semitones)?;
                let begin=rendered.len();
                let features=SourceFeatures {large321:&frame.large321,small15:&frame.small15,
                    pitch,auxiliary:frame.auxiliary,target128:&input.target128};
                let generated=if pipelined {engine.process_pipelined(frame.ordinal,features,rendered)}
                    else {engine.process(features,rendered)};
                generated?;
                pitch_observation.observe(input.source_tag,frame.ordinal,pitch);
                let adjusted=levels.output_block(&mut rendered[begin..]);
                adjusted?;
                drop(pending.pop_front());
            }
            if pipelined {
                pipeline_finish_hop(&mut remaining,|| {
                    let begin=rendered.len();
                    engine.flush_pipeline(rendered)?;
                    levels.output_block(&mut rendered[begin..])?;
                    engine.pipeline_idle()
                })?;
            }
            Ok(())
        })?;
        if pipelined {
            if remaining!=0 {return Err("Synthesis pipeline hop countdown incomplete".into());}
            engine.pipeline_idle()?;
        }
        Ok(())
        })();
        if let Err(error)=&result {engine.cancel_pipeline(error);}
        result
    }
}

fn pipeline_complete_hops(buffered:usize,incoming:usize)->Result<usize> {
    if buffered>=240 {return Err("Invalid healthy pipeline framing remainder".into());}
    Ok(buffered.checked_add(incoming).ok_or("Pipeline input extent overflow")?/240)
}
fn pipeline_finish_hop(remaining:&mut usize,finish:impl FnOnce()->Result<()>)->Result<()> {
    *remaining=remaining.checked_sub(1).ok_or("Unexpected pipeline hop callback")?;
    if *remaining==0 {finish()?;}
    Ok(())
}

fn adjust_pitch(mut pitch:crate::pitch_selected_head::PitchRecord,shift:f32)
    ->Result<crate::pitch_selected_head::PitchRecord> {
    if !shift.is_finite() {return Err("Invalid pitch control".into());}

    if pitch.valid {pitch.value+=shift*(std::f32::consts::LN_2/12.);}
    else {pitch.value*=2f32.powf(shift/12.);}
    Ok(pitch)
}

