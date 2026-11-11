pub const X_MAX:u64=0x4001333333333333;
pub const X_MIN:u64=0xc001333333333333;
pub const Y_MAX:u64=0x3ff6666666666666;
pub const Y_MIN:u64=0xbff6666666666666;
pub const HORIZONTAL_EDGE:u64=0x3ffff15f15f15f17;
pub const VERTICAL_EDGE:u64=0x3ff6e147ae147ae2;
pub const CAP:u64=0x3fe6666666666666;
pub const NEGATIVE_THREE_TENTHS:u64=0xbfd3333333333334;
pub const ONE_POINT_THREE:u64=0x3ff4cccccccccccd;
pub const ONE:u64=0x3ff0000000000000;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Operation {
    WidenF32 {bits:u32},Add {left:u64,right:u64},Sub {left:u64,right:u64},
    Mul {left:u64,right:u64},Div {left:u64,right:u64},Min {first:u64,second:u64},
}
pub trait Math {
    type Error;

    fn operation(&mut self,operation:Operation)->Result<u64,Self::Error>;

    fn ordered_less(&mut self,left:u64,right:u64)->Result<bool,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Value {WidenedX,WidenedY,ClampedX,ClampedY,RightDistance,LeftDistance,Horizontal,
    BottomDistance,LowerDistance,TopDistance,Distance,CappedDistance,ScaledDistance,CapRatio,Denominator,Zoom,
    UpperXMinimum,UpperYMinimum}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Site {Value(Value),ClampLowerX,ClampLowerY}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Entry {PublicClamp,PublicZoom,UnclampedZoom,F64Clamp}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Input {Public([Option<u32>;2]),Unclamped([u64;2]),F64Clamp([u64;2])}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Call {Operation(Operation),OrderedLess {left:u64,right:u64}}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub entry:Entry,pub input:Input,pub site:Site,pub attempted:Option<Call>,pub completed_operations:usize,
    pub values:[Option<u64>;18],pub public_pair:Option<[u64;2]>,pub unclamped_pair:Option<[u64;2]>,
    pub missing_pair:bool,pub comparison:Option<bool>,pub clamp_comparisons:[Option<bool>;2],
}
impl Progress {
    fn new(entry:Entry,input:Input)->Self {Self {entry,input,
        site:Site::Value(match entry {Entry::UnclampedZoom=>Value::RightDistance,
            Entry::F64Clamp=>Value::UpperXMinimum,Entry::PublicClamp|Entry::PublicZoom=>Value::WidenedX}),attempted:None,
        completed_operations:0,values:[None;18],public_pair:None,unclamped_pair:None,
        missing_pair:false,comparison:None,clamp_comparisons:[None;2]}}
}
#[derive(Debug)]
#[must_use="caller retains SAME advanced math/state and partial camera inputs; no automatic retry"]
pub struct Failure<E> {pub progress:Progress,pub cause:E}

#[derive(Debug)]
#[must_use="retain through caller normalization failure; consume once with SAME math owner"]
pub struct Prefix {progress:Progress,capped:u64}
impl Prefix {

    pub fn progress(&self)->&Progress {&self.progress}
    pub fn finish_with_math<M:Math>(self,math:&mut M)->Result<u64,Failure<M::Error>> {
        let Self {mut progress,capped}=self;
        let result=camera_finish_graph(capped,math,&mut progress);finish(result,progress)
    }
}

