use std::{collections::VecDeque, path::Path};
use crate::{Result, current_condition::{ConditionBus, ConditionReader}};

#[path = "morph_count_plan.rs"]
mod count_plan;
#[path = "morph_network_stream.rs"]
mod network_stream;
#[path = "morph_vocoder.rs"]
mod vocoder;
#[path = "morph_coefficients.rs"]
mod coefficients;
#[path = "morph_synthesis.rs"]
mod synthesis;
#[path = "morph_decoder.rs"]
mod decoder;
#[path = "morph_engine.rs"]
mod engine;
#[path = "morph_source.rs"]
mod source;
#[path = "morph_features.rs"]
mod features;
#[path = "morph_excitation.rs"]
mod excitation;
#[path = "morph_generator.rs"]
mod generator;
#[path = "morph_generator_math.rs"]
mod generator_math;
#[path = "morph_generator_table.rs"]
mod generator_table;
#[path = "morph_live.rs"]
mod live;
#[path="morph_input_gain.rs"]
pub(crate) mod input_gain;
use input_gain::InputGain;
#[path="morph_input_meter.rs"]
pub(crate) mod input_meter;

pub(crate) fn inspect_analysis_model(bytes: &[u8]) -> Result<()> {
    let tree = crate::model::ModelTree::parse(bytes)?;
    if tree.root.children.len() != 9 {
        return Err("The supplied analysis model does not contain the expected nine analysis branches.".into());
    }
    drop(tree);
    for profile in [crate::prepare_quality::PrepareProfile::Highest,
        crate::prepare_quality::PrepareProfile::Higher, crate::prepare_quality::PrepareProfile::Lower] {
        drop(source::SourceProducer::with_head(bytes, profile.pitch_head())?);
        drop(decoder::Decoder::from_dnni(bytes, profile.decoder_child(), 0x853c_49e6_748f_ea9b)?);
    }
    drop(decoder::DecoderOutput::from_dnni(bytes)?);
    Ok(())
}

pub(crate) fn inspect_morphing_model(bytes: &[u8]) -> Result<()> {
    let tree = crate::model::ModelTree::parse(bytes)?;
    let config = tree.root.children.first().ok_or("Missing synthesis root configuration")?.data;
    if config.len() != 18 { return Err("Synthesis root configuration extent differs".into()); }
    if config[12] != 0 { return Err("Connected synthesis requires the selected no-premodule graph".into()); }
    let gain = f32::from_le_bytes(config[4..8].try_into().unwrap());
    let offset = f32::from_le_bytes(config[14..18].try_into().unwrap());
    let noise = match config[13] { 0 => false, 1 => true, _ => return Err("Invalid synthesis subgroup flag".into()) };
    drop(features::FeatureTransform::from_userdata(bytes)?);
    drop(synthesis::WaveformSynthesis::new(
        crate::prototype_synthesis_networks::SynthesisNetworks::from_userdata(bytes)?,
        noise, 1, gain, offset)?);
    Ok(())
}

type MonoRender=Box<dyn FnMut(&[f32],&mut Vec<f32>)->Result<()>>;

pub struct MorphProcessor {inner:crate::live_audio::MonoProcessor<MonoRender>}
impl MorphProcessor {
    fn from_graph(mut graph:live::LiveGraph,input_channel:usize,mut meter:input_meter::Writer)->Self {
        let mut gain=InputGain::new();
        let render:MonoRender=Box::new(move |input,output| {
            let pending=meter.begin_block(input,&gain);
            let result=graph.process(input,output,&mut gain);
            if result.is_ok() {meter.finish_block_with_pitch(pending,graph.pitch_sample());}
            result
        });
        Self {inner:crate::live_audio::MonoProcessor::new(input_channel,render)}
    }
}

