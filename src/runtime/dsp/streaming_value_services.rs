use std::collections::VecDeque;
use crate::conditional_gate::GateError;
use crate::matrix_value_traits::{SourceAccumulatingMatrix,GenericFirstTap};
use crate::row_pool_values::{pool_put_plan,pool_cached_take_index,SourceRowGrowthPlan,RowGrowthError};
use crate::projected_conv_binding::{ProjectedConvBinding,ProjectedConvMode};
use crate::streaming_projection::{Services,Pool,Row,RowRefusal};
use crate::row_acquire_values::{self as acquire,AcquireServices};
use crate::row_put_values::{apply_put_plan,PutServices};
pub struct ValuePool{pub width:usize,pub maximum:u64,pub source_capacity:usize,pub cached:Vec<Option<Row>>}
impl ValuePool{

    fn valid(&self,width:usize)->bool{self.width==width&&width!=0&&self.cached.len()<=self.source_capacity&&self.source_capacity<=self.cached.capacity()&&self.cached.iter().flatten().all(|r|r.len()==width)}
}

pub trait RequiredRows{
    type Error;
    fn create_uncleared(&mut self,width:usize,first:bool,second:bool)->Result<Row,Self::Error>;

    fn grow_pool_slots(&mut self,pool:Pool,storage:&mut ValuePool,plan:SourceRowGrowthPlan)->Result<usize,Self::Error>;
    fn delete_row(&mut self,pool:Pool,row:Row)->Result<(),RowRefusal<Self::Error>>;
    fn clear_history_6490(&mut self,history:&mut VecDeque<Option<Row>>)->Result<(),Self::Error>;
    fn construct_pool_30a00(&mut self,pool:Pool,width:usize,initial:u32,maximum:u32)->Result<ValuePool,Self::Error>;
    fn retire_replaced_pool(&mut self,pool:Pool,old:&mut ValuePool)->Result<(),Self::Error>;
}

pub trait RequiredPacked{
    fn forward_333c0(&mut self,input:&[&[f32]],output:&mut[&mut[f32]],stride:i32,dilation:i32,fifth:u32)->Result<(),GateError>;
}
pub enum NoPacked{}
impl RequiredPacked for NoPacked{
    fn forward_333c0(&mut self,_input:&[&[f32]],_output:&mut[&mut[f32]],_stride:i32,_dilation:i32,_fifth:u32)->Result<(),GateError>{match *self{}}
}
pub struct Parts<M,F,P>{pub input_pool:Option<ValuePool>,pub output_pool:Option<ValuePool>,pub matrices:Vec<M>,pub bias:Option<Vec<f32>>,pub rows:F,pub packed:Option<P>}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Admission{Pool,TapCount,Dimensions,Bias,PackedRequired}
pub struct Refusal<M,F,P>{pub parts:Parts<M,F,P>,pub reason:Admission}
pub enum Error<E>{PendingLifecycle,PoolRequired,PoolWidth,PoolGrowth(RowGrowthError),GrowthAcknowledgement,
    RowLifecycle(E),Matrix(GateError),TapIndex,BiasRequired,PackedRequired,PackedArguments,ReplacementState}
pub struct ValueServices<M,F,P>{parts:Parts<M,F,P>,pending_old_pool:Option<(Pool,ValuePool)>,
    rejected_new_pool:Option<(Pool,ValuePool)>,rejected_growth:Option<(Pool,usize,usize)>,
    rejected_created_row:Option<(Pool,Row)>}