fn operation<M:Math>(math:&mut M,p:&mut Progress,value:Value,op:Operation)->Result<u64,M::Error> {
    p.site=Site::Value(value);p.attempted=Some(Call::Operation(op));
    let bits=math.operation(op)?;
    p.values[value as usize]=Some(bits);p.completed_operations=p.completed_operations.wrapping_add(1);Ok(bits)
}
fn lower<M:Math>(math:&mut M,p:&mut Progress,site:Site,left:u64,right:u64)->Result<bool,M::Error> {
    p.site=site;p.attempted=Some(Call::OrderedLess {left,right});p.comparison=None;
    let result=math.ordered_less(left,right)?;
    p.comparison=Some(result);
    match site {Site::ClampLowerX=>p.clamp_comparisons[0]=Some(result),
        Site::ClampLowerY=>p.clamp_comparisons[1]=Some(result),Site::Value(_)=>{}}
    p.completed_operations=p.completed_operations.wrapping_add(1);Ok(result)
}
fn clamp_graph<M:Math>(pair:[Option<u32>;2],math:&mut M,p:&mut Progress)->Result<[u64;2],M::Error> {
    let [Some(x),Some(y)]=pair else {

        p.missing_pair=true;p.values[Value::ClampedX as usize]=Some(0);p.values[Value::ClampedY as usize]=Some(0);
        p.public_pair=Some([0,0]);return Ok([0,0]);
    };
    let x=operation(math,p,Value::WidenedX,Operation::WidenF32 {bits:x})?;
    let y=operation(math,p,Value::WidenedY,Operation::WidenF32 {bits:y})?;
    clamp_f64_tail([x,y],math,p)
}

fn clamp_f64_tail<M:Math>([x,y]:[u64;2],math:&mut M,p:&mut Progress)->Result<[u64;2],M::Error> {

    let x_upper=operation(math,p,Value::UpperXMinimum,Operation::Min {first:X_MAX,second:x})?;
    let x=if lower(math,p,Site::ClampLowerX,x,X_MIN)? {X_MIN}else{x_upper};
    p.values[Value::ClampedX as usize]=Some(x);
    let y_upper=operation(math,p,Value::UpperYMinimum,Operation::Min {first:Y_MAX,second:y})?;
    let y=if lower(math,p,Site::ClampLowerY,y,Y_MIN)? {Y_MIN}else{y_upper};
    p.values[Value::ClampedY as usize]=Some(y);p.public_pair=Some([x,y]);Ok([x,y])
}
fn camera_edge_prefix<M:Math>([x,y]:[u64;2],math:&mut M,p:&mut Progress)->Result<u64,M::Error> {
    p.unclamped_pair=Some([x,y]);
    let right=operation(math,p,Value::RightDistance,Operation::Sub {left:HORIZONTAL_EDGE,right:x})?;
    let left=operation(math,p,Value::LeftDistance,Operation::Add {left:x,right:HORIZONTAL_EDGE})?;
    let horizontal=operation(math,p,Value::Horizontal,Operation::Min {first:right,second:left})?;
    let bottom=operation(math,p,Value::BottomDistance,Operation::Add {left:y,right:VERTICAL_EDGE})?;
    let lower=operation(math,p,Value::LowerDistance,Operation::Min {first:bottom,second:horizontal})?;
    let top=operation(math,p,Value::TopDistance,Operation::Sub {left:VERTICAL_EDGE,right:y})?;
    let distance=operation(math,p,Value::Distance,Operation::Min {first:top,second:lower})?;

    operation(math,p,Value::CappedDistance,Operation::Min {first:distance,second:CAP})
}
fn camera_finish_graph<M:Math>(capped:u64,math:&mut M,p:&mut Progress)->Result<u64,M::Error> {
    let value=operation(math,p,Value::ScaledDistance,Operation::Mul {left:capped,right:NEGATIVE_THREE_TENTHS})?;
    let ratio=operation(math,p,Value::CapRatio,Operation::Div {left:value,right:CAP})?;
    let denominator=operation(math,p,Value::Denominator,Operation::Add {left:ratio,right:ONE_POINT_THREE})?;
    operation(math,p,Value::Zoom,Operation::Div {left:ONE,right:denominator})
}
fn finish<T,E>(result:Result<T,E>,progress:Progress)->Result<T,Failure<E>> {
    match result {Ok(value)=>Ok(value),Err(cause)=>Err(Failure {progress,cause})}
}
fn prefix_result<E>(result:Result<u64,E>,progress:Progress)->Result<Prefix,Failure<E>> {
    match result {Ok(capped)=>Ok(Prefix {progress,capped}),Err(cause)=>Err(Failure {progress,cause})}
}

