use super::{Batch,GateError};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum NcwnConditionalStage {Head,CheckHead,Residual,CheckResidual,ResidualCount,Gate,CheckGate,Skip,CheckSkip,
    ResidualAvailable,AddResidual,SelectCount,Mix,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct NcwnConditionalProgress {pub stage:NcwnConditionalStage,pub block:usize,pub row:usize,pub skip_block:usize}
pub struct NcwnConditionalWork {
    pub(super) progress:NcwnConditionalProgress,
    pub(super) head:Option<Batch>,pub(super) current:Option<Batch>,pub(super) residual:Option<Batch>,
    pub(super) gated:Option<Batch>,pub(super) skips:Option<Batch>,
    pub(super) popped_residual:Option<Vec<f32>>,pub(super) active_output:Option<Vec<f32>>,
    pub(super) popped_skip:Option<Vec<f32>>,pub(super) scratch_batches:Vec<Batch>,
    pub(super) retired_rows:Vec<Vec<f32>>,pub(super) output:Vec<Vec<f32>>,pub(super) failure:Option<GateError>,
}
impl NcwnConditionalWork {
    pub(super) fn new()->Self {
        Self{progress:NcwnConditionalProgress{stage:NcwnConditionalStage::Head,block:0,row:0,skip_block:0},
            head:None,current:None,residual:None,gated:None,skips:None,popped_residual:None,active_output:None,
            popped_skip:None,scratch_batches:Vec::new(),retired_rows:Vec::new(),output:Vec::new(),failure:None}
    }
    pub fn progress(&self)->NcwnConditionalProgress{self.progress}
    pub fn head(&self)->Option<&Batch>{self.head.as_ref()}
    pub fn current(&self)->Option<&Batch>{self.current.as_ref()}
    pub fn residual(&self)->Option<&Batch>{self.residual.as_ref()}
    pub fn gated(&self)->Option<&Batch>{self.gated.as_ref()}
    pub fn skips(&self)->Option<&Batch>{self.skips.as_ref()}
    pub fn popped_residual(&self)->Option<&[f32]>{self.popped_residual.as_deref()}
    pub fn active_output(&self)->Option<&[f32]>{self.active_output.as_deref()}
    pub fn popped_skip(&self)->Option<&[f32]>{self.popped_skip.as_deref()}
    pub fn scratch_batches(&self)->&[Batch]{&self.scratch_batches}
    pub fn retired_rows(&self)->&[Vec<f32>]{&self.retired_rows}
    pub fn output_prefix(&self)->&[Vec<f32>]{&self.output}
    pub fn failure(&self)->Option<&GateError>{self.failure.as_ref()}
}
