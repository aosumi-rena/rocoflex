use crate::condition_history::ConditionHistory;
use crate::root_binding::GateConfig;
#[path="conditional_gate_work.rs"]
mod conditional_work;
pub use self::conditional_work::{GateConditionalWork,GateConditionalProgress,GateConditionalStage};

#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub rows: Vec<Vec<f32>>,
    pub status: u8,
}
impl crate::logical_read_rows::ReadRows for Batch {
    fn row_count(&self)->usize{self.rows.len()}
    fn source_status(&self)->u8{self.status}
    fn read_row(&self,index:usize)->Option<&[f32]>{self.rows.get(index).map(Vec::as_slice)}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    UnsupportedConfiguration,
    MissingInput { row:usize },
    RowWidth { expected: usize, actual: usize },
    MissingCondition { required: usize, available: usize },
    MissingResidual { required: usize, available: usize },
    ResidualPreparationRequired { required: usize, available: usize },
    ResetRequired,
    Dependency(&'static str),
}

pub trait Projection {
    fn reset(&mut self) -> Result<(), GateError>;
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError>;
}

pub trait BorrowedProjection:Projection {
    fn forward_borrowed(&mut self,input:&dyn crate::logical_read_rows::ReadRows)->Result<Batch,GateError>;
}

pub trait Auxiliary {
    fn reset(&mut self) -> Result<(), GateError>;
    fn forward_row(&mut self, row: &mut [f32]) -> Result<(), GateError>;
}

pub trait BatchAuxiliary:Auxiliary {
    fn forward_batch_in_place(&mut self,batch:&mut Batch)->Result<(),GateError>;
}

pub trait VectorKernels {
    fn slot88(&mut self, condition: &[f32], gate: &mut [f32]) -> Result<(), GateError>;
    fn split_activate_combine(&mut self, gate: &[f32], half_width: usize)
        -> Result<Vec<f32>, GateError>;
}

pub struct ConditionalGate<P, C, A, K> {
    config: GateConfig,
    main: P,
    condition: C,
    auxiliary: Option<A>,
    kernels: K,
    history: ConditionHistory<Vec<f32>>,
    needs_reset: bool,
    pending_conditional_work:Option<GateConditionalWork>,
}

pub struct GateExplicitState<P,C,A,K> {
    pub config:GateConfig,pub main:P,pub condition:C,pub auxiliary:Option<A>,
    pub kernels:K,pub history:ConditionHistory<Vec<f32>>,pub needs_reset:bool,
    pub pending_conditional_work:Option<GateConditionalWork>,
}
pub struct GateStateRefusal<P,C,A,K> {
    pub state:GateExplicitState<P,C,A,K>,pub error:GateError,
}

impl<P: Projection, C: Projection, A: Auxiliary, K: VectorKernels>
    ConditionalGate<P, C, A, K>
{

    pub fn from_explicit_state(state:GateExplicitState<P,C,A,K>)
        ->Result<Self,GateStateRefusal<P,C,A,K>> {
        let config=state.config;
        if config.input_width<=0||config.half_width<=0||config.condition_width<=0
            ||config.warmup_rows<0||config.has_auxiliary!=state.auxiliary.is_some(){
            return Err(GateStateRefusal{state,error:GateError::UnsupportedConfiguration});
        }
        let Some(width)=(config.half_width as usize).checked_mul(2) else{
            return Err(GateStateRefusal{state,error:GateError::UnsupportedConfiguration});
        };
        if width>i32::MAX as usize{
            return Err(GateStateRefusal{state,error:GateError::UnsupportedConfiguration});
        }
        if !state.needs_reset&&state.pending_conditional_work.is_some(){
            return Err(GateStateRefusal{state,error:GateError::ResetRequired});
        }
        let wrong=state.history.pending().find_map(|row|
            (row.len()!=width).then_some(row.len()));
        if let Some(actual)=wrong{
            return Err(GateStateRefusal{state,error:GateError::RowWidth{expected:width,actual}});
        }
        Ok(Self{config:state.config,main:state.main,condition:state.condition,
            auxiliary:state.auxiliary,kernels:state.kernels,history:state.history,
            needs_reset:state.needs_reset,pending_conditional_work:state.pending_conditional_work})
    }
    pub fn into_explicit_state(self)->GateExplicitState<P,C,A,K>{
        GateExplicitState{config:self.config,main:self.main,condition:self.condition,
            auxiliary:self.auxiliary,kernels:self.kernels,history:self.history,
            needs_reset:self.needs_reset,pending_conditional_work:self.pending_conditional_work}
    }

    pub fn new(config: GateConfig, main: P, condition: C, auxiliary: Option<A>, kernels: K)
        -> Result<Self, GateError>
    {
        if config.input_width <= 0 || config.half_width <= 0 || config.condition_width <= 0 ||
            config.warmup_rows < 0 || config.has_auxiliary != auxiliary.is_some() {
            return Err(GateError::UnsupportedConfiguration);
        }
        let mut gate = Self { config, main, condition, auxiliary, kernels,
                             history: ConditionHistory::default(), needs_reset: true,pending_conditional_work:None };
        gate.reset()?;
        Ok(gate)
    }

    pub fn reset(&mut self) -> Result<(), GateError> {
        self.needs_reset = true;
        self.pending_conditional_work=None;
        self.history.clear();
        self.main.reset()?;
        self.condition.reset()?;
        if let Some(aux) = &mut self.auxiliary { aux.reset()?; }
        if self.config.warmup_rows > 0 {
            let zeros = Batch {
                rows: vec![vec![0.0; self.config.input_width as usize];
                           self.config.warmup_rows as usize],
                status: 0,
            };
            let _discarded = self.main.forward(&zeros)?;
        }
        self.needs_reset = false;
        Ok(())
    }

    pub fn pending_condition_rows(&self) -> usize { self.history.pending_rows() }
    pub fn pending_conditional_work(&self)->Option<&GateConditionalWork>{self.pending_conditional_work.as_ref()}

    pub fn forward(&mut self, input: &Batch, condition: &Batch) -> Result<Batch, GateError> {
        self.forward_with_main(input,condition,|main|main.forward(input))
    }

    pub fn forward_borrowed(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch)
        ->Result<Batch,GateError> where P:BorrowedProjection {
        self.forward_with_main(input,condition,|main|main.forward_borrowed(input))
    }

    fn forward_with_main<FMain>(&mut self,input:&dyn crate::logical_read_rows::ReadRows,condition:&Batch,
        main_forward:FMain)->Result<Batch,GateError>
        where FMain:FnOnce(&mut P)->Result<Batch,GateError> {
        if self.needs_reset { return Err(GateError::ResetRequired); }
        check_read_rows(input,self.config.input_width as usize)?;
        check_rows(&condition.rows, self.config.condition_width as usize)?;

        self.needs_reset = true;
        self.pending_conditional_work=Some(GateConditionalWork::new());
        let result=(||{
            let work=self.pending_conditional_work.as_mut().expect("owned conditional gate work");
            work.progress.stage=GateConditionalStage::Main;
            work.main=Some(main_forward(&mut self.main)?);
            work.progress.stage=GateConditionalStage::Condition;
            work.projected=Some(self.condition.forward(condition)?);
            let width=2*self.config.half_width as usize;
            work.progress.stage=GateConditionalStage::CheckMain;
            check_rows(&work.main.as_ref().expect("returned main owner").rows,width)?;
            work.progress.stage=GateConditionalStage::CheckProjected;
            check_rows(&work.projected.as_ref().expect("returned condition owner").rows,width)?;
            work.progress.stage=GateConditionalStage::History;
            let count=work.main.as_ref().expect("returned main owner").rows.len();
            let projected=std::mem::take(&mut work.projected.as_mut().expect("returned condition owner").rows);
            let conditions=self.history.append_and_take(projected,count)
                .map_err(|error|GateError::MissingCondition{required:error.required,available:error.available})?;
            work.conditions=conditions.into();
            work.output=Vec::with_capacity(count);
            for index in 0..count {
                work.progress.row=index;
                work.active_condition=Some(work.conditions.pop_front().expect("selected condition count"));
                work.progress.stage=GateConditionalStage::Slot88;
                let row=&mut work.main.as_mut().expect("retained main owner").rows[index];
                self.kernels.slot88(work.active_condition.as_deref().expect("active condition owner"),row)?;
                if let Some(auxiliary)=&mut self.auxiliary {
                    work.progress.stage=GateConditionalStage::Auxiliary;auxiliary.forward_row(row)?;
                }
                work.progress.stage=GateConditionalStage::Split;
                work.active_output=Some(self.kernels.split_activate_combine(row,self.config.half_width as usize)?);
                work.progress.stage=GateConditionalStage::OutputWidth;
                let actual=work.active_output.as_ref().expect("returned split owner").len();
                if actual!=self.config.half_width as usize {
                    return Err(GateError::RowWidth{expected:self.config.half_width as usize,actual});
                }
                work.output.push(work.active_output.take().expect("checked split owner"));
                work.consumed_conditions.push(work.active_condition.take().expect("completed condition owner"));
                work.progress.row=index+1;
            }
            work.progress.stage=GateConditionalStage::Complete;

            Ok(Batch{rows:std::mem::take(&mut work.output),status:work.main.as_ref().expect("retained main status").status})
        })();
        match result {
            Ok(output)=>{
                self.pending_conditional_work=None;self.needs_reset=false;Ok(output)
            }
            Err(error)=>{
                self.pending_conditional_work.as_mut().expect("failed conditional gate work").failure=Some(error.clone());Err(error)
            }
        }
    }
}

impl<P:Projection,C:Projection,A:BatchAuxiliary,K:VectorKernels> ConditionalGate<P,C,A,K> {

