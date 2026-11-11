use super::{edge_table,path,rectangle};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Rejection {RightVsClipLeft,BottomVsClipTop,LeftVsClipRight,ClipHeight,ClipWidth,BoundsHeight,BoundsWidth,TopVsClipBottom}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Bounds,Rejection(Rejection),Scan,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub bounds:Option<rectangle::bounds::Progress>,pub edges:Option<[i32;4]>,
    pub tests:[Option<bool>;8],pub rejected:Option<Rejection>,pub scan:Option<edge_table::path_source::Progress>,
}
pub enum Cause<'p,E> {Bounds(rectangle::bounds::Failure<E>),Scan(edge_table::path_source::Failure<'p,E>)}
#[must_use="table is produced only after actual bounds/rejections/scan; None is an actual rejection"]
pub struct Completed<'p> {
    pub path:&'p path::Path,pub clip:[i32;4],pub affine_words:[u32;6],
    pub table:Option<edge_table::EdgeTable>,pub progress:Progress,
}
#[must_use="retain SAME input Path, partial bounds/scan/table and advanced caller service; no replay"]
pub struct Failure<'p,E=core::convert::Infallible> {
    pub path:&'p path::Path,pub clip:[i32;4],pub affine_words:[u32;6],pub progress:Progress,pub cause:Cause<'p,E>,
}
pub enum TerminalCause<E> {Bounds(rectangle::bounds::Failure<E>),Scan(edge_table::path_source::TerminalFailure<E>)}
#[must_use="retain SAME input and numeric owners; terminal graph cannot resume"]
pub struct TerminalFailure<E> {
    clip:[i32;4],affine_words:[u32;6],progress:Progress,cause:TerminalCause<E>,
}
impl<E> TerminalFailure<E> {
    pub fn clip(&self)->&[i32;4] {&self.clip}
    pub fn affine_words(&self)->&[u32;6] {&self.affine_words}
    pub fn progress(&self)->&Progress {&self.progress}
    pub fn cause(&self)->&TerminalCause<E> {&self.cause}
}
impl<E> Failure<'_,E> {
    pub fn freeze_terminal(self)->TerminalFailure<E> {
        let Self {path:_,clip,affine_words,progress,cause}=self;
        let cause=match cause {
            Cause::Bounds(value)=>TerminalCause::Bounds(value),
            Cause::Scan(value)=>TerminalCause::Scan(value.freeze_terminal()),
        };
        TerminalFailure {clip,affine_words,progress,cause}
    }
}
impl Failure<'_,core::convert::Infallible> {

    pub fn into_error(self)->edge_table::Error {match self.cause {
        Cause::Bounds(failure)=>match failure.failure.error {},
        Cause::Scan(failure)=>failure.into_error(),
    }}
}
pub fn path_mask_source_with_services<'p,S,E>(clip:[i32;4],path:&'p path::Path,affine_words:[u32;6],service:&mut S)
    ->Result<Completed<'p>,Failure<'p,E>>
where S:rectangle::BoundsServices<Error=E>+edge_table::path_source::ScanServices<Error=E>
    +path::comparison::PackedServices<Error=E>+path::point_source::Fp<Error=E>+path::curve_source::Fp<Error=E>
{
    let mut progress=Progress {stage:Stage::Bounds,bounds:None,edges:None,tests:[None;8],rejected:None,scan:None};
    let completed=match rectangle::path_transformed_edges_with_services(path.bounds.map(f32::to_bits),affine_words,service) {
        Ok(completed)=>completed,
        Err(failure)=>{

            progress.bounds=Some(failure.progress);
            return Err(Failure {path,clip,affine_words,progress,cause:Cause::Bounds(failure)});
        },
    };
    progress.bounds=Some(completed.progress);progress.edges=Some(completed.edges);
    let [left,top,right,bottom]=completed.edges;
    macro_rules! reject {($test:ident,$condition:expr)=>{{
        let test=Rejection::$test;progress.stage=Stage::Rejection(test);
        let rejected=$condition;progress.tests[test as usize]=Some(rejected);
        if rejected {
            progress.rejected=Some(test);progress.stage=Stage::Complete;
            return Ok(Completed {path,clip,affine_words,table:None,progress});
        }
    }};}

    reject!(RightVsClipLeft,right<=clip[0]);
    reject!(BottomVsClipTop,bottom<=clip[1]);
    reject!(LeftVsClipRight,left>=clip[0].wrapping_add(clip[2]));
    reject!(ClipHeight,clip[3]<=0);
    reject!(ClipWidth,clip[2]<=0);
    reject!(BoundsHeight,bottom<=top);
    reject!(BoundsWidth,right<=left);
    reject!(TopVsClipBottom,top>=clip[1].wrapping_add(clip[3]));
    progress.stage=Stage::Scan;
    match edge_table::path_source::run_with_services(path,clip,affine_words,service) {
        Ok(completed)=>{
            progress.scan=Some(completed.progress);progress.stage=Stage::Complete;
            Ok(Completed {path,clip,affine_words,table:Some(completed.table),progress})
        },
        Err(failure)=>{
            progress.scan=Some(failure.progress);
            Err(Failure {path,clip,affine_words,progress,cause:Cause::Scan(failure)})
        },
    }
}
