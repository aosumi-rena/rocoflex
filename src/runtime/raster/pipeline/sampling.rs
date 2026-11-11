use super::image::BitmapLayout;
#[path = "sampling_texels.rs"] mod immutable_texels;
use immutable_texels::ImmutableTexels;
#[path = "shader_prefix.rs"] pub mod source_caller;
#[path = "shader_memory.rs"] pub mod source_memory;
#[derive(Debug)]
pub enum Error { Layout, EmptySource, Size, Span, Divide, NativeAddress, Allocation, Unwritten, EdgeCount,
    Initialization { cause: ScratchError, renderer: EdgeRenderer } }
#[derive(Clone, Copy, Debug)]
pub enum ScratchError { Size, Allocation }
impl From<ScratchError> for Error {
    fn from(error: ScratchError) -> Self { match error {
        ScratchError::Size => Self::Size, ScratchError::Allocation => Self::Allocation,
    } }
}

fn cvtt_i32(value: f32) -> i32 {
    if !value.is_finite() || value >= 2147483648.0 || value < -2147483648.0 {
        i32::MIN
    } else { value as i32 }
}
#[derive(Clone, Copy, Debug)]
pub struct CoordinateStep {
    pub current: i32, pub divisor: i32, pub step: i32,
    pub error: i32, pub remainder: i32,
}
impl CoordinateStep {
    fn start(first: i32, end: i32, length: i32, bias: i32) -> Result<Self, Error> {
        let delta = end.wrapping_sub(first);
        let mut step = delta.checked_div(length).ok_or(Error::Divide)?;
        let mut remainder = delta.checked_rem(length).ok_or(Error::Divide)?;

        if remainder <= 0 {
            remainder = remainder.wrapping_add(length);
            step = step.wrapping_sub(1);
        }
        Ok(Self { current: first.wrapping_add(bias), divisor: length, step,
            error: remainder.wrapping_sub(length), remainder })
    }
    fn source_next(&mut self) -> i32 {
        let prior = self.current;
        self.current = self.current.wrapping_add(self.step);
        self.error = self.error.wrapping_add(self.remainder);
        if self.error > 0 {
            self.error = self.error.wrapping_sub(self.divisor);
            self.current = self.current.wrapping_add(1);
        }
        prior
    }
}
#[derive(Debug)]
pub struct Sampler {
    pub inverse_affine: [f32; 6], pub sample_shift: f32, pub fixed_bias: i32,
    pub source_quality: i32, pub source_row: i32,
    pub x_step: Option<CoordinateStep>, pub y_step: Option<CoordinateStep>,
}
impl Sampler {

    pub fn for_source_region(inverse_affine: [f32; 6], quality: i32) -> Self {
        Self { inverse_affine, sample_shift: if quality == 0 { 0.0 } else { 0.5 },
            fixed_bias: if quality == 0 { 0 } else { -128 }, source_quality: quality,
            source_row: 0, x_step: None, y_step: None }
    }

