use super::{path_allocation,EdgeTable,Error,Row};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {ReadCount,GlobalGrowth,Reserve,Append,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub row:usize,pub entry:[i32;2],pub step:Step,pub count_before:Option<usize>,pub count_after:Option<usize>,
    pub capacity_before:i32,pub capacity_after:i32,pub stride_before:i32,pub stride_after:i32,
    pub allocated_before:usize,pub allocated_after:usize,
}
#[derive(Debug)]
#[must_use="SAME borrowed table retains advanced global metadata/rows on failure"]
pub struct Failure {pub progress:Progress,pub error:Error}
pub fn insert_source(table:&mut EdgeTable,row:usize,entry:[i32;2])->Result<Progress,Failure> {
    let mut progress=Progress {row,entry,step:Step::ReadCount,count_before:None,count_after:None,
        capacity_before:table.source_capacity_edges,capacity_after:table.source_capacity_edges,
        stride_before:table.source_stride_words,stride_after:table.source_stride_words,
        allocated_before:table.allocated_words,allocated_after:table.allocated_words};
    let result=(||->Result<(),Error> {
        let count=match table.rows.get(row).and_then(Option::as_ref) {
            Some(Row::Edges {entries})=>entries.len(),_=>return Err(Error::NativeOutOfBounds),
        };progress.count_before=Some(count);progress.count_after=Some(count);
        if count>=table.source_capacity_edges as usize {
            progress.step=Step::GlobalGrowth;

            let doubled=i32::try_from(count).map_err(|_|Error::Size)?.checked_mul(2).ok_or(Error::Size)?;
            let (stride,words)=path_allocation(table.bounds[3],doubled)?;
            table.source_capacity_edges=doubled;table.source_stride_words=stride;table.allocated_words=words;
        }
        let capacity=table.source_capacity_edges as usize;
        let Some(Some(Row::Edges {entries}))=table.rows.get_mut(row) else {return Err(Error::NativeOutOfBounds);};
        progress.step=Step::Reserve;
        let next=count.checked_add(1).ok_or(Error::Size)?;
        if entries.capacity()<next {
            let additional=capacity.checked_sub(entries.len()).ok_or(Error::Size)?;
            entries.try_reserve_exact(additional).map_err(|_|Error::Allocation)?;
        }
        progress.step=Step::Append;entries.push(entry);progress.count_after=Some(entries.len());
        progress.step=Step::Complete;Ok(())
    })();
    progress.capacity_after=table.source_capacity_edges;progress.stride_after=table.source_stride_words;
    progress.allocated_after=table.allocated_words;
    match result {Ok(())=>Ok(progress),Err(error)=>Err(Failure {progress,error})}
}
