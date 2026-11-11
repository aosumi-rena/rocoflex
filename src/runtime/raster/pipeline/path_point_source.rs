use super::{comparison,ellipse_source,Error,LineIterator,Path};
pub use ellipse_source::Operation;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Role {Ordinary,QuadraticControl,QuadraticEnd,CubicFirst,CubicSecond,CubicEnd}
impl Role {fn updates_current(self)->bool {matches!(self,Self::Ordinary|Self::QuadraticControl|Self::CubicFirst)}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Value {FirstYProduct,SecondYProduct,YSum,Y,FirstXProduct,SecondXProduct,XSum,X}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub role:Role,pub value:Value,pub operation:Operation}
pub trait Fp {
    type Error;

    fn point_operation(&mut self,request:Request)->Result<u32,Self::Error>;
}
#[derive(Clone,Copy,Debug)]
pub struct IdentityProgress {
    pub affine:[u32;6],pub path_word_count:usize,
    pub completed:u8,pub attempted:Option<comparison::Request>,pub identity:Option<bool>,
    pub packed_attempt:Option<comparison::PackedRequest>,pub packed_masks:[Option<[u32;4]>;2],
    pub combined_mask:Option<[u32;4]>,pub scalar_results:[Option<bool>;4],
}
#[derive(Debug)]
pub enum ConstructionCause<E> {Storage(Error),Comparison {request:comparison::Request,error:E},
    PackedComparison {request:comparison::PackedRequest,error:E}}
#[derive(Debug)]
pub struct ConstructionFailure<E> {pub progress:IdentityProgress,pub cause:ConstructionCause<E>}
pub struct ConstructionCompleted<'p> {pub walker:LineIterator<'p>,pub progress:IdentityProgress}
#[derive(Clone,Copy,Debug)]
pub struct ControllerBoundary {pub cursor:usize,pub stack:usize,pub current:[u32;2]}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub role:Role,pub stacked:bool,pub identity:bool,
    pub cursor_before:usize,pub cursor_after:usize,pub stack_before:usize,pub stack_after:usize,
    pub raw:[Option<u32>;2],pub values:[Option<u32>;8],
    pub current_before:[u32;2],pub current_after:[u32;2],

    pub transform_before:Option<ControllerBoundary>,
    pub attempted:Option<Request>,pub completed:u8,
}
#[derive(Debug)]
pub enum Cause<E> {Storage(Error),Arithmetic(E)}
#[derive(Debug)]
#[must_use="SAME iterator cursor/stack and caller FP owner remain advanced on failure"]
pub struct Failure<E> {pub progress:Progress,pub cause:Cause<E>}
#[derive(Debug)]
pub struct ReadFailure {pub progress:Progress,pub error:Error}
pub struct ReadCompleted {point:[u32;2],progress:Progress}
impl ReadCompleted {
    pub fn point(&self)->[u32;2] {self.point}
    pub fn progress(&self)->&Progress {&self.progress}
}
pub struct TransformedCompleted {point:[u32;2],progress:Progress}
impl TransformedCompleted {
    pub fn point(&self)->[u32;2] {self.point}
    pub fn progress(&self)->&Progress {&self.progress}
}

