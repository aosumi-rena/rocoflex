use super::{HeadOutputs, IndicatorAccumulation, SelectedPitchHead, SelectedPitchHeadStages,
    SelectedPitchRows, SelectedPitchStream, SelectedPitchBatch, SelectedPitchStreamFailure,
    SelectedPitchState, SelectedPitchWork, validate_owner_layout};
use crate::{Result, model::{ModelNode, ModelTree}, model_tags::decoded_hashes,
    pitch_score::{PitchProfile, PitchScore}, timbre::{QuantizedLinear, runtime_silu, u32_at}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchHeadSelection { Head0, Head2 }
impl TryFrom<u32> for PitchHeadSelection {
    type Error = String;
    fn try_from(index: u32) -> Result<Self> {
        match index { 0 => Ok(Self::Head0), 2 => Ok(Self::Head2),
            _ => Err("Only admitted pitch heads 0 and 2 are supported".into()) }
    }
}

pub enum AdmittedPitchHead { Head0(PointwisePitchHead), Head2(SelectedPitchHead) }
pub enum AdmittedPitchStages { Head0(PointwisePitchStages), Head2(SelectedPitchHeadStages) }
pub enum AdmittedPitchStream<'a> { Head0(PointwisePitchStream<'a>), Head2(SelectedPitchStream<'a>) }
pub enum AdmittedPitchBatch<'a> { Head0(PointwisePitchBatch<'a>), Head2(SelectedPitchBatch<'a>) }
pub enum AdmittedPitchFailure<'a> { Head0(PointwisePitchFailure<'a>), Head2(SelectedPitchStreamFailure<'a>) }
pub(crate) enum AdmittedPitchState { Head0(PointwiseState), Head2(SelectedPitchState) }

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum PitchHeadCountError {UnknownTopology,MissingCursor,AdvancedCursor,FailedCursor}
#[derive(Clone,Copy)]
pub(crate) struct PitchHeadCount {pub selection:PitchHeadSelection,pub right_rows:usize}
impl AdmittedPitchHead {

    pub(crate) fn cold_count_metadata(&self,state:&AdmittedPitchState)
        ->std::result::Result<PitchHeadCount,PitchHeadCountError>
    {
        use PitchHeadCountError::*;
        let (selection,right_rows,advanced,failed,missing)=match (self,state) {
            (Self::Head0(_),AdmittedPitchState::Head0(s))=>
                (PitchHeadSelection::Head0,Some(0),s.accepted!=0 || s.emitted!=0 || s.ended,s.halted,false),
            (Self::Head2(h),AdmittedPitchState::Head2(s))=>{
                let (advanced,failed,missing)=s.count_cursor_state();
                (PitchHeadSelection::Head2,h.prepared_right_rows(),advanced,failed,missing)
            },
            _=>return Err(UnknownTopology),
        };
        if failed {return Err(FailedCursor);}
        if missing {return Err(MissingCursor);}
        if advanced {return Err(AdvancedCursor);}
        Ok(PitchHeadCount {selection,right_rows:right_rows.ok_or(UnknownTopology)?})
    }
    pub fn from_dnni(model: &[u8], selection: PitchHeadSelection, policy: IndicatorAccumulation)
        -> Result<Self>
    {
        match selection {
            PitchHeadSelection::Head0 => Ok(Self::Head0(PointwisePitchHead::from_dnni(model,policy)?)),
            PitchHeadSelection::Head2 => Ok(Self::Head2(SelectedPitchHead::from_dnni(model,policy)?)),
        }
    }
    pub fn forward384_final(&self, input: &[f32]) -> Result<AdmittedPitchStages> {
        match self {
            Self::Head0(head) => head.forward384_final(input).map(AdmittedPitchStages::Head0),
            Self::Head2(head) => head.forward384_final(input).map(AdmittedPitchStages::Head2),
        }
    }
    pub fn stream_from_explicit_position(&self, previous: i32, maximum_rows: usize)
        -> Result<AdmittedPitchStream<'_>>
    {
        match self {
            Self::Head0(head) => PointwisePitchStream::new(head,previous,maximum_rows).map(AdmittedPitchStream::Head0),
            Self::Head2(head) => head.stream_from_explicit_position(previous,maximum_rows).map(AdmittedPitchStream::Head2),
        }
    }
}
impl<'a> AdmittedPitchStream<'a> {
    pub fn push(self, input: &[f32], final_input: bool)
        -> std::result::Result<AdmittedPitchBatch<'a>,AdmittedPitchFailure<'a>>
    {
        match self {
            Self::Head0(stream) => stream.push(input,final_input).map(AdmittedPitchBatch::Head0).map_err(AdmittedPitchFailure::Head0),
            Self::Head2(stream) => stream.push(input,final_input).map(AdmittedPitchBatch::Head2).map_err(AdmittedPitchFailure::Head2),
        }
    }
    pub(crate) fn into_state(self) -> AdmittedPitchState {
        match self {
            Self::Head0(stream) => AdmittedPitchState::Head0(stream.into_state()),
            Self::Head2(stream) => AdmittedPitchState::Head2(stream.into_state()),
        }
    }

    pub(crate) fn from_state(head: &'a AdmittedPitchHead, state: AdmittedPitchState) -> Result<Self> {
        match (head,state) {
            (AdmittedPitchHead::Head0(head),AdmittedPitchState::Head0(state)) =>
                Ok(Self::Head0(PointwisePitchStream::from_state(head,state))),
            (AdmittedPitchHead::Head2(head),AdmittedPitchState::Head2(state)) =>
                Ok(Self::Head2(SelectedPitchStream::from_state(head,state))),
            _ => Err("Pitch stream state and prepared head kind differ".into()),
        }
    }
}
impl<'a> AdmittedPitchBatch<'a> {
    pub fn first_row(&self) -> usize { match self {Self::Head0(b)=>b.first_row,Self::Head2(b)=>b.first_row} }
    pub fn rows(&self) -> Option<&SelectedPitchRows> { match self {Self::Head0(b)=>b.rows.as_ref(),Self::Head2(b)=>b.rows.as_ref()} }

    pub fn into_parts(self) -> (usize,Option<SelectedPitchRows>,AdmittedPitchStream<'a>) {
        match self {
            Self::Head0(b)=>(b.first_row,b.rows,AdmittedPitchStream::Head0(b.state)),
            Self::Head2(b)=>(b.first_row,b.rows,AdmittedPitchStream::Head2(b.state)),
        }
    }
}
impl<'a> AdmittedPitchFailure<'a> {
    pub fn reason(&self) -> &str { match self {Self::Head0(f)=>&f.reason,Self::Head2(f)=>&f.reason} }

    pub fn into_parts(self) -> (String,AdmittedPitchStream<'a>) {
        match self {
            Self::Head0(f)=>(f.reason,AdmittedPitchStream::Head0(f.advanced)),
            Self::Head2(f)=>(f.reason,AdmittedPitchStream::Head2(f.advanced)),
        }
    }
}

pub struct PointwisePitchHead { base: PointwiseBase, outputs: HeadOutputs }
struct PointwiseBase { linear: QuantizedLinear }
pub struct PointwiseBaseStages { pub linear256: Vec<f32>, pub base256: Vec<f32> }
pub struct PointwisePitchStages { pub base: PointwiseBaseStages, pub rows: SelectedPitchRows }

enum MatrixShift { Serialized, Required(u32) }
fn linear(node: &ModelNode<'_>, dimensions: (usize,usize), shift: MatrixShift) -> Result<QuantizedLinear> {
    node.require(0x7fca41ff,b"modl0",decoded_hashes("modl0"))?;
    if !node.data.is_empty() || node.children.len()!=2 {return Err("pointwise pitch Linear topology".into());}
    node.children[0].require(0x7fca40ff,b"prim4",decoded_hashes("prim4"))?;
    node.children[1].require(0x7fca40ff,b"prim1",decoded_hashes("prim1"))?;
    if node.children.iter().any(|p|!p.children.is_empty()) {return Err("pointwise pitch parameters must be leaves".into());}
    let shift=match shift {MatrixShift::Serialized=>u32_at(node.children[0].data,4)?,MatrixShift::Required(value)=>value};
    let owner=QuantizedLinear::from_payloads(node.children[0].data,node.children[1].data,shift)?;
    if (owner.in_dim,owner.out_dim)!=dimensions {return Err("pointwise pitch Linear dimensions".into());}
    Ok(owner)
}
impl PointwisePitchHead {
    fn from_dnni(model: &[u8], policy: IndicatorAccumulation) -> Result<Self> {
        let tree=ModelTree::parse(model)?;
        if tree.version!=2 {return Err("admitted pointwise pitch head requires v2".into());}
        let owner=tree.root.children.get(1).ok_or("pitch owner missing")?;
        owner.require(0x7fca41ff,b"",&[0x471cb7441ab4128a])?;
        validate_owner_layout(owner.data,owner.children.len())?;
        let parent=&owner.children[8];
        parent.require(0x7fca41ff,b"",&[0xe22de8eeae50d337])?;
        if parent.children.len()!=5 || parent.data!=[0,0,0,0,16,0,0,0] {
            return Err("pointwise pitch head configuration".into());
        }
        let sequence=&parent.children[0];
        sequence.require(0x7fca41ff,b"",&[0xe7faf83005bb74f9])?;
        if !sequence.data.is_empty() || sequence.children.len()!=2 {return Err("pointwise pitch base topology".into());}
        let activation=&sequence.children[1];
        activation.require(0x7fca41ff,b"moda7",decoded_hashes("moda7"))?;
        if !activation.data.is_empty() || !activation.children.is_empty() {return Err("pointwise pitch SiLU leaf".into());}
        let base=&sequence.children[0];

        let base=PointwiseBase {linear:linear(base,(384,256),MatrixShift::Serialized)?};
        Ok(Self {base,outputs:HeadOutputs {
            profile:PitchProfile::from_validated_parent(parent,9)?,
            score:PitchScore::from_validated_parent(parent,9,9)?,
            voicing:linear(&parent.children[2],(256,1),MatrixShift::Required(9))?,
            auxiliary:linear(&parent.children[3],(256,1),MatrixShift::Required(9))?,
            indicator_accumulation:policy,
        }})
    }
    pub fn forward384_final(&self,input:&[f32]) -> Result<PointwisePitchStages> {
        validate_input(input)?;
        if input.is_empty() {return Err("pointwise pitch requires nonempty final input".into());}
        let mut base=PointwiseBaseStages::empty();
        self.base.forward_into(input,&mut base)?;
        let rows=self.outputs.forward_base256_with_state(&base.base256,&mut -1)?;
        Ok(PointwisePitchStages {base,rows})
    }
}
fn validate_input(input:&[f32]) -> Result<()> {
    if input.len()%384!=0 || input.iter().any(|v|!v.is_finite()) {
        Err("Pointwise pitch requires finite complete 384-lane rows".into())
    } else {Ok(())}
}
impl PointwiseBaseStages {
    fn empty()->Self {Self {linear256:Vec::new(),base256:Vec::new()}}
}
impl PointwiseBase {

    fn forward_into(&self,input:&[f32],stages:&mut PointwiseBaseStages)->Result<()> {
        let count=(input.len()/384).checked_mul(256).ok_or("pointwise pitch output extent overflow")?;
        stages.linear256.try_reserve(count).map_err(|e|e.to_string())?;
        stages.base256.try_reserve(count).map_err(|e|e.to_string())?;
        for row in input.chunks_exact(384) {
            let values=self.linear.forward_saturation(row,true)?;
            stages.linear256.extend_from_slice(&values);
            if values.iter().any(|v|!v.is_finite()) {return Err("Nonfinite pointwise pitch Linear result".into());}
            let activated=values.iter().map(|&v|runtime_silu(v)).collect::<Vec<_>>();
            stages.base256.extend_from_slice(&activated);
            if activated.iter().any(|v|!v.is_finite()) {return Err("Nonfinite pointwise pitch SiLU result".into());}
        }
        Ok(())
    }
}

pub struct PointwisePitchStream<'a> {
    head:&'a PointwisePitchHead, previous:i32, accepted:usize, emitted:usize,
    row_limit:usize,ended:bool,halted:bool,
}
pub(crate) struct PointwiseState {previous:i32,accepted:usize,emitted:usize,row_limit:usize,ended:bool,halted:bool}
pub struct PointwisePitchBatch<'a> {
    pub first_row:usize,pub base:PointwiseBaseStages,pub rows:Option<SelectedPitchRows>,
    pub final_input:bool,pub state:PointwisePitchStream<'a>,
}
pub struct PointwisePitchFailure<'a> {
    pub reason:String,

    pub base_work:Option<PointwiseBaseStages>,pub base_complete:bool,
    pub pending_previous_position:Option<i32>,pub pending_work:Option<SelectedPitchWork>,
    pub advanced:PointwisePitchStream<'a>,
}
impl<'a> PointwisePitchStream<'a> {
    fn new(head:&'a PointwisePitchHead,previous:i32,row_limit:usize)->Result<Self> {
        if !(-1..128).contains(&previous) {return Err("Pitch head stream requires an explicit previous position in -1..128".into());}
        if row_limit==0 || row_limit>i32::MAX as usize {return Err("Pointwise pitch stream requires a positive i32 row limit".into());}
        Ok(Self {head,previous,accepted:0,emitted:0,row_limit,ended:false,halted:false})
    }
    fn into_state(self)->PointwiseState {
        let Self {previous,accepted,emitted,row_limit,ended,halted,..}=self;
        PointwiseState {previous,accepted,emitted,row_limit,ended,halted}
    }
    fn from_state(head:&'a PointwisePitchHead,state:PointwiseState)->Self {
        let PointwiseState {previous,accepted,emitted,row_limit,ended,halted}=state;
        Self {head,previous,accepted,emitted,row_limit,ended,halted}
    }
    pub fn previous_position(&self)->i32 {self.previous}
    pub fn accepted_rows(&self)->usize {self.accepted}
    pub fn emitted_rows(&self)->usize {self.emitted}
    pub fn is_ended(&self)->bool {self.ended}
    pub fn is_halted(&self)->bool {self.halted}
    fn failure(self,reason:impl Into<String>,base:Option<PointwiseBaseStages>)->PointwisePitchFailure<'a> {
        PointwisePitchFailure {reason:reason.into(),base_work:base,base_complete:false,
            pending_previous_position:None,pending_work:None,advanced:self}
    }
    pub fn push(mut self,input:&[f32],final_input:bool)
        -> std::result::Result<PointwisePitchBatch<'a>,PointwisePitchFailure<'a>>
    {
        if self.ended || self.halted {return Err(self.failure("Pointwise pitch stream is already ended or halted",None));}
        if let Err(reason)=validate_input(input) {self.halted=true;return Err(self.failure(reason,None));}
        let Some(accepted)=self.accepted.checked_add(input.len()/384).filter(|&n|n<=self.row_limit) else {
            self.halted=true;return Err(self.failure("Pointwise pitch stream row limit exceeded",None));
        };
        if final_input && accepted==0 {self.halted=true;return Err(self.failure("Pointwise pitch requires nonempty final input",None));}
        let first_row=self.emitted;
        self.accepted=accepted;self.ended=final_input;
        let mut base=PointwiseBaseStages::empty();
        if let Err(reason)=self.head.base.forward_into(input,&mut base) {
            self.halted=true;return Err(self.failure(reason,Some(base)));
        }
        let rows=if base.base256.is_empty() {None} else {
            let mut previous=self.previous;let mut work=SelectedPitchWork::empty();
            match self.head.outputs.forward_base256_into_work(&base.base256,&mut previous,&mut work) {
                Ok(rows)=>{self.previous=previous;Some(rows)},
                Err(reason)=>{
                    self.halted=true;let mut failure=self.failure(reason,Some(base));failure.base_complete=true;
                    failure.pending_previous_position=Some(previous);failure.pending_work=Some(work);
                    return Err(failure);
                },
            }
        };
        self.emitted=accepted;
        Ok(PointwisePitchBatch {first_row,base,rows,final_input,state:self})
    }
}

