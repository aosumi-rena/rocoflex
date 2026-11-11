use super::{truncated_fixed,CoverageEvent,Row};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {ReadPair,FringeCall,SpanCall,AdvancePair,TerminalCall,Complete,Stopped}
#[derive(Clone,Copy,Debug)]
pub struct Emission {pub event:CoverageEvent,pub terminal:bool}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub stopped_from:Option<Stage>,pub pair_index:usize,pub completed_pairs:usize,
    pub current_x:Option<i32>,pub accumulated:i32,pub end_pixel:Option<i32>,pub first_pixel:Option<i32>,
    pub level:Option<i32>,pub next_x:Option<i32>,pub pending:Option<Emission>,pub completed_emissions:usize,
}
#[must_use="borrowed original row and stopped scanner state must be retained"]
pub struct Walker<'a> {entries:Entries<'a>,source_row:Option<&'a Row>,progress:Progress}
enum Entries<'a> {Borrowed(&'a [[i32;2]]),Span([[i32;2];2])}
impl Entries<'_> {
    fn as_slice(&self)->&[[i32;2]] {match self {Self::Borrowed(entries)=>entries,Self::Span(entries)=>entries}}
}
impl<'a> Walker<'a> {
    pub fn entries(&self)->&[[i32;2]] {self.entries.as_slice()}
    pub fn source_row(&self)->Option<&'a Row> {self.source_row}
    pub fn progress(&self)->&Progress {&self.progress}

    fn emit<E>(&mut self,event:CoverageEvent,terminal:bool,stage:Stage,
        emitter:&mut impl FnMut(CoverageEvent,bool)->Result<(),E>)->Result<(),E>
    {
        self.progress.stage=stage;self.progress.pending=Some(Emission {event,terminal});
        if let Err(error)=emitter(event,terminal) {
            self.progress.stopped_from=Some(stage);self.progress.stage=Stage::Stopped;return Err(error);
        }
        self.progress.completed_emissions+=1;self.progress.pending=None;Ok(())
    }
    fn run<E>(&mut self,emitter:&mut impl FnMut(CoverageEvent,bool)->Result<(),E>)->Result<(),E> {
        if self.entries().len()<2 {self.progress.stage=Stage::Complete;return Ok(());}
        let mut current_x=self.entries()[0][0];self.progress.current_x=Some(current_x);
        let mut accumulated=0i32;let mut end_pixel=0i32;
        for pair_index in 0..self.entries().len()-1 {
            self.progress.stage=Stage::ReadPair;self.progress.pair_index=pair_index;
            let level=self.entries()[pair_index][1];let next_x=self.entries()[pair_index+1][0];
            end_pixel=truncated_fixed(next_x);let first_pixel=truncated_fixed(current_x);
            self.progress.level=Some(level);self.progress.next_x=Some(next_x);
            self.progress.end_pixel=Some(end_pixel);self.progress.first_pixel=Some(first_pixel);
            if end_pixel==first_pixel {
                accumulated=next_x.wrapping_sub(current_x).wrapping_mul(level).wrapping_add(accumulated);
            }else {
                let coverage=256i32.wrapping_sub(current_x&255).wrapping_mul(level).wrapping_add(accumulated);
                if coverage>=256 {
                    self.emit(CoverageEvent::Fringe {x:first_pixel,accumulated:coverage},false,Stage::FringeCall,emitter)?;
                }
                if level>0 {
                    let x=first_pixel.wrapping_add(1);let count=end_pixel.wrapping_sub(x);
                    if count>0 {self.emit(CoverageEvent::Span {x,count,level},false,Stage::SpanCall,emitter)?;}
                }
                accumulated=(next_x&255).wrapping_mul(level);
            }
            self.progress.stage=Stage::AdvancePair;current_x=next_x;
            self.progress.current_x=Some(current_x);self.progress.accumulated=accumulated;
            self.progress.completed_pairs+=1;
        }
        if accumulated>=256 {
            self.emit(CoverageEvent::Fringe {x:end_pixel,accumulated},true,Stage::TerminalCall,emitter)?;
        }
        self.progress.stage=Stage::Complete;Ok(())
    }
}
pub struct Completed<'a> {pub walker:Walker<'a>}
#[must_use="retain SAME borrowed row, failed call and original advanced emitter"]
pub struct Failure<'a,E> {pub walker:Walker<'a>,pub error:E}
pub fn walk_ordered<'a,E>(entries:&'a [[i32;2]],emitter:&mut impl FnMut(CoverageEvent,bool)->Result<(),E>)
    ->Result<Completed<'a>,Failure<'a,E>>
{
    drive(Entries::Borrowed(entries),None,emitter)
}

pub fn walk_row_ordered<'a,E>(row:&'a Row,emitter:&mut impl FnMut(CoverageEvent,bool)->Result<(),E>)
    ->Result<Completed<'a>,Failure<'a,E>>
{
    let entries=match row {
        Row::Empty=>Entries::Borrowed(&[]),
        Row::Span {fixed_x,level}=>Entries::Span([[fixed_x[0],*level],[fixed_x[1],0]]),
        Row::Edges {entries}=>Entries::Borrowed(entries),
    };
    drive(entries,Some(row),emitter)
}
fn drive<'a,E>(entries:Entries<'a>,source_row:Option<&'a Row>,emitter:&mut impl FnMut(CoverageEvent,bool)->Result<(),E>)
    ->Result<Completed<'a>,Failure<'a,E>>
{
    let mut walker=Walker {entries,source_row,progress:Progress {stage:Stage::ReadPair,stopped_from:None,pair_index:0,completed_pairs:0,
        current_x:None,accumulated:0,end_pixel:None,first_pixel:None,level:None,next_x:None,pending:None,completed_emissions:0}};
    match walker.run(emitter) {Ok(())=>Ok(Completed {walker}),Err(error)=>Err(Failure {walker,error})}
}