pub fn clamped_public_pair_with_math<M:Math>(pair:[Option<u32>;2],math:&mut M)->Result<[u64;2],Failure<M::Error>> {
    let mut p=Progress::new(Entry::PublicClamp,Input::Public(pair));let result=clamp_graph(pair,math,&mut p);finish(result,p)
}

pub fn clamped_pair_f64_with_math<M:Math>(pair:[u64;2],math:&mut M)->Result<[u64;2],Failure<M::Error>> {
    let mut p=Progress::new(Entry::F64Clamp,Input::F64Clamp(pair));
    let result=clamp_f64_tail(pair,math,&mut p);finish(result,p)
}

pub fn source_zoom_f64_with_math<M:Math>(pair:[Option<u32>;2],math:&mut M)->Result<u64,Failure<M::Error>> {
    source_zoom_prefix_with_math(pair,math)?.finish_with_math(math)
}

pub fn source_zoom_prefix_with_math<M:Math>(pair:[Option<u32>;2],math:&mut M)->Result<Prefix,Failure<M::Error>> {
    let mut p=Progress::new(Entry::PublicZoom,Input::Public(pair));
    let result=(||{let pair=clamp_graph(pair,math,&mut p)?;camera_edge_prefix(pair,math,&mut p)})();prefix_result(result,p)
}

pub fn source_zoom_from_unclamped_pair_with_math<M:Math>(pair:[u64;2],math:&mut M)->Result<u64,Failure<M::Error>> {
    let mut p=Progress::new(Entry::UnclampedZoom,Input::Unclamped(pair));
    let result=camera_edge_prefix(pair,math,&mut p);
    prefix_result(result,p)?.finish_with_math(math)
}

struct ExistingHostValueMath;
impl Math for ExistingHostValueMath {
    type Error=core::convert::Infallible;
    fn operation(&mut self,operation:Operation)->Result<u64,Self::Error> {
        let bits=match operation {
            Operation::WidenF32 {bits}=>(f32::from_bits(bits) as f64).to_bits(),
            Operation::Add {left,right}=>(f64::from_bits(left)+f64::from_bits(right)).to_bits(),
            Operation::Sub {left,right}=>(f64::from_bits(left)-f64::from_bits(right)).to_bits(),
            Operation::Mul {left,right}=>(f64::from_bits(left)*f64::from_bits(right)).to_bits(),
            Operation::Div {left,right}=>(f64::from_bits(left)/f64::from_bits(right)).to_bits(),
            Operation::Min {first,second}=>if f64::from_bits(first)<f64::from_bits(second) {first}else{second},
        };Ok(bits)
    }
    fn ordered_less(&mut self,left:u64,right:u64)->Result<bool,Self::Error> {Ok(f64::from_bits(left)<f64::from_bits(right))}
}
fn host_result<T>(result:Result<T,Failure<core::convert::Infallible>>)->T {
    match result {Ok(value)=>value,Err(Failure {cause,..})=>match cause {}}
}

pub fn source_probe(pair: [Option<f32>; 2]) -> [f32; 2] {
    let pair=host_result(clamped_public_pair_with_math(pair.map(|v|v.map(f32::to_bits)),&mut ExistingHostValueMath));
    pair.map(|bits|f64::from_bits(bits) as f32)
}
pub fn source_zoom(pair: [Option<f32>; 2]) -> f32 {
    source_zoom_f64(pair) as f32
}

pub fn source_zoom_f64(pair: [Option<f32>; 2]) -> f64 {
    f64::from_bits(host_result(source_zoom_f64_with_math(pair.map(|v|v.map(f32::to_bits)),&mut ExistingHostValueMath)))
}

pub fn source_zoom_from_unclamped_pair(pair: [f64; 2]) -> f64 {
    f64::from_bits(host_result(source_zoom_from_unclamped_pair_with_math(pair.map(f64::to_bits),&mut ExistingHostValueMath)))
}