impl crate::wasapi_duplex::Processor for MorphProcessor {
    type Error = String;
    fn process(&mut self, input: &crate::wasapi_capture::Packet,
        format: crate::wasapi_duplex::Format, output: &mut Vec<f32>) -> Result<()>
    {
        crate::wasapi_duplex::Processor::process(&mut self.inner,input,format,output)
    }
}

pub(crate) fn build_processor(analysis_model: &Path, synthesis_model: &Path,
    reader: ConditionReader, sample_rate: u32, input_channel: usize,
    meter:input_meter::Writer) -> Result<MorphProcessor>
{
    build_processor_with_profile(analysis_model,synthesis_model,reader,sample_rate,input_channel,meter,
        crate::prepare_quality::PrepareProfile::Highest)
}

pub(crate) fn build_processor_with_profile(analysis_model:&Path,synthesis_model:&Path,
    reader:ConditionReader,sample_rate:u32,input_channel:usize,meter:input_meter::Writer,
    profile:crate::prepare_quality::PrepareProfile)->Result<MorphProcessor> {
    build_processor_inner(analysis_model,synthesis_model,reader,sample_rate,input_channel,meter,profile,None)
}

pub(crate) fn build_processor_with_instance_policy(analysis_model:&Path,synthesis_model:&Path,
    reader:ConditionReader,sample_rate:u32,input_channel:usize,meter:input_meter::Writer,
    profile:crate::prepare_quality::PrepareProfile,multithread:bool)->Result<MorphProcessor> {
    build_processor_inner(analysis_model,synthesis_model,reader,sample_rate,input_channel,meter,profile,Some(multithread))
}

fn build_processor_inner(analysis_model:&Path,synthesis_model:&Path,
    mut reader:ConditionReader,sample_rate:u32,input_channel:usize,meter:input_meter::Writer,
    profile:crate::prepare_quality::PrepareProfile,multithread:Option<bool>)->Result<MorphProcessor> {
    if sample_rate != 48_000 {
        return Err("The synthesis graph currently requires 48000 Hz input and output.".into());
    }
    if input_channel>1 {
        return Err("Choose the left or right channel of the stereo input transport.".into());
    }
    let (analysis_bytes,synthesis_bytes)=read_models(analysis_model,synthesis_model)?;
    if reader.poll().is_none() {
        return Err("Choose an enabled voice on the canvas before starting audio.".into());
    }
    let graph = match multithread {
        Some(value)=>live::LiveGraph::with_instance_policy(&analysis_bytes,&synthesis_bytes,reader,profile,value)?,
        None=>live::LiveGraph::with_profile(&analysis_bytes,&synthesis_bytes,reader,profile)?,
    };
    Ok(MorphProcessor::from_graph(graph,input_channel,meter))
}

fn read_models(analysis_model:&Path,synthesis_model:&Path)->Result<(Vec<u8>,Vec<u8>)> {
    let analysis_bytes = crate::model::read_bytes(analysis_model, 512 * 1024 * 1024)?;
    let analysis = crate::model::ModelTree::parse(&analysis_bytes)
        .map_err(|error| format!("Analysis model: {error}"))?;
    if analysis.root.children.len() != 9 {
        return Err("The supplied analysis model does not contain the expected nine analysis branches.".into());
    }
    drop(analysis);
    let synthesis_bytes = crate::model::read_bytes(synthesis_model, 256 * 1024 * 1024)?;
    Ok((analysis_bytes,synthesis_bytes))
}

