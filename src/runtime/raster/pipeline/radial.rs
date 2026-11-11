use super::{gradient::PreparedRender, image::BitmapLayout, lookup};
#[path = "radial_rectangles.rs"] pub mod rectangles;
pub use rectangles::{RectangleShader, RectangleStatus};
#[derive(Debug)]
pub enum Error {
    UnsupportedBranch, Layout, Size, NativeAddress, Unwritten, EdgeCount, Lookup(lookup::Error),
    RequiredSquareRoot { operand: u64, phase: SquareRootPhase },
}
#[derive(Clone, Copy, Debug)]
pub enum SquareRootPhase { Scale, Pixel { x: i32, y: i32, terminal: bool } }

pub trait SqrtServices { fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()>; }
struct HostSquareRoot;
impl SqrtServices for HostSquareRoot {
    fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()> {
        Ok(f64::from_bits(operand).sqrt().to_bits())
    }
}

struct TranslationParameters { center: [f64; 2], radius_squared: f64, scale: f64, last: i32 }
impl TranslationParameters {
    fn new(prepared: &PreparedRender, table: &lookup::Table, math: &mut impl SqrtServices) -> Result<Self, Error> {

        let [sx, sy] = prepared.gradient.start;
        let [ex, ey] = prepared.gradient.end;
        let center = [sx as f64, sy as f64];
        let last = table.source_size.wrapping_sub(1);
        let dy = sy - ey; let dx = sx - ex;
        let dy_squared = dy * dy; let dx_squared = dx * dx;
        let radius_squared = (dx_squared + dy_squared) as f64;

        let operand = radius_squared.to_bits();
        let root = math.sqrt_f64_bits(operand).map_err(|_| Error::RequiredSquareRoot {
            operand, phase: SquareRootPhase::Scale,
        })?;
        let scale = last as f64 / f64::from_bits(root);
        Ok(Self { center, radius_squared, scale, last })
    }
    fn sample(&self, x: i32, y: i32, y_squared: f64, terminal: bool, table: &lookup::Table,
        math: &mut impl SqrtServices)
        -> Result<u32, Error> {
        let distance = x as f64 - self.center[0];
        let x_squared = distance * distance;
        let squared = if terminal { y_squared + x_squared } else { x_squared + y_squared };
        let use_last = squared >= self.radius_squared;

        let operand = squared.to_bits();
        let root = math.sqrt_f64_bits(operand).map_err(|_| Error::RequiredSquareRoot {
            operand, phase: SquareRootPhase::Pixel { x, y, terminal },
        })?;
        let rounded = (f64::from_bits(root) * self.scale + 6755399441055744.0).to_bits() as u32 as i32;
        table.read_source_index(if use_last { self.last } else { rounded }).map_err(Error::Lookup)
    }
}

pub fn blend_full_coverage(source: u32, destination: u32) -> u32 {
    let inverse = 256u32.wrapping_sub(source >> 24);
    let mask = 0x00ff00ff;
    let rb = ((((destination & mask).wrapping_mul(inverse) >> 8) & mask))
        .wrapping_add(source & mask);
    let ga = (((((destination >> 8) & mask).wrapping_mul(inverse) >> 8) & mask))
        .wrapping_add((source >> 8) & mask);
    let rb_overflow = (rb >> 8) & 0x00010001;
    let ga_overflow = (ga >> 8) & 0x00010001;
    let rb = (0x100u32.wrapping_sub(rb_overflow) | rb) & mask;
    let ga = ((0x100u32.wrapping_sub(ga_overflow) | ga) << 8) & 0xff00ff00;
    rb | ga
}

pub fn shade_rectangles_format2_translation(bytes: &mut [u8], layout: BitmapLayout,
    rectangles: &[[i32; 4]], prepared: &PreparedRender, table: &lookup::Table)
    -> Result<(), Error> {
    shade_rectangles_format2_translation_with_math(bytes, layout, rectangles, prepared, table, &mut HostSquareRoot)
}
pub fn shade_rectangles_format2_translation_with_math(bytes: &mut [u8], layout: BitmapLayout,
    rectangles: &[[i32; 4]], prepared: &PreparedRender, table: &lookup::Table,
    math: &mut impl SqrtServices) -> Result<(), Error> {

    RectangleShader::new(bytes, layout, rectangles, prepared, table, math).finish()
}