pub struct Preserved<M,F,P>{pub parts:Parts<M,F,P>,pub pending_old_pool:Option<(Pool,ValuePool)>,
    pub rejected_new_pool:Option<(Pool,ValuePool)>,pub rejected_growth:Option<(Pool,usize,usize)>,
    pub rejected_created_row:Option<(Pool,Row)>}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> ValueServices<M,F,P>{

    pub fn from_explicit_parts(binding:&ProjectedConvBinding,has_bias:bool,parts:Parts<M,F,P>)->Result<Self,Refusal<M,F,P>>{
        Self::admit_parts(binding,has_bias,parts,false)
    }

    pub fn from_before_reset_parts(binding:&ProjectedConvBinding,has_bias:bool,parts:Parts<M,F,P>)->Result<Self,Refusal<M,F,P>>{
        Self::admit_parts(binding,has_bias,parts,true)
    }
    fn admit_parts(binding:&ProjectedConvBinding,has_bias:bool,parts:Parts<M,F,P>,before_reset:bool)->Result<Self,Refusal<M,F,P>>{
        let input=binding.input_width();let output=binding.output_width();
        let pools=if before_reset{parts.input_pool.is_none()&&parts.output_pool.is_none()}
            else{parts.input_pool.as_ref().is_some_and(|pool|pool.valid(input))
                &&parts.output_pool.as_ref().is_some_and(|pool|pool.valid(output))};
        let reason=if !pools{Some(Admission::Pool)}
            else if parts.matrices.len()!=binding.config().taps as usize{Some(Admission::TapCount)}
            else if parts.matrices.iter().any(|m|m.input_width()!=input||m.output_width()!=output){Some(Admission::Dimensions)}
            else if has_bias!=parts.bias.is_some()||parts.bias.as_ref().is_some_and(|b|b.len()!=output){Some(Admission::Bias)}
            else if binding.mode()==ProjectedConvMode::PackedMatrix1&&parts.packed.is_none(){Some(Admission::PackedRequired)}else{None};
        if let Some(reason)=reason{return Err(Refusal{parts,reason});}
        Ok(Self{parts,pending_old_pool:None,rejected_new_pool:None,rejected_growth:None,rejected_created_row:None})
    }
    pub fn parts(&self)->&Parts<M,F,P>{&self.parts}
    pub fn into_preserved(self)->Preserved<M,F,P>{Preserved{parts:self.parts,pending_old_pool:self.pending_old_pool,rejected_new_pool:self.rejected_new_pool,rejected_growth:self.rejected_growth,rejected_created_row:self.rejected_created_row}}
    fn ready(&self)->Result<(),Error<F::Error>>{
        if self.pending_old_pool.is_some()||self.rejected_new_pool.is_some()||self.rejected_growth.is_some()||self.rejected_created_row.is_some(){Err(Error::PendingLifecycle)}else{Ok(())}
    }
    fn pool(&self,pool:Pool)->Option<&ValuePool>{match pool{Pool::Input=>self.parts.input_pool.as_ref(),Pool::Output=>self.parts.output_pool.as_ref()}}
    fn pool_mut(&mut self,pool:Pool)->Option<&mut ValuePool>{match pool{Pool::Input=>self.parts.input_pool.as_mut(),Pool::Output=>self.parts.output_pool.as_mut()}}
    fn put(&mut self,pool:Pool,donor:Option<Row>)->Result<(),(Error<F::Error>,Option<Row>)>{
        if let Err(e)=self.ready(){return Err((e,donor));}
        let Some(p)=self.pool(pool)else{return Err((Error::PoolRequired,donor));};
        if donor.as_ref().is_some_and(|r|r.len()!=p.width){return Err((Error::PoolWidth,donor));}
        let policy=match pool_put_plan(p.cached.len(),p.source_capacity,p.maximum){Ok(p)=>p,Err(e)=>return Err((Error::PoolGrowth(e),donor))};
        let mut adapter=ValueRowPut{services:self,pool,donor};
        apply_put_plan(policy,&mut adapter)
    }
}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> Services for ValueServices<M,F,P>{
    type Error=Error<F::Error>;
    fn take_row(&mut self,pool:Pool,width:usize)->Result<Row,Self::Error>{
        self.ready()?;if self.pool(pool).ok_or(Error::PoolRequired)?.width!=width{return Err(Error::PoolWidth);}
        let mut work=acquire::AcquireWork::after_admission();
        let mut adapter=ValueRowAcquire{services:self,pool,width,created:None,completed:None};
        loop{match acquire::advance_acquire(&mut work,&mut adapter,usize::MAX){
            Ok(acquire::AcquireAdvance::Paused)=>{},
            Ok(acquire::AcquireAdvance::Complete(_))=>return Ok(adapter.completed.take().expect("completed value row acquire")),
            Err(acquire::AcquireError::Service(error))=>return Err(error),
            Err(acquire::AcquireError::PriorFailure|acquire::AcquireError::AlreadyComplete)=>
                unreachable!("fresh synchronous value row acquire cannot replay"),
        }}
    }
    fn copy_input_slot72(&mut self,input:&[f32],row:&mut[f32])->Result<(),Self::Error>{
        self.ready()?;if input.len()!=row.len(){return Err(Error::PoolWidth);}row.copy_from_slice(input);Ok(())
    }
    fn clear_input_slot48(&mut self,row:&mut[f32])->Result<(),Self::Error>{
        self.ready()?;if row.len()!=self.parts.input_pool.as_ref().ok_or(Error::PoolRequired)?.width{return Err(Error::PoolWidth);}

        row.fill(0.0);Ok(())
    }
    fn return_input_row_30a70(&mut self,row:Row)->Result<(),RowRefusal<Self::Error>>{
        self.put(Pool::Input,Some(row)).map_err(|(reason,row)|RowRefusal{reason,row:row.expect("nonnull input donor retained")})
    }
    fn return_null_output_30a70(&mut self)->Result<(),Self::Error>{self.put(Pool::Output,None).map_err(|(reason,_)|reason)}
    fn delete_output_row(&mut self,row:Row)->Result<(),RowRefusal<Self::Error>>{
        if let Err(reason)=self.ready(){return Err(RowRefusal{reason,row});}
        self.parts.rows.delete_row(Pool::Output,row).map_err(|e|RowRefusal{reason:Error::RowLifecycle(e.reason),row:e.row})
    }
    fn overwrite_and_postprocess(&mut self,tap:usize,input:&[f32],output:&mut[f32])->Result<(),Self::Error>{
        self.ready()?;let m=self.parts.matrices.get_mut(tap).ok_or(Error::TapIndex)?;
        m.overwrite_and_postprocess(input,output).map_err(Error::Matrix)
    }
    fn accumulate_and_postprocess(&mut self,tap:usize,input:&[f32],output:&mut[f32])->Result<(),Self::Error>{
        self.ready()?;let m=self.parts.matrices.get_mut(tap).ok_or(Error::TapIndex)?;
        m.accumulate(input,output).map_err(Error::Matrix)
    }
    fn bias_slot88(&mut self,output:&mut[f32])->Result<(),Self::Error>{
        self.ready()?;let bias=self.parts.bias.as_ref().ok_or(Error::BiasRequired)?;
        crate::row_arithmetic::add_condition(bias,output).map_err(Error::Matrix)
    }
    fn packed_333c0(&mut self,input:&[&[f32]],output:&mut[&mut[f32]],stride:i32,dilation:i32,fifth:u32)->Result<(),Self::Error>{
        self.ready()?;if fifth!=1{return Err(Error::PackedArguments);}
        self.parts.packed.as_mut().ok_or(Error::PackedRequired)?.forward_333c0(input,output,stride,dilation,fifth).map_err(Error::Matrix)
    }
    fn clear_history_6490(&mut self,history:&mut VecDeque<Option<Row>>)->Result<(),Self::Error>{
        self.ready()?;self.parts.rows.clear_history_6490(history).map_err(Error::RowLifecycle)
    }
    fn replace_pool_327d0(&mut self,pool:Pool,width:usize,initial:u32,maximum:u32)->Result<(),Self::Error>{
        self.ready()?;
        let new=self.parts.rows.construct_pool_30a00(pool,width,initial,maximum).map_err(Error::RowLifecycle)?;
        if !new.valid(width)||!new.cached.is_empty()||new.source_capacity!=initial as usize||new.maximum!=u64::from(maximum){
            self.rejected_new_pool=Some((pool,new));return Err(Error::ReplacementState);
        }

        let old=match pool{Pool::Input=>self.parts.input_pool.replace(new),Pool::Output=>self.parts.output_pool.replace(new)};
        if let Some(mut old)=old{
            if let Err(e)=self.parts.rows.retire_replaced_pool(pool,&mut old){self.pending_old_pool=Some((pool,old));return Err(Error::RowLifecycle(e));}

            if !old.cached.is_empty(){self.pending_old_pool=Some((pool,old));return Err(Error::ReplacementState);}
        }
        Ok(())
    }
}

