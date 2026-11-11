use super::{copy_pairs,intersection_capacity,row_count,Error,Row};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Counts,CopyLeft,CopyRight,FullSpan,ReserveOutput,Merge,Grow,Append,Terminal,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub optimized:Option<bool>,pub left_cursor:usize,pub right_cursor:usize,
    pub left_level:i32,pub right_level:i32,pub last_x:i32,pub output_level:i32,
    pub chosen_x:Option<i32>,pub product:Option<i32>,pub capacity_before:i32,pub capacity_after:i32,
}
pub struct Scratch {pub left:Option<Vec<[i32;2]>>,pub right:Option<Vec<[i32;2]>>,pub output:Vec<[i32;2]>}
impl Drop for Scratch {
    fn drop(&mut self) {

        drop(core::mem::take(&mut self.output));drop(self.right.take());drop(self.left.take());
    }
}
pub struct Completed {pub row:Row,pub scratch:Scratch,pub progress:Progress}
impl Completed {

    pub fn into_row(self)->Row {
        let Self {row,scratch,progress:_}=self;drop(scratch);row
    }
}
#[must_use="retain same input rows/scratch and original advanced capacity; no replay"]
pub struct Failure<'a> {
    pub left:&'a Row,pub right:&'a Row,pub fixed_right_bound:i32,
    pub scratch:Scratch,pub progress:Progress,pub error:Error,
}
pub struct OwnedFailure {pub fixed_right_bound:i32,pub scratch:Scratch,pub progress:Progress,pub error:Error}
impl Failure<'_> {

    pub fn into_owned(self)->OwnedFailure {
        OwnedFailure {fixed_right_bound:self.fixed_right_bound,scratch:self.scratch,progress:self.progress,error:self.error}
    }
}
pub fn run_source<'a>(left:&'a Row,right:&'a Row,fixed_right_bound:i32,capacity:&mut i32)
    ->Result<Completed,Failure<'a>>
{
    let mut progress=Progress {stage:Stage::Counts,optimized:None,left_cursor:0,right_cursor:0,left_level:0,right_level:0,
        last_x:i32::MIN,output_level:0,chosen_x:None,product:None,capacity_before:*capacity,capacity_after:*capacity};
    let mut scratch=Scratch {left:None,right:None,output:Vec::new()};
    let result=(||->Result<Row,Error> {
        if row_count(left)==0 {return Ok(Row::Empty);}
        if row_count(right)==0 {return Ok(Row::Empty);}
        progress.stage=Stage::CopyLeft;scratch.left=Some(copy_pairs(left)?);
        progress.stage=Stage::CopyRight;scratch.right=Some(copy_pairs(right)?);
        let left=scratch.left.as_mut().ok_or(Error::NativeOutOfBounds)?;
        let right=scratch.right.as_ref().ok_or(Error::NativeOutOfBounds)?;
        let optimized=right.len()==2&&right[0][1]>=255;progress.optimized=Some(optimized);
        if optimized {
            progress.stage=Stage::FullSpan;

            let upper=fixed_right_bound.min(right[1][0]);
            if left.last().ok_or(Error::NativeOutOfBounds)?[0]>upper {
                if left[0][0]>=upper {return Ok(Row::Empty);}
                let mut end=left.len()-1;
                while end>0&&left[end-1][0]>upper {end-=1;}
                left.truncate(end+1);left[end]=[upper,0];
            }
            let lower=right[0][0];
            if left[0][0]<lower {
                let mut start=left.len()-1;
                while start>0&&left[start][0]>lower {start-=1;}
                if start>0 {left.drain(..start);}
                left[0][0]=lower;
            }
            return Ok(Row::Edges {entries:core::mem::take(left)});
        }
        progress.stage=Stage::ReserveOutput;
        scratch.output.try_reserve_exact(left.len()).map_err(|_|Error::Allocation)?;
        while progress.left_cursor<left.len()&&progress.right_cursor<right.len() {
            progress.stage=Stage::Merge;
            let x;
            if left[progress.left_cursor][0]>right[progress.right_cursor][0] {
                x=right[progress.right_cursor][0];progress.right_level=right[progress.right_cursor][1];progress.right_cursor+=1;
            }else {
                x=left[progress.left_cursor][0];
                if x==right[progress.right_cursor][0] {progress.right_level=right[progress.right_cursor][1];progress.right_cursor+=1;}
                progress.left_level=left[progress.left_cursor][1];progress.left_cursor+=1;
            }
            progress.chosen_x=Some(x);
            if x<=progress.last_x {continue;}
            if x>=fixed_right_bound {break;}
            let product=progress.right_level.wrapping_add(1).wrapping_mul(progress.left_level);
            let product=(if product<0 {product.wrapping_add(255)}else {product})>>8;progress.product=Some(product);
            if product!=progress.output_level {
                progress.stage=Stage::Grow;intersection_capacity(capacity,scratch.output.len())?;
                progress.stage=Stage::Append;scratch.output.try_reserve_exact(1).map_err(|_|Error::Allocation)?;
                scratch.output.push([x,product]);progress.output_level=product;
            }
            progress.last_x=x;
        }
        if progress.output_level>0 {
            progress.stage=Stage::Terminal;intersection_capacity(capacity,scratch.output.len())?;
            scratch.output.try_reserve_exact(1).map_err(|_|Error::Allocation)?;
            scratch.output.push([fixed_right_bound,0]);
        }
        if scratch.output.is_empty() {Ok(Row::Empty)}else {Ok(Row::Edges {entries:core::mem::take(&mut scratch.output)})}
    })();
    progress.capacity_after=*capacity;
    match result {
        Ok(row)=>{progress.stage=Stage::Complete;Ok(Completed {row,scratch,progress})},
        Err(error)=>Err(Failure {left,right,fixed_right_bound,scratch,progress,error}),
    }
}