pub fn shade_edge_table_format2_translation(bytes: &mut [u8], layout: &BitmapLayout,
    table: &super::edge_table::EdgeTable, prepared: &PreparedRender, colors: &lookup::Table)
    -> Result<(), Error> {
    shade_edge_table_format2_translation_with_math(bytes, layout, table, prepared, colors, &mut HostSquareRoot)
}
pub fn shade_edge_table_format2_translation_with_math(bytes: &mut [u8], layout: &BitmapLayout,
    table: &super::edge_table::EdgeTable, prepared: &PreparedRender, colors: &lookup::Table,
    math: &mut impl SqrtServices) -> Result<(), Error> {
    use super::edge_table::{self, CoverageEvent, Row};
    if !prepared.gradient.radial || !prepared.source_translation_flag { return Err(Error::UnsupportedBranch); }
    if table.bounds[3] <= 0 { return Ok(()); }
    if layout.pixel_stride != 4 || layout.row_stride < layout.width.checked_mul(4).ok_or(Error::Size)? {
        return Err(Error::Layout);
    }
    let parameters = TranslationParameters::new(prepared, colors, math)?;
    let stride = i64::try_from(layout.row_stride).map_err(|_| Error::Size)?;
    for ordinal in 0..table.bounds[3] {
        let row = table.rows.get(ordinal as usize).and_then(Option::as_ref).ok_or(Error::Unwritten)?;
        let count = match row { Row::Empty => 0, Row::Span { .. } => 2, Row::Edges { entries } => entries.len() };
        if count < 2 { continue; }
        if count > usize::try_from(table.source_capacity_edges).map_err(|_| Error::EdgeCount)? {
            return Err(Error::EdgeCount);
        }
        let y = i64::from(table.bounds[1]) + i64::from(ordinal);
        let row_offset = stride.checked_mul(y).ok_or(Error::NativeAddress)?;
        let distance_y = (y as i32) as f64 - parameters.center[1];
        let y_squared = distance_y * distance_y;
        let mut store = |x: i32, factor: Option<u32>, terminal: bool| -> Result<(), Error> {
            let mut source = parameters.sample(x, y as i32, y_squared, terminal, colors, math)?;
            if let Some(factor) = factor {
                let rb = ((source & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff;
                let ga = ((((source >> 8) & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff) << 8;
                source = rb | ga;
            }
            let at = row_offset.checked_add(i64::from(x) * 4).ok_or(Error::NativeAddress)?;
            let at = usize::try_from(at).map_err(|_| Error::NativeAddress)?;
            let end = at.checked_add(4).ok_or(Error::NativeAddress)?;
            let pixel = bytes.get_mut(at..end).ok_or(Error::NativeAddress)?;
            let old = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
            pixel.copy_from_slice(&blend_full_coverage(source, old).to_le_bytes());
            Ok(())
        };
        let mut emit = |event, terminal| match event {
            CoverageEvent::Fringe { x, accumulated } => store(x,
                if (accumulated as u32) >= 0xff00 { None } else { Some((accumulated >> 8) as u32) }, terminal),
            CoverageEvent::Span { x, count, level } => {
                for ordinal in 0..count {
                    store(x.wrapping_add(ordinal), if (level as u32) >= 255 { None } else { Some(level as u32) }, false)?;
                }
                Ok(())
            }
        };
        match row {
            Row::Empty => {},
            Row::Span { fixed_x, level } => edge_table::walk_coverage_row_ordered(
                &[[fixed_x[0], *level], [fixed_x[1], 0]], &mut emit)?,
            Row::Edges { entries } => edge_table::walk_coverage_row_ordered(entries, &mut emit)?,
        }
    }
    Ok(())
}