    pub fn forward_without_condition(&mut self,input:&Batch)->Result<Batch,GateError> {
        if self.needs_reset{return Err(GateError::ResetRequired);}
        check_rows(&input.rows,self.config.input_width as usize)?;
        self.needs_reset=true;
        let mut main=self.main.forward(input)?;

        if let Some(auxiliary)=&mut self.auxiliary{
            auxiliary.forward_batch_in_place(&mut main)?;
        }
        let width=2*self.config.half_width as usize;
        check_rows(&main.rows,width)?;
        let mut rows=Vec::with_capacity(main.rows.len());
        for row in &main.rows{
            let output=self.kernels.split_activate_combine(row,self.config.half_width as usize)?;
            if output.len()!=self.config.half_width as usize{
                return Err(GateError::RowWidth{expected:self.config.half_width as usize,actual:output.len()});
            }
            rows.push(output);
        }
        self.needs_reset=false;

        Ok(Batch{rows,status:main.status})
    }
}

struct AbsentCondition;
impl Projection for AbsentCondition {
    fn reset(&mut self)->Result<(),GateError>{Ok(())}
    fn forward(&mut self,_input:&Batch)->Result<Batch,GateError>{
        Err(GateError::UnsupportedConfiguration)
    }
}

pub struct AbsentAuxiliary;
impl Auxiliary for AbsentAuxiliary {
    fn reset(&mut self)->Result<(),GateError>{Err(GateError::UnsupportedConfiguration)}
    fn forward_row(&mut self,_row:&mut[f32])->Result<(),GateError>{Err(GateError::UnsupportedConfiguration)}
}
impl BatchAuxiliary for AbsentAuxiliary {
    fn forward_batch_in_place(&mut self,_batch:&mut Batch)->Result<(),GateError>{
        Err(GateError::UnsupportedConfiguration)
    }
}

pub struct GenericGateExplicitState<P,A,K> {
    pub config:GateConfig,pub main:P,pub auxiliary:Option<A>,pub kernels:K,
    pub unused_condition_history:ConditionHistory<Vec<f32>>,pub needs_reset:bool,
}
pub struct GenericGateStateRefusal<P,A,K> {
    pub state:GenericGateExplicitState<P,A,K>,pub error:GateError,
}

pub struct GenericGate<P,A,K> {gate:ConditionalGate<P,AbsentCondition,A,K>}
impl<P:Projection,A:BatchAuxiliary,K:VectorKernels> GenericGate<P,A,K> {
    pub fn from_explicit_state(state:GenericGateExplicitState<P,A,K>)
        ->Result<Self,GenericGateStateRefusal<P,A,K>> {
        let config=state.config;
        if config.input_width<=0||config.half_width<=0||config.condition_width>0
            ||config.warmup_rows<0||config.half_width>i32::MAX/2
            ||config.has_auxiliary!=state.auxiliary.is_some(){
            return Err(GenericGateStateRefusal{state,error:GateError::UnsupportedConfiguration});
        }
        let width=2*config.half_width as usize;
        let wrong=state.unused_condition_history.pending()
            .find_map(|row|(row.len()!=width).then_some(row.len()));
        if let Some(actual)=wrong{
            return Err(GenericGateStateRefusal{state,error:GateError::RowWidth{expected:width,actual}});
        }
        Ok(Self{gate:ConditionalGate{config,main:state.main,condition:AbsentCondition,
            auxiliary:state.auxiliary,kernels:state.kernels,history:state.unused_condition_history,
            needs_reset:state.needs_reset,pending_conditional_work:None}})
    }
    pub fn into_explicit_state(self)->GenericGateExplicitState<P,A,K>{
        GenericGateExplicitState{config:self.gate.config,main:self.gate.main,
            auxiliary:self.gate.auxiliary,kernels:self.gate.kernels,
            unused_condition_history:self.gate.history,needs_reset:self.gate.needs_reset}
    }

    pub fn reset(&mut self)->Result<(),GateError>{self.gate.reset()}
    pub fn forward(&mut self,input:&Batch)->Result<Batch,GateError>{
        self.gate.forward_without_condition(input)
    }
    pub fn pending_unused_condition_rows(&self)->usize{self.gate.pending_condition_rows()}
}

fn check_rows(rows: &[Vec<f32>], expected: usize) -> Result<(), GateError> {
    for row in rows {
        if row.len() != expected {
            return Err(GateError::RowWidth { expected, actual: row.len() });
        }
    }
    Ok(())
}
pub(crate) fn check_read_rows(input:&dyn crate::logical_read_rows::ReadRows,expected:usize)->Result<(),GateError>{
    for index in 0..input.row_count(){
        let row=input.read_row(index).ok_or(GateError::MissingInput{row:index})?;
        if row.len()!=expected{return Err(GateError::RowWidth{expected,actual:row.len()});}
    }
    Ok(())
}
