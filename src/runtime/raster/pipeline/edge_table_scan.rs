use super::path;
use super::super::{cvtt_i32,cvtt_i64};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Endpoint {From,To}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Site {
    CountWide,CountSqrt,CountInteger,
    YHalf(Endpoint),YScale(Endpoint),YBias(Endpoint),YInteger(Endpoint),
    DeltaX,StartXScale,StartXWide,DeltaY,Slope,SlopeWide,SlopeInteger,
    MidpointWide,SampleMul,SampleAdd,SampleInteger,
    CountSqrtGuard,CountSqrtCrt,
}
impl Site {
    pub const VALUE_COUNT:usize=24;
    pub(super) fn index(self)->usize {match self {
        Self::CountWide=>0,Self::CountSqrt=>1,Self::CountInteger=>2,
        Self::YHalf(endpoint)=>3+4*endpoint.index(),Self::YScale(endpoint)=>4+4*endpoint.index(),
        Self::YBias(endpoint)=>5+4*endpoint.index(),Self::YInteger(endpoint)=>6+4*endpoint.index(),
        Self::DeltaX=>11,Self::StartXScale=>12,Self::StartXWide=>13,Self::DeltaY=>14,
        Self::Slope=>15,Self::SlopeWide=>16,Self::SlopeInteger=>17,
        Self::MidpointWide=>18,Self::SampleMul=>19,Self::SampleAdd=>20,Self::SampleInteger=>21,
        Self::CountSqrtGuard=>22,Self::CountSqrtCrt=>23,
    }}

    pub(super) fn clear_line_values(values:&mut [Option<Value>;Self::VALUE_COUNT]) {values[3..22].fill(None);}
    pub(super) fn clear_sample_values(values:&mut [Option<Value>;Self::VALUE_COUNT]) {values[18..22].fill(None);}
}
impl Endpoint {pub(super) fn index(self)->usize {match self {Self::From=>0,Self::To=>1}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum F32Operation {
    Arithmetic(path::ellipse_source::Operation),Divide {destination:u32,source:u32},
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum F64Operation {Sqrt {source:u64},Mul {destination:u64,source:u64},Add {destination:u64,source:u64}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct F64Comparison {pub left:u64,pub right:u64}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ComparisonFlags {pub carry:bool,pub parity:bool,pub zero:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CountSqrtCrtCall {pub argument:u64}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CountSqrtBranch {Fast,Crt}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Predicate {OrderedLessEqual {left:u32,right:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum IntegerSource {Signed32(i32),Signed64(i64)}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Integer32Operation {

    CountResult {source:u64},Slope {source:u64},
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Integer64Operation {FromF32 {source:u32},FromF64 {source:u64}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Operation {
    F32(F32Operation),F64(F64Operation),Predicate(Predicate),F32ToF64(u32),
    IntegerToF64(IntegerSource),Integer32(Integer32Operation),Integer64(Integer64Operation),
    F64Comparison(F64Comparison),CountSqrtCrt(CountSqrtCrtCall),
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub site:Site,pub operation:Operation}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Value {F32(u32),F64(u64),Integer32(i32),Integer64(i64),Predicate(bool),Flags(ComparisonFlags)}
#[derive(Debug)]
pub struct Failure<E> {pub request:Request,pub error:E}
pub trait ScanServices {
    type Error;

    fn f32(&mut self,site:Site,operation:F32Operation)->Result<u32,Self::Error>;
    fn f64(&mut self,site:Site,operation:F64Operation)->Result<u64,Self::Error>;

    fn f64_comparison(&mut self,site:Site,operation:F64Comparison)->Result<ComparisonFlags,Self::Error>;

    fn count_sqrt_crt(&mut self,site:Site,call:CountSqrtCrtCall)->Result<u64,Self::Error>;
    fn predicate(&mut self,site:Site,operation:Predicate)->Result<bool,Self::Error>;
    fn widen(&mut self,site:Site,source:u32)->Result<u64,Self::Error>;
    fn integer_wide(&mut self,site:Site,source:IntegerSource)->Result<u64,Self::Error>;
    fn integer32(&mut self,site:Site,operation:Integer32Operation)->Result<i32,Self::Error>;
    fn integer64(&mut self,site:Site,operation:Integer64Operation)->Result<i64,Self::Error>;
}

impl ScanServices for path::ellipse_source::ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn f32(&mut self,_site:Site,operation:F32Operation)->Result<u32,Self::Error> {
        Ok(match operation {
            F32Operation::Arithmetic(operation)=>path::ellipse_source::existing_host_operation(operation),
            F32Operation::Divide {destination,source}=>(f32::from_bits(destination)/f32::from_bits(source)).to_bits(),
        })
    }
    fn f64(&mut self,_site:Site,operation:F64Operation)->Result<u64,Self::Error> {
        Ok(match operation {
            F64Operation::Sqrt {source}=>f64::from_bits(source).sqrt().to_bits(),
            F64Operation::Mul {destination,source}=>(f64::from_bits(destination)*f64::from_bits(source)).to_bits(),
            F64Operation::Add {destination,source}=>(f64::from_bits(destination)+f64::from_bits(source)).to_bits(),
        })
    }
    fn predicate(&mut self,_site:Site,operation:Predicate)->Result<bool,Self::Error> {
        let Predicate::OrderedLessEqual {left,right}=operation;
        Ok(path::comparison::existing_host_relation(path::comparison::Relation::OrderedLessEqual,left,right))
    }
    fn f64_comparison(&mut self,_site:Site,operation:F64Comparison)->Result<ComparisonFlags,Self::Error> {

        let relation=f64::from_bits(operation.left).partial_cmp(&f64::from_bits(operation.right));
        Ok(match relation {
            Some(core::cmp::Ordering::Less)=>ComparisonFlags {carry:true,parity:false,zero:false},
            Some(core::cmp::Ordering::Equal)=>ComparisonFlags {carry:false,parity:false,zero:true},
            Some(core::cmp::Ordering::Greater)=>ComparisonFlags {carry:false,parity:false,zero:false},
            None=>ComparisonFlags {carry:true,parity:true,zero:true},
        })
    }
    fn count_sqrt_crt(&mut self,site:Site,call:CountSqrtCrtCall)->Result<u64,Self::Error> {

        self.f64(site,F64Operation::Sqrt {source:call.argument})
    }
    fn widen(&mut self,_site:Site,source:u32)->Result<u64,Self::Error> {Ok((f32::from_bits(source) as f64).to_bits())}
    fn integer_wide(&mut self,_site:Site,source:IntegerSource)->Result<u64,Self::Error> {
        Ok(match source {IntegerSource::Signed32(value)=>(value as f64).to_bits(),IntegerSource::Signed64(value)=>(value as f64).to_bits()})
    }
    fn integer32(&mut self,_site:Site,operation:Integer32Operation)->Result<i32,Self::Error> {
        Ok(match operation {
            Integer32Operation::CountResult {source}=>f64::from_bits(source) as i32,

            Integer32Operation::Slope {source}=>cvtt_i32(f64::from_bits(source) as f32),
        })
    }
    fn integer64(&mut self,_site:Site,operation:Integer64Operation)->Result<i64,Self::Error> {
        Ok(match operation {
            Integer64Operation::FromF32 {source}=>cvtt_i64(f32::from_bits(source) as f64),
            Integer64Operation::FromF64 {source}=>cvtt_i64(f64::from_bits(source)),
        })
    }
}
