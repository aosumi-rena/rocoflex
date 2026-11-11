use super::{path_allocation,EdgeTable,Error,Row};
use super::super::{region,sort};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Bounds,Allocation,Initialize,Rectangle,ReadRow,Grow,Reserve,Append,Sort,Accumulate,Write,Truncate,FinalZero,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub bounds:Option<[i32;4]>,pub initialized_rows:usize,
    pub rectangle:Option<usize>,pub row:Option<usize>,pub row_ordinal:Option<i32>,
    pub count:Option<usize>,pub appended_pairs:usize,pub collapsed_rows:usize,
    pub read:usize,pub written:usize,pub winding:i32,pub x:Option<i32>,pub level:Option<u32>,
}
pub enum Cause {Source(Error),Sort(sort::Error)}
impl Cause {
    pub fn into_error(self)->Error {match self {Self::Source(error)=>error,Self::Sort(_)=>Error::Size}}
}
#[must_use="table is the actual produced owned value, input remains borrowed"]
pub struct Completed<'r> {pub rectangles:&'r region::Rectangles,pub table:EdgeTable,pub progress:Progress}
#[must_use="retain partial rows/table and original ordered rectangle list; no replay"]
pub struct Failure<'r> {
    pub rectangles:&'r region::Rectangles,pub table:Option<EdgeTable>,
    pub initial_rows:Vec<Option<Row>>,pub progress:Progress,pub cause:Cause,
}
pub fn run_source(rectangles:&region::Rectangles)->Result<Completed<'_>,Failure<'_>> {
    let mut progress=Progress {stage:Stage::Bounds,bounds:None,initialized_rows:0,
        rectangle:None,row:None,row_ordinal:None,count:None,appended_pairs:0,collapsed_rows:0,
        read:0,written:0,winding:0,x:None,level:None};
    let mut initial_rows=Vec::new();let mut table=None;
    let result=(||->Result<(),Cause> {
        let bounds=rectangles.bounds_source();progress.bounds=Some(bounds);
        if bounds[3]<0 {return Err(Cause::Source(Error::Domain));}
        progress.stage=Stage::Allocation;
        let (stride,words)=path_allocation(bounds[3],32).map_err(Cause::Source)?;
        initial_rows.try_reserve_exact(bounds[3] as usize+2).map_err(|_|Cause::Source(Error::Allocation))?;
        initial_rows.resize(bounds[3] as usize+2,None);
        progress.stage=Stage::Initialize;
        for row in initial_rows.iter_mut().take(bounds[3] as usize) {
            *row=Some(Row::Edges {entries:Vec::new()});progress.initialized_rows+=1;
        }
        table=Some(EdgeTable {bounds,source_capacity_edges:32,source_stride_words:stride,
            source_flag_32:1,allocated_words:words,rows:core::mem::take(&mut initial_rows)});
        let table=table.as_mut().ok_or(Cause::Source(Error::NativeOutOfBounds))?;
        for (rectangle,&[x,y,width,height]) in rectangles.entries().iter().enumerate() {
            progress.stage=Stage::Rectangle;progress.rectangle=Some(rectangle);
            progress.row=None;progress.row_ordinal=None;progress.count=None;
            if height<=0 {continue;}
            let first=x.wrapping_shl(8);let last=x.wrapping_add(width).wrapping_shl(8);
            let mut row_index=y.wrapping_sub(bounds[1]);
            for ordinal in 0..height {
                progress.stage=Stage::ReadRow;progress.row_ordinal=Some(ordinal);
                let index=usize::try_from(row_index).map_err(|_|Cause::Source(Error::NativeOutOfBounds))?;
                progress.row=Some(index);
                if index>=bounds[3] as usize {return Err(Cause::Source(Error::NativeOutOfBounds));}
                let count=match table.rows.get(index).and_then(Option::as_ref) {
                    Some(Row::Edges {entries})=>entries.len(),_=>return Err(Cause::Source(Error::NativeOutOfBounds)),
                };progress.count=Some(count);
                let next=i32::try_from(count).map_err(|_|Cause::Source(Error::Size))?
                    .checked_add(1).ok_or(Cause::Source(Error::Size))?;
                if next>=table.source_capacity_edges {

                    progress.stage=Stage::Grow;
                    let capacity=next.checked_mul(2).ok_or(Cause::Source(Error::Size))?;
                    let (stride,words)=path_allocation(bounds[3],capacity).map_err(Cause::Source)?;
                    table.source_capacity_edges=capacity;table.source_stride_words=stride;table.allocated_words=words;
                }
                let Some(Some(Row::Edges {entries}))=table.rows.get_mut(index) else {
                    return Err(Cause::Source(Error::NativeOutOfBounds));
                };
                progress.stage=Stage::Reserve;entries.try_reserve_exact(2).map_err(|_|Cause::Source(Error::Allocation))?;
                progress.stage=Stage::Append;
                entries.push([first,255]);progress.appended_pairs+=1;
                entries.push([last,-255]);progress.appended_pairs+=1;
                row_index=row_index.wrapping_add(1);
            }
        }
        for (index,row) in table.rows.iter_mut().take(bounds[3] as usize).enumerate() {
            progress.stage=Stage::ReadRow;progress.row=Some(index);
            progress.read=0;progress.written=0;progress.winding=0;progress.x=None;progress.level=None;
            let Some(Row::Edges {entries})=row else {return Err(Cause::Source(Error::NativeOutOfBounds));};
            if entries.is_empty() {*row=Some(Row::Empty);progress.collapsed_rows+=1;continue;}
            progress.stage=Stage::Sort;sort::sort_integer_pairs_source(entries).map_err(Cause::Sort)?;
            while progress.read<entries.len() {
                progress.stage=Stage::Accumulate;
                let x=entries[progress.read][0];progress.x=Some(x);
                progress.winding=progress.winding.wrapping_add(entries[progress.read][1]);progress.read+=1;
                while progress.read<entries.len()&&entries[progress.read][0]==x {
                    progress.winding=progress.winding.wrapping_add(entries[progress.read][1]);progress.read+=1;
                }
                let absolute=if progress.winding<0 {progress.winding.wrapping_neg() as u32}else {progress.winding as u32};
                let level=absolute.min(255);progress.level=Some(level);progress.stage=Stage::Write;
                entries[progress.written]=[x,level as i32];progress.written+=1;
            }
            progress.stage=Stage::Truncate;entries.truncate(progress.written);
            progress.stage=Stage::FinalZero;if let Some(last)=entries.last_mut() {last[1]=0;}
            progress.collapsed_rows+=1;
        }
        Ok(())
    })();
    match result {
        Ok(())=>match table {
            Some(table)=>{progress.stage=Stage::Complete;Ok(Completed {rectangles,table,progress})},
            None=>Err(Failure {rectangles,table:None,initial_rows,progress,cause:Cause::Source(Error::NativeOutOfBounds)}),
        },
        Err(cause)=>Err(Failure {rectangles,table,initial_rows,progress,cause}),
    }
}
