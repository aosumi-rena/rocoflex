#[path="region_subtract.rs"] pub mod subtract;
#[derive(Debug)]
pub enum Error { Count, Capacity, Allocation }
pub struct Rectangles { entries: Vec<[i32; 4]>, source_capacity: i32 }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PairStep {Incoming,Current,Horizontal,Vertical,Append,Publish,ReleaseOld,Complete}
#[derive(Clone,Copy,Debug)]
pub struct PairProgress {
    pub step:PairStep,pub incoming:Option<usize>,pub current:Option<usize>,
    pub candidate:Option<[i32;4]>,pub visited_pairs:usize,pub appended:usize,
}
pub struct PairCompleted {pub present:bool,pub progress:PairProgress}
#[must_use="SAME supplied lists stay with caller; partial replacement is not a rollback or retry"]
pub struct PairFailure {pub replacement:Rectangles,pub progress:PairProgress,pub error:Error}
impl Rectangles {
    fn empty() -> Self { Self { entries: Vec::new(), source_capacity: 0 } }
    pub fn entries(&self) -> &[[i32; 4]] { &self.entries }
    pub fn source_capacity(&self) -> i32 { self.source_capacity }

    pub fn path_intersection_source(&self, path: &super::path::Path, affine: [f32; 6])
        -> Result<Option<super::edge_table::EdgeTable>, super::edge_table::Error> {
        let mut table = super::edge_table::from_rectangle_list_source(self)?;
        if table.intersect_path_mut_source(path, affine)? { Ok(Some(table)) } else { Ok(None) }
    }

    pub fn initial_clip(rectangle: [i32; 4]) -> Result<Self, Error> {
        if rectangle[2] <= 0 || rectangle[3] <= 0 { return Ok(Self::empty()); }
        Self::single(rectangle)
    }

    pub fn single(rectangle: [i32; 4]) -> Result<Self, Error> {
        let mut entries = Vec::new();
        entries.try_reserve_exact(8).map_err(|_| Error::Allocation)?;
        entries.push(rectangle);
        Ok(Self { entries, source_capacity: 8 })
    }

    pub fn copy_source(&self) -> Result<Self, Error> {
        let count = i32::try_from(self.entries.len()).map_err(|_| Error::Count)?;
        let capacity = if count == 0 { 0 } else {
            let rounded = (count as u32).wrapping_add((count as u32) >> 1) & 0x7ffffff8;
            let result = rounded.wrapping_add(8);
            if result > i32::MAX as u32 || result < count as u32 { return Err(Error::Capacity); }
            result as i32
        };
        let mut entries = Vec::new();
        entries.try_reserve_exact(capacity as usize).map_err(|_| Error::Allocation)?;
        entries.extend_from_slice(&self.entries);
        Ok(Self { entries, source_capacity: capacity })
    }

    pub fn bounds_source(&self) -> [i32; 4] {
        let Some(&first) = self.entries.first() else { return [0; 4]; };
        if self.entries.len() == 1 { return first; }
        let [mut left, mut top, width, height] = first;
        let mut right = left.wrapping_add(width);
        let mut bottom = top.wrapping_add(height);
        for &[x, y, width, height] in self.entries[1..].iter().rev() {
            left = left.min(x); top = top.min(y);
            right = right.max(x.wrapping_add(width));
            bottom = bottom.max(y.wrapping_add(height));
        }
        [left, top, right.wrapping_sub(left), bottom.wrapping_sub(top)]
    }

    fn append_intersection(&mut self, rectangle: [i32; 4]) -> Result<(), Error> {
        let old_count = i32::try_from(self.entries.len()).map_err(|_| Error::Count)?;
        let next_count = old_count.checked_add(1).ok_or(Error::Count)?;
        if self.source_capacity <= old_count {

            let rounded = next_count.wrapping_add(next_count >> 1) & !7;
            let capacity = rounded.wrapping_add(8);
            if rounded < -7 || capacity < next_count || capacity <= 0 { return Err(Error::Capacity); }
            self.entries.try_reserve_exact(capacity as usize - self.entries.len())
                .map_err(|_| Error::Allocation)?;
            self.source_capacity = capacity;
        }
        self.entries.push(rectangle);
        Ok(())
    }

    pub fn intersect_ordered_mut_source(&mut self, current_clip: &Self) -> Result<bool, Error> {
        self.intersect_ordered_retaining(current_clip).map(|completed|completed.present).map_err(|failure|failure.error)
    }

    pub fn intersect_ordered_retaining(&mut self,current_clip:&Self)->Result<PairCompleted,PairFailure> {
        let mut progress=PairProgress {step:PairStep::Incoming,incoming:None,current:None,candidate:None,visited_pairs:0,appended:0};
        if self.entries.is_empty() {
            progress.step=PairStep::Complete;return Ok(PairCompleted {present:false,progress});
        }
        let mut replacement = Self::empty();
        let result=(||->Result<(),Error> {
            for (incoming,&[x,y,width,height]) in self.entries.iter().enumerate() {
                progress.step=PairStep::Incoming;progress.incoming=Some(incoming);progress.current=None;
                for (current,&[clip_x,clip_y,clip_width,clip_height]) in current_clip.entries.iter().enumerate() {
                    progress.step=PairStep::Current;progress.current=Some(current);progress.candidate=None;progress.visited_pairs+=1;
                    progress.step=PairStep::Horizontal;
                    let left=x.max(clip_x);
                    let intersection_width=x.wrapping_add(width).min(clip_x.wrapping_add(clip_width)).wrapping_sub(left);
                    if intersection_width<=0 {continue;}
                    progress.step=PairStep::Vertical;
                    let top=y.max(clip_y);
                    let intersection_height=y.wrapping_add(height).min(clip_y.wrapping_add(clip_height)).wrapping_sub(top);
                    if intersection_height<=0 {continue;}
                    let candidate=[left,top,intersection_width,intersection_height];progress.candidate=Some(candidate);
                    progress.step=PairStep::Append;replacement.append_intersection(candidate)?;progress.appended+=1;
                }
            }
            Ok(())
        })();
        if let Err(error)=result {return Err(PairFailure {replacement,progress,error});}
        progress.step=PairStep::Publish;
        let old=core::mem::replace(self,replacement);
        progress.step=PairStep::ReleaseOld;
        drop(old);
        progress.step=PairStep::Complete;
        Ok(PairCompleted {present:!self.entries.is_empty(),progress})
    }

    fn insert_after_source(&mut self, index: usize, rectangle: [i32; 4]) -> Result<(), Error> {
        let count = i32::try_from(self.entries.len()).map_err(|_| Error::Count)?;
        if self.source_capacity <= count {

            let next = count.checked_add(1).ok_or(Error::Count)?;
            let rounded = count.checked_add(next >> 1).and_then(|n| n.checked_add(1))
                .ok_or(Error::Capacity)? & !7;
            let capacity = rounded.checked_add(8).ok_or(Error::Capacity)?;
            if capacity <= count { return Err(Error::Capacity); }
            self.entries.try_reserve_exact(capacity as usize - self.entries.len())
                .map_err(|_| Error::Allocation)?;
            self.source_capacity = capacity;
        }
        if index >= self.entries.len() { return Err(Error::Count); }
        self.entries.insert(index + 1, rectangle);
        Ok(())
    }

    pub fn subtract_source(&mut self, cut: [i32; 4]) -> Result<(), Error> {
        subtract::run_source(self,cut).map(|_|()).map_err(|failure|failure.error)
    }
}
