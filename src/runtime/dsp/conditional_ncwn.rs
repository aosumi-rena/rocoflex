use std::collections::VecDeque;
use crate::conditional_gate::{Auxiliary,Batch,BatchAuxiliary,BorrowedProjection,ConditionalGate,GenericGate,GateError,Projection,VectorKernels};
use crate::row_arithmetic::add_condition;
#[path="conditional_ncwn_work.rs"]
mod conditional_work;
pub use self::conditional_work::{NcwnConditionalWork,NcwnConditionalProgress,NcwnConditionalStage};

pub trait ConditionedTransform {
    fn reset(&mut self) -> Result<(),GateError>;
    fn forward(&mut self,input:&Batch,condition:&Batch)->Result<Batch,GateError>;
}

pub trait BorrowedConditionedTransform:ConditionedTransform {
    fn forward_borrowed(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch)->Result<Batch,GateError>;
}
pub trait UnconditionedTransform {
    fn forward_without_condition(&mut self,input:&Batch)->Result<Batch,GateError>;
}
impl<P:Projection,C:Projection,A:BatchAuxiliary,K:VectorKernels> UnconditionedTransform for ConditionalGate<P,C,A,K>{
    fn forward_without_condition(&mut self,input:&Batch)->Result<Batch,GateError>{
        ConditionalGate::forward_without_condition(self,input)
    }
}
impl<P:Projection,A:BatchAuxiliary,K:VectorKernels> UnconditionedTransform for GenericGate<P,A,K>{
    fn forward_without_condition(&mut self,input:&Batch)->Result<Batch,GateError>{self.forward(input)}
}
impl<P:Projection,C:Projection,A:Auxiliary,K:VectorKernels> ConditionedTransform for ConditionalGate<P,C,A,K> {
    fn reset(&mut self)->Result<(),GateError> { ConditionalGate::reset(self) }
    fn forward(&mut self,input:&Batch,condition:&Batch)->Result<Batch,GateError> {
        ConditionalGate::forward(self,input,condition)
    }
}
impl<P:BorrowedProjection,C:Projection,A:Auxiliary,K:VectorKernels> BorrowedConditionedTransform for ConditionalGate<P,C,A,K>{
    fn forward_borrowed(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch)->Result<Batch,GateError>{
        ConditionalGate::forward_borrowed(self,input,condition)
    }
}

#[derive(Debug,Clone,Copy)]
pub struct NcwnConfig {
    pub input_width:usize,
    pub output_width:usize,
    pub condition_width:usize,
}

pub struct Block<G,S,R> {
    pub gate:G,
    pub skip:S,
    pub residual:R,
    residual_rows:VecDeque<Vec<f32>>,
    skip_rows:VecDeque<Vec<f32>>,
    pending_residual_preparation:Option<Batch>,
}
impl<G,S,R> Block<G,S,R> {
    pub fn from_explicit_queues(gate:G,skip:S,residual:R,
        residual_rows:VecDeque<Vec<f32>>,skip_rows:VecDeque<Vec<f32>>,
        pending_residual_preparation:Option<Batch>)->Self {
        Self{gate,skip,residual,residual_rows,skip_rows,pending_residual_preparation}
    }
    pub fn pending_counts(&self)->(usize,usize){(self.residual_rows.len(),self.skip_rows.len())}
    pub fn pending_residual_preparation(&self)->Option<&Batch>{self.pending_residual_preparation.as_ref()}
    pub fn into_explicit_parts(self)->(G,S,R,VecDeque<Vec<f32>>,VecDeque<Vec<f32>>,Option<Batch>){
        (self.gate,self.skip,self.residual,self.residual_rows,self.skip_rows,self.pending_residual_preparation)
    }
    pub fn from_initialized_dependencies(gate:G,skip:S,residual:R)->Self {
        Self {gate,skip,residual,residual_rows:VecDeque::new(),skip_rows:VecDeque::new(),pending_residual_preparation:None}
    }
}

pub struct ConditionalNcwn<G,S,R,H> {
    config:NcwnConfig,
    blocks:Vec<Block<G,S,R>>,
    condition_head:H,
    condition_rows:VecDeque<Vec<f32>>,
    needs_reset:bool,
    pending_conditional_work:Option<NcwnConditionalWork>,
}

