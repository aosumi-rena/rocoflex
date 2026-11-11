use super::{comparison,ellipse_source::{Fp,Operation,Value},Error,Path};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {Right,Bottom,CompareWidth,CompareHeight,Bounds,Reserve,Append,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub input:[u32;4],pub step:Step,pub operation:Option<Operation>,
    pub comparison:Option<comparison::Request>,pub right:Option<u32>,pub bottom:Option<u32>,
    pub swaps:[Option<bool>;2],pub selected_bounds:Option<[u32;4]>,
}
#[derive(Debug)]
pub enum Cause<E> {Arithmetic(E),Comparison {request:comparison::Request,error:E},Path(Error)}
#[derive(Debug)]
#[must_use="preserve partial Path/progress and SAME advanced FP/comparison owner"]
pub struct Failure<E> {pub partial:Path,pub progress:Progress,pub cause:Cause<E>}

pub fn build_with_fp<P:Fp>(rectangle:[u32;4],fp:&mut P)->Result<Path,Failure<P::Error>> {
    let [x,y,w,h]=rectangle;
    let mut partial=Path {words:Vec::new(),source_capacity:0,bounds:[0.0;4],source_flag_32:1};
    let mut progress=Progress {input:rectangle,step:Step::Right,operation:None,comparison:None,
        right:None,bottom:None,swaps:[None;2],selected_bounds:None};
    let result=(||->Result<(),Cause<P::Error>> {
        let right_op=Operation::Add {destination:x,source:w};progress.operation=Some(right_op);
        let right=fp.operation(Value::RectangleRight,right_op).map_err(Cause::Arithmetic)?;progress.right=Some(right);
        progress.step=Step::Bottom;
        let bottom_op=Operation::Add {destination:y,source:h};progress.operation=Some(bottom_op);
        let bottom=fp.operation(Value::RectangleBottom,bottom_op).map_err(Cause::Arithmetic)?;progress.bottom=Some(bottom);
        let mut left=x;let mut right=right;let mut top=y;let mut bottom=bottom;

        progress.step=Step::CompareWidth;progress.operation=None;
        let request=comparison::Request {site:comparison::Site::RectangleWidth,
            relation:comparison::Relation::OrderedGreater,left:0,right:w};progress.comparison=Some(request);
        let swap=fp.comparison(request).map_err(|error|Cause::Comparison {request,error})?;
        progress.swaps[0]=Some(swap);if swap {core::mem::swap(&mut left,&mut right);}
        progress.step=Step::CompareHeight;
        let request=comparison::Request {site:comparison::Site::RectangleHeight,
            relation:comparison::Relation::OrderedGreater,left:0,right:h};progress.comparison=Some(request);
        let swap=fp.comparison(request).map_err(|error|Cause::Comparison {request,error})?;
        progress.swaps[1]=Some(swap);if swap {core::mem::swap(&mut top,&mut bottom);}
        progress.step=Step::Bounds;
        let bounds=[left,right,top,bottom];progress.selected_bounds=Some(bounds);
        partial.bounds=bounds.map(f32::from_bits);
        progress.step=Step::Reserve;partial.ensure(13).map_err(Cause::Path)?;
        progress.step=Step::Append;
        partial.words.extend_from_slice(&[
            0x47c35100,left,top,0x47c35080,left,bottom,
            0x47c35080,right,bottom,0x47c35080,right,top,0x47c35280,
        ]);
        progress.step=Step::Complete;Ok(())
    })();
    match result {Ok(())=>Ok(partial),Err(cause)=>Err(Failure {partial,progress,cause})}
}
