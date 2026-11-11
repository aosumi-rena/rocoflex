use super::{comparison,Error,Path};
#[path = "ellipse_scalar_binding.rs"] mod scalar_binding;
pub(crate) use self::scalar_binding::bind_source_scalar_endpoint;

pub const HALF:u32=0x3f000000;
pub const CONTROL:u32=0x3f0ccccd;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Operation {Mul {destination:u32,source:u32},Add {destination:u32,source:u32},Sub {destination:u32,source:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Value {HalfWidth,ControlX,HalfHeight,ControlY,CenterX,CenterY,Top,Right,UpperY,RightControl,Bottom,LowerY,Left,LeftControl,
    RectangleRight,RectangleBottom}
pub trait Fp {
    type Error;

    fn operation(&mut self,value:Value,operation:Operation)->Result<u32,Self::Error>;

    fn comparison(&mut self,request:comparison::Request)->Result<bool,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {Arithmetic(Value),Move,Cubic(u8),Close,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {

    pub step:Step,pub operation:Option<Operation>,pub values:[Option<u32>;16],
    pub completed_arithmetic:u8,pub completed_commands:u8,
    pub comparison:Option<comparison::Request>,pub completed_comparisons:u32,
    pub append:Option<comparison::AppendProgress>,
}
#[derive(Debug)]
pub enum Cause<E> {Arithmetic(E),Path(Error),Comparison {request:comparison::Request,error:E}}
#[derive(Debug)]
#[must_use="caller keeps SAME partial path and advanced FP provider; no automatic retry"]
pub struct Failure<E> {pub progress:Progress,pub cause:Cause<E>}
#[derive(Debug)]
#[must_use="partial path/backing survives construction failure"]
pub struct BuildFailure<E> {pub partial:Path,pub failure:Failure<E>}

struct Compared<'a,P> {fp:&'a mut P,progress:&'a mut Progress}
impl<P:Fp> comparison::Services for Compared<'_,P> {
    type Error=P::Error;
    fn compare(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        self.progress.comparison=Some(request);
        let result=self.fp.comparison(request)?;
        self.progress.completed_comparisons=self.progress.completed_comparisons.wrapping_add(1);
        Ok(result)
    }
}

pub fn append_with_fp<P:Fp>(path:&mut Path,rectangle:[u32;4],fp:&mut P)->Result<Progress,Failure<P::Error>> {
    let mut progress=Progress {step:Step::Arithmetic(Value::HalfWidth),operation:None,
        values:[None;16],completed_arithmetic:0,completed_commands:0,
        comparison:None,completed_comparisons:0,append:None};
    let result=(||->Result<(),Cause<P::Error>> {
        macro_rules! arithmetic {($value:ident,$operation:expr)=>{{
            let value=Value::$value;let operation=$operation;
            progress.step=Step::Arithmetic(value);progress.operation=Some(operation);
            let bits=fp.operation(value,operation).map_err(Cause::Arithmetic)?;
            progress.values[value as usize]=Some(bits);progress.completed_arithmetic+=1;bits
        }};}
        macro_rules! command {($step:expr,$method:ident $(,$arg:expr)*)=>{{
            progress.step=$step;progress.operation=None;progress.append=None;
            let result={
                let mut compared=Compared {fp:&mut *fp,progress:&mut progress};
                path.$method($($arg,)* &mut compared)
            };
            let result=match result {Ok(append)=>{progress.append=Some(append);Ok(())},Err(failure)=>{
                progress.append=Some(failure.progress);Err(failure.cause)
            }};
            result.map_err(|failure|match failure {
                comparison::Failure::Storage(error)=>Cause::Path(error),
                comparison::Failure::Comparison {request,error}=>Cause::Comparison {request,error},
            })?;
            progress.completed_commands+=1;
        }};}
        use Operation::*;
        let half_w=arithmetic!(HalfWidth,Mul {destination:rectangle[2],source:HALF});
        let control_x=arithmetic!(ControlX,Mul {destination:half_w,source:CONTROL});
        let half_h=arithmetic!(HalfHeight,Mul {destination:HALF,source:rectangle[3]});
        let control_y=arithmetic!(ControlY,Mul {destination:CONTROL,source:half_h});
        let cx=arithmetic!(CenterX,Add {destination:rectangle[0],source:half_w});
        let cy=arithmetic!(CenterY,Add {destination:rectangle[1],source:half_h});
        let top=arithmetic!(Top,Sub {destination:cy,source:half_h});
        let point=|x,y|[f32::from_bits(x),f32::from_bits(y)];
        command!(Step::Move,start_with_comparisons_retaining,point(cx,top));
        let right=arithmetic!(Right,Add {destination:half_w,source:cx});
        let upper_y=arithmetic!(UpperY,Sub {destination:cy,source:control_y});
        let right_x=arithmetic!(RightControl,Add {destination:control_x,source:cx});
        command!(Step::Cubic(0),cubic_with_comparisons_retaining,point(right_x,top),point(right,upper_y),point(right,cy));
        let bottom=arithmetic!(Bottom,Add {destination:half_h,source:cy});
        let lower_y=arithmetic!(LowerY,Add {destination:control_y,source:cy});
        command!(Step::Cubic(1),cubic_with_comparisons_retaining,point(right,lower_y),point(right_x,bottom),point(cx,bottom));
        let left=arithmetic!(Left,Sub {destination:cx,source:half_w});
        let left_x=arithmetic!(LeftControl,Sub {destination:cx,source:control_x});
        command!(Step::Cubic(2),cubic_with_comparisons_retaining,point(left_x,bottom),point(left,lower_y),point(left,cy));
        command!(Step::Cubic(3),cubic_with_comparisons_retaining,point(left,upper_y),point(left_x,top),point(cx,top));

        command!(Step::Close,close_with_comparisons_retaining);
        progress.step=Step::Complete;Ok(())
    })();
    match result {Ok(())=>Ok(progress),Err(cause)=>Err(Failure {progress,cause})}
}

pub fn build_with_fp<P:Fp>(rectangle:[u32;4],fp:&mut P)->Result<Path,BuildFailure<P::Error>> {
    let mut path=Path {words:Vec::new(),source_capacity:0,bounds:[0.0;4],source_flag_32:1};
    match append_with_fp(&mut path,rectangle,fp) {
        Ok(_)=>Ok(path),Err(failure)=>Err(BuildFailure {partial:path,failure}),
    }
}

pub(crate) struct ExistingHostValueFp;
impl comparison::Services for ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn compare(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        comparison::Services::compare(&mut comparison::ExistingHostValueComparisons,request)
    }
}
impl comparison::PackedServices for ExistingHostValueFp {
    fn compare_equal_xmm(&mut self,request:comparison::PackedRequest)->Result<[u32;4],Self::Error> {
        comparison::PackedServices::compare_equal_xmm(&mut comparison::ExistingHostValueComparisons,request)
    }
}
impl Fp for ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn operation(&mut self,_value:Value,operation:Operation)->Result<u32,Self::Error> {
        Ok(existing_host_operation(operation))
    }
    fn comparison(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        comparison::Services::compare(&mut comparison::ExistingHostValueComparisons,request)
    }
}

pub(crate) fn existing_host_operation(operation:Operation)->u32 {
    match operation {
        Operation::Mul {destination,source}=>(f32::from_bits(destination)*f32::from_bits(source)).to_bits(),
        Operation::Add {destination,source}=>(f32::from_bits(destination)+f32::from_bits(source)).to_bits(),
        Operation::Sub {destination,source}=>(f32::from_bits(destination)-f32::from_bits(source)).to_bits(),
    }
}