pub struct NcwnExplicitState<G,S,R,H> {
    pub config:NcwnConfig,pub blocks:Vec<Block<G,S,R>>,pub condition_head:H,
    pub condition_rows:VecDeque<Vec<f32>>,pub needs_reset:bool,
    pub pending_conditional_work:Option<NcwnConditionalWork>,
}
pub struct NcwnStateRefusal<G,S,R,H> {
    pub state:NcwnExplicitState<G,S,R,H>,pub error:GateError,
}

impl<G:ConditionedTransform,S:Projection,R:Projection,H:Projection> ConditionalNcwn<G,S,R,H> {

    pub fn config(&self)->NcwnConfig{self.config}

    pub fn from_explicit_state(state:NcwnExplicitState<G,S,R,H>)
        ->Result<Self,NcwnStateRefusal<G,S,R,H>> {
        let config=state.config;
        if state.blocks.is_empty()||config.input_width==0||config.output_width==0
            ||config.condition_width==0
            ||[config.input_width,config.output_width,config.condition_width].iter()
                .any(|&width|width>i32::MAX as usize){
            return Err(NcwnStateRefusal{state,error:GateError::UnsupportedConfiguration});
        }
        if !state.needs_reset&&(state.pending_conditional_work.is_some()
            ||state.blocks.iter().any(|b|b.pending_residual_preparation.is_some())){
            return Err(NcwnStateRefusal{state,error:GateError::ResetRequired});
        }
        let wrong=state.condition_rows.iter()
            .chain(state.blocks.iter().flat_map(|block|
                block.residual_rows.iter().chain(&block.skip_rows)))
            .find_map(|row|(row.len()!=config.output_width).then_some(row.len()));
        if let Some(actual)=wrong{
            return Err(NcwnStateRefusal{state,error:GateError::RowWidth{expected:config.output_width,actual}});
        }
        Ok(Self{config,blocks:state.blocks,condition_head:state.condition_head,
            condition_rows:state.condition_rows,needs_reset:state.needs_reset,pending_conditional_work:state.pending_conditional_work})
    }
    pub fn into_explicit_state(self)->NcwnExplicitState<G,S,R,H>{
        NcwnExplicitState{config:self.config,blocks:self.blocks,condition_head:self.condition_head,
            condition_rows:self.condition_rows,needs_reset:self.needs_reset,pending_conditional_work:self.pending_conditional_work}
    }

    pub fn from_initialized_dependencies(config:NcwnConfig,blocks:Vec<Block<G,S,R>>,condition_head:H)
        ->Result<Self,GateError>
    {
        if blocks.is_empty() || config.input_width==0 || config.output_width==0 || config.condition_width==0 {
            return Err(GateError::UnsupportedConfiguration);
        }
        let needs_reset=blocks.iter().any(|b|b.pending_residual_preparation.is_some());
        Ok(Self {config,blocks,condition_head,condition_rows:VecDeque::new(),needs_reset,pending_conditional_work:None})
    }

    pub fn reset(&mut self)->Result<(),GateError> {
        self.needs_reset=true;
        self.pending_conditional_work=None;
        self.condition_rows.clear();
        for block in &mut self.blocks { block.residual_rows.clear();block.skip_rows.clear();block.pending_residual_preparation=None; }

        for block in &mut self.blocks {block.gate.reset()?;}
        for block in &mut self.blocks {block.skip.reset()?;}
        for block in &mut self.blocks {block.residual.reset()?;}
        self.condition_head.reset()?;
        self.needs_reset=false;
        Ok(())
    }

    pub fn pending_conditional_work(&self)->Option<&NcwnConditionalWork>{self.pending_conditional_work.as_ref()}

    pub fn forward(&mut self,input:&Batch,condition:&Batch)->Result<Batch,GateError> {
        self.forward_with_root(input,condition,|residual|residual.forward(input),|gate|gate.forward(input,condition))
    }

    pub fn forward_borrowed(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch)
        ->Result<Batch,GateError> where G:BorrowedConditionedTransform,R:BorrowedProjection {
        self.forward_with_root(input,condition,|residual|residual.forward_borrowed(input),
            |gate|gate.forward_borrowed(input,condition))
    }