    pub fn fill_span_nonrepeating_format2(&mut self, source: &[u8], layout: &BitmapLayout,
        output: &mut [u8], source_x: i32, length: i32) -> Result<(), Error> {
        if length <= 0 { return Err(Error::Span); }
        let x0 = source_x as f32 + self.sample_shift;
        let y = self.source_row as f32 + self.sample_shift;
        let x1 = length as f32 + x0;
        let [a, b, tx, c, d, ty] = self.inverse_affine;
        let by = b * y;
        let start_x = ((a * x0) + by) + tx;
        let dy = y * d;
        let start_y = ((x0 * c) + dy) + ty;
        let end_x = ((a * x1) + by) + tx;
        let end_y_before_translation = (x1 * c) + dy;
        let start_x_fixed = cvtt_i32(start_x * 256.0);
        let end_x_fixed = cvtt_i32(end_x * 256.0);
        self.x_step = Some(CoordinateStep::start(start_x_fixed, end_x_fixed, length, self.fixed_bias)?);

        let end_y = end_y_before_translation + ty;
        let start_y_fixed = cvtt_i32(start_y * 256.0);
        let end_y_fixed = cvtt_i32(end_y * 256.0);
        self.y_step = Some(CoordinateStep::start(start_y_fixed, end_y_fixed, length, self.fixed_bias)?);

        let mut texels = ImmutableTexels::new(source, layout);
        for ordinal in 0..length {
            let x = self.x_step.as_mut().ok_or(Error::Span)?.source_next();
            let y = self.y_step.as_mut().ok_or(Error::Span)?.source_next();
            let pixel = sample_fixed_with_texels(&mut texels, x, y, self.source_quality)?;
            let offset = (ordinal as usize).checked_mul(4).ok_or(Error::Size)?;
            let end = offset.checked_add(4).ok_or(Error::Size)?;
            output.get_mut(offset..end).ok_or(Error::NativeAddress)?.copy_from_slice(&pixel);
        }
        Ok(())
    }
}
fn source_pixel(source: &[u8], layout: &BitmapLayout, x: i32, y: i32) -> Result<[u8; 4], Error> {
    if layout.pixel_stride != 4 || layout.row_stride < layout.width.checked_mul(4).ok_or(Error::Size)? {
        return Err(Error::Layout);
    }
    let x = usize::try_from(x).map_err(|_| Error::NativeAddress)?;
    let y = usize::try_from(y).map_err(|_| Error::NativeAddress)?;
    let at = y.checked_mul(layout.row_stride)
        .and_then(|row| x.checked_mul(layout.pixel_stride).and_then(|x| row.checked_add(x)))
        .ok_or(Error::NativeAddress)?;
    source_memory::read_dword(source,at).map(u32::to_le_bytes).map_err(|_|Error::NativeAddress)
}
fn pair(first: [u8; 4], second: [u8; 4], fraction: u32) -> [u8; 4] {
    let mut out = [0; 4];
    for channel in 0..4 {

        out[channel] = ((u32::from(first[channel]) * (256 - fraction)
            + u32::from(second[channel]) * fraction + 128) >> 8) as u8;
    }
    out
}
fn validate_store_layout(layout: &BitmapLayout) -> Result<(), Error> {
    if layout.pixel_stride != 4 || layout.row_stride < layout.width.checked_mul(4).ok_or(Error::Size)? {
        return Err(Error::Layout);
    }
    Ok(())
}

pub fn sample_fixed_nonrepeating_format2(source: &[u8], layout: &BitmapLayout,
    fixed_x: i32, fixed_y: i32, quality: i32) -> Result<[u8; 4], Error> {

    sample_fixed_with_texels(&mut ImmutableTexels::new(source, layout), fixed_x, fixed_y, quality)
}
fn sample_fixed_with_texels(texels: &mut ImmutableTexels<'_>, fixed_x: i32,
    fixed_y: i32, quality: i32) -> Result<[u8; 4], Error> {
    let layout = texels.layout();
    let width = i32::try_from(layout.width).map_err(|_| Error::Size)?;
    let height = i32::try_from(layout.height).map_err(|_| Error::Size)?;
    if width <= 0 || height <= 0 { return Err(Error::EmptySource); }
    let max_x = width - 1; let max_y = height - 1;
    let x = fixed_x >> 8; let y = fixed_y >> 8;
    let x_interior = (x as u32) < max_x as u32;
    let y_interior = (y as u32) < max_y as u32;
    if quality != 0 && x_interior {
        let fx = (fixed_x & 255) as u32;
        if y_interior {
            let fy = (fixed_y & 255) as u32;
            let [tl, tr, br, bl] = texels.quad(x, y)?;
            let weights = [(256 - fy) * (256 - fx), (256 - fy) * fx, fy * fx, fy * (256 - fx)];
            let mut out = [0; 4];
            for channel in 0..4 {
                let sum = u32::from(tl[channel]) * weights[0] + u32::from(tr[channel]) * weights[1];
                let sum = u32::from(br[channel]) * weights[2] + sum;
                out[channel] = ((u32::from(bl[channel]) * weights[3] + sum + 32768) >> 16) as u8;
            }
            return Ok(out);
        }
        let row = if y < 0 { 0 } else { max_y };
        let [first, second] = texels.horizontal(x, row)?;
        return Ok(pair(first, second, fx));
    }
    if quality != 0 && y_interior {
        let column = if x < 0 { 0 } else { max_x };
        let [first, second] = texels.vertical(column, y)?;
        return Ok(pair(first, second, (fixed_y & 255) as u32));
    }

    texels.single(x.max(0).min(max_x), y.max(0).min(max_y))
}

