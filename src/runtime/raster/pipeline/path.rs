#[path = "ellipse_source.rs"] pub mod ellipse_source;
#[path = "path_comparison.rs"] pub mod comparison;
#[path = "rectangle_source.rs"] pub mod rectangle_source;
#[path = "path_point_source.rs"] pub mod point_source;
#[path = "path_curve_source.rs"] pub mod curve_source;
#[path = "path_iterator_source.rs"] pub mod iterator_source;
#[path = "path_old212200_source.rs"] pub mod old212200_source;
#[path = "path_old_fill_source.rs"] pub mod old_fill_source;
#[path = "path_old_constructor_source.rs"] pub mod old_constructor_source;
#[path = "path_old_bounds_source.rs"] pub mod old_bounds_source;
#[derive(Debug)]
pub enum Error { Count, Capacity, Allocation, MalformedStream }
#[derive(Debug)]
pub struct Path {
    pub words: Vec<u32>, pub source_capacity: i32,

    pub bounds: [f32; 4], pub source_flag_32: u8,
}
impl Path {
    fn ensure(&mut self, additional: usize) -> Result<(), Error> {
        let needed = self.words.len().checked_add(additional).ok_or(Error::Count)?;
        let needed = i32::try_from(needed).map_err(|_| Error::Count)?;
        if self.source_capacity >= needed { return Ok(()); }

        let rounded = needed.wrapping_add(needed >> 1) & !7;
        let capacity = rounded.wrapping_add(8);
        if rounded < -7 || capacity < needed || capacity <= 0 { return Err(Error::Capacity); }
        self.words.try_reserve_exact(capacity as usize - self.words.len())
            .map_err(|_| Error::Allocation)?;
        self.source_capacity = capacity;
        Ok(())
    }
    fn start(&mut self, point: [f32; 2]) -> Result<(), Error> {
        self.start_with_comparisons(point,&mut comparison::ExistingHostValueComparisons).map_err(comparison::legacy_error)
    }
    fn cubic(&mut self, a: [f32; 2], b: [f32; 2], end: [f32; 2]) -> Result<(), Error> {
        self.cubic_with_comparisons(a,b,end,&mut comparison::ExistingHostValueComparisons).map_err(comparison::legacy_error)
    }
    fn close(&mut self) -> Result<(), Error> {
        self.close_with_comparisons(&mut comparison::ExistingHostValueComparisons).map_err(comparison::legacy_error)
    }

    pub fn source_fill_candidate(&self) -> Result<bool, Error> {
        self.fill_candidate_with_comparisons(&mut comparison::ExistingHostValueComparisons).map_err(comparison::legacy_error)
    }
}

pub fn ellipse(rectangle: [f32; 4]) -> Result<Path, Error> {

    ellipse_retaining(rectangle).map_err(|failure|match failure.failure.cause {
        ellipse_source::Cause::Path(error)=>error,
        ellipse_source::Cause::Arithmetic(never)=>match never {},
        ellipse_source::Cause::Comparison {error:never,..}=>match never {},
    })
}

pub fn ellipse_retaining(rectangle:[f32;4])
    ->Result<Path,ellipse_source::BuildFailure<core::convert::Infallible>>
{
    ellipse_source::build_with_fp(rectangle.map(f32::to_bits),&mut ellipse_source::ExistingHostValueFp)
}

pub fn rectangle(rectangle: [f32; 4]) -> Result<Path, Error> {
    rectangle_retaining(rectangle).map_err(|failure|match failure.cause {
        rectangle_source::Cause::Path(error)=>error,
        rectangle_source::Cause::Arithmetic(never)=>match never {},
        rectangle_source::Cause::Comparison {error:never,..}=>match never {},
    })
}
pub fn rectangle_retaining(rectangle:[f32;4])
    ->Result<Path,rectangle_source::Failure<core::convert::Infallible>>
{
    rectangle_source::build_with_fp(rectangle.map(f32::to_bits),&mut ellipse_source::ExistingHostValueFp)
}

pub struct Line { pub from: [f32; 2], pub to: [f32; 2], pub source_closed_flag: bool }

pub struct LineIterator<'a> {
    path: &'a Path, pub cursor: usize, affine: [f32; 6], identity: bool,
    pub current: [f32; 2], pub subpath_start: [f32; 2], pub segment_index: i32,
    stack: Vec<u32>, pub source_stack_capacity: usize, pub source_tolerance_squared: f32,
}

pub struct TerminalIterator {
    cursor:usize,affine:[f32;6],identity:bool,current:[f32;2],subpath_start:[f32;2],segment_index:i32,
    stack:Vec<u32>,source_stack_capacity:usize,source_tolerance_squared:f32,
}
impl TerminalIterator {
    pub fn cursor(&self)->usize {self.cursor}
    pub fn affine(&self)->&[f32;6] {&self.affine}
    pub fn identity(&self)->bool {self.identity}
    pub fn current(&self)->&[f32;2] {&self.current}
    pub fn subpath_start(&self)->&[f32;2] {&self.subpath_start}
    pub fn segment_index(&self)->i32 {self.segment_index}
    pub fn stack_words(&self)->&[u32] {&self.stack}
    pub fn source_stack_capacity(&self)->usize {self.source_stack_capacity}
    pub fn source_tolerance_squared(&self)->f32 {self.source_tolerance_squared}
}
impl<'a> LineIterator<'a> {
    pub fn freeze_terminal(self)->TerminalIterator {
        let Self {path:_,cursor,affine,identity,current,subpath_start,segment_index,
            stack,source_stack_capacity,source_tolerance_squared}=self;
        TerminalIterator {cursor,affine,identity,current,subpath_start,segment_index,
            stack,source_stack_capacity,source_tolerance_squared}
    }
    pub fn new_source(path: &'a Path, affine: [f32; 6]) -> Result<Self, Error> {
        Self::new_source_retaining_existing_host(path,affine).map_err(point_source::legacy_construction_error)
    }
    pub fn new_source_retaining_existing_host(path:&'a Path,affine:[f32;6])
        ->Result<Self,point_source::ConstructionFailure<core::convert::Infallible>>
    {
        Self::new_source_with_comparisons(path,affine.map(f32::to_bits),
            &mut comparison::ExistingHostValueComparisons)
    }
    fn curve_stack(&mut self, margin: usize) -> Result<(), Error> {

        if self.stack.len() < self.source_stack_capacity.wrapping_sub(margin) { return Ok(()); }
        let capacity = self.source_stack_capacity.checked_mul(2).ok_or(Error::Capacity)?;
        self.source_stack_capacity = capacity;
        self.stack.try_reserve_exact(capacity.checked_sub(self.stack.len()).ok_or(Error::Capacity)?)
            .map_err(|_| Error::Allocation)
    }
    fn push_words(&mut self, words: &[u32]) -> Result<(), Error> {
        if self.stack.len().checked_add(words.len()).ok_or(Error::Count)? > self.source_stack_capacity {
            return Err(Error::Capacity);
        }
        self.stack.extend_from_slice(words);
        Ok(())
    }
    pub fn next_source_line(&mut self) -> Result<Option<Line>, Error> {
        self.next_source_line_retaining_existing_host()
            .map(|completed|completed.line).map_err(iterator_source::legacy_error)
    }
    pub fn next_source_line_retaining_existing_host(&mut self)
        ->Result<iterator_source::Completed,iterator_source::Failure<core::convert::Infallible>>
    {
        self.next_source_line_with_fp(&mut ellipse_source::ExistingHostValueFp)
    }
}
