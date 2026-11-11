use super::{gradient::Gradient, rectangle};
#[derive(Debug)]
pub enum Error { Count, Size, Allocation, NativeOutOfBounds, Unwritten,
    NormCrtOverflow { operands: [f32; 2] }, NormCrtInfinity { operands: [f32; 2] } }
pub struct Table { pub words: Vec<Option<u32>>, pub source_size: i32 }
impl Table {
    pub fn read_source_index(&self, index: i32) -> Result<u32, Error> {
        let index = usize::try_from(index).map_err(|_| Error::NativeOutOfBounds)?;
        self.words.get(index).ok_or(Error::NativeOutOfBounds)?.ok_or(Error::Unwritten)
    }
}
pub struct Slot { pub table: Option<Table> }
impl Slot { pub fn fresh() -> Self { Self { table: None } } }

pub enum LookupProgress {
    Interpolation { stop: usize, next_index: i32, numerator: i32, end: i32, length: i32 },
    FinalFill { next_index: i32 },
}
pub enum ScheduledRebuild<C> {
    Complete(usize),
    Superseded { reason: C, progress: LookupProgress },
}
fn cvtt_i32(x: f32) -> i32 {
    if !x.is_finite() || x >= 2147483648.0 || x < -2147483648.0 { i32::MIN }
    else { x as i32 }
}
fn rounded_index(x: f64) -> i32 {
    (x + 6755399441055744.0).to_bits() as u32 as i32
}
pub fn norm_operands(gradient: &Gradient, affine: [f32; 6]) -> [f32; 2] {
    let [a, b, tx, c, d, ty] = affine;
    let [sx, sy] = gradient.start; let [ex, ey] = gradient.end;
    let start_y = ((c * sx) + (d * sy)) + ty;
    let start_x = ((sx * a) + (sy * b)) + tx;
    let end_y = ((c * ex) + (d * ey)) + ty;
    let end_x = ((a * ex) + (b * ey)) + tx;
    [start_x - end_x, start_y - end_y]
}
pub fn source_norm_checked(operands: [f32; 2]) -> Result<f32, Error> {
    let [x, y] = operands;
    if x.is_finite() && y.is_finite() {

        let yy = (y as f64) * (y as f64);
        let xx = (x as f64) * (x as f64);
        let magnitude = (yy + xx).sqrt();
        if magnitude > f64::from_bits(0x47efffffe0000000) {

            return Err(Error::NormCrtOverflow { operands });
        }
        return Ok(magnitude as f32);
    }
    if x.is_infinite() || y.is_infinite() {

        return Err(Error::NormCrtInfinity { operands });
    }

    Ok((x * x) + (y * y))
}
fn write(table: &mut Table, index: i32, value: u32) -> Result<(), Error> {
    let index = usize::try_from(index).map_err(|_| Error::NativeOutOfBounds)?;
    *table.words.get_mut(index).ok_or(Error::NativeOutOfBounds)? = Some(value);
    Ok(())
}

pub fn rebuild_with_source_norm(slot: &mut Slot, gradient: &Gradient, source_norm: f32)
    -> Result<usize, Error> {
    match rebuild_with_source_norm_scheduled(slot, gradient, source_norm,
        &mut || None::<std::convert::Infallible>)? {
        ScheduledRebuild::Complete(size) => Ok(size),
        ScheduledRebuild::Superseded { reason, .. } => match reason {},
    }
}

pub fn rebuild_with_source_norm_scheduled<C>(slot: &mut Slot, gradient: &Gradient,
    source_norm: f32, checkpoint: &mut impl FnMut() -> Option<C>)
    -> Result<ScheduledRebuild<C>, Error> {
    let norm_integer = cvtt_i32(source_norm);
    let triple = norm_integer.wrapping_mul(3);
    let count = i32::try_from(gradient.stops().len()).map_err(|_| Error::Count)?;
    let stop_limit = count.wrapping_shl(8).wrapping_sub(256);
    let mut size = if stop_limit < 2 { 1 } else { stop_limit };
    if size >= triple { size = triple; }
    if norm_integer <= 0 { size = 1; }

    drop(slot.table.take());
    if size <= 0 { return Err(Error::Size); }
    let size_usize = size as usize;
    if size_usize.checked_mul(4).is_none() { return Err(Error::Size); }
    let mut words = Vec::new();
    words.try_reserve_exact(size_usize).map_err(|_| Error::Allocation)?;
    words.resize(size_usize, None);
    slot.table = Some(Table { words, source_size: size });
    let table = slot.table.as_mut().ok_or(Error::Allocation)?;
    let mut cursor = 0i32;
    if count >= 2 {
        let scale = (size - 1) as f64;
        for stop in 1..gradient.stops().len() {
            let previous = gradient.stops()[stop - 1].argb;
            let next = gradient.stops()[stop];
            let end = rounded_index(next.position * scale);
            let length = end.wrapping_sub(cursor);
            if length <= 0 { continue; }

            if end < cursor || end > size { return Err(Error::NativeOutOfBounds); }
            let rb = previous & 0x00ff00ff;
            let delta_rb = (next.argb & 0x00ff00ff).wrapping_sub(rb);
            let ga = (previous >> 8) & 0x00ff00ff;
            let delta_ga = ((next.argb >> 8) & 0x00ff00ff).wrapping_sub(ga);
            let ga = ga.wrapping_shl(8);
            let mut numerator = 0i32;
            while cursor != end {
                let weight = numerator / length;
                let mixed_rb = ((weight as u32).wrapping_mul(delta_rb) >> 8).wrapping_add(rb);
                let mixed_ga = (weight as u32).wrapping_mul(delta_ga).wrapping_add(ga);
                let argb = (mixed_ga & 0xff000000) | (mixed_rb & 0x00ff00ff)
                    | (mixed_ga & 0x0000ff00);
                write(table, cursor, rectangle::premultiply_argb(argb))?;
                cursor = cursor.wrapping_add(1);
                numerator = numerator.wrapping_add(256);
                if let Some(reason) = checkpoint() {
                    return Ok(ScheduledRebuild::Superseded { reason,
                        progress: LookupProgress::Interpolation {
                            stop, next_index: cursor, numerator, end, length,
                        } });
                }
            }
        }
    }
    let final_argb = gradient.stops().last().map_or(0, |stop| stop.argb);
    let final_argb = rectangle::premultiply_argb(final_argb);
    if cursor < 0 || cursor > size { return Err(Error::NativeOutOfBounds); }
    while cursor != size {
        write(table, cursor, final_argb)?;
        cursor += 1;
        if let Some(reason) = checkpoint() {
            return Ok(ScheduledRebuild::Superseded { reason,
                progress: LookupProgress::FinalFill { next_index: cursor } });
        }
    }
    Ok(ScheduledRebuild::Complete(size_usize))
}
