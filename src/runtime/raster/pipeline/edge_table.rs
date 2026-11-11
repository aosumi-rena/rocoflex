#[path="edge_table_sample.rs"] pub mod sample;
#[path="edge_table_coverage.rs"] pub mod coverage;
#[path="edge_table_intersection.rs"] pub mod intersection;
#[path="edge_table_join.rs"] pub mod join;
#[path="edge_table_clip.rs"] pub mod clip;
#[path="edge_table_rectangles.rs"] pub mod rectangles;
#[path="edge_table_path.rs"] pub mod path_source;
#[derive(Debug)]
pub enum Error { Domain, Size, Allocation, NativeOutOfBounds, Divide, Path(super::path::Error), Region(super::region::Error) }
#[derive(Clone)]
pub enum Row {
    Empty,

    Span { fixed_x: [i32; 2], level: i32 },

    Edges { entries: Vec<[i32; 2]> },
}
pub struct EdgeTable {
    pub bounds: [i32; 4], pub source_capacity_edges: i32,
    pub source_stride_words: i32, pub source_flag_32: u8,
    pub allocated_words: usize,

    pub rows: Vec<Option<Row>>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PathRowsStep {Prepare,ReadRow,Sort,Accumulate,Write,Truncate,FinalZero,Complete}
#[derive(Clone,Copy,Debug)]
pub struct PathRowsProgress {
    pub step:PathRowsStep,pub row:usize,pub logical_rows:usize,pub completed_rows:usize,
    pub read:usize,pub written:usize,pub winding:i32,pub x:Option<i32>,pub level:Option<u32>,
}
#[derive(Debug)]
pub enum PathRowsCause {Table(Error),Sort(super::sort::Error)}
#[derive(Debug)]
#[must_use="SAME borrowed EdgeTable retains sorted/rewritten rows on failure"]
pub struct PathRowsFailure {pub progress:PathRowsProgress,pub cause:PathRowsCause}

pub fn collapse_path_rows_source(table:&mut EdgeTable,source_flag_32:u8)->Result<PathRowsProgress,PathRowsFailure> {
    let mut progress=PathRowsProgress {step:PathRowsStep::Prepare,row:0,logical_rows:0,completed_rows:0,
        read:0,written:0,winding:0,x:None,level:None};
    let result=(||->Result<(),PathRowsCause> {
        let logical_rows=usize::try_from(table.bounds[3]).map_err(|_|PathRowsCause::Table(Error::Domain))?;
        progress.logical_rows=logical_rows;
        if table.rows.len()<logical_rows {return Err(PathRowsCause::Table(Error::NativeOutOfBounds));}
        for (row_index,row) in table.rows.iter_mut().take(logical_rows).enumerate() {
            progress.row=row_index;progress.step=PathRowsStep::ReadRow;
            progress.read=0;progress.written=0;progress.winding=0;progress.x=None;progress.level=None;
            let Some(Row::Edges {entries})=row else {return Err(PathRowsCause::Table(Error::NativeOutOfBounds));};
            if entries.is_empty() {*row=Some(Row::Empty);progress.completed_rows+=1;continue;}
            progress.step=PathRowsStep::Sort;
            super::sort::sort_integer_pairs_source(entries).map_err(PathRowsCause::Sort)?;
            while progress.read<entries.len() {
                progress.step=PathRowsStep::Accumulate;
                let x=entries[progress.read][0];progress.x=Some(x);
                progress.winding=progress.winding.wrapping_add(entries[progress.read][1]);progress.read+=1;
                while progress.read<entries.len()&&entries[progress.read][0]==x {
                    progress.winding=progress.winding.wrapping_add(entries[progress.read][1]);progress.read+=1;
                }
                let absolute=if progress.winding<0 {progress.winding.wrapping_neg() as u32}else {progress.winding as u32};
                let periodic=absolute&511;
                let triangle=if periodic<256 {periodic}else {periodic^511};
                let mut level=if source_flag_32!=0 {255}else {triangle};
                if absolute<256 {level=absolute;}
                progress.level=Some(level);progress.step=PathRowsStep::Write;
                entries[progress.written]=[x,level as i32];progress.written+=1;
            }
            progress.step=PathRowsStep::Truncate;entries.truncate(progress.written);
            progress.step=PathRowsStep::FinalZero;
            if let Some(last)=entries.last_mut() {last[1]=0;}
            progress.completed_rows+=1;
        }
        progress.step=PathRowsStep::Complete;Ok(())
    })();
    match result {Ok(())=>Ok(progress),Err(cause)=>Err(PathRowsFailure {progress,cause})}
}
#[link(name = "ucrt")]
unsafe extern "C" { fn floorf(x: f32) -> f32; }
fn cvtt_i32(x: f32) -> i32 {
    if !x.is_finite() || x >= 2147483648.0 || x < -2147483648.0 { i32::MIN }
    else { x as i32 }
}
fn fixed(x: f32) -> i32 {
    let scaled = x * 256.0;

    ((scaled as f64 + 6755399441055744.0).to_bits() as u32) as i32
}
fn adjusted_for_div256(x: i32) -> i32 {
    if x < 0 { x.wrapping_add(255) } else { x }
}
fn put(table: &mut EdgeTable, row: usize, value: Row) -> Result<(), Error> {
    *table.rows.get_mut(row).ok_or(Error::NativeOutOfBounds)? = Some(value);
    Ok(())
}

pub fn from_rectangle(rectangle: [f32; 4]) -> Result<EdgeTable, Error> {

    if rectangle.iter().any(|x| !x.is_finite()) || rectangle[2] < 0.0 || rectangle[3] < 0.0 {
        return Err(Error::Domain);
    }
    let raw_h = cvtt_i32(rectangle[3]); let raw_w = cvtt_i32(rectangle[2]);
    let height = raw_h.checked_add(2).ok_or(Error::Size)?;
    let width = raw_w.checked_add(2).ok_or(Error::Size)?;
    if raw_h < 0 || raw_w < 0 || height < 0 || width < 0 { return Err(Error::Size); }
    let allocated_rows = (height as usize).checked_add(2).ok_or(Error::Size)?;
    let allocated_words = allocated_rows.checked_mul(65).ok_or(Error::Size)?;

    if allocated_words > i32::MAX as usize { return Err(Error::Size); }
    let y_top = fixed(rectangle[1]);
    let adjusted_y = adjusted_for_div256(y_top);
    let source_y = adjusted_y >> 8;
    let source_x = cvtt_i32(unsafe { floorf(rectangle[0]) });
    let mut rows = Vec::new();
    rows.try_reserve_exact(allocated_rows).map_err(|_| Error::Allocation)?;
    rows.resize(allocated_rows, None);
    let mut table = EdgeTable { bounds: [source_x, source_y, width, height],
        source_capacity_edges: 32, source_stride_words: 65, source_flag_32: 1,
        allocated_words, rows };
    put(&mut table, 0, Row::Empty)?;
    let x_left = fixed(rectangle[0]);
    let x_right = fixed(rectangle[0] + rectangle[2]);
    let y_bottom = fixed(rectangle[1] + rectangle[3]);
    if x_right <= x_left || y_bottom <= y_top {
        table.bounds[3] = 0;
        return Ok(table);
    }
    let base_y = adjusted_y & !255;
    let relative_top = y_top.wrapping_sub(base_y);
    let relative_bottom = y_bottom.wrapping_sub(base_y);
    let top_row = adjusted_for_div256(relative_top) >> 8;
    let bottom_row = adjusted_for_div256(relative_bottom) >> 8;
    let span = |level| Row::Span { fixed_x: [x_left, x_right], level };
    let mut cursor = 1usize;
    let mut next_row;
    if top_row == bottom_row {
        put(&mut table, 0, span(relative_bottom.wrapping_sub(relative_top)))?;
        next_row = 1i32;
    } else {

        put(&mut table, 0, span((!y_top) & 255))?;
        next_row = 2;
        if relative_bottom >= 512 {
            let source_limit = bottom_row.max(2);
            for _ in 0..source_limit - 1 {
                put(&mut table, cursor, span(255))?;
                cursor = cursor.checked_add(1).ok_or(Error::Size)?;
            }
            next_row = source_limit.wrapping_add(1);
        }

        put(&mut table, cursor, span(y_bottom & 255))?;
        cursor = cursor.checked_add(1).ok_or(Error::Size)?;
    }
    while next_row < height {
        put(&mut table, cursor, Row::Empty)?;
        cursor = cursor.checked_add(1).ok_or(Error::Size)?;
        next_row = next_row.wrapping_add(1);
    }
    Ok(table)
}

pub fn from_image_translation_rectangle(rectangle: [i32; 4]) -> Result<EdgeTable, Error> {
    if rectangle[2] <= 0 || rectangle[3] <= 0 { return Err(Error::Domain); }
    let height = rectangle[3] as usize;
    let allocated_rows = height.checked_add(2).ok_or(Error::Size)?;
    let allocated_words = allocated_rows.checked_mul(65).ok_or(Error::Size)?;
    if allocated_words > i32::MAX as usize { return Err(Error::Size); }
    let left = rectangle[0].wrapping_shl(8);
    let right = rectangle[0].wrapping_add(rectangle[2]).wrapping_shl(8);
    let mut rows = Vec::new();
    rows.try_reserve_exact(allocated_rows).map_err(|_| Error::Allocation)?;
    rows.resize(allocated_rows, None);
    for row in rows.iter_mut().take(height) {
        *row = Some(Row::Span { fixed_x: [left, right], level: 255 });
    }
    Ok(EdgeTable { bounds: rectangle, source_capacity_edges: 32,
        source_stride_words: 65, source_flag_32: 1, allocated_words, rows })
}

fn cvtt_i64(x: f64) -> i64 {
    if !x.is_finite() || x >= 9223372036854775808.0 || x < -9223372036854775808.0 { i64::MIN }
    else { x as i64 }
}
fn path_allocation(height: i32, capacity: i32) -> Result<(i32, usize), Error> {
    let stride = capacity.checked_mul(2).and_then(|n| n.checked_add(1)).ok_or(Error::Size)?;
    let rows = height.max(0).checked_add(2).ok_or(Error::Size)?;
    let words = rows.checked_mul(stride).ok_or(Error::Size)?;
    Ok((stride, usize::try_from(words).map_err(|_| Error::Size)?))
}

pub fn from_path_source(bounds: [i32; 4], path: &super::path::Path, affine: [f32; 6])
    -> Result<EdgeTable, Error> {
    path_source::run_existing_host_source(path,bounds,affine)
        .map(|completed|completed.table).map_err(path_source::Failure::into_error)
}

pub fn from_rectangle_list_source(rectangles: &super::region::Rectangles) -> Result<EdgeTable, Error> {
    rectangles::run_source(rectangles).map(|completed|completed.table).map_err(|failure|failure.cause.into_error())
}

fn copy_pairs(row: &Row) -> Result<Vec<[i32; 2]>, Error> {
    let count = row_count(row);
    let mut out = Vec::new();
    out.try_reserve_exact(count).map_err(|_| Error::Allocation)?;
    match row {
        Row::Empty => {},
        Row::Span { fixed_x, level } => out.extend_from_slice(&[[fixed_x[0], *level], [fixed_x[1], 0]]),
        Row::Edges { entries } => out.extend_from_slice(entries),
    }
    Ok(out)
}
fn row_count(row: &Row) -> usize {
    match row { Row::Empty => 0, Row::Span { .. } => 2, Row::Edges { entries } => entries.len() }
}
fn intersection_capacity(capacity: &mut i32, count: usize) -> Result<(), Error> {
    let count = i32::try_from(count).map_err(|_| Error::Size)?;
    if count >= *capacity { *capacity = count.checked_mul(2).ok_or(Error::Size)?.max(256); }
    Ok(())
}
impl EdgeTable {

