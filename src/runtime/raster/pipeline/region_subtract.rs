use super::{Error,Rectangles};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {Read,Overlap,Left,InsertLeft,Right,InsertRight,Top,InsertTop,Bottom,Remove,Count,Shrink,Advance,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub step:Step,pub cut:[i32;4],pub cursor:usize,pub index:Option<usize>,
    pub entry_before:Option<[i32;4]>,pub completed_visits:usize,pub inserted:usize,pub removed:usize,
    pub capacity_before:i32,pub capacity_after:i32,
}
#[must_use="SAME caller retains advanced list; stopped cursor is not replay permission"]
pub struct Failure {pub progress:Progress,pub error:Error}
pub fn run_source(target:&mut Rectangles,cut:[i32;4])->Result<Progress,Failure> {
    let mut progress=Progress {step:Step::Read,cut,cursor:target.entries.len(),index:None,entry_before:None,
        completed_visits:0,inserted:0,removed:0,capacity_before:target.source_capacity,capacity_after:target.source_capacity};
    let result=(||->Result<(),Error> {
        let [left,top,width,height]=cut;
        let right=left.wrapping_add(width);let bottom=top.wrapping_add(height);
        while progress.cursor>0 {
            progress.step=Step::Read;
            let index=progress.cursor-1;progress.index=Some(index);
            let [x,y,w,h]=target.entries[index];progress.entry_before=Some([x,y,w,h]);
            let xr=x.wrapping_add(w);let yb=y.wrapping_add(h);
            progress.step=Step::Overlap;

            if right.wrapping_sub(x)<=0||xr.wrapping_sub(left)<=0||y>=bottom||yb.wrapping_sub(top)<=0 {
                progress.cursor-=1;progress.completed_visits+=1;continue;
            }
            let left_width=left.wrapping_sub(x);
            if left_width>0 {
                progress.step=Step::Left;
                if top<=y&&yb<=bottom&&xr<=right {
                    target.entries[index][2]=left_width;progress.cursor-=1;
                }else {
                    target.entries[index][0]=left;target.entries[index][2]=xr.wrapping_sub(left);
                    progress.step=Step::InsertLeft;
                    target.insert_after_source(index,[x,y,left_width,h])?;progress.inserted+=1;
                    progress.step=Step::Advance;progress.cursor=progress.cursor.checked_add(1).ok_or(Error::Count)?;
                }
                progress.completed_visits+=1;continue;
            }
            let right_width=xr.wrapping_sub(right);
            if right_width>0 {
                progress.step=Step::Right;
                target.entries[index][0]=right;target.entries[index][2]=right_width;
                if top<=y&&yb<=bottom {progress.cursor-=1;}else {
                    progress.step=Step::InsertRight;
                    target.insert_after_source(index,[x,y,right.wrapping_sub(x),h])?;progress.inserted+=1;
                    progress.step=Step::Advance;progress.cursor=progress.cursor.checked_add(1).ok_or(Error::Count)?;
                }
                progress.completed_visits+=1;continue;
            }
            let top_height=top.wrapping_sub(y);
            if top_height>0 {
                progress.step=Step::Top;
                if yb>bottom {
                    target.entries[index][1]=top;target.entries[index][3]=yb.wrapping_sub(top);
                    progress.step=Step::InsertTop;
                    target.insert_after_source(index,[x,y,w,top_height])?;progress.inserted+=1;
                    progress.step=Step::Advance;progress.cursor=progress.cursor.checked_add(1).ok_or(Error::Count)?;
                }else {target.entries[index][3]=top_height;progress.cursor-=1;}
                progress.completed_visits+=1;continue;
            }
            let bottom_height=yb.wrapping_sub(bottom);
            if bottom_height>0 {
                progress.step=Step::Bottom;
                target.entries[index][1]=bottom;target.entries[index][3]=bottom_height;
                progress.cursor-=1;progress.completed_visits+=1;continue;
            }
            progress.step=Step::Remove;target.entries.remove(index);progress.removed+=1;
            progress.step=Step::Count;
            let count=i32::try_from(target.entries.len()).map_err(|_|Error::Count)?;
            let twice=count.checked_mul(2).ok_or(Error::Capacity)?.max(0);
            if target.source_capacity>twice {
                let capacity=count.max(4);
                if target.source_capacity>capacity {
                    progress.step=Step::Shrink;
                    target.entries.shrink_to(capacity as usize);target.source_capacity=capacity;
                }
            }
            progress.step=Step::Advance;progress.cursor-=1;progress.completed_visits+=1;
        }
        Ok(())
    })();
    progress.capacity_after=target.source_capacity;
    match result {
        Ok(())=>{progress.step=Step::Complete;Ok(progress)},
        Err(error)=>Err(Failure {progress,error}),
    }
}