    fn forward_with_root<FResidual,FGate>(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch,
        mut first_residual:FResidual,mut first_gate:FGate)->Result<Batch,GateError>
        where FResidual:FnMut(&mut R)->Result<Batch,GateError>,FGate:FnMut(&mut G)->Result<Batch,GateError> {
        if self.needs_reset {return Err(GateError::ResetRequired);}
        crate::conditional_gate::check_read_rows(input,self.config.input_width)?;
        check(&condition.rows,self.config.condition_width)?;
        self.needs_reset=true;
        self.pending_conditional_work=Some(NcwnConditionalWork::new());
        let result=(||{
            let work=self.pending_conditional_work.as_mut().expect("owned conditional Ncwn work");
            work.progress.stage=NcwnConditionalStage::Head;
            work.head=Some(self.condition_head.forward(condition)?);
            work.progress.stage=NcwnConditionalStage::CheckHead;
            check(&work.head.as_ref().expect("returned head owner").rows,self.config.output_width)?;
            self.condition_rows.extend(std::mem::take(&mut work.head.as_mut().expect("checked head owner").rows));
            for (index,block) in self.blocks.iter_mut().enumerate() {
                work.progress.block=index;work.progress.row=0;work.progress.skip_block=0;
                work.progress.stage=NcwnConditionalStage::Residual;
                work.residual=Some(if let Some(current)=work.current.as_ref(){block.residual.forward(current)?}
                    else{first_residual(&mut block.residual)?});
                work.progress.stage=NcwnConditionalStage::CheckResidual;
                if let Err(error)=check(&work.residual.as_ref().expect("returned residual owner").rows,self.config.output_width){
                    block.pending_residual_preparation=work.residual.take();return Err(error);
                }
                work.progress.stage=NcwnConditionalStage::ResidualCount;
                let required=work.current.as_ref().map_or_else(||input.row_count(),|current|current.rows.len());
                let available=work.residual.as_ref().expect("returned residual owner").rows.len();
                if available<required{
                    block.pending_residual_preparation=work.residual.take();
                    return Err(GateError::ResidualPreparationRequired{required,available});
                }
                block.residual_rows.extend(std::mem::take(&mut work.residual.as_mut().expect("checked residual owner").rows));
                work.progress.stage=NcwnConditionalStage::Gate;
                work.gated=Some(if let Some(current)=work.current.as_ref(){block.gate.forward(current,condition)?}
                    else{first_gate(&mut block.gate)?});
                work.progress.stage=NcwnConditionalStage::CheckGate;
                check(&work.gated.as_ref().expect("returned gated owner").rows,self.config.output_width)?;

                work.progress.stage=NcwnConditionalStage::Skip;
                work.skips=Some(block.skip.forward(work.gated.as_ref().expect("checked gated owner"))?);
                work.progress.stage=NcwnConditionalStage::CheckSkip;
                check(&work.skips.as_ref().expect("returned skips owner").rows,self.config.output_width)?;
                block.skip_rows.extend(std::mem::take(&mut work.skips.as_mut().expect("checked skips owner").rows));
                work.progress.stage=NcwnConditionalStage::ResidualAvailable;
                let required=work.gated.as_ref().expect("retained gated owner").rows.len();
                if block.residual_rows.len()<required{
                    return Err(GateError::MissingResidual{required,available:block.residual_rows.len()});
                }
                work.progress.stage=NcwnConditionalStage::AddResidual;
                for (row_index,row) in work.gated.as_mut().expect("retained gated owner").rows.iter_mut().enumerate(){
                    work.progress.row=row_index;
                    work.popped_residual=Some(block.residual_rows.pop_front().expect("admitted residual count"));
                    add_condition(work.popped_residual.as_deref().expect("popped residual owner"),row)?;
                    work.retired_rows.push(work.popped_residual.take().expect("completed residual owner"));
                    work.progress.row=row_index+1;
                }
                if let Some(previous)=work.current.take(){work.scratch_batches.push(previous);}
                work.current=work.gated.take();
                work.scratch_batches.push(work.residual.take().expect("moved residual header"));
                work.scratch_batches.push(work.skips.take().expect("moved skips header"));
                work.progress.block=index+1;
            }
            work.progress.stage=NcwnConditionalStage::SelectCount;
            let count=self.blocks.iter().map(|block|block.skip_rows.len()).fold(self.condition_rows.len(),usize::min);
            work.output=Vec::with_capacity(count);
            for row in 0..count{
                work.progress.stage=NcwnConditionalStage::Mix;work.progress.row=row;work.progress.skip_block=0;

                work.active_output=Some(self.condition_rows.pop_front().expect("selected head count"));
                for (index,block) in self.blocks.iter_mut().enumerate(){
                    work.progress.skip_block=index;
                    work.popped_skip=Some(block.skip_rows.pop_front().expect("selected skip count"));
                    add_condition(work.popped_skip.as_deref().expect("popped skip owner"),
                        work.active_output.as_deref_mut().expect("active output owner"))?;
                    work.retired_rows.push(work.popped_skip.take().expect("completed skip owner"));
                    work.progress.skip_block=index+1;
                }
                work.output.push(work.active_output.take().expect("completed output owner"));
                work.progress.row=row+1;
            }
            work.progress.stage=NcwnConditionalStage::Complete;
            Ok(Batch{rows:std::mem::take(&mut work.output),status:input.source_status()})
        })();
        match result {
            Ok(output)=>{
                self.pending_conditional_work=None;self.needs_reset=false;Ok(output)
            }
            Err(error)=>{
                self.pending_conditional_work.as_mut().expect("failed conditional Ncwn work").failure=Some(error.clone());Err(error)
            }
        }
    }
}

