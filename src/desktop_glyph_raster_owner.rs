use super::{Completed as TransformCompleted, Prepared, glyph_math};
use super::super::empty_background::literal;
use literal::{image::path_mask, raster_numeric_binding as binding};
use binding::Admission;

pub struct RasterGraph {
    clip:[i32;4],affine_words:[u32;6],table:Option<literal::edge_table::EdgeTable>,
    progress:path_mask::Progress,
}
impl RasterGraph {
    pub fn clip(&self)->&[i32;4] {&self.clip}
    pub fn affine_words(&self)->&[u32;6] {&self.affine_words}
    pub fn table(&self)->Option<&literal::edge_table::EdgeTable> {self.table.as_ref()}
    pub fn progress(&self)->&path_mask::Progress {&self.progress}
    pub fn into_parts(self)->([i32;4],[u32;6],Option<literal::edge_table::EdgeTable>,path_mask::Progress) {
        (self.clip,self.affine_words,self.table,self.progress)
    }
}
#[must_use="normal mask graph and SAME admission/transformed owner"]
pub struct RasterCompleted<A:Admission> {
    graph:RasterGraph,admission:A,owner:TransformCompleted,
}
impl<A:Admission> RasterCompleted<A> {
    pub fn graph(&self)->&RasterGraph {&self.graph}
    pub fn admission(&self)->&A {&self.admission}
    pub fn owner(&self)->&TransformCompleted {&self.owner}
    pub fn into_parts(self)->(RasterGraph,A,TransformCompleted) {
        (self.graph,self.admission,self.owner)
    }
}
#[must_use="retain terminal graph FIRST, actual admission SECOND, SAME owner THIRD"]
pub struct RasterFailure<A:Admission> {

    graph:binding::TerminalMaskFailure<binding::Error<A::Error,glyph_math::Stopped>>,
    admission:A,
    owner:TransformCompleted,
}
impl<A:Admission> RasterFailure<A> {
    pub fn graph(&self)->&binding::TerminalMaskFailure<binding::Error<A::Error,glyph_math::Stopped>> {&self.graph}
    pub fn admission(&self)->&A {&self.admission}
    pub fn owner(&self)->&TransformCompleted {&self.owner}

}
impl TransformCompleted {
    pub fn into_raster_with_admission<A:Admission>(mut self,clip:[i32;4],affine_words:[u32;6],mut admission:A)
        ->Result<RasterCompleted<A>,RasterFailure<A>>
    {
        let result={

            let Prepared {path,math,..}=&mut self.owner;
            let borrowed=binding::Binding::borrow(math,&mut admission);
            match borrowed.path_mask_source_retaining(clip,&*path,affine_words) {
                Ok(completed)=>{
                    let (graph,borrowed)=completed.into_parts();
                    let path_mask::Completed {path:_,clip,affine_words,table,progress}=graph;

                    drop(borrowed);
                    Ok(RasterGraph {clip,affine_words,table,progress})
                },
                Err(failure)=>Err(failure.freeze_terminal()),
            }
        };

        match result {
            Ok(graph)=>Ok(RasterCompleted {graph,admission,owner:self}),
            Err(graph)=>Err(RasterFailure {graph,admission,owner:self}),
        }
    }
}