pub const PLUGIN_LATENCY_SAMPLES:u32=25*240;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum LatencyPreparation {Fixed6000,PreparedCount}
impl TryFrom<u32> for LatencyPreparation {
    type Error=String;
    fn try_from(value:u32)->Result<Self> {match value {
        0=>Ok(Self::Fixed6000),1=>Ok(Self::PreparedCount),
        _=>Err("Invalid private latency preparation policy".into()),
    }}
}
impl LatencyPreparation {
    fn select(self,query:impl FnOnce()->count_plan::Query<count_plan::PreparedCountPlan>)->Result<PreparedLatency> {
        match self {
            Self::Fixed6000=>Ok(PreparedLatency {delay_samples:PLUGIN_LATENCY_SAMPLES,policy:1}),
            Self::PreparedCount=>PreparedLatency::from_count_plan(query()),
        }
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct PreparedLatency {delay_samples:u32,policy:u32}
impl PreparedLatency {
    pub(crate) fn delay_samples(self)->u32 {self.delay_samples}
    pub(crate) fn policy(self)->u32 {self.policy}
    fn from_count_plan(query:count_plan::Query<count_plan::PreparedCountPlan>)->Result<Self> {
        match query {
            Ok(plan) if matches!(plan.policy_version(),count_plan::COUNT_POLICY_VERSION
                | count_plan::ALTERNATE_COUNT_POLICY_VERSION)=>{
                let delay=u32::try_from(plan.whole_hop_deficit_bound())
                    .map_err(|_|"Prepared publication delay does not fit the plugin ABI")?;
                if delay==0 {return Err("Prepared publication delay must be positive".into());}
                Ok(Self {delay_samples:delay,policy:2})
            }
            Ok(_) | Err(count_plan::Unavailable::Unsupported(_))=>
                Ok(Self {delay_samples:PLUGIN_LATENCY_SAMPLES,policy:1}),
            Err(count_plan::Unavailable::Invalid(_))=>Err("Prepared publication count metadata is invalid".into()),
        }
    }
}
fn latency_buffers(latency:PreparedLatency,max_block:usize)->Result<(VecDeque<f32>,Vec<f32>)> {
    if !(1..=65_536).contains(&max_block) || latency.delay_samples==0
        || !matches!(latency.policy,1|2) || (latency.policy==1 && latency.delay_samples!=PLUGIN_LATENCY_SAMPLES) {
        return Err("Invalid prepared plugin delay or buffer dimensions".into());
    }
    let delay=usize::try_from(latency.delay_samples).map_err(|_|"Prepared delay extent is not representable")?;
    let rendered_capacity=max_block.checked_add(240).ok_or("Plugin rendered capacity overflow")?;
    let queue_capacity=delay.checked_add(rendered_capacity).ok_or("Plugin FIFO capacity overflow")?;
    let mut queued=VecDeque::new();
    queued.try_reserve_exact(queue_capacity).map_err(|e|format!("Cannot reserve plugin latency FIFO: {e}"))?;
    queued.resize(delay,0.);
    let mut rendered=Vec::new();
    rendered.try_reserve_exact(rendered_capacity).map_err(|e|format!("Cannot reserve plugin output scratch: {e}"))?;
    Ok((queued,rendered))
}

pub(crate) struct PluginProcessor {
    graph:live::LiveGraph,
    availability:ConditionReader,
    queued:VecDeque<f32>,
    rendered:Vec<f32>,
    max_block_frames:usize,
    prepared_latency:PreparedLatency,
    begun:bool,
    failure:Option<String>,
}

impl PluginProcessor {
    pub(crate) fn new(analysis_model:&Path,synthesis_model:&Path,bus:&ConditionBus,
        sample_rate:u32,max_block_frames:usize)->Result<Self>
    {
        Self::with_profile(analysis_model,synthesis_model,bus,sample_rate,max_block_frames,
            crate::prepare_quality::PrepareProfile::Highest)
    }
    pub(crate) fn with_profile(analysis_model:&Path,synthesis_model:&Path,bus:&ConditionBus,
        sample_rate:u32,max_block_frames:usize,profile:crate::prepare_quality::PrepareProfile)->Result<Self> {
        Self::with_profile_and_latency_policy(analysis_model,synthesis_model,bus,sample_rate,max_block_frames,
            profile,LatencyPreparation::Fixed6000)
    }
    pub(crate) fn with_profile_and_latency_policy(analysis_model:&Path,synthesis_model:&Path,bus:&ConditionBus,
        sample_rate:u32,max_block_frames:usize,profile:crate::prepare_quality::PrepareProfile,
        latency_policy:LatencyPreparation)->Result<Self> {
        Self::prepare_inner(analysis_model,synthesis_model,bus,sample_rate,max_block_frames,profile,latency_policy,None)
    }
    pub(crate) fn with_instance_policy(analysis_model:&Path,synthesis_model:&Path,bus:&ConditionBus,
        sample_rate:u32,max_block_frames:usize,profile:crate::prepare_quality::PrepareProfile,
        latency_policy:LatencyPreparation,multithread:bool)->Result<Self> {
        Self::prepare_inner(analysis_model,synthesis_model,bus,sample_rate,max_block_frames,profile,latency_policy,Some(multithread))
    }
    fn prepare_inner(analysis_model:&Path,synthesis_model:&Path,bus:&ConditionBus,
        sample_rate:u32,max_block_frames:usize,profile:crate::prepare_quality::PrepareProfile,
        latency_policy:LatencyPreparation,multithread:Option<bool>)->Result<Self> {
        if sample_rate!=48_000 {return Err("The plugin requires 48000 Hz audio.".into());}
        if !(1..=65_536).contains(&max_block_frames) {
            return Err("Plugin maximum block size must be in 1..=65536.".into());
        }
        let (analysis,userdata)=read_models(analysis_model,synthesis_model)?;
        let graph=match multithread {
            Some(value)=>live::LiveGraph::with_instance_policy(&analysis,&userdata,bus.reader(),profile,value)?,
            None=>live::LiveGraph::with_profile(&analysis,&userdata,bus.reader(),profile)?,
        };

        let prepared_latency=latency_policy.select(||graph.prepared_count_plan())?;
        let (queued,rendered)=latency_buffers(prepared_latency,max_block_frames)?;
        Ok(Self {graph,availability:bus.reader(),queued,rendered,max_block_frames,prepared_latency,
            begun:false,failure:None})
    }

    pub(crate) fn prepared_latency(&self)->PreparedLatency {self.prepared_latency}
    pub(crate) fn pitch_sample(&self)->crate::pitch_display::Observation {self.graph.pitch_sample()}
    pub(crate) fn set_pitch_source(&mut self,epoch:u64,channel:u32,bus:u64) {self.graph.set_pitch_source(epoch,channel,bus);}

    pub(crate) fn has_condition(&mut self)->bool {
        let available=self.availability.poll().is_some();
        if !available && self.begun && self.failure.is_none() {

            self.failure=Some("Plugin target was retired; prepare again before restarting audio.".into());
        }
        available
    }

    pub(crate) fn process(&mut self,input:&[f32],output:&mut[f32],gain:&mut InputGain)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(input,output,gain);
        if let Err(error)=&result {self.failure=Some(error.clone());}
        result
    }

    fn advance(&mut self,input:&[f32],output:&mut[f32],gain:&mut InputGain)->Result<()> {
        if input.len()!=output.len() || input.len()>self.max_block_frames {
            return Err("Plugin audio block exceeds its prepared dimensions.".into());
        }
        if input.iter().any(|sample|!sample.is_finite()) {
            return Err("Plugin input contains nonfinite audio.".into());
        }
        self.begun=true;
        self.rendered.clear();
        self.graph.process(input,&mut self.rendered,gain)?;
        if self.rendered.iter().any(|sample|!sample.is_finite()) {
            return Err("Plugin graph produced nonfinite audio.".into());
        }
        self.queued.extend(self.rendered.drain(..));
        if self.queued.len()<output.len() {
            return Err("Plugin graph exhausted its declared latency buffer; prepare a new instance.".into());
        }
        for sample in output {
            *sample=self.queued.pop_front().ok_or("Plugin output FIFO lost alignment")?;
        }
        Ok(())
    }
}