pub struct Scratch { pub source_capacity: i64, bytes: Option<Vec<u8>> }
impl std::fmt::Debug for Scratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scratch").field("source_capacity", &self.source_capacity)
            .field("backing_bytes", &self.bytes.as_ref().map(Vec::len)).finish()
    }
}
impl Scratch {
    pub fn new_source() -> Result<Self, Error> {
        let mut scratch = Self { source_capacity: 0, bytes: None };
        scratch.grow_before_sample(2048).map_err(Error::from)?;
        Ok(scratch)
    }
    fn grow_before_sample(&mut self, length: i32) -> Result<(), ScratchError> {
        if self.source_capacity >= i64::from(length) { return Ok(()); }
        self.source_capacity = i64::from(length);
        drop(self.bytes.take());
        let count = usize::try_from(length).map_err(|_| ScratchError::Size)?
            .checked_mul(4).ok_or(ScratchError::Size)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(count).map_err(|_| ScratchError::Allocation)?;
        bytes.resize(count, 0);
        self.bytes = Some(bytes);
        Ok(())
    }
}
#[derive(Debug)]
pub struct EdgeRenderer {
    pub sampler: Sampler, pub brush_alpha_plus_one: i32,
    pub target_row_offset: Option<i64>, pub scratch: Scratch,
}
pub enum ScheduledRows<C> {
    Complete,
    Superseded { reason: C, next_ordinal: i32 },
}
impl EdgeRenderer {

    pub fn new_source(affine: [f32; 6], quality: i32, brush_alpha: u8) -> Result<Self, Error> {
        let inverse = super::rectangle::inverse_source(affine);
        let sampler = Sampler::for_source_region(inverse, quality);
        let mut renderer = Self { sampler, brush_alpha_plus_one: i32::from(brush_alpha) + 1,
            target_row_offset: None, scratch: Scratch { source_capacity: 0, bytes: None } };
        if let Err(cause) = renderer.scratch.grow_before_sample(2048) {

            return Err(Error::Initialization { cause, renderer });
        }
        Ok(renderer)
    }
    fn store_sample(&self, target: &mut [u8], layout: &BitmapLayout, x: i32,
        source: u32, factor: Option<u32>) -> Result<(), Error> {
        validate_store_layout(layout)?;
        self.store_sample_in_validated_layout(target, x, source, factor)
    }