impl<G:ConditionedTransform+UnconditionedTransform,S:Projection,R:Projection,H:Projection> ConditionalNcwn<G,S,R,H>{

    pub fn forward_without_condition(&mut self,input:&Batch,
        output_factory:crate::backend_factory::SourceFloatFactory)->Result<Batch,GateError>{
        if self.needs_reset{return Err(GateError::ResetRequired);}
        check(&input.rows,self.config.input_width)?;
        if output_factory.vector_vtable_rva()!=crate::backend_factory::FLOAT_VECTOR_VTABLE_RVA{
            return Err(GateError::Dependency("generic output requires the bound FloatVector factory"));
        }
        self.needs_reset=true;
        let mut current=input.clone();
        for block in &mut self.blocks{
            let residual=block.residual.forward(&current)?;
            if let Err(error)=check(&residual.rows,self.config.output_width){
                block.pending_residual_preparation=Some(residual);return Err(error);
            }
            if residual.rows.len()<current.rows.len(){
                let error=GateError::ResidualPreparationRequired{required:current.rows.len(),available:residual.rows.len()};
                block.pending_residual_preparation=Some(residual);return Err(error);
            }
            block.residual_rows.extend(residual.rows);
            let mut gated=block.gate.forward_without_condition(&current)?;
            check(&gated.rows,self.config.output_width)?;
            let skips=block.skip.forward(&gated)?;
            check(&skips.rows,self.config.output_width)?;
            block.skip_rows.extend(skips.rows);
            if block.residual_rows.len()<gated.rows.len(){
                return Err(GateError::MissingResidual{required:gated.rows.len(),available:block.residual_rows.len()});
            }
            for row in &mut gated.rows{
                let residual=block.residual_rows.pop_front().expect("admitted residual count");
                add_condition(&residual,row)?;
            }
            current=gated;
        }
        let count=self.blocks.iter().map(|b|b.skip_rows.len()).min()
            .ok_or(GateError::UnsupportedConfiguration)?;
        let mut rows=Vec::with_capacity(count);
        for _ in 0..count{

            let mut output=vec![0.0_f32;self.config.output_width];
            for block in &mut self.blocks{
                let skip=block.skip_rows.pop_front().expect("admitted skip count");
                add_condition(&skip,&mut output)?;
            }
            rows.push(output);
        }
        self.needs_reset=false;
        Ok(Batch{rows,status:input.status})
    }
}

fn check(rows:&[Vec<f32>],expected:usize)->Result<(),GateError> {
    for row in rows {
        if row.len()!=expected {return Err(GateError::RowWidth {expected,actual:row.len()});}
    }
    Ok(())
}
