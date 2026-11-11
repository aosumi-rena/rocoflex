use super::{Error,Path};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Relation {OrderedGreater,OrderedEqual,OrderedLessEqual,OrderedGreaterEqual}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum OperandOrder {RequestLeftRight,RequestRightLeft}

pub fn ordered_relation_from_flags(relation:Relation,operands:OperandOrder,
    carry:bool,parity:bool,zero:bool)->bool
{
    match relation {
        Relation::OrderedEqual=>zero&&!parity,
        Relation::OrderedGreater=>match operands {
            OperandOrder::RequestLeftRight=>!carry&&!zero&&!parity,
            OperandOrder::RequestRightLeft=>carry&&!parity,
        },
        Relation::OrderedLessEqual=>match operands {
            OperandOrder::RequestLeftRight=>(carry||zero)&&!parity,
            OperandOrder::RequestRightLeft=>!carry&&!parity,
        },
        Relation::OrderedGreaterEqual=>match operands {
            OperandOrder::RequestLeftRight=>!carry&&!parity,
            OperandOrder::RequestRightLeft=>(carry||zero)&&!parity,
        },
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CurveKind {Line,Quadratic,Cubic}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum NearPhase {ExceptionalEqual,AbsoluteThreshold,RelativeThreshold}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TokenPhase {InitialClose,QuadraticAfterRead,CubicAfterRead,LineAfterTransform,
    QuadraticAfterTransform,CubicAfterTransform,CloseBeforeMove}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PointRole {Move,FirstControl,SecondControl,End}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Site {LowerX(PointRole),UpperX(PointRole),LowerY(PointRole),UpperY(PointRole),CloseMarker,
    FillToken {word_index:usize,marker:u32},RectangleWidth,RectangleHeight,
    AffineIdentity {coefficient:u8},CurveDistance {kind:CurveKind,half:u8},
    CurveNear {kind:CurveKind,pair:u8,phase:NearPhase},
    IteratorToken {phase:TokenPhase,stacked:bool,word_index:usize,marker:u32},IteratorPeekClose {word_index:usize}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub site:Site,pub relation:Relation,pub left:u32,pub right:u32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct PackedRequest {pub site:Site,pub left:[u32;4],pub right:[u32;4]}
pub trait Services {
    type Error;

    fn compare(&mut self,request:Request)->Result<bool,Self::Error>;
}
pub trait PackedServices:Services {

    fn compare_equal_xmm(&mut self,request:PackedRequest)->Result<[u32;4],Self::Error>;
}
#[derive(Debug)]
pub enum Failure<E> {Storage(Error),Comparison {request:Request,error:E}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AppendKind {Move,Cubic,Close}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AppendPhase {Bounds,Capacity,Words,CloseTest,Complete}
#[derive(Clone,Copy,Debug)]
pub struct BoundWrite {pub role:PointRole,pub slot:u8,pub before:u32,pub after:u32}
#[derive(Clone,Copy,Debug)]
pub struct AppendProgress {
    pub kind:AppendKind,pub phase:AppendPhase,pub words_before:usize,pub words_after:usize,
    pub capacity_before:i32,pub capacity_after:i32,pub comparison:Option<Request>,

    pub writes:[Option<BoundWrite>;10],pub completed_writes:u8,
}
impl AppendProgress {
    fn new(path:&Path,kind:AppendKind)->Self {Self {kind,phase:AppendPhase::Bounds,
        words_before:path.words.len(),words_after:path.words.len(),capacity_before:path.source_capacity,
        capacity_after:path.source_capacity,comparison:None,writes:[None;10],completed_writes:0}}
    fn finish<E>(mut self,path:&Path,result:Result<(),Failure<E>>)->Result<Self,AppendFailure<E>> {
        self.words_after=path.words.len();self.capacity_after=path.source_capacity;
        match result {Ok(())=>{self.phase=AppendPhase::Complete;Ok(self)},Err(cause)=>Err(AppendFailure {progress:self,cause})}
    }
}
#[derive(Debug)]
#[must_use="retain SAME partially advanced Path and append/bounds order; no replay"]
pub struct AppendFailure<E> {pub progress:AppendProgress,pub cause:Failure<E>}
fn compare<S:Services>(service:&mut S,site:Site,relation:Relation,left:u32,right:u32)->Result<bool,Failure<S::Error>> {
    let request=Request {site,relation,left,right};
    service.compare(request).map_err(|error|Failure::Comparison {request,error})
}
fn bound_compare<S:Services>(service:&mut S,progress:&mut AppendProgress,site:Site,relation:Relation,left:u32,right:u32)
    ->Result<bool,Failure<S::Error>>
{
    progress.comparison=Some(Request {site,relation,left,right});compare(service,site,relation,left,right)
}

impl Path {
    fn store_bound(&mut self,slot:usize,value:f32,role:PointRole,progress:&mut AppendProgress) {
        let before=self.bounds[slot].to_bits();self.bounds[slot]=value;
        progress.writes[progress.completed_writes as usize]=Some(BoundWrite {role,slot:slot as u8,before,after:value.to_bits()});
        progress.completed_writes+=1;
    }
    fn point_bounds_with_comparisons<S:Services>(&mut self,point:[f32;2],role:PointRole,service:&mut S,progress:&mut AppendProgress)
        ->Result<(),Failure<S::Error>>
    {
        use Relation::OrderedGreater;

        if bound_compare(service,progress,Site::LowerX(role),OrderedGreater,self.bounds[0].to_bits(),point[0].to_bits())? {
            self.store_bound(0,point[0],role,progress);
        }else if bound_compare(service,progress,Site::UpperX(role),OrderedGreater,point[0].to_bits(),self.bounds[1].to_bits())? {
            self.store_bound(1,point[0],role,progress);
        }
        if bound_compare(service,progress,Site::LowerY(role),OrderedGreater,self.bounds[2].to_bits(),point[1].to_bits())? {
            self.store_bound(2,point[1],role,progress);
        }else if bound_compare(service,progress,Site::UpperY(role),OrderedGreater,point[1].to_bits(),self.bounds[3].to_bits())? {
            self.store_bound(3,point[1],role,progress);
        }
        Ok(())
    }
    pub fn start_with_comparisons<S:Services>(&mut self,point:[f32;2],service:&mut S)
        ->Result<(),Failure<S::Error>>
    {
        self.start_with_comparisons_retaining(point,service).map(|_|()).map_err(|failure|failure.cause)
    }
    pub fn start_with_comparisons_retaining<S:Services>(&mut self,point:[f32;2],service:&mut S)
        ->Result<AppendProgress,AppendFailure<S::Error>>
    {
        let mut progress=AppendProgress::new(self,AppendKind::Move);
        let result=self.start_graph(point,service,&mut progress);progress.finish(self,result)
    }
    fn start_graph<S:Services>(&mut self,point:[f32;2],service:&mut S,progress:&mut AppendProgress)
        ->Result<(),Failure<S::Error>>
    {

        progress.phase=AppendPhase::Bounds;
        if self.words.is_empty() {

            self.store_bound(1,point[0],PointRole::Move,progress);
            self.store_bound(0,point[0],PointRole::Move,progress);
            self.store_bound(3,point[1],PointRole::Move,progress);
            self.store_bound(2,point[1],PointRole::Move,progress);
        }else {self.point_bounds_with_comparisons(point,PointRole::Move,service,progress)?;}
        progress.phase=AppendPhase::Capacity;
        self.ensure(3).map_err(Failure::Storage)?;
        progress.phase=AppendPhase::Words;
        self.words.extend_from_slice(&[0x47c35100,point[0].to_bits(),point[1].to_bits()]);progress.words_after=self.words.len();Ok(())
    }
    pub fn cubic_with_comparisons<S:Services>(&mut self,a:[f32;2],b:[f32;2],end:[f32;2],service:&mut S)
        ->Result<(),Failure<S::Error>>
    {
        self.cubic_with_comparisons_retaining(a,b,end,service).map(|_|()).map_err(|failure|failure.cause)
    }
    pub fn cubic_with_comparisons_retaining<S:Services>(&mut self,a:[f32;2],b:[f32;2],end:[f32;2],service:&mut S)
        ->Result<AppendProgress,AppendFailure<S::Error>>
    {
        let mut progress=AppendProgress::new(self,AppendKind::Cubic);
        let result=self.cubic_graph(a,b,end,service,&mut progress);progress.finish(self,result)
    }
    fn cubic_graph<S:Services>(&mut self,a:[f32;2],b:[f32;2],end:[f32;2],service:&mut S,progress:&mut AppendProgress)
        ->Result<(),Failure<S::Error>>
    {

        if self.words.is_empty() {self.start_graph([0.0,0.0],service,progress)?;}
        progress.phase=AppendPhase::Capacity;
        self.ensure(7).map_err(Failure::Storage)?;

        progress.phase=AppendPhase::Words;
        self.words.extend_from_slice(&[0x47c35200,a[0].to_bits(),a[1].to_bits(),
            b[0].to_bits(),b[1].to_bits(),end[0].to_bits(),end[1].to_bits()]);
        progress.words_after=self.words.len();progress.phase=AppendPhase::Bounds;
        self.point_bounds_with_comparisons(a,PointRole::FirstControl,service,progress)?;
        self.point_bounds_with_comparisons(b,PointRole::SecondControl,service,progress)?;
        self.point_bounds_with_comparisons(end,PointRole::End,service,progress)
    }
    pub fn close_with_comparisons<S:Services>(&mut self,service:&mut S)->Result<(),Failure<S::Error>> {
        self.close_with_comparisons_retaining(service).map(|_|()).map_err(|failure|failure.cause)
    }
    pub fn close_with_comparisons_retaining<S:Services>(&mut self,service:&mut S)->Result<AppendProgress,AppendFailure<S::Error>> {
        let mut progress=AppendProgress::new(self,AppendKind::Close);
        let result=self.close_graph(service,&mut progress);progress.finish(self,result)
    }
    fn close_graph<S:Services>(&mut self,service:&mut S,progress:&mut AppendProgress)->Result<(),Failure<S::Error>> {
        progress.phase=AppendPhase::CloseTest;
        let Some(&last)=self.words.last() else {return Ok(());};
        progress.comparison=Some(Request {site:Site::CloseMarker,relation:Relation::OrderedEqual,left:last,right:0x47c35280});
        if compare(service,Site::CloseMarker,Relation::OrderedEqual,last,0x47c35280)? {return Ok(());}
        progress.phase=AppendPhase::Capacity;self.ensure(1).map_err(Failure::Storage)?;
        progress.phase=AppendPhase::Words;self.words.push(0x47c35280);progress.words_after=self.words.len();Ok(())
    }

    pub fn fill_candidate_with_comparisons<S:Services>(&self,service:&mut S)->Result<bool,Failure<S::Error>> {
        let mut at=0;
        while at<self.words.len() {
            let token=self.words[at];let word_index=at;
            let mut equal=|marker|compare(service,Site::FillToken {word_index,marker},Relation::OrderedEqual,token,marker);
            if equal(0x47c35100)? {
                at=at.checked_add(3).ok_or(Failure::Storage(Error::MalformedStream))?;
                if at>self.words.len() {return Err(Failure::Storage(Error::MalformedStream));}
            }else if equal(0x47c35200)?||equal(0x47c35080)?||equal(0x47c35180)? {
                return Ok(true);
            }else {at+=1;}
        }
        Ok(false)
    }
}

pub(super) struct ExistingHostValueComparisons;
impl Services for ExistingHostValueComparisons {
    type Error=core::convert::Infallible;
    fn compare(&mut self,request:Request)->Result<bool,Self::Error> {
        Ok(existing_host_relation(request.relation,request.left,request.right))
    }
}

pub(crate) fn existing_host_relation(relation:Relation,left:u32,right:u32)->bool {
    let left=f32::from_bits(left);let right=f32::from_bits(right);
    match relation {Relation::OrderedGreater=>left>right,Relation::OrderedEqual=>left==right,
        Relation::OrderedLessEqual=>left<=right,Relation::OrderedGreaterEqual=>left>=right}
}
impl PackedServices for ExistingHostValueComparisons {
    fn compare_equal_xmm(&mut self,request:PackedRequest)->Result<[u32;4],Self::Error> {

        let mut result=[0;4];
        for lane in 0..4 {
            let equal=self.compare(Request {site:request.site,relation:Relation::OrderedEqual,
                left:request.left[lane],right:request.right[lane]})?;
            result[lane]=if equal {u32::MAX}else {0};
        }
        Ok(result)
    }
}
pub(super) fn legacy_error(failure:Failure<core::convert::Infallible>)->Error {
    match failure {Failure::Storage(error)=>error,Failure::Comparison {error,..}=>match error {}}
}
