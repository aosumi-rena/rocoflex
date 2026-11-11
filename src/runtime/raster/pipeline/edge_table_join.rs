use super::{intersection,path_allocation,row_count,EdgeTable,Error,Row};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Header,Empty,Truncate,Prefix,OwnRow,OtherRow,Intersect,PublishRow,Resize,Dirty,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub original_bounds:[i32;4],pub other_bounds:[i32;4],pub first:Option<i32>,pub end:Option<i32>,
    pub index:Option<i32>,pub other_index:Option<usize>,pub fixed_right_bound:Option<i32>,
    pub emptied_prefix:usize,pub published_rows:usize,pub skipped_empty:usize,
}
pub enum Cause {Source(Error),Row(intersection::OwnedFailure)}
impl Cause {
    pub fn into_error(self)->Error {match self {Self::Source(error)=>error,Self::Row(failure)=>failure.error}}
}
#[must_use="same supplied target and other table remain the actual owners"]
pub struct Completed<'t,'o> {pub target:&'t mut EdgeTable,pub other:&'o EdgeTable,pub progress:Progress}
#[must_use="retain original advanced target, other table and stopped row scratch; do not replay"]
pub struct Failure<'t,'o> {pub target:&'t mut EdgeTable,pub other:&'o EdgeTable,pub progress:Progress,pub cause:Cause}
pub fn run_source<'t,'o>(target:&'t mut EdgeTable,other:&'o EdgeTable)
    ->Result<Completed<'t,'o>,Failure<'t,'o>>
{
    let mut progress=Progress {stage:Stage::Header,original_bounds:target.bounds,other_bounds:other.bounds,
        first:None,end:None,index:None,other_index:None,fixed_right_bound:None,emptied_prefix:0,published_rows:0,skipped_empty:0};
    let result=(||->Result<(),Cause> {
        let [x,y,width,height]=target.bounds;let [ox,oy,ow,oh]=other.bounds;
        let left=x.max(ox);let top=y.max(oy);
        let own_right=x.wrapping_add(width);let other_right=ox.wrapping_add(ow);
        let right=own_right.min(other_right);let bottom=y.wrapping_add(height).min(oy.wrapping_add(oh));
        if right<=left||bottom<=top {
            progress.stage=Stage::Empty;target.source_flag_32=0;target.bounds[3]=0;return Ok(());
        }
        progress.stage=Stage::Truncate;
        let end=bottom.wrapping_sub(y);progress.end=Some(end);
        if end<height {target.bounds[3]=end;}
        let first=top.wrapping_sub(y);progress.first=Some(first);
        if other_right<own_right {target.bounds[0]=x.min(right);target.bounds[2]=right.wrapping_sub(target.bounds[0]);}
        if first<0||end<first {return Err(Cause::Source(Error::NativeOutOfBounds));}
        progress.stage=Stage::Prefix;
        for index in 0..first {
            progress.index=Some(index);
            *target.rows.get_mut(index as usize).ok_or(Cause::Source(Error::NativeOutOfBounds))?=Some(Row::Empty);
            progress.emptied_prefix+=1;
        }
        let bound=target.bounds[0].wrapping_add(target.bounds[2]).wrapping_shl(8);progress.fixed_right_bound=Some(bound);
        for index in first..end {
            progress.stage=Stage::OwnRow;progress.index=Some(index);progress.other_index=None;
            let own=target.rows.get(index as usize).and_then(Option::as_ref).ok_or(Cause::Source(Error::NativeOutOfBounds))?;
            if row_count(own)==0 {progress.skipped_empty+=1;continue;}
            progress.stage=Stage::OtherRow;
            let other_index=usize::try_from(y.wrapping_add(index).wrapping_sub(oy)).map_err(|_|Cause::Source(Error::NativeOutOfBounds))?;
            progress.other_index=Some(other_index);
            let foreign=other.rows.get(other_index).and_then(Option::as_ref).ok_or(Cause::Source(Error::NativeOutOfBounds))?;
            let prior_capacity=target.source_capacity_edges;progress.stage=Stage::Intersect;

            let completed=intersection::run_source(own,foreign,bound,&mut target.source_capacity_edges)
                .map_err(|failure|Cause::Row(failure.into_owned()))?;
            let row=completed.into_row();
            progress.stage=Stage::PublishRow;target.rows[index as usize]=Some(row);progress.published_rows+=1;

            if target.source_capacity_edges!=prior_capacity {
                progress.stage=Stage::Resize;
                let (stride,words)=path_allocation(target.bounds[3],target.source_capacity_edges).map_err(Cause::Source)?;
                target.source_stride_words=stride;target.allocated_words=words;
            }
        }
        progress.stage=Stage::Dirty;target.source_flag_32=1;Ok(())
    })();
    match result {
        Ok(())=>{progress.stage=Stage::Complete;Ok(Completed {target,other,progress})},
        Err(cause)=>Err(Failure {target,other,progress,cause}),
    }
}