struct ValueRowPut<'a,M,F,P>{services:&'a mut ValueServices<M,F,P>,pool:Pool,donor:Option<Row>}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> PutServices
    for ValueRowPut<'_,M,F,P>{
    type Output=();
    type Error=(Error<F::Error>,Option<Row>);
    fn retire_donor_if_nonnull(&mut self)->Result<(),Self::Error>{
        if let Some(row)=self.donor.take(){
            self.services.parts.rows.delete_row(self.pool,row).map_err(|e|(Error::RowLifecycle(e.reason),Some(e.row)))?;
        }
        Ok(())
    }
    fn append_existing(&mut self)->Result<(),Self::Error>{
        self.services.pool_mut(self.pool).expect("admitted put pool").cached.push(self.donor.take());Ok(())
    }
    fn grow_and_append(&mut self,plan:SourceRowGrowthPlan)->Result<(),Self::Error>{
        let Parts{rows,input_pool,output_pool,..}=&mut self.services.parts;
        let storage=match self.pool{Pool::Input=>input_pool.as_mut(),Pool::Output=>output_pool.as_mut()}.expect("admitted put pool");
        let capacity=match rows.grow_pool_slots(self.pool,storage,plan){
            Ok(n)=>n,Err(e)=>return Err((Error::RowLifecycle(e),self.donor.take())),
        };
        if capacity!=plan.new_capacity()||self.services.pool(self.pool).expect("admitted put pool").cached.capacity()<capacity{
            self.services.rejected_growth=Some((self.pool,plan.new_capacity(),capacity));
            return Err((Error::GrowthAcknowledgement,self.donor.take()));
        }
        let p=self.services.pool_mut(self.pool).expect("admitted put pool");p.source_capacity=capacity;p.cached.push(self.donor.take());Ok(())
    }
}