impl<'a> LineIterator<'a> {
    pub fn new_source_with_comparisons<S:comparison::PackedServices>(path:&'a Path,affine:[u32;6],service:&mut S)
        ->Result<Self,ConstructionFailure<S::Error>>
    {
        Self::new_source_with_comparisons_retaining(path,affine,service).map(|completed|completed.walker)
    }

    pub fn new_source_with_comparisons_retaining<S:comparison::PackedServices>(path:&'a Path,affine:[u32;6],service:&mut S)
        ->Result<ConstructionCompleted<'a>,ConstructionFailure<S::Error>>
    {
        let mut progress=IdentityProgress {affine,path_word_count:path.words.len(),completed:0,attempted:None,identity:None,packed_attempt:None,
            packed_masks:[None;2],combined_mask:None,scalar_results:[None;4]};
        let result=(||->Result<Self,ConstructionCause<S::Error>> {
            if path.words.len()>i32::MAX as usize {return Err(ConstructionCause::Storage(Error::Count));}

            let b_request=comparison::PackedRequest {site:comparison::Site::AffineIdentity {coefficient:1},
                left:[affine[1],0,0,0],right:[0;4]};progress.packed_attempt=Some(b_request);
            let b_mask=service.compare_equal_xmm(b_request)
                .map_err(|error|ConstructionCause::PackedComparison {request:b_request,error})?;
            progress.packed_masks[0]=Some(b_mask);progress.completed+=1;
            let a_request=comparison::PackedRequest {site:comparison::Site::AffineIdentity {coefficient:0},
                left:[affine[0],0,0,0],right:[0x3f800000,0,0,0]};progress.packed_attempt=Some(a_request);
            let a_mask=service.compare_equal_xmm(a_request)
                .map_err(|error|ConstructionCause::PackedComparison {request:a_request,error})?;
            progress.packed_masks[1]=Some(a_mask);progress.completed+=1;
            let mask:[u32;4]=core::array::from_fn(|lane|a_mask[lane]&b_mask[lane]);progress.combined_mask=Some(mask);
            let mut identity=mask[0]&1!=0;

            for (slot,(coefficient,left,right)) in [(2,0,affine[2]),(3,0,affine[3]),
                (4,affine[4],0x3f800000),(5,0,affine[5])].into_iter().enumerate() {
                let request=comparison::Request {site:comparison::Site::AffineIdentity {coefficient},
                    relation:comparison::Relation::OrderedEqual,left,right};
                progress.attempted=Some(request);
                let equal=service.compare(request).map_err(|error|ConstructionCause::Comparison {request,error})?;
                progress.scalar_results[slot]=Some(equal);progress.completed+=1;identity&=equal;
            }
            progress.identity=Some(identity);
            let mut stack=Vec::new();
            stack.try_reserve_exact(32).map_err(|_|ConstructionCause::Storage(Error::Allocation))?;
            Ok(Self {path,cursor:0,affine:affine.map(f32::from_bits),identity,current:[0.0;2],subpath_start:[0.0;2],
                segment_index:-1,stack,source_stack_capacity:32,source_tolerance_squared:f32::from_bits(0x3eb851ec)})
        })();
        match result {
            Ok(walker)=>Ok(ConstructionCompleted {walker,progress}),
            Err(cause)=>Err(ConstructionFailure {progress,cause}),
        }
    }

    pub fn read_point_source(&mut self,stacked:bool,role:Role)->Result<ReadCompleted,ReadFailure> {
        let mut progress=Progress {role,stacked,identity:self.identity,cursor_before:self.cursor,cursor_after:self.cursor,
            stack_before:self.stack.len(),stack_after:self.stack.len(),raw:[None;2],values:[None;8],
            current_before:self.current.map(f32::to_bits),current_after:self.current.map(f32::to_bits),
            transform_before:None,attempted:None,completed:0};
        let result=(||->Result<[u32;2],Error> {
            let mut point=[0;2];
            for axis in 0..2 {
                let word=if stacked {self.stack.pop().ok_or(Error::MalformedStream)?}else {
                    let cursor=self.cursor;self.cursor=self.cursor.checked_add(1).ok_or(Error::MalformedStream)?;
                    *self.path.words.get(cursor).ok_or(Error::MalformedStream)?
                };
                progress.raw[axis]=Some(word);point[axis]=word;
                if role.updates_current() {self.current[axis]=f32::from_bits(word);}
                progress.stack_after=self.stack.len();progress.cursor_after=self.cursor;
            }
            Ok(point)
        })();
        progress.cursor_after=self.cursor;progress.stack_after=self.stack.len();progress.current_after=self.current.map(f32::to_bits);
        match result {Ok(point)=>Ok(ReadCompleted {point,progress}),Err(error)=>Err(ReadFailure {progress,error})}
    }

    pub fn finish_point_with_fp<P:Fp>(&mut self,read:ReadCompleted,role:Role,fp:&mut P)->Result<[f32;2],Failure<P::Error>> {
        self.finish_point_with_fp_retaining(read,role,fp).map(|completed|completed.point.map(f32::from_bits))
    }
    pub fn finish_point_with_fp_retaining<P:Fp>(&mut self,read:ReadCompleted,role:Role,fp:&mut P)
        ->Result<TransformedCompleted,Failure<P::Error>>
    {
        let ReadCompleted {point:[x,y],mut progress}=read;progress.role=role;
        progress.transform_before=Some(ControllerBoundary {cursor:self.cursor,stack:self.stack.len(),current:self.current.map(f32::to_bits)});
        let result=(||->Result<[f32;2],Cause<P::Error>> {
            if progress.stacked||self.identity {return Ok([f32::from_bits(x),f32::from_bits(y)]);}
            let [a,b,tx,c,d,ty]=self.affine.map(f32::to_bits);
            macro_rules! op {($value:ident,$operation:expr)=>{{
                let value=Value::$value;let operation=$operation;
                let request=Request {role,value,operation};progress.attempted=Some(request);
                let bits=fp.point_operation(request).map_err(Cause::Arithmetic)?;
                progress.values[value as usize]=Some(bits);progress.completed+=1;bits
            }};}
            use Operation::{Add,Mul};

            let by=match role {Role::QuadraticControl|Role::CubicFirst|Role::CubicSecond=>(y,b),_=>(b,y)};
            let sx=op!(SecondXProduct,Mul {destination:by.0,source:by.1});
            let fx=op!(FirstXProduct,Mul {destination:a,source:x});
            let sum_x=op!(XSum,Add {destination:fx,source:sx});
            let out_x=op!(X,Add {destination:sum_x,source:tx});
            if role.updates_current() {self.current[0]=f32::from_bits(out_x);}
            let fy=op!(FirstYProduct,Mul {destination:y,source:d});
            let xc=match role {Role::QuadraticEnd|Role::CubicEnd=>(c,x),_=>(x,c)};
            let sy=op!(SecondYProduct,Mul {destination:xc.0,source:xc.1});
            let (left,right)=match role {
                Role::CubicSecond=>(fy,sy),_=>(sy,fy),
            };
            let sum_y=op!(YSum,Add {destination:left,source:right});
            let out_y=op!(Y,Add {destination:sum_y,source:ty});
            if role.updates_current() {self.current[1]=f32::from_bits(out_y);}
            Ok([f32::from_bits(out_x),f32::from_bits(out_y)])
        })();
        progress.cursor_after=self.cursor;progress.stack_after=self.stack.len();
        progress.current_after=self.current.map(f32::to_bits);
        match result {
            Ok(point)=>Ok(TransformedCompleted {point:point.map(f32::to_bits),progress}),
            Err(cause)=>Err(Failure {progress,cause}),
        }
    }
    pub fn point_with_fp<P:Fp>(&mut self,stacked:bool,role:Role,fp:&mut P)->Result<[f32;2],Failure<P::Error>> {
        let read=self.read_point_source(stacked,role).map_err(|failure|Failure {progress:failure.progress,cause:Cause::Storage(failure.error)})?;
        self.finish_point_with_fp(read,role,fp)
    }
}
pub(crate) fn legacy_construction_error(failure:ConstructionFailure<core::convert::Infallible>)->Error {
    match failure.cause {ConstructionCause::Storage(error)=>error,
        ConstructionCause::Comparison {error,..}|ConstructionCause::PackedComparison {error,..}=>match error {}}
}
impl Fp for ellipse_source::ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn point_operation(&mut self,request:Request)->Result<u32,Self::Error> {
        Ok(ellipse_source::existing_host_operation(request.operation))
    }
}