    pub fn copy_source(&self) -> Result<Self, Error> {
        if self.bounds[3] < 0 || self.source_stride_words < 0 { return Err(Error::Domain); }
        let height = self.bounds[3] as usize;
        let row_count = height.checked_add(2).ok_or(Error::Size)?;
        let words = row_count.checked_mul(self.source_stride_words as usize).ok_or(Error::Size)?;
        if words > i32::MAX as usize || (height > 0 && self.source_stride_words == 0) {
            return Err(Error::Size);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(row_count).map_err(|_| Error::Allocation)?;
        rows.resize(row_count, None);
        for index in 0..height {
            rows[index] = Some(match self.rows.get(index).and_then(Option::as_ref)
                .ok_or(Error::NativeOutOfBounds)? {
                Row::Empty => Row::Empty,
                Row::Span { fixed_x, level } => Row::Span { fixed_x: *fixed_x, level: *level },
                Row::Edges { entries } => {
                    let mut copied = Vec::new();
                    copied.try_reserve_exact(entries.len()).map_err(|_| Error::Allocation)?;
                    copied.extend_from_slice(entries);
                    Row::Edges { entries: copied }
                }
            });
        }
        Ok(Self { bounds: self.bounds, source_capacity_edges: self.source_capacity_edges,
            source_stride_words: self.source_stride_words, source_flag_32: self.source_flag_32,
            allocated_words: words, rows })
    }

    pub fn intersect_rectangles_mut_source(&mut self, current: &super::region::Rectangles)
        -> Result<bool, Error> {
        clip::intersect_rectangles_source(self,current)
            .map(|completed|completed.scan.present).map_err(|failure|failure.cause.into_error())
    }

    pub fn intersect_path_mut_source(&mut self, path: &super::path::Path, affine: [f32; 6])
        -> Result<bool, Error> {
        let temporary = from_path_source(self.bounds, path, affine)?;
        self.intersect_mut_source(&temporary)?;
        let nonempty = self.has_source_spans();
        drop(temporary);
        nonempty
    }

    pub fn intersect_mut_source(&mut self, other: &Self) -> Result<(), Error> {
        join::run_source(self,other).map(|_|()).map_err(|failure|failure.cause.into_error())
    }

    pub fn has_source_spans(&mut self) -> Result<bool, Error> {
        clip::scan_source(self).map(|completed|completed.present).map_err(|failure|failure.error)
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CoverageEvent {
    Fringe { x: i32, accumulated: i32 },
    Span { x: i32, count: i32, level: i32 },
}

pub fn walk_coverage_row<E>(entries: &[[i32; 2]], mut emit: impl FnMut(CoverageEvent) -> Result<(), E>)
    -> Result<(), E> {
    walk_coverage_row_ordered(entries, |event, _terminal| emit(event))
}

pub fn walk_coverage_row_ordered<E>(entries: &[[i32; 2]],
    mut emit: impl FnMut(CoverageEvent, bool) -> Result<(), E>) -> Result<(), E> {
    coverage::walk_ordered(entries,&mut emit).map(|_|()).map_err(|failure|failure.error)
}
fn truncated_fixed(value: i32) -> i32 {
    (if value < 0 { value.wrapping_add(255) } else { value }) >> 8
}