struct ValueRowAcquire<'a,M,F,P>{
    services:&'a mut ValueServices<M,F,P>,pool:Pool,width:usize,
    created:Option<Row>,completed:Option<Row>,
}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> AcquireServices
    for ValueRowAcquire<'_,M,F,P>{
    type Error=Error<F::Error>;
    fn take_cached(&mut self)->Result<bool,Self::Error>{

        if pool_cached_take_index(self.services.pool(self.pool).expect("admitted acquire pool").cached.len()).is_some(){
            if let Some(row)=self.services.pool_mut(self.pool).expect("admitted acquire pool").cached.pop().expect("same counted slot"){
                self.completed=Some(row);return Ok(true);
            }
        }
        Ok(false)
    }
    fn create_uncleared(&mut self,first:bool,second:bool)->Result<(),Self::Error>{
        self.created=Some(self.services.parts.rows.create_uncleared(self.width,first,second)
            .map_err(Error::RowLifecycle)?);Ok(())
    }
    fn admit_created_and_finish(&mut self)->Result<(),Self::Error>{
        let row=self.created.take().expect("actual created value row");
        if row.len()!=self.width{

            self.services.rejected_created_row=Some((self.pool,row));return Err(Error::PoolWidth);
        }
        self.completed=Some(row);Ok(())
    }
}

pub struct OwnedStreaming<M,F,P>{controller:crate::streaming_projection::StreamingProjectedConv,services:ValueServices<M,F,P>}
pub struct OwnedAdmissionFailure<M,F,P>{pub controller:crate::streaming_projection::StreamingProjectedConv,pub refusal:Refusal<M,F,P>}
pub struct OwnedPreserved<M,F,P>{pub state:crate::streaming_projection::State,pub services:Preserved<M,F,P>}
pub struct ForwardFailure<M,F:RequiredRows,P>{owner:OwnedStreaming<M,F,P>,input:crate::conditional_gate::Batch,
    error:crate::streaming_projection::Error<Error<F::Error>>}
pub struct ResetFailure<M,F:RequiredRows,P>{owner:OwnedStreaming<M,F,P>,error:crate::streaming_projection::Error<Error<F::Error>>}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> OwnedStreaming<M,F,P>{

    pub fn from_controller(controller:crate::streaming_projection::StreamingProjectedConv,parts:Parts<M,F,P>)
        ->Result<Self,OwnedAdmissionFailure<M,F,P>>{
        match ValueServices::from_explicit_parts(&controller.state().binding,controller.state().has_bias,parts){
            Ok(services)=>Ok(Self{controller,services}),Err(refusal)=>Err(OwnedAdmissionFailure{controller,refusal}),
        }
    }
    pub fn state(&self)->&crate::streaming_projection::State{self.controller.state()}
    pub fn parts(&self)->&Parts<M,F,P>{self.services.parts()}
    pub fn forward(mut self,input:crate::conditional_gate::Batch)
        ->Result<(Self,crate::conditional_gate::Batch),ForwardFailure<M,F,P>>{
        match self.controller.forward(&input,&mut self.services){
            Ok(output)=>Ok((self,output)),Err(error)=>Err(ForwardFailure{owner:self,input,error}),
        }
    }
    pub fn reset(mut self)->Result<Self,ResetFailure<M,F,P>>{
        match self.controller.reset(&mut self.services){Ok(())=>Ok(self),Err(error)=>Err(ResetFailure{owner:self,error})}
    }
    pub fn into_preserved(self)->OwnedPreserved<M,F,P>{OwnedPreserved{state:self.controller.into_explicit_state(),services:self.services.into_preserved()}}
}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> ForwardFailure<M,F,P>{
    pub fn state(&self)->&crate::streaming_projection::State{self.owner.state()}
    pub fn input(&self)->&crate::conditional_gate::Batch{&self.input}
    pub fn error(&self)->&crate::streaming_projection::Error<Error<F::Error>>{&self.error}
    pub fn into_preserved(self)->(OwnedPreserved<M,F,P>,crate::conditional_gate::Batch,crate::streaming_projection::Error<Error<F::Error>>){
        (self.owner.into_preserved(),self.input,self.error)
    }
}
impl<M:GenericFirstTap+SourceAccumulatingMatrix,F:RequiredRows,P:RequiredPacked> ResetFailure<M,F,P>{
    pub fn state(&self)->&crate::streaming_projection::State{self.owner.state()}
    pub fn error(&self)->&crate::streaming_projection::Error<Error<F::Error>>{&self.error}
    pub fn into_preserved(self)->(OwnedPreserved<M,F,P>,crate::streaming_projection::Error<Error<F::Error>>){(self.owner.into_preserved(),self.error)}
}
