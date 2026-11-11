use super::{comparison,ellipse_source,Error,LineIterator};

#[path="../../numeric/scalar_f32_comparison.rs"]
mod source_classifier;
pub use comparison::{CurveKind,NearPhase};
pub use ellipse_source::Operation;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ArithmeticSite {
    MidpointAdd {point:u8,axis:u8},MidpointHalf {point:u8,axis:u8},
    Delta {half:u8,axis:u8},Square {half:u8,axis:u8},Distance {half:u8},
    NearSubtract {kind:CurveKind,pair:u8},NearScale {kind:CurveKind,pair:u8},
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub kind:CurveKind,pub site:ArithmeticSite,pub operation:Operation}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct MaximumRequest {pub kind:CurveKind,pub pair:u8,pub left:u32,pub right:u32}
pub trait Fp {
    type Error;

    fn curve_operation(&mut self,request:Request)->Result<u32,Self::Error>;
    fn curve_comparison(&mut self,request:comparison::Request)->Result<bool,Self::Error>;

    fn near_maximum(&mut self,request:MaximumRequest)->Result<u32,Self::Error>;
}
#[derive(Clone,Copy,Debug)]
pub struct NearProgress {
    pub kind:CurveKind,pub pair:u8,pub input:[u32;2],pub exceptional:bool,
    pub classifications:[Option<i16>;2],
    pub operation:Option<Request>,pub comparison:Option<comparison::Request>,
    pub maximum_request:Option<MaximumRequest>,pub difference:Option<u32>,pub absolute_difference:Option<u32>,
    pub absolute_test:Option<bool>,pub maximum:Option<u32>,pub scaled:Option<u32>,pub result:Option<bool>,
}
#[derive(Debug)]
pub enum NumericCause<E> {
    Arithmetic {request:Request,error:E},
    Comparison {request:comparison::Request,error:E},Maximum {request:MaximumRequest,error:E},
}
#[derive(Debug)]
#[must_use="SAME supplied FP owner remains advanced; no automatic retry"]
pub struct NearFailure<E> {pub progress:NearProgress,pub cause:NumericCause<E>}
#[derive(Clone,Copy,Debug)]
pub struct NearCompleted {pub progress:NearProgress,pub result:bool}

fn compare<P:Fp>(fp:&mut P,request:comparison::Request)->Result<bool,NumericCause<P::Error>> {
    fp.curve_comparison(request).map_err(|error|NumericCause::Comparison {request,error})
}
fn arithmetic<P:Fp>(fp:&mut P,request:Request)->Result<u32,NumericCause<P::Error>> {
    fp.curve_operation(request).map_err(|error|NumericCause::Arithmetic {request,error})
}

pub fn near_with_fp<P:Fp>(kind:CurveKind,pair:u8,first:u32,second:u32,fp:&mut P)
    ->Result<NearCompleted,NearFailure<P::Error>>
{
    let mut progress=NearProgress {kind,pair,input:[first,second],exceptional:false,classifications:[None;2],operation:None,comparison:None,
        maximum_request:None,difference:None,absolute_difference:None,absolute_test:None,maximum:None,scaled:None,result:None};
    let result=(||->Result<bool,NumericCause<P::Error>> {

        let first_class=source_classifier::fdtest_f32_low_word(first);progress.classifications[0]=Some(first_class);
        let exceptional=if first_class>0 {true}else {
            let second_class=source_classifier::fdtest_f32_low_word(second);progress.classifications[1]=Some(second_class);second_class>0
        };progress.exceptional=exceptional;
        if exceptional {
            let request=comparison::Request {site:comparison::Site::CurveNear {kind,pair,phase:NearPhase::ExceptionalEqual},
                relation:comparison::Relation::OrderedEqual,left:first,right:second};progress.comparison=Some(request);
            return compare(fp,request);
        }
        let site=ArithmeticSite::NearSubtract {kind,pair};let operation=Operation::Sub {destination:first,source:second};
        let request=Request {kind,site,operation};progress.operation=Some(request);
        let difference=arithmetic(fp,request)?;progress.difference=Some(difference);
        let delta=difference&0x7fffffff;progress.absolute_difference=Some(delta);
        let request=comparison::Request {site:comparison::Site::CurveNear {kind,pair,phase:NearPhase::AbsoluteThreshold},
            relation:comparison::Relation::OrderedLessEqual,left:delta,right:0x00800000};progress.comparison=Some(request);
        let absolute=compare(fp,request)?;progress.absolute_test=Some(absolute);
        if absolute {return Ok(true);}
        let request=MaximumRequest {kind,pair,left:first&0x7fffffff,right:second&0x7fffffff};
        progress.maximum_request=Some(request);
        let maximum=fp.near_maximum(request).map_err(|error|NumericCause::Maximum {request,error})?;progress.maximum=Some(maximum);
        let site=ArithmeticSite::NearScale {kind,pair};let operation=Operation::Mul {destination:maximum,source:0x34000000};
        let request=Request {kind,site,operation};progress.operation=Some(request);
        let scaled=arithmetic(fp,request)?;progress.scaled=Some(scaled);
        let request=comparison::Request {site:comparison::Site::CurveNear {kind,pair,phase:NearPhase::RelativeThreshold},
            relation:comparison::Relation::OrderedGreaterEqual,left:scaled,right:delta};progress.comparison=Some(request);
        compare(fp,request)
    })();
    match result {Ok(result)=>{progress.result=Some(result);Ok(NearCompleted {progress,result})},Err(cause)=>Err(NearFailure {progress,cause})}
}

