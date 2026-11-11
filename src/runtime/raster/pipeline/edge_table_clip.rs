use super::{intersection,path_allocation,row_count,EdgeTable,Error,Row};
use super::super::region;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ExclusionStage {Header,Mask,ReadRow,Intersect,PublishRow,Resize,Dirty,Complete}
#[derive(Clone,Copy,Debug)]
pub struct ExclusionProgress {
    pub stage:ExclusionStage,pub bounds:[i32;4],pub rectangle:[i32;4],
    pub start:Option<i32>,pub end:Option<i32>,pub index:Option<usize>,
    pub fixed_right:Option<i32>,pub published_rows:usize,
}
pub enum ExclusionCause {Source(Error),Row(intersection::OwnedFailure)}
impl ExclusionCause {
    pub fn into_error(self)->Error {match self {Self::Source(error)=>error,Self::Row(failure)=>failure.error}}
}
pub struct ExclusionCompleted {pub progress:ExclusionProgress,pub mask:Option<Row>}
#[must_use="retain mask/scratch and SAME caller's advanced target; no replay"]
pub struct ExclusionFailure {pub progress:ExclusionProgress,pub mask:Option<Row>,pub cause:ExclusionCause}

pub fn exclude_source(target:&mut EdgeTable,rectangle:[i32;4])
    ->Result<ExclusionCompleted,ExclusionFailure>
{
    let mut progress=ExclusionProgress {stage:ExclusionStage::Header,bounds:target.bounds,rectangle,
        start:None,end:None,index:None,fixed_right:None,published_rows:0};
    let mut mask=None;
    let result=(||->Result<(),ExclusionCause> {
        let [x,y,w,h]=target.bounds;let [rx,ry,rw,rh]=rectangle;
        let left=x.max(rx);let top=y.max(ry);
        let right=x.wrapping_add(w).min(rx.wrapping_add(rw));
        let bottom=y.wrapping_add(h).min(ry.wrapping_add(rh));
        if right<=left||bottom<=top {return Ok(());}
        progress.stage=ExclusionStage::Mask;
        mask=Some(Row::Edges {entries:vec![[i32::MIN,255],
            [left.wrapping_shl(8),0],[right.wrapping_shl(8),255],[i32::MAX,0]]});
        let start=top.wrapping_sub(y);let end=bottom.wrapping_sub(y);
        let fixed_right=x.wrapping_add(w).wrapping_shl(8);
        progress.start=Some(start);progress.end=Some(end);progress.fixed_right=Some(fixed_right);
        for index in start..end {
            progress.stage=ExclusionStage::ReadRow;
            let index=usize::try_from(index).map_err(|_|ExclusionCause::Source(Error::NativeOutOfBounds))?;
            progress.index=Some(index);
            let original_capacity=target.source_capacity_edges;
            let row=target.rows.get(index).and_then(Option::as_ref)
                .ok_or(ExclusionCause::Source(Error::NativeOutOfBounds))?;
            let supplied_mask=mask.as_ref().ok_or(ExclusionCause::Source(Error::NativeOutOfBounds))?;
            progress.stage=ExclusionStage::Intersect;
            let completed=intersection::run_source(row,supplied_mask,fixed_right,&mut target.source_capacity_edges)
                .map_err(|failure|ExclusionCause::Row(failure.into_owned()))?;
            let row=completed.into_row();
            progress.stage=ExclusionStage::PublishRow;
            target.rows[index]=Some(row);progress.published_rows+=1;
            if target.source_capacity_edges!=original_capacity {
                progress.stage=ExclusionStage::Resize;
                let (stride,words)=path_allocation(target.bounds[3],target.source_capacity_edges)
                    .map_err(ExclusionCause::Source)?;
                target.source_stride_words=stride;target.allocated_words=words;
            }
        }
        progress.stage=ExclusionStage::Dirty;target.source_flag_32=1;Ok(())
    })();
    match result {
        Ok(())=>{progress.stage=ExclusionStage::Complete;Ok(ExclusionCompleted {progress,mask})},
        Err(cause)=>Err(ExclusionFailure {progress,mask,cause}),
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ScanStage {Flag,ReadRow,Empty,Complete}
#[derive(Clone,Copy,Debug)]
pub struct ScanProgress {
    pub stage:ScanStage,pub original_flag:u8,pub original_height:i32,
    pub row:Option<usize>,pub count:Option<usize>,pub completed_rows:usize,
}
pub struct ScanCompleted {pub present:bool,pub progress:ScanProgress}
#[must_use="SAME target retains dirty clear and earlier row reads"]
pub struct ScanFailure {pub progress:ScanProgress,pub error:Error}
pub fn scan_source(target:&mut EdgeTable)->Result<ScanCompleted,ScanFailure> {
    let mut progress=ScanProgress {stage:ScanStage::Flag,original_flag:target.source_flag_32,
        original_height:target.bounds[3],row:None,count:None,completed_rows:0};
    let result=(||->Result<bool,Error> {
        if target.source_flag_32==0 {return Ok(target.bounds[3]!=0);}
        target.source_flag_32=0;
        for index in 0..target.bounds[3].max(0) as usize {
            progress.stage=ScanStage::ReadRow;progress.row=Some(index);progress.count=None;
            let row=target.rows.get(index).and_then(Option::as_ref).ok_or(Error::NativeOutOfBounds)?;
            let count=row_count(row);progress.count=Some(count);progress.completed_rows+=1;

            if count>1 {return Ok(true);}
        }
        progress.stage=ScanStage::Empty;target.bounds[3]=0;Ok(false)
    })();
    match result {
        Ok(present)=>{progress.stage=ScanStage::Complete;Ok(ScanCompleted {present,progress})},
        Err(error)=>Err(ScanFailure {progress,error}),
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ClipStage {Initial,Subtract,Exclude,Scan,Complete}
#[derive(Clone,Copy,Debug)]
pub struct ClipProgress {
    pub stage:ClipStage,pub current_index:Option<usize>,pub outside_index:Option<usize>,
    pub subtracted:usize,pub excluded:usize,
}
pub enum ClipCause {Source(Error),Subtract(region::subtract::Failure),Exclusion(ExclusionFailure),Scan(ScanFailure)}
impl ClipCause {
    pub fn into_error(self)->Error {match self {
        Self::Source(error)=>error,Self::Subtract(failure)=>Error::Region(failure.error),
        Self::Exclusion(failure)=>failure.cause.into_error(),Self::Scan(failure)=>failure.error,
    }}
}
#[must_use="returned target/current are the supplied table/list, complement stays owned"]
pub struct Completed<'t,'c> {
    pub target:&'t mut EdgeTable,pub current:&'c region::Rectangles,
    pub outside:region::Rectangles,pub progress:ClipProgress,pub scan:ScanCompleted,
}
#[must_use="retain SAME target, current, partial complement and stopped mask/row scratch"]
pub struct Failure<'t,'c> {
    pub target:&'t mut EdgeTable,pub current:&'c region::Rectangles,
    pub outside:Option<region::Rectangles>,pub progress:ClipProgress,pub cause:ClipCause,
}
pub fn intersect_rectangles_source<'t,'c>(target:&'t mut EdgeTable,current:&'c region::Rectangles)
    ->Result<Completed<'t,'c>,Failure<'t,'c>>
{
    let mut progress=ClipProgress {stage:ClipStage::Initial,current_index:None,outside_index:None,subtracted:0,excluded:0};
    let mut outside=None;
    let result=(||->Result<ScanCompleted,ClipCause> {
        outside=Some(region::Rectangles::initial_clip(target.bounds).map_err(|error|ClipCause::Source(Error::Region(error)))?);
        let complement=outside.as_mut().ok_or(ClipCause::Source(Error::NativeOutOfBounds))?;
        for (index,rectangle) in current.entries().iter().enumerate() {
            if complement.entries().is_empty() {break;}
            progress.stage=ClipStage::Subtract;progress.current_index=Some(index);
            region::subtract::run_source(complement,*rectangle).map_err(ClipCause::Subtract)?;
            progress.subtracted+=1;
        }
        for (index,rectangle) in complement.entries().iter().enumerate() {
            progress.stage=ClipStage::Exclude;progress.outside_index=Some(index);
            exclude_source(target,*rectangle).map_err(ClipCause::Exclusion)?;
            progress.excluded+=1;
        }
        progress.stage=ClipStage::Scan;scan_source(target).map_err(ClipCause::Scan)
    })();
    match result {
        Ok(scan)=>match outside {
            Some(outside)=>{progress.stage=ClipStage::Complete;Ok(Completed {target,current,outside,progress,scan})},
            None=>Err(Failure {target,current,outside:None,progress,cause:ClipCause::Source(Error::NativeOutOfBounds)}),
        },
        Err(cause)=>Err(Failure {target,current,outside,progress,cause}),
    }
}
