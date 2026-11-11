use crate::{Result,model::ModelTree,pitch_selected_head::PitchRecord,
    prototype_synthesis_networks::SynthesisNetworks};
use super::{decoder::{Decoder,DecoderFrame},features::{FeatureTransform,SynthesisFamily},
    synthesis::SynthesisBackend};
#[path="morph_synthesis_worker.rs"]
mod synthesis_worker;
pub(super) use synthesis_worker::Selection as SynthesisSelection;
use synthesis_worker::{Pipeline,Prepared,Row};

pub(crate) struct SourceFeatures<'a> {
    pub large321:&'a [f32], pub small15:&'a [f32],
    pub pitch:PitchRecord, pub auxiliary:f32, pub target128:&'a [f32],
}

pub(crate) struct MorphEngine {
    decoder:Decoder, synthesis:Option<SynthesisBackend>,pipeline:Option<Pipeline>,
    feature_transform:FeatureTransform,
    synthesis_rows:Vec<Row>,submitted:u64,
    decoded_frames:Vec<DecoderFrame>,
    accepted:u64,
    decoded:u64,failure:Option<String>,
    count_inputs:super::count_plan::Query<super::count_plan::EngineCounts>,
}
impl MorphEngine {
    pub(crate) fn new(analysis:&[u8],userdata:&[u8],profile:crate::prepare_quality::PrepareProfile,seed:u32,
        selection:SynthesisSelection)->Result<Self> {

        let categorical_state=0x853c_49e6_748f_ea9b_u64;
        let decoder=Decoder::from_dnni(analysis,profile.decoder_child(),categorical_state)?;
        let family=profile.synthesis_family();
        let feature_transform=FeatureTransform::from_userdata_for_family(userdata,family)?;
        let synthesis=match family {
            SynthesisFamily::Primary=>{
                let tree=ModelTree::parse(userdata)?;
                let config=tree.root.children.first().ok_or("Missing synthesis root configuration")?.data;
                if config.len()!=18 {return Err("Synthesis root configuration extent differs".into());}
                if config[12]!=0 {return Err("Connected synthesis requires the selected no-premodule graph".into());}
                let gain=f32::from_le_bytes(config[4..8].try_into().unwrap());
                let offset=f32::from_le_bytes(config[14..18].try_into().unwrap());
                let noise=match config[13] {0=>false,1=>true,_=>return Err("Invalid synthesis subgroup flag".into())};
                SynthesisBackend::primary(SynthesisNetworks::from_userdata(userdata)?,noise,seed,gain,offset)?
            }
            SynthesisFamily::Alternate=>SynthesisBackend::alternate(userdata,seed)?,
        };
        let selection=selection.avoid_nested(synthesis.uses_inner_worker())?;

        let count_inputs=(|| {
            let (coefficient_right_rows,policy_version)=synthesis.prepared_count_inputs()?;
            Ok(super::count_plan::EngineCounts {decoder:decoder.prepared_count_inputs()?,
                coefficient_right_rows,policy_version})
        })();
        let (synthesis,pipeline)=match Pipeline::prepare(synthesis,selection)? {
            Prepared::Serial(owner)=>(Some(owner),None),
            Prepared::Worker(worker)=>(None,Some(worker)),
        };
        Ok(Self {decoder,feature_transform,synthesis,pipeline,count_inputs,
            synthesis_rows:Vec::new(),submitted:0,
            decoded_frames:Vec::new(),accepted:0,decoded:0,failure:None})
    }
    pub(crate) fn prepared_count_inputs(&self)->super::count_plan::Query<super::count_plan::EngineCounts> {
        super::count_plan::cold(self.accepted!=0 || self.decoded!=0 || self.submitted!=0
            || !self.synthesis_rows.is_empty() || !self.decoded_frames.is_empty(),self.failure.is_some())?;
        self.count_inputs
    }
    pub(crate) fn process(&mut self,features:SourceFeatures<'_>,
        output:&mut Vec<f32>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        if self.pipeline.is_some() {return Err("Use the pipeline source-frame entry for worker execution".into());}
        let result=self.advance(features,output);
        if let Err(error)=&result {self.failure=Some(error.clone());} result
    }
    fn advance(&mut self,features:SourceFeatures<'_>,output:&mut Vec<f32>)->Result<()> {
        if !features.auxiliary.is_finite() {return Err("Nonfinite selected-head auxiliary value".into());}
        self.accepted=self.accepted.checked_add(1).ok_or("Morph source ordinal overflow")?;

        let auxiliary=[if features.pitch.voiced {1.} else {0.},features.auxiliary];
        self.decoded_frames.clear();
        let decoder_result=self.decoder.process(features.large321,features.small15,&auxiliary,features.target128,
            features.pitch.value,features.pitch.valid,&mut self.decoded_frames);
        if let Err(error)=decoder_result {
            self.decoded_frames.clear();return Err(error);
        }

        for frame in self.decoded_frames.drain(..) {
            if self.decoded!=frame.ordinal {return Err("Decoder feature ordinal mismatch".into());}
            let coefficients_result=self.feature_transform.transform(&frame.feature72);
            let coefficients=coefficients_result?;

            let synthesis_result=self.synthesis.as_mut().ok_or("Missing serial synthesis owner")?
                .process(&frame.feature72,&coefficients,output);
            synthesis_result?;
            self.decoded+=1;
        }
        Ok(())
    }
    pub(crate) fn pipeline_active(&self)->bool {self.pipeline.is_some()}
    pub(crate) fn pipeline_idle(&self)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        if self.pipeline.as_ref().is_some_and(|p|p.has_pending()) {
            return Err("Healthy host return requires no pending synthesis".into());
        }
        Ok(())
    }

    pub(crate) fn cancel_pipeline(&mut self,error:&str) {
        if let Some(pipeline)=&mut self.pipeline {
            pipeline.cancel();
            if self.failure.is_none() {self.failure=Some(error.to_owned());}
        }
    }
    pub(crate) fn flush_pipeline(&mut self,output:&mut Vec<f32>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.finish_pipeline(output);
        if let Err(error)=&result {self.cancel_pipeline(error);}
        result
    }
    fn finish_pipeline(&mut self,output:&mut Vec<f32>)->Result<()> {
        let pipeline=self.pipeline.as_mut().ok_or("Missing pipeline synthesis owner")?;
        if let Some(finished)=pipeline.finish(output)? {
            if finished.first!=self.decoded || finished.end>self.submitted || finished.trigger>=self.accepted {
                return Err("Synthesis successful-completion ordinal mismatch".into());
            }
            self.decoded=finished.end;
        }
        Ok(())
    }

    pub(crate) fn process_pipelined(&mut self,trigger:u64,features:SourceFeatures<'_>,
        output:&mut Vec<f32>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance_pipelined(trigger,features,output);
        if let Err(error)=&result {self.cancel_pipeline(error);}
        result
    }
    fn advance_pipelined(&mut self,trigger:u64,features:SourceFeatures<'_>,output:&mut Vec<f32>)->Result<()> {
        if self.pipeline.is_none() || trigger!=self.accepted {
            return Err("Pipeline source group admission differs".into());
        }
        if !features.auxiliary.is_finite() {return Err("Nonfinite selected-head auxiliary value".into());}
        self.accepted=self.accepted.checked_add(1).ok_or("Morph source ordinal overflow")?;
        let auxiliary=[if features.pitch.voiced {1.} else {0.},features.auxiliary];
        self.decoded_frames.clear();
        self.decoder.process(features.large321,features.small15,&auxiliary,features.target128,
            features.pitch.value,features.pitch.valid,&mut self.decoded_frames)?;
        self.synthesis_rows.clear();
        self.synthesis_rows.try_reserve(self.decoded_frames.len()).map_err(|e|e.to_string())?;
        let mut end=self.submitted;
        for frame in self.decoded_frames.drain(..) {
            if frame.ordinal!=end {return Err("Pipeline decoded feature ordinal mismatch".into());}
            let normalized=self.feature_transform.transform(&frame.feature72)?;
            self.synthesis_rows.push(Row {ordinal:frame.ordinal,raw:frame.feature72,normalized});
            end=end.checked_add(1).ok_or("Pipeline submitted ordinal overflow")?;
        }

        self.finish_pipeline(output)?;
        self.pipeline.as_mut().ok_or("Missing pipeline synthesis owner")?
            .submit(trigger,&mut self.synthesis_rows,self.accepted)?;
        self.submitted=end;
        Ok(())
    }
}

