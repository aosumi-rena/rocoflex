use super::{Batch,GateError};
use std::collections::VecDeque;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum GateConditionalStage {Main,Condition,CheckMain,CheckProjected,History,Slot88,Auxiliary,Split,OutputWidth,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct GateConditionalProgress {pub stage:GateConditionalStage,pub row:usize}
pub struct GateConditionalWork {
    pub(super) progress:GateConditionalProgress,
    pub(super) main:Option<Batch>,pub(super) projected:Option<Batch>,
    pub(super) conditions:VecDeque<Vec<f32>>,pub(super) active_condition:Option<Vec<f32>>,
    pub(super) consumed_conditions:Vec<Vec<f32>>,pub(super) active_output:Option<Vec<f32>>,
    pub(super) output:Vec<Vec<f32>>,pub(super) failure:Option<GateError>,
}
impl GateConditionalWork {
    pub(super) fn new()->Self {
        Self{progress:GateConditionalProgress{stage:GateConditionalStage::Main,row:0},main:None,projected:None,
            conditions:VecDeque::new(),active_condition:None,consumed_conditions:Vec::new(),active_output:None,
            output:Vec::new(),failure:None}
    }
    pub fn progress(&self)->GateConditionalProgress{self.progress}
    pub fn main(&self)->Option<&Batch>{self.main.as_ref()}
    pub fn projected(&self)->Option<&Batch>{self.projected.as_ref()}
    pub fn remaining_conditions(&self)->&VecDeque<Vec<f32>>{&self.conditions}
    pub fn active_condition(&self)->Option<&[f32]>{self.active_condition.as_deref()}
    pub fn consumed_conditions(&self)->&[Vec<f32>]{&self.consumed_conditions}
    pub fn active_output(&self)->Option<&[f32]>{self.active_output.as_deref()}
    pub fn output_prefix(&self)->&[Vec<f32>]{&self.output}
    pub fn failure(&self)->Option<&GateError>{self.failure.as_ref()}
}