#[derive(Clone,Copy,Debug)]
pub enum Input {Quadratic {begin:[u32;2],control:[u32;2],end:[u32;2]},
    Cubic {begin:[u32;2],first:[u32;2],second:[u32;2],end:[u32;2]}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {Grow,Arithmetic(ArithmeticSite),Near(u8),DistanceCompare(u8),Push(u8),Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub input:Input,pub step:Step,pub cursor:usize,pub current:[u32;2],pub tolerance:u32,
    pub stack_before:usize,pub stack_after:usize,pub capacity_before:usize,pub capacity_after:usize,
    pub attempted:Option<Request>,pub comparison:Option<comparison::Request>,
    pub midpoint_sums:[[Option<u32>;2];6],pub midpoints:[[Option<u32>;2];6],
    pub deltas:[[Option<u32>;2];2],pub squares:[[Option<u32>;2];2],pub distances:[Option<u32>;2],
    pub distance_tests:[Option<bool>;2],pub near:[Option<NearProgress>;8],pub degenerate:Option<bool>,
    pub flattened:Option<bool>,pub pushes:u8,
}
#[derive(Debug)]
pub enum Cause<E> {Numeric(NumericCause<E>),Storage(Error)}
#[derive(Debug)]
#[must_use="SAME iterator stack/capacity and FP owner retain all partial effects"]
pub struct Failure<E> {pub progress:Progress,pub cause:Cause<E>}

struct Graph<'a,P> {fp:&'a mut P,progress:&'a mut Progress,kind:CurveKind}
impl<P:Fp> Graph<'_,P> {
    fn operation(&mut self,site:ArithmeticSite,operation:Operation)->Result<u32,Cause<P::Error>> {
        let request=Request {kind:self.kind,site,operation};
        self.progress.step=Step::Arithmetic(site);self.progress.attempted=Some(request);
        arithmetic(self.fp,request).map_err(Cause::Numeric)
    }
    fn midpoint(&mut self,point:u8,first:[u32;2],second:[u32;2])->Result<[u32;2],Cause<P::Error>> {
        let mut out=[0;2];
        for axis in 0..2 {
            out[axis]=self.midpoint_axis(point,axis,first[axis],second[axis])?;
        }
        Ok(out)
    }
    fn midpoint_axis(&mut self,point:u8,axis:usize,first:u32,second:u32)->Result<u32,Cause<P::Error>> {
        let sum=self.operation(ArithmeticSite::MidpointAdd {point,axis:axis as u8},Operation::Add {destination:first,source:second})?;
        self.progress.midpoint_sums[point as usize][axis]=Some(sum);
        let value=self.operation(ArithmeticSite::MidpointHalf {point,axis:axis as u8},Operation::Mul {destination:sum,source:0x3f000000})?;
        self.progress.midpoints[point as usize][axis]=Some(value);Ok(value)
    }
    fn near(&mut self,pair:u8,first:u32,second:u32)->Result<bool,Cause<P::Error>> {
        self.progress.step=Step::Near(pair);
        match near_with_fp(self.kind,pair,first,second,&mut *self.fp) {
            Ok(completed)=>{
                self.progress.near[pair as usize]=Some(completed.progress);Ok(completed.result)
            },
            Err(failure)=>{
                self.progress.near[pair as usize]=Some(failure.progress);Err(Cause::Numeric(failure.cause))
            },
        }
    }
    fn distance(&mut self,half:u8,point:[u32;2],control:[u32;2],y_first:bool)->Result<bool,Cause<P::Error>> {
        let mut delta=[0;2];

        for axis in if y_first {[1,0]}else {[0,1]} {
            delta[axis]=self.delta_axis(half,axis,point[axis],control[axis])?;
        }
        let mut square=[0;2];
        for axis in [1,0] {
            square[axis]=self.square_axis(half,axis,delta[axis])?;
        }
        self.distance_compare(half,square)
    }
    fn delta_axis(&mut self,half:u8,axis:usize,point:u32,control:u32)->Result<u32,Cause<P::Error>> {
        let value=self.operation(ArithmeticSite::Delta {half,axis:axis as u8},Operation::Sub {destination:point,source:control})?;
        self.progress.deltas[half as usize][axis]=Some(value);Ok(value)
    }
    fn square_axis(&mut self,half:u8,axis:usize,delta:u32)->Result<u32,Cause<P::Error>> {
        let value=self.operation(ArithmeticSite::Square {half,axis:axis as u8},Operation::Mul {destination:delta,source:delta})?;
        self.progress.squares[half as usize][axis]=Some(value);Ok(value)
    }
    fn distance_compare(&mut self,half:u8,square:[u32;2])->Result<bool,Cause<P::Error>> {
        let distance=self.operation(ArithmeticSite::Distance {half},Operation::Add {destination:square[0],source:square[1]})?;
        self.progress.distances[half as usize]=Some(distance);
        self.progress.step=Step::DistanceCompare(half);
        let request=comparison::Request {site:comparison::Site::CurveDistance {kind:self.kind,half},
            relation:comparison::Relation::OrderedGreater,left:distance,right:self.progress.tolerance};
        self.progress.comparison=Some(request);
        let result=compare(self.fp,request).map_err(Cause::Numeric)?;
        self.progress.distance_tests[half as usize]=Some(result);Ok(result)
    }
}
impl<'a> LineIterator<'a> {
    fn curve_progress(&self,input:Input)->Progress {
        Progress {input,step:Step::Grow,cursor:self.cursor,current:self.current.map(f32::to_bits),tolerance:self.source_tolerance_squared.to_bits(),
            stack_before:self.stack.len(),stack_after:self.stack.len(),capacity_before:self.source_stack_capacity,capacity_after:self.source_stack_capacity,
            attempted:None,comparison:None,midpoint_sums:[[None;2];6],midpoints:[[None;2];6],deltas:[[None;2];2],squares:[[None;2];2],
            distances:[None;2],distance_tests:[None;2],near:[None;8],degenerate:None,flattened:None,pushes:0}
    }
    fn curve_push<E>(&mut self,progress:&mut Progress,words:&[u32])->Result<(),Cause<E>> {
        progress.step=Step::Push(progress.pushes);self.push_words(words).map_err(Cause::Storage)?;
        progress.pushes+=1;progress.stack_after=self.stack.len();Ok(())
    }
    pub fn quadratic_with_fp<P:Fp>(&mut self,begin:[u32;2],control:[u32;2],end:[u32;2],fp:&mut P)
        ->Result<Progress,Failure<P::Error>>
    {
        let mut progress=self.curve_progress(Input::Quadratic {begin,control,end});
        let result=(||->Result<(),Cause<P::Error>> {
            self.curve_stack(10).map_err(Cause::Storage)?;
            progress.capacity_after=self.source_stack_capacity;
            let mut graph=Graph {fp,progress:&mut progress,kind:CurveKind::Quadratic};
            let q0=graph.midpoint(0,begin,control)?;
            let q1=graph.midpoint(1,end,control)?;
            let middle=graph.midpoint(2,q0,q1)?;
            let same_x=graph.near(0,middle[0],q0[0])?||graph.near(1,middle[0],q1[0])?;

            let mut degenerate=false;let mut direct_flat=false;
            if same_x {
                if graph.near(2,middle[1],q0[1])? {degenerate=true;direct_flat=true;}
                else {
                    degenerate=graph.near(3,middle[1],q1[1])?;
                    direct_flat=graph.progress.near[3].as_ref().is_some_and(|near|near.absolute_test==Some(true));
                }
            }
            graph.progress.degenerate=Some(degenerate);
            let flat=if direct_flat {true}else {!graph.distance(0,middle,control,false)?||degenerate};
            graph.progress.flattened=Some(flat);
            if flat {
                self.curve_push(&mut progress,&[end[1],end[0],0x47c35080,middle[1],middle[0],0x47c35080])?;
            }else {
                self.curve_push(&mut progress,&[end[1],end[0],q1[1],q1[0],0x47c35180,
                    middle[1],middle[0],q0[1],q0[0],0x47c35180])?;
            }
            progress.step=Step::Complete;Ok(())
        })();
        progress.stack_after=self.stack.len();progress.capacity_after=self.source_stack_capacity;
        match result {Ok(())=>Ok(progress),Err(cause)=>Err(Failure {progress,cause})}
    }
    pub fn cubic_with_fp<P:Fp>(&mut self,begin:[u32;2],first:[u32;2],second:[u32;2],end:[u32;2],fp:&mut P)
        ->Result<Progress,Failure<P::Error>>
    {
        let mut progress=self.curve_progress(Input::Cubic {begin,first,second,end});
        let result=(||->Result<(),Cause<P::Error>> {
            self.curve_stack(16).map_err(Cause::Storage)?;
            progress.capacity_after=self.source_stack_capacity;
            let mut graph=Graph {fp:&mut *fp,progress:&mut progress,kind:CurveKind::Cubic};
            let q0=graph.midpoint(0,begin,first)?;
            let q1=graph.midpoint(1,second,first)?;
            let q2=graph.midpoint(2,second,end)?;
            let r0=graph.midpoint(3,q0,q1)?;

            let dx=graph.delta_axis(0,0,r0[0],first[0])?;
            let dy=graph.delta_axis(0,1,r0[1],first[1])?;
            let r1_x=graph.midpoint_axis(4,0,q2[0],q1[0])?;
            let sy=graph.square_axis(0,1,dy)?;
            let sx=graph.square_axis(0,0,dx)?;
            let mut needs_split=graph.distance_compare(0,[sx,sy])?;
            let r1_y=graph.midpoint_axis(4,1,q2[1],q1[1])?;
            let r1=[r1_x,r1_y];
            if !needs_split {needs_split=graph.distance(1,r1,second,true)?;}

            let degenerate=(graph.near(0,r0[0],q0[0])?||graph.near(1,r0[0],q1[0])?)
                &&(graph.near(2,r0[1],q0[1])?||graph.near(3,r0[1],q1[1])?)
                &&(graph.near(4,r1[0],q2[0])?||graph.near(5,r1[0],q1[0])?)
                &&(graph.near(6,r1[1],q2[1])?||graph.near(7,r1[1],q1[1])?);
            graph.progress.degenerate=Some(degenerate);
            let flat=degenerate||!needs_split;graph.progress.flattened=Some(flat);
            if flat {
                self.curve_push(&mut progress,&[end[1],end[0],0x47c35080,r1[1],r1[0],0x47c35080,r0[1],r0[0],0x47c35080])?;
            }else {

                self.curve_push(&mut progress,&[end[1],end[0],q2[1],q2[0],r1[1],r1[0],0x47c35200])?;
                let middle_y=Graph {fp:&mut *fp,progress:&mut progress,kind:CurveKind::Cubic}.midpoint_axis(5,1,r1[1],r0[1])?;
                self.curve_push(&mut progress,&[middle_y])?;
                let middle_x=Graph {fp:&mut *fp,progress:&mut progress,kind:CurveKind::Cubic}.midpoint_axis(5,0,r1[0],r0[0])?;
                self.curve_push(&mut progress,&[middle_x,r0[1],r0[0],q0[1],q0[0],0x47c35200])?;
            }
            progress.step=Step::Complete;Ok(())
        })();
        progress.stack_after=self.stack.len();progress.capacity_after=self.source_stack_capacity;
        match result {Ok(())=>Ok(progress),Err(cause)=>Err(Failure {progress,cause})}
    }
}
impl Fp for ellipse_source::ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn curve_operation(&mut self,request:Request)->Result<u32,Self::Error> {
        Ok(ellipse_source::existing_host_operation(request.operation))
    }
    fn curve_comparison(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        comparison::Services::compare(&mut comparison::ExistingHostValueComparisons,request)
    }
    fn near_maximum(&mut self,request:MaximumRequest)->Result<u32,Self::Error> {

        Ok(f32::from_bits(request.right).max(f32::from_bits(request.left)).to_bits())
    }
}
pub(super) fn legacy_error(failure:Failure<core::convert::Infallible>)->Error {
    match failure.cause {Cause::Storage(error)=>error,Cause::Numeric(cause)=>match cause {
        NumericCause::Arithmetic {error,..}|NumericCause::Comparison {error,..}|NumericCause::Maximum {error,..}=>match error {},
    }}
}
