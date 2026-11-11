use std::collections::VecDeque;
use crate::conditional_gate::Batch;
use crate::logical_read_rows::ReadRows;
use crate::projected_conv_binding::{ProjectedConvBinding,ProjectedConvMode};
pub type Row=Vec<f32>;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CountError{DivideByZero,SignedDivisionOverflow}

pub fn output_count_10e390(queue_low32:u32,taps:u32,stride:u32,dilation:u32)->Result<u32,CountError>{
    let product=taps.wrapping_sub(1).wrapping_mul(dilation);
    let numerator=(!product).wrapping_add(queue_low32.wrapping_add(stride))as i32;
    let divisor=stride as i32;
    if divisor==0{return Err(CountError::DivideByZero);}
    if numerator==i32::MIN&&divisor== -1{return Err(CountError::SignedDivisionOverflow);}
    let quotient=numerator/divisor;Ok(if quotient<=0{0}else{quotient as u32})
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Pool{Input,Output}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase{Ready,Input{row:usize},EndPadding{row:usize},Count,OutputStatus,Trim,
    Prepare{row:usize},Generic{row:usize,tap:usize},Bias{row:usize},Packed,
    Retire{slot:usize},Erase{count:usize},ResetClear,ResetInputPool,ResetOutputPool,ResetPadding{row:usize},Complete}
pub struct RowRefusal<E>{pub reason:E,pub row:Row}

pub trait Services{
    type Error;
    fn take_row(&mut self,pool:Pool,width:usize)->Result<Row,Self::Error>;
    fn copy_input_slot72(&mut self,input:&[f32],row:&mut[f32])->Result<(),Self::Error>;
    fn clear_input_slot48(&mut self,row:&mut[f32])->Result<(),Self::Error>;
    fn return_input_row_30a70(&mut self,row:Row)->Result<(),RowRefusal<Self::Error>>;

    fn return_null_output_30a70(&mut self)->Result<(),Self::Error>;
    fn delete_output_row(&mut self,row:Row)->Result<(),RowRefusal<Self::Error>>;

    fn overwrite_and_postprocess(&mut self,tap:usize,input:&[f32],output:&mut[f32])->Result<(),Self::Error>;
    fn accumulate_and_postprocess(&mut self,tap:usize,input:&[f32],output:&mut[f32])->Result<(),Self::Error>;
    fn bias_slot88(&mut self,output:&mut[f32])->Result<(),Self::Error>;

    fn packed_333c0(&mut self,input:&[&[f32]],output:&mut[&mut[f32]],stride:i32,dilation:i32,fifth:u32)->Result<(),Self::Error>;

    fn clear_history_6490(&mut self,history:&mut VecDeque<Option<Row>>)->Result<(),Self::Error>;

    fn replace_pool_327d0(&mut self,pool:Pool,width:usize,initial:u32,growth:u32)->Result<(),Self::Error>;
}
pub struct Output{pub rows:Vec<Option<Row>>,pub status:u8}
pub struct State{
    pub binding:ProjectedConvBinding,pub history:VecDeque<Option<Row>>,pub flags224:u8,pub has_bias:bool,
    pub output:Output,pub pending_row:Option<Row>,pub phase:Phase,pub halted:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Admission{Configuration,CountDomain,MissingHistory,HistoryWidth,PendingRow,ActiveFailure}
pub struct RefusedState{pub state:State,pub reason:Admission}
pub enum Error<E>{PriorFailure,MissingInput{row:usize},InputWidth{row:usize,expected:usize,actual:usize},Count(CountError),
    CountDomain,IndexRequired{row:usize,tap:usize,index:i64},MissingHistory{index:usize},MissingOutput{row:usize},
    RowWidth{pool:Pool,expected:usize,actual:usize},MissingRetirement{required:usize,available:usize},
    Service{phase:Phase,reason:E}}
pub struct StreamingProjectedConv{state:State}
impl StreamingProjectedConv{

    pub fn from_explicit_state(state:State)->Result<Self,RefusedState>{
        let c=state.binding.config();let input=state.binding.input_width();let output=state.binding.output_width();
        let reason=if c.taps<=0||c.fifth_word!=1||input==0||output==0{Some(Admission::Configuration)}
            else if state.history.len()>i32::MAX as usize||state.output.rows.len()>i32::MAX as usize{Some(Admission::CountDomain)}
            else if !state.halted&&state.history.iter().any(Option::is_none){Some(Admission::MissingHistory)}
            else if state.history.iter().flatten().any(|r|r.len()!=input){Some(Admission::HistoryWidth)}
            else if !state.halted&&state.pending_row.is_some(){Some(Admission::PendingRow)}
            else if !state.halted&&!matches!(state.phase,Phase::Ready|Phase::Complete){Some(Admission::ActiveFailure)}else{None};
        if let Some(reason)=reason{return Err(RefusedState{state,reason});}Ok(Self{state})
    }
    pub fn state(&self)->&State{&self.state}
    pub fn into_explicit_state(self)->State{self.state}
    fn service<E>(&self,reason:E)->Error<E>{Error::Service{phase:self.state.phase,reason}}
    fn take<S:Services>(&mut self,pool:Pool,width:usize,services:&mut S)->Result<(),Error<S::Error>>{

        if self.state.pending_row.is_some(){return Err(Error::PriorFailure);}
        let row=services.take_row(pool,width).map_err(|e|self.service(e))?;
        let actual=row.len();self.state.pending_row=Some(row);
        if actual!=width{return Err(Error::RowWidth{pool,expected:width,actual});}Ok(())
    }

    pub fn forward<S:Services>(&mut self,input:&Batch,services:&mut S)->Result<Batch,Error<S::Error>>{
        self.forward_borrowed(input,services)
    }

    pub fn forward_borrowed<S:Services>(&mut self,input:&dyn ReadRows,services:&mut S)->Result<Batch,Error<S::Error>>{
        if self.state.halted{return Err(Error::PriorFailure);}
        let width=self.state.binding.input_width();
        let input_count=input.row_count();
        for i in 0..input_count{
            let row=input.read_row(i).ok_or(Error::MissingInput{row:i})?;
            if row.len()!=width{return Err(Error::InputWidth{row:i,expected:width,actual:row.len()});}
        }
        if input_count>i32::MAX as usize{return Err(Error::CountDomain);}
        self.state.halted=true;
        for i in 0..input_count{
            let source=input.read_row(i).ok_or(Error::MissingInput{row:i})?;
            self.state.phase=Phase::Input{row:i};self.take(Pool::Input,width,services)?;
            let result=services.copy_input_slot72(source,self.state.pending_row.as_mut().expect("taken logical row"));
            result.map_err(|e|self.service(e))?;
            self.state.history.push_back(self.state.pending_row.take());
        }
        let config=self.state.binding.config();
        if input.source_status()!=0{

            for i in 0..config.padding.max(0)as usize{
                self.state.phase=Phase::EndPadding{row:i};self.take(Pool::Input,width,services)?;
                let result=services.clear_input_slot48(self.state.pending_row.as_mut().expect("taken padding row"));
                result.map_err(|e|self.service(e))?;
                self.state.history.push_back(self.state.pending_row.take());
            }
            self.state.flags224|=1;
        }
        self.state.phase=Phase::Count;
        if self.state.history.len()>i32::MAX as usize{return Err(Error::CountDomain);}

        let count=output_count_10e390(self.state.history.len()as u32,config.taps as u32,config.stride as u32,config.dilation as u32).map_err(Error::Count)? as usize;
        self.state.phase=Phase::OutputStatus;self.state.output.status=self.state.flags224&1;
        self.state.phase=Phase::Trim;
        while self.state.output.rows.len()>count{
            let i=self.state.output.rows.len()-1;
            let row=self.state.output.rows[i].take().ok_or(Error::MissingOutput{row:i})?;
            let matches=row.len()==self.state.binding.output_width();self.state.pending_row=Some(row);
            if matches{services.return_null_output_30a70().map_err(|e|self.service(e))?;}
            let _=self.state.output.rows.pop();
            let row=self.state.pending_row.take().expect("removed output retained");
            if let Err(failure)=services.delete_output_row(row){self.state.pending_row=Some(failure.row);return Err(self.service(failure.reason));}
        }
        while self.state.output.rows.len()<count{self.state.output.rows.push(None);}
        let out_width=self.state.binding.output_width();
        for i in 0..count{
            self.state.phase=Phase::Prepare{row:i};
            if self.state.output.rows[i].as_ref().is_some_and(|r|r.len()==out_width){continue;}
            self.take(Pool::Output,out_width,services)?;
            let old=std::mem::replace(&mut self.state.output.rows[i],self.state.pending_row.take());
            if let Some(row)=old{if let Err(failure)=services.delete_output_row(row){self.state.pending_row=Some(failure.row);return Err(self.service(failure.reason));}}
        }
        if count!=0{
            match self.state.binding.mode(){
                ProjectedConvMode::GenericTaps0=>{
                    let mut start=0i32;
                    for row in 0..count{
                        for tap in 0..config.taps as usize{
                            self.state.phase=Phase::Generic{row,tap};
                            let index=start.wrapping_add(config.dilation.wrapping_mul(tap as i32));
                            if index<0{return Err(Error::IndexRequired{row,tap,index:i64::from(index)});}
                            let source=self.state.history.get(index as usize).and_then(Option::as_deref).ok_or(Error::MissingHistory{index:index as usize})?;
                            let destination=self.state.output.rows[row].as_mut().expect("prepared output row");
                            let result=if tap==0{services.overwrite_and_postprocess(tap,source,destination)}else{services.accumulate_and_postprocess(tap,source,destination)};
                            result.map_err(|e|self.service(e))?;
                        }
                        if self.state.has_bias{self.state.phase=Phase::Bias{row};let result=services.bias_slot88(self.state.output.rows[row].as_mut().expect("prepared output row"));result.map_err(|e|self.service(e))?;}
                        start=start.wrapping_add(config.stride);
                    }
                },
                ProjectedConvMode::PackedMatrix1=>{
                    self.state.phase=Phase::Packed;
                    let mut inputs=Vec::with_capacity(self.state.history.len());
                    for(i,row)in self.state.history.iter().enumerate(){inputs.push(row.as_deref().ok_or(Error::MissingHistory{index:i})?);}
                    let mut outputs=self.state.output.rows.iter_mut().map(|r|r.as_mut().expect("prepared packed output").as_mut_slice()).collect::<Vec<_>>();
                    let result=services.packed_333c0(&inputs,&mut outputs,config.stride,config.dilation,1);
                    result.map_err(|reason|Error::Service{phase:Phase::Packed,reason})?;
                    for row in 0..count{if self.state.has_bias{self.state.phase=Phase::Bias{row};let result=services.bias_slot88(self.state.output.rows[row].as_mut().expect("packed output"));result.map_err(|e|self.service(e))?;}}
                },
            }

            let full=(config.stride as i64).checked_mul(count as i64).ok_or(Error::CountDomain)?;
            if full<0||full>i64::from(i32::MAX){return Err(Error::CountDomain);}
            let retired=full as usize;
            if retired>self.state.history.len(){return Err(Error::MissingRetirement{required:retired,available:self.state.history.len()});}
            for i in 0..retired{
                self.state.phase=Phase::Retire{slot:i};
                let row=self.state.history[i].take().ok_or(Error::MissingHistory{index:i})?;
                if let Err(failure)=services.return_input_row_30a70(row){self.state.pending_row=Some(failure.row);return Err(self.service(failure.reason));}
            }
            self.state.phase=Phase::Erase{count:retired};drop(self.state.history.drain(..retired));
        }
        self.state.phase=Phase::Complete;self.state.halted=false;
        Ok(Batch{rows:self.state.output.rows.iter().map(|r|r.as_ref().expect("completed output").clone()).collect(),status:self.state.output.status})
    }
    pub fn reset<S:Services>(&mut self,services:&mut S)->Result<(),Error<S::Error>>{

        if self.state.pending_row.is_some(){return Err(Error::PriorFailure);}
        if self.state.halted&&matches!(self.state.phase,Phase::ResetClear|Phase::ResetInputPool|Phase::ResetOutputPool|Phase::ResetPadding{..}){
            return Err(Error::PriorFailure);
        }
        self.state.halted=true;self.state.phase=Phase::ResetClear;
        services.clear_history_6490(&mut self.state.history).map_err(|e|self.service(e))?;
        if !self.state.history.is_empty(){return Err(Error::MissingRetirement{required:self.state.history.len(),available:0});}
        self.state.phase=Phase::ResetInputPool;services.replace_pool_327d0(Pool::Input,self.state.binding.input_width(),128,128).map_err(|e|self.service(e))?;
        self.state.phase=Phase::ResetOutputPool;services.replace_pool_327d0(Pool::Output,self.state.binding.output_width(),128,128).map_err(|e|self.service(e))?;
        for i in 0..self.state.binding.config().padding.max(0)as usize{
            self.state.phase=Phase::ResetPadding{row:i};self.take(Pool::Input,self.state.binding.input_width(),services)?;
            self.state.history.push_back(self.state.pending_row.take());
            let result=services.clear_input_slot48(self.state.history.back_mut().and_then(Option::as_mut).expect("source appended reset padding"));
            result.map_err(|e|self.service(e))?;
        }
        self.state.flags224&=!1;self.state.phase=Phase::Ready;self.state.halted=false;Ok(())
    }
}
