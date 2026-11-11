use super::path;
pub use path::ellipse_source::Operation as F32Operation;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum ArithmeticSite {
    InputRightDifference,InputRight,InputBottomDifference,InputBottom,
    YTopX,XLeftX,TopLeftXSum,TopLeftX,YTopY,XLeftY,TopLeftYSum,TopLeftY,
    XRightX,TopRightXSum,TopRightX,XRightY,TopRightYSum,TopRightY,
    YBottomX,BottomLeftXSum,BottomLeftX,YBottomY,BottomLeftYSum,BottomLeftY,
    BottomRightXSum,BottomRightX,BottomRightYSum,BottomRightY,
    OutputBottomDifference,OutputBottom,OutputRightDifference,OutputRight,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum SelectionSite {MinXInner,MinXMiddle,MinXOuter,MaxXInner,MaxXMiddle,MaxXOuter,
    MinYInner,MinYMiddle,MinYOuter,MaxYInner,MaxYMiddle,MaxYOuter}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Edge {Left,Top,Right,Bottom}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Site {Arithmetic(ArithmeticSite),Selection(SelectionSite),Guard(Edge),Round(Edge),Integer(Edge)}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Selection {Min {left:u32,right:u32},Max {left:u32,right:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Predicate {OrderedGreater {left:u32,right:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Rounding {Ceil {source:u32},Floor {source:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Operation {Arithmetic(F32Operation),Selection(Selection),Predicate(Predicate),Round(Rounding),Integer {source:u32}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub site:Site,pub operation:Operation}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Returned {F32(u32),Predicate(bool),Integer(i32)}
pub trait BoundsServices {
    type Error;
    fn bounds_f32(&mut self,site:Site,operation:F32Operation)->Result<u32,Self::Error>;
    fn bounds_select(&mut self,site:Site,operation:Selection)->Result<u32,Self::Error>;
    fn bounds_compare(&mut self,site:Site,operation:Predicate)->Result<bool,Self::Error>;

    fn bounds_round(&mut self,site:Site,operation:Rounding)->Result<u32,Self::Error>;
    fn bounds_integer(&mut self,site:Site,source:u32)->Result<i32,Self::Error>;
}
#[derive(Clone,Copy,Debug)]
pub enum Input {Corners([u32;4]),PathBounds([u32;4])}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase {InputCancellation,Transform,Selection,Edge(Edge),Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub input:Input,pub affine:[u32;6],pub phase:Phase,
    pub corners:Option<[u32;4]>,pub transformed:Option<[u32;4]>,
    pub arithmetic:[Option<u32>;32],pub selections:[Option<u32>;12],
    pub guards:[Option<bool>;4],pub rounded:[Option<u32>;4],pub edges:[Option<i32>;4],
    pub request:Option<Request>,pub returned:Option<Returned>,pub completed:u64,
}
impl Progress {
    fn new(input:Input,affine:[u32;6])->Self {Self {input,affine,phase:Phase::InputCancellation,
        corners:None,transformed:None,arithmetic:[None;32],selections:[None;12],guards:[None;4],
        rounded:[None;4],edges:[None;4],request:None,returned:None,completed:0}}
}
#[derive(Debug)]
pub struct NumericFailure<E> {pub request:Request,pub error:E}
#[derive(Debug)]
#[must_use="retain numeric prefix and SAME advanced service; no retry or HOST fallback"]
pub struct Failure<E> {pub progress:Progress,pub failure:NumericFailure<E>}
#[derive(Clone,Copy,Debug)]
pub struct CornersCompleted {pub words:[u32;4],pub progress:Progress}
#[derive(Clone,Copy,Debug)]
pub struct EdgesCompleted {pub edges:[i32;4],pub progress:Progress}
fn perform<T:Copy,E>(progress:&mut Progress,request:Request,operation:impl FnOnce()->Result<T,E>,as_returned:fn(T)->Returned)
    ->Result<T,NumericFailure<E>>
{
    progress.request=Some(request);progress.returned=None;
    let value=operation().map_err(|error|NumericFailure {request,error})?;
    progress.returned=Some(as_returned(value));progress.completed=progress.completed.wrapping_add(1);Ok(value)
}
fn arithmetic<S:BoundsServices>(progress:&mut Progress,service:&mut S,site:ArithmeticSite,operation:F32Operation)
    ->Result<u32,NumericFailure<S::Error>>
{
    let request=Request {site:Site::Arithmetic(site),operation:Operation::Arithmetic(operation)};
    let value=perform(progress,request,||service.bounds_f32(request.site,operation),Returned::F32)?;
    progress.arithmetic[site as usize]=Some(value);Ok(value)
}
fn selection<S:BoundsServices>(progress:&mut Progress,service:&mut S,site:SelectionSite,operation:Selection)
    ->Result<u32,NumericFailure<S::Error>>
{
    let request=Request {site:Site::Selection(site),operation:Operation::Selection(operation)};
    let value=perform(progress,request,||service.bounds_select(request.site,operation),Returned::F32)?;
    progress.selections[site as usize]=Some(value);Ok(value)
}
fn transform_graph<S:BoundsServices>(affine:[u32;6],corners:[u32;4],service:&mut S,progress:&mut Progress)
    ->Result<[u32;4],NumericFailure<S::Error>>
{
    use ArithmeticSite::*;use F32Operation::*;use SelectionSite::*;
    let [x,y,xr,yb]=corners;let [a,b,tx,c,d,ty]=affine;
    progress.corners=Some(corners);progress.phase=Phase::Transform;
    macro_rules! a {($site:ident,$operation:expr)=>{arithmetic(progress,service,$site,$operation)?};}
    let y_top_x=a!(YTopX,Mul {destination:y,source:b});
    let x_left_x=a!(XLeftX,Mul {destination:a,source:x});
    let sum=a!(TopLeftXSum,Add {destination:x_left_x,source:y_top_x});
    let top_left_x=a!(TopLeftX,Add {destination:sum,source:tx});
    let y_top_y=a!(YTopY,Mul {destination:y,source:d});
    let x_left_y=a!(XLeftY,Mul {destination:x,source:c});
    let sum=a!(TopLeftYSum,Add {destination:x_left_y,source:y_top_y});
    let top_left_y=a!(TopLeftY,Add {destination:sum,source:ty});
    let x_right_x=a!(XRightX,Mul {destination:a,source:xr});
    let sum=a!(TopRightXSum,Add {destination:y_top_x,source:x_right_x});
    let top_right_x=a!(TopRightX,Add {destination:sum,source:tx});
    let x_right_y=a!(XRightY,Mul {destination:xr,source:c});
    let sum=a!(TopRightYSum,Add {destination:y_top_y,source:x_right_y});
    let top_right_y=a!(TopRightY,Add {destination:sum,source:ty});
    let y_bottom_x=a!(YBottomX,Mul {destination:b,source:yb});
    let sum=a!(BottomLeftXSum,Add {destination:x_left_x,source:y_bottom_x});
    let bottom_left_x=a!(BottomLeftX,Add {destination:sum,source:tx});
    let y_bottom_y=a!(YBottomY,Mul {destination:yb,source:d});
    let sum=a!(BottomLeftYSum,Add {destination:x_left_y,source:y_bottom_y});
    let bottom_left_y=a!(BottomLeftY,Add {destination:sum,source:ty});
    let sum=a!(BottomRightXSum,Add {destination:y_bottom_x,source:x_right_x});
    let bottom_right_x=a!(BottomRightX,Add {destination:sum,source:tx});
    let sum=a!(BottomRightYSum,Add {destination:y_bottom_y,source:x_right_y});
    let bottom_right_y=a!(BottomRightY,Add {destination:sum,source:ty});
    progress.phase=Phase::Selection;
    macro_rules! s {($site:ident,$operation:expr)=>{selection(progress,service,$site,$operation)?};}

    let inner=s!(MinXInner,Selection::Min {left:bottom_left_x,right:top_right_x});
    let middle=s!(MinXMiddle,Selection::Min {left:bottom_right_x,right:inner});
    let xmin=s!(MinXOuter,Selection::Min {left:middle,right:top_left_x});
    let inner=s!(MaxXInner,Selection::Max {left:bottom_left_x,right:top_right_x});
    let middle=s!(MaxXMiddle,Selection::Max {left:bottom_right_x,right:inner});
    let xmax=s!(MaxXOuter,Selection::Max {left:middle,right:top_left_x});
    let inner=s!(MinYInner,Selection::Min {left:bottom_left_y,right:top_right_y});
    let middle=s!(MinYMiddle,Selection::Min {left:bottom_right_y,right:inner});
    let ymin=s!(MinYOuter,Selection::Min {left:middle,right:top_left_y});
    let inner=s!(MaxYInner,Selection::Max {left:bottom_left_y,right:top_right_y});
    let middle=s!(MaxYMiddle,Selection::Max {left:bottom_right_y,right:inner});
    let ymax=s!(MaxYOuter,Selection::Max {left:middle,right:top_left_y});
    let words=[xmin,ymin,xmax,ymax];progress.transformed=Some(words);Ok(words)
}
pub fn transformed_corners_with_services<S:BoundsServices>(affine:[u32;6],corners:[u32;4],service:&mut S)
    ->Result<CornersCompleted,Failure<S::Error>>
{
    let mut progress=Progress::new(Input::Corners(corners),affine);
    match transform_graph(affine,corners,service,&mut progress) {
        Ok(words)=>{progress.phase=Phase::Complete;Ok(CornersCompleted {words,progress})},
        Err(failure)=>Err(Failure {progress,failure}),
    }
}
fn edge_graph<S:BoundsServices>(edge:Edge,value:u32,upper:bool,service:&mut S,progress:&mut Progress)
    ->Result<i32,NumericFailure<S::Error>>
{
    progress.phase=Phase::Edge(edge);
    let predicate=if upper {Predicate::OrderedGreater {left:0x4f000000,right:value}}
        else {Predicate::OrderedGreater {left:value,right:0xcf000000}};
    let request=Request {site:Site::Guard(edge),operation:Operation::Predicate(predicate)};
    let admitted=perform(progress,request,||service.bounds_compare(request.site,predicate),Returned::Predicate)?;
    progress.guards[edge as usize]=Some(admitted);
    let integer=if admitted {
        let rounding=if upper {Rounding::Ceil {source:value}}else {Rounding::Floor {source:value}};
        let request=Request {site:Site::Round(edge),operation:Operation::Round(rounding)};
        let rounded=perform(progress,request,||service.bounds_round(request.site,rounding),Returned::F32)?;
        progress.rounded[edge as usize]=Some(rounded);
        let request=Request {site:Site::Integer(edge),operation:Operation::Integer {source:rounded}};
        perform(progress,request,||service.bounds_integer(request.site,rounded),Returned::Integer)?
    }else if upper {i32::MAX}else {i32::MIN};
    progress.edges[edge as usize]=Some(integer);Ok(integer)
}
pub fn path_transformed_edges_with_services<S:BoundsServices>(bounds:[u32;4],affine:[u32;6],service:&mut S)
    ->Result<EdgesCompleted,Failure<S::Error>>
{
    let mut progress=Progress::new(Input::PathBounds(bounds),affine);
    let result=(||->Result<[i32;4],NumericFailure<S::Error>> {
        use ArithmeticSite::*;use F32Operation::*;
        let [min_x,max_x,min_y,max_y]=bounds;
        let difference=arithmetic(&mut progress,service,InputRightDifference,Sub {destination:max_x,source:min_x})?;
        let right_x=arithmetic(&mut progress,service,InputRight,Add {destination:difference,source:min_x})?;
        let difference=arithmetic(&mut progress,service,InputBottomDifference,Sub {destination:max_y,source:min_y})?;
        let bottom_y=arithmetic(&mut progress,service,InputBottom,Add {destination:difference,source:min_y})?;
        let [xmin,ymin,xmax,ymax]=transform_graph(affine,[min_x,min_y,right_x,bottom_y],service,&mut progress)?;
        progress.phase=Phase::Edge(Edge::Bottom);
        let difference=arithmetic(&mut progress,service,OutputBottomDifference,Sub {destination:ymax,source:ymin})?;
        let recombined=arithmetic(&mut progress,service,OutputBottom,Add {destination:difference,source:ymin})?;
        let bottom=edge_graph(Edge::Bottom,recombined,true,service,&mut progress)?;
        progress.phase=Phase::Edge(Edge::Right);
        let difference=arithmetic(&mut progress,service,OutputRightDifference,Sub {destination:xmax,source:xmin})?;
        let recombined=arithmetic(&mut progress,service,OutputRight,Add {destination:difference,source:xmin})?;
        let right=edge_graph(Edge::Right,recombined,true,service,&mut progress)?;
        let top=edge_graph(Edge::Top,ymin,false,service,&mut progress)?;
        let left=edge_graph(Edge::Left,xmin,false,service,&mut progress)?;
        Ok([left,top,right,bottom])
    })();
    match result {Ok(edges)=>{progress.phase=Phase::Complete;Ok(EdgesCompleted {edges,progress})},Err(failure)=>Err(Failure {progress,failure})}
}

impl BoundsServices for path::ellipse_source::ExistingHostValueFp {
    type Error=core::convert::Infallible;
    fn bounds_f32(&mut self,_site:Site,operation:F32Operation)->Result<u32,Self::Error> {
        Ok(path::ellipse_source::existing_host_operation(operation))
    }
    fn bounds_select(&mut self,_site:Site,operation:Selection)->Result<u32,Self::Error> {
        Ok(match operation {
            Selection::Min {left,right}=>super::minss(f32::from_bits(left),f32::from_bits(right)).to_bits(),
            Selection::Max {left,right}=>super::maxss(f32::from_bits(left),f32::from_bits(right)).to_bits(),
        })
    }
    fn bounds_compare(&mut self,_site:Site,operation:Predicate)->Result<bool,Self::Error> {
        let Predicate::OrderedGreater {left,right}=operation;
        Ok(path::comparison::existing_host_relation(path::comparison::Relation::OrderedGreater,left,right))
    }
    fn bounds_round(&mut self,_site:Site,operation:Rounding)->Result<u32,Self::Error> {
        Ok(unsafe {match operation {Rounding::Ceil {source}=>super::ceilf(f32::from_bits(source)).to_bits(),
            Rounding::Floor {source}=>super::floorf(f32::from_bits(source)).to_bits()}})
    }
    fn bounds_integer(&mut self,_site:Site,source:u32)->Result<i32,Self::Error> {Ok(super::cvtt_i32(f32::from_bits(source)))}
}