    fn store_sample_in_validated_layout(&self, target: &mut [u8], x: i32,
        source: u32, factor: Option<u32>) -> Result<(), Error> {
        let at = self.target_row_offset.ok_or(Error::Unwritten)?
            .checked_add(i64::from(x).checked_mul(4).ok_or(Error::NativeAddress)?)
            .ok_or(Error::NativeAddress)?;
        let at = usize::try_from(at).map_err(|_| Error::NativeAddress)?;

        let source = match factor {
            None => source,
            Some(factor) => {
                let rb = ((source & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff;
                let ga = ((((source >> 8) & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff) << 8;
                rb | ga
            }
        };
        let old = source_memory::read_dword(target,at).map_err(|_|Error::NativeAddress)?;
        source_memory::write_dword(target,at,super::radial::blend_full_coverage(source, old))
            .map_err(|_|Error::NativeAddress)
    }
    fn one(&mut self, source: &[u8], source_layout: &BitmapLayout,
        target: &mut [u8], target_layout: &BitmapLayout, x: i32, factor: u32) -> Result<(), Error> {
        let mut sample = [0; 4];
        self.sampler.fill_span_nonrepeating_format2(source, source_layout, &mut sample, x, 1)?;
        self.store_sample(target, target_layout, x, u32::from_le_bytes(sample), Some(factor))
    }
    fn bulk(&mut self, source: &[u8], source_layout: &BitmapLayout,
        target: &mut [u8], target_layout: &BitmapLayout, x: i32, count: i32, level: i32)
        -> Result<(), Error> {

        self.scratch.grow_before_sample(count).map_err(Error::from)?;
        let bytes = self.scratch.bytes.as_mut().ok_or(Error::Unwritten)?;
        self.sampler.fill_span_nonrepeating_format2(source, source_layout, bytes, x, count)?;
        let factor = level.wrapping_mul(self.brush_alpha_plus_one) >> 8;
        for ordinal in 0..count {
            let offset = ordinal as usize * 4;
            let b = self.scratch.bytes.as_ref().ok_or(Error::Unwritten)?;
            let p = b.get(offset..offset + 4).ok_or(Error::Unwritten)?;
            let sample = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
            let target_x = x.wrapping_add(ordinal);

            let scale = if factor >= 254 { None } else { Some(factor as u32) };

            if ordinal == 0 { validate_store_layout(target_layout)?; }
            self.store_sample_in_validated_layout(target, target_x, sample, scale)?;
        }
        Ok(())
    }
    fn accumulated_pixel(&mut self, source: &[u8], source_layout: &BitmapLayout,
        target: &mut [u8], target_layout: &BitmapLayout, x: i32, accumulated: i32)
        -> Result<(), Error> {

        if accumulated < 256 { return Ok(()); }
        let factor = if (accumulated as u32) >= 0xff00 { self.brush_alpha_plus_one as u32 }
            else { ((accumulated >> 8).wrapping_mul(self.brush_alpha_plus_one) as u32) >> 8 };
        self.one(source, source_layout, target, target_layout, x, factor)
    }

    fn row(&mut self, entries: &[[i32; 2]], source: &[u8], source_layout: &BitmapLayout,
        target: &mut [u8], target_layout: &BitmapLayout) -> Result<(), Error> {
        if entries.len() < 2 { return Ok(()); }
        let mut current_x = entries[0][0];
        let mut accumulated = 0i32;
        let mut end_pixel = 0;
        for pair in entries.windows(2) {
            let level = pair[0][1];
            let next_x = pair[1][0];
            end_pixel = truncated_fixed_div256(next_x);
            let first_pixel = truncated_fixed_div256(current_x);
            if end_pixel == first_pixel {
                accumulated = next_x.wrapping_sub(current_x).wrapping_mul(level).wrapping_add(accumulated);
            } else {
                let coverage = 256i32.wrapping_sub(current_x & 255).wrapping_mul(level)
                    .wrapping_add(accumulated);
                self.accumulated_pixel(source, source_layout, target, target_layout, first_pixel, coverage)?;
                if level > 0 {
                    let start = first_pixel.wrapping_add(1);
                    let count = end_pixel.wrapping_sub(start);
                    if count > 0 { self.bulk(source, source_layout, target, target_layout, start, count, level)?; }
                }
                accumulated = (next_x & 255).wrapping_mul(level);
            }
            current_x = next_x;
        }
        self.accumulated_pixel(source, source_layout, target, target_layout, end_pixel, accumulated)
    }

    pub fn shade_source_edge_table(&mut self, table: &super::edge_table::EdgeTable,
        source: &[u8], source_layout: &BitmapLayout, target: &mut [u8], target_layout: &BitmapLayout)
        -> Result<(), Error> {
        match self.shade_source_edge_table_scheduled(table, source, source_layout, target,
            target_layout, &mut || None::<std::convert::Infallible>)? {
            ScheduledRows::Complete => Ok(()),
            ScheduledRows::Superseded { reason, .. } => match reason {},
        }
    }

    pub fn shade_source_edge_table_scheduled<C>(&mut self, table: &super::edge_table::EdgeTable,
        source: &[u8], source_layout: &BitmapLayout, target: &mut [u8], target_layout: &BitmapLayout,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledRows<C>, Error> {
        use super::edge_table::Row;
        if table.bounds[3] <= 0 { return Ok(ScheduledRows::Complete); }
        let row_stride = i64::try_from(target_layout.row_stride).map_err(|_| Error::Size)?;
        for ordinal in 0..table.bounds[3] {
            let row = table.rows.get(ordinal as usize).and_then(Option::as_ref).ok_or(Error::Unwritten)?;
            let count = match row { Row::Empty => 0, Row::Span { .. } => 2, Row::Edges { entries } => entries.len() };
            if count < 2 {

                if let Some(reason) = checkpoint() {
                    return Ok(ScheduledRows::Superseded { reason, next_ordinal: ordinal + 1 });
                }
                continue;
            }
            if count > usize::try_from(table.source_capacity_edges).map_err(|_| Error::EdgeCount)? {
                return Err(Error::EdgeCount);
            }
            let y = i64::from(ordinal) + i64::from(table.bounds[1]);
            self.sampler.source_row = y as i32;
            self.target_row_offset = Some(row_stride.checked_mul(y).ok_or(Error::NativeAddress)?);
            match row {
                Row::Empty => {},
                Row::Span { fixed_x, level } => self.row(&[[fixed_x[0], *level], [fixed_x[1], 0]],
                    source, source_layout, target, target_layout)?,
                Row::Edges { entries } => self.row(entries, source, source_layout, target, target_layout)?,
            }
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledRows::Superseded { reason, next_ordinal: ordinal + 1 });
            }
        }
        Ok(ScheduledRows::Complete)
    }
}
fn truncated_fixed_div256(value: i32) -> i32 {
    (if value < 0 { value.wrapping_add(255) } else { value }) >> 8
}
