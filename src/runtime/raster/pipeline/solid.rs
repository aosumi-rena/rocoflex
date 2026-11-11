use super::{image::BitmapLayout, radial, rectangle};
#[derive(Debug)]
pub enum Error { Layout, Size, NativeAddress, FixedCoordinateDomain, Unwritten, EdgeCount }
pub struct SpanPainter {
    pub premultiplied_argb: u32,

    pub current_row_offset: Option<i64>,
}
impl SpanPainter {
    pub fn new(premultiplied_argb: u32) -> Self {
        Self { premultiplied_argb, current_row_offset: None }
    }
}
fn layout_strides(layout: &BitmapLayout) -> Result<(i64, i64), Error> {
    if layout.pixel_stride != 4 || layout.row_stride < layout.width.checked_mul(4).ok_or(Error::Size)? {
        return Err(Error::Layout);
    }
    Ok((i64::try_from(layout.row_stride).map_err(|_| Error::Size)?,
        i64::try_from(layout.pixel_stride).map_err(|_| Error::Size)?))
}
fn pixel(bytes: &mut [u8], at: i64, source: u32, overwrite: bool) -> Result<(), Error> {
    let at = usize::try_from(at).map_err(|_| Error::NativeAddress)?;
    let end = at.checked_add(4).ok_or(Error::NativeAddress)?;
    let target = bytes.get_mut(at..end).ok_or(Error::NativeAddress)?;
    let result = if overwrite { source } else {
        let old = u32::from_le_bytes([target[0], target[1], target[2], target[3]]);
        radial::blend_full_coverage(source, old)
    };
    target.copy_from_slice(&result.to_le_bytes());
    Ok(())
}
pub fn paint_span(bytes: &mut [u8], layout: &BitmapLayout, painter: &mut SpanPainter,
    rectangle: [i32; 4], source_coverage: u32) -> Result<(), Error> {
    let (row_stride, pixel_stride) = layout_strides(layout)?;
    let factor = source_coverage.wrapping_add(1);
    let ga = ((painter.premultiplied_argb >> 8) & 0x00ff00ff).wrapping_mul(factor);
    let rb = ((painter.premultiplied_argb & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff;
    let source = (ga & 0xff00ff00) | rb;
    let overwrite = ga > 0xfeffffff;
    let [x, y, width, height] = rectangle;
    let mut row = row_stride.checked_mul(i64::from(y)).ok_or(Error::NativeAddress)?;
    painter.current_row_offset = Some(row);
    let x_offset = pixel_stride.checked_mul(i64::from(x)).ok_or(Error::NativeAddress)?;
    if height <= 0 { return Ok(()); }
    for _ in 0..height {
        let mut address = row.checked_add(x_offset).ok_or(Error::NativeAddress)?;
        let mut remaining = width.wrapping_add(1);
        loop {
            pixel(bytes, address, source, overwrite)?;
            address = address.checked_add(pixel_stride).ok_or(Error::NativeAddress)?;
            remaining = remaining.wrapping_sub(1);
            if remaining <= 1 { break; }
        }
        row = row.checked_add(row_stride).ok_or(Error::NativeAddress)?;
    }
    Ok(())
}
pub fn paint_integer_clipped(bytes: &mut [u8], layout: &BitmapLayout, clips: &[[i32; 4]],
    rectangle: [i32; 4], premultiplied_argb: u32, replace_contents: bool) -> Result<(), Error> {
    let mut painter = SpanPainter::new(premultiplied_argb);

    for &clip in clips {
        let Some(intersection) = rectangle::gradient_integer_intersection(rectangle, clip) else { continue; };
        if !replace_contents {
            paint_span(bytes, layout, &mut painter, intersection, 255)?;
        } else {
            let (row_stride, pixel_stride) = layout_strides(layout)?;
            let [x, y, width, height] = intersection;
            let mut row = row_stride.checked_mul(i64::from(y))
                .and_then(|at| pixel_stride.checked_mul(i64::from(x)).and_then(|dx| at.checked_add(dx)))
                .ok_or(Error::NativeAddress)?;
            for _ in 0..height {
                let mut address = row;
                for _ in 0..width {
                    pixel(bytes, address, premultiplied_argb, true)?;
                    address = address.checked_add(pixel_stride).ok_or(Error::NativeAddress)?;
                }
                row = row.checked_add(row_stride).ok_or(Error::NativeAddress)?;
            }
        }
    }
    Ok(())
}

fn fixed_coordinate(coordinate: f32) -> Result<i32, Error> {
    let scaled = (coordinate * 256.0f32) as f64;
    if !scaled.is_finite() || scaled < i32::MIN as f64 || scaled > i32::MAX as f64 {
        return Err(Error::FixedCoordinateDomain);
    }
    Ok((scaled + 6755399441055744.0).to_bits() as u32 as i32)
}
#[derive(Clone, Copy)]
struct Axis {
    first: i32, low: i32, full_start: i32, full_end: i32,
    last_exclusive: i32, high: i32,
}
impl Axis {
    fn split(start: i32, end: i32) -> Self {
        let first = start >> 8;
        if ((end ^ start) as u32) <= 255 {
            let next = first.wrapping_add(1);
            Self { first, low: end.wrapping_sub(start), full_start: next,
                full_end: next, last_exclusive: next, high: 0 }
        } else {
            let fraction = start & 255;
            let full_end = end >> 8;
            let high = end & 255;
            Self { first, low: if fraction == 0 { 0 } else { fraction ^ 255 },
                full_start: first.wrapping_add(i32::from(fraction != 0)),
                full_end, last_exclusive: full_end.wrapping_add(i32::from(high != 0)), high }
        }
    }
}

fn factor_source(argb: u32, factor: u32) -> (u32, u32) {
    let ga = ((argb >> 8) & 0x00ff00ff).wrapping_mul(factor);
    let rb = ((argb & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff;
    ((ga & 0xff00ff00) | rb, ga)
}
fn set_row(layout: &BitmapLayout, painter: &mut SpanPainter, y: i32) -> Result<i64, Error> {
    let (stride, _) = layout_strides(layout)?;
    let row = stride.checked_mul(i64::from(y)).ok_or(Error::NativeAddress)?;
    painter.current_row_offset = Some(row);
    Ok(row)
}
fn direct_factor_pixel(bytes: &mut [u8], layout: &BitmapLayout, painter: &SpanPainter,
    row: i64, x: i32, factor: u32) -> Result<(), Error> {
    let (_, stride) = layout_strides(layout)?;
    let address = stride.checked_mul(i64::from(x)).and_then(|x| row.checked_add(x))
        .ok_or(Error::NativeAddress)?;
    pixel(bytes, address, factor_source(painter.premultiplied_argb, factor).0, false)
}
fn direct_factor_row(bytes: &mut [u8], layout: &BitmapLayout, painter: &SpanPainter,
    row: i64, x: i32, width: i32, factor: u32) -> Result<(), Error> {
    if width <= 0 { return Ok(()); }
    let (_, stride) = layout_strides(layout)?;
    let (source, ga) = factor_source(painter.premultiplied_argb, factor);
    let overwrite = ga >= 0xff000000;
    let mut address = stride.checked_mul(i64::from(x)).and_then(|x| row.checked_add(x))
        .ok_or(Error::NativeAddress)?;
    for _ in 0..width {
        pixel(bytes, address, source, overwrite)?;
        address = address.checked_add(stride).ok_or(Error::NativeAddress)?;
    }
    Ok(())
}

pub fn paint_float_clipped(bytes: &mut [u8], layout: &BitmapLayout, clips: &[[i32; 4]],
    rectangle: [f32; 4], painter: &mut SpanPainter) -> Result<(), Error> {
    let [left, top, width, height] = rectangle;
    let fixed_left = fixed_coordinate(left)?;
    let fixed_top = fixed_coordinate(top)?;
    let fixed_right = fixed_coordinate(width + left)?;
    let fixed_bottom = fixed_coordinate(height + top)?;
    let x = Axis::split(fixed_left, fixed_right);
    let y = Axis::split(fixed_top, fixed_bottom);
    let one_column = ((x.full_end.wrapping_sub(x.full_start) ^ 1)
        | (x.high.wrapping_neg() ^ x.low)) == 0;
    let top_left = x.low.wrapping_mul(y.low) >> 8;
    let top_right = x.high.wrapping_mul(y.low) >> 8;
    let bottom_left = x.low.wrapping_mul(y.high) >> 8;
    let bottom_right = (x.high.wrapping_mul(y.high) as u32) >> 8;
    for &[clip_x, clip_y, clip_width, clip_height] in clips {

        if y.last_exclusive <= clip_y { continue; }
        let clip_bottom = clip_y.wrapping_add(clip_height);
        if y.first >= clip_bottom { continue; }
        if x.last_exclusive <= clip_x { continue; }
        let clip_right = clip_x.wrapping_add(clip_width);
        if x.first >= clip_right { continue; }
        if one_column {
            if y.low != 0 && y.first >= clip_y {
                let row = set_row(layout, painter, y.first)?;
                direct_factor_pixel(bytes, layout, painter, row, x.full_start, y.low as u32)?;
            }
            let start_y = clip_y.max(y.full_start);
            let count = clip_bottom.min(y.full_end).wrapping_sub(start_y);
            if count > 0 {
                paint_span(bytes, layout, painter, [x.full_start, start_y, 1, count], 255)?;
            }
            if y.high != 0 && y.full_end < clip_bottom {
                let row = set_row(layout, painter, y.full_end)?;
                direct_factor_pixel(bytes, layout, painter, row, x.full_start, y.high as u32)?;
            }
            continue;
        }
        let left_edge = x.low != 0 && x.first >= clip_x;
        let right_edge = x.high != 0 && x.full_end < clip_right;
        let start_x = clip_x.max(x.full_start);
        let count_x = clip_right.min(x.full_end).wrapping_sub(start_x);
        if y.low != 0 && y.first >= clip_y {
            let row = set_row(layout, painter, y.first)?;
            if left_edge { direct_factor_pixel(bytes, layout, painter, row, x.first, top_left as u32)?; }
            direct_factor_row(bytes, layout, painter, row, start_x, count_x, y.low.wrapping_add(1) as u32)?;
            if right_edge { direct_factor_pixel(bytes, layout, painter, row, x.full_end, top_right as u32)?; }
        }
        let start_y = clip_y.max(y.full_start);
        let count_y = clip_bottom.min(y.full_end).wrapping_sub(start_y);
        if count_y == 1 {

            let row = set_row(layout, painter, start_y)?;
            if left_edge { direct_factor_pixel(bytes, layout, painter, row, x.first, x.low as u32)?; }
            direct_factor_row(bytes, layout, painter, row, start_x, count_x, 256)?;
            if right_edge { direct_factor_pixel(bytes, layout, painter, row, x.full_end, x.high as u32)?; }
        } else if count_y > 1 {
            if left_edge { paint_span(bytes, layout, painter, [x.first, start_y, 1, count_y], x.low as u32)?; }
            if count_x > 0 { paint_span(bytes, layout, painter, [start_x, start_y, count_x, count_y], 255)?; }
            if right_edge { paint_span(bytes, layout, painter, [x.full_end, start_y, 1, count_y], x.high as u32)?; }
        }
        if y.high != 0 && y.full_end < clip_bottom {
            let row = set_row(layout, painter, y.full_end)?;
            if left_edge { direct_factor_pixel(bytes, layout, painter, row, x.first, bottom_left as u32)?; }
            direct_factor_row(bytes, layout, painter, row, start_x, count_x, y.high.wrapping_add(1) as u32)?;
            if right_edge { direct_factor_pixel(bytes, layout, painter, row, x.full_end, bottom_right)?; }
        }
    }
    Ok(())
}

pub fn paint_region_format2(bytes: &mut [u8], layout: &BitmapLayout,
    rectangles: &[[i32; 4]], premultiplied_argb: u32, replace_contents: bool)
    -> Result<(), Error> {
    let (row_stride, pixel_stride) = layout_strides(layout)?;
    let overwrite = replace_contents || (premultiplied_argb >> 24) == 255;
    for &[x, y, width, height] in rectangles {
        if height <= 0 { continue; }
        let bottom = i64::from(y) + i64::from(height);
        let mut row_y = i64::from(y);
        while row_y < bottom {
            let mut address = row_stride.checked_mul(row_y)
                .and_then(|at| pixel_stride.checked_mul(i64::from(x)).and_then(|dx| at.checked_add(dx)))
                .ok_or(Error::NativeAddress)?;
            let mut remaining = width.wrapping_add(1);
            loop {
                pixel(bytes, address, premultiplied_argb, overwrite)?;
                address = address.checked_add(pixel_stride).ok_or(Error::NativeAddress)?;
                remaining = remaining.wrapping_sub(1);
                if remaining <= 1 { break; }
            }
            row_y += 1;
        }
    }
    Ok(())
}

pub fn shade_edge_table_format2_blend(bytes: &mut [u8], layout: &BitmapLayout,
    table: &super::edge_table::EdgeTable, painter: &mut SpanPainter) -> Result<(), Error> {
    shade_edge_table_format2(bytes, layout, table, painter, false)
}

pub fn shade_edge_table_format2(bytes: &mut [u8], layout: &BitmapLayout,
    table: &super::edge_table::EdgeTable, painter: &mut SpanPainter, replace: bool) -> Result<(), Error> {
    shade_edge_table_format2_retaining(bytes,layout,table,painter,replace).map(|_|()).map_err(|failure|match failure.cause {
        EdgePaintCause::Source(error)=>error,EdgePaintCause::Coverage(failure)=>failure.error,
    })
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EdgePaintStep {Header,Layout,ReadRow,Capacity,RowOffset,Scan,Complete}
#[derive(Clone,Copy,Debug)]
pub struct EdgePaintProgress {
    pub step:EdgePaintStep,pub replace:bool,pub ordinal:Option<i32>,pub row_offset:Option<i64>,
    pub count:Option<usize>,pub completed_rows:usize,
}
pub enum ScheduledEdgePaint<C> {
    Complete(EdgePaintProgress),
    Superseded { reason: C, progress: EdgePaintProgress },
}
pub enum EdgePaintCause<'a> {Source(Error),Coverage(super::edge_table::coverage::Failure<'a,Error>)}
#[must_use="retain same table/row and original partially written bytes/painter; do not replay"]
pub struct EdgePaintFailure<'a> {
    pub table:&'a super::edge_table::EdgeTable,pub progress:EdgePaintProgress,pub cause:EdgePaintCause<'a>,
}

pub fn shade_edge_table_format2_retaining<'a>(bytes:&mut [u8],layout:&BitmapLayout,
    table:&'a super::edge_table::EdgeTable,painter:&mut SpanPainter,replace:bool)
    ->Result<EdgePaintProgress,EdgePaintFailure<'a>>
{
    match shade_edge_table_format2_retaining_scheduled(bytes,layout,table,painter,replace,
        &mut || None::<core::convert::Infallible>)? {
        ScheduledEdgePaint::Complete(progress)=>Ok(progress),
        ScheduledEdgePaint::Superseded {reason,..}=>match reason {},
    }
}

pub fn shade_edge_table_format2_retaining_scheduled<'a,C>(bytes:&mut [u8],layout:&BitmapLayout,
    table:&'a super::edge_table::EdgeTable,painter:&mut SpanPainter,replace:bool,
    checkpoint:&mut impl FnMut()->Option<C>)
    ->Result<ScheduledEdgePaint<C>,EdgePaintFailure<'a>>
{
    use super::edge_table::{Row, CoverageEvent};
    let mut progress=EdgePaintProgress {step:EdgePaintStep::Header,replace,ordinal:None,row_offset:None,count:None,completed_rows:0};
    let result=(||->Result<Option<C>,EdgePaintCause<'a>> {
    if table.bounds[3] <= 0 {progress.step=EdgePaintStep::Complete;return Ok(None);}
    progress.step=EdgePaintStep::Layout;
    let (row_stride, _) = layout_strides(layout).map_err(EdgePaintCause::Source)?;
    for ordinal in 0..table.bounds[3] {
        progress.step=EdgePaintStep::ReadRow;progress.ordinal=Some(ordinal);progress.row_offset=None;progress.count=None;
        let row = table.rows.get(ordinal as usize).and_then(Option::as_ref).ok_or(EdgePaintCause::Source(Error::Unwritten))?;
        let count = match row { Row::Empty => 0, Row::Span { .. } => 2, Row::Edges { entries } => entries.len() };
        progress.count=Some(count);
        if count < 2 {
            progress.completed_rows+=1;
            if let Some(reason)=checkpoint() {return Ok(Some(reason));}
            continue;
        }
        progress.step=EdgePaintStep::Capacity;
        if count > usize::try_from(table.source_capacity_edges).map_err(|_| EdgePaintCause::Source(Error::EdgeCount))? {
            return Err(EdgePaintCause::Source(Error::EdgeCount));
        }
        progress.step=EdgePaintStep::RowOffset;
        let y = i64::from(ordinal) + i64::from(table.bounds[1]);
        let row_offset = row_stride.checked_mul(y).ok_or(EdgePaintCause::Source(Error::NativeAddress))?;
        painter.current_row_offset = Some(row_offset);
        progress.row_offset=Some(row_offset);progress.step=EdgePaintStep::Scan;
        let mut draw = |event| match event {
            CoverageEvent::Fringe { x, accumulated } => {
                if replace {
                    if accumulated < 256 { return Ok(()); }
                    let at = row_offset.checked_add(i64::from(x) * 4).ok_or(Error::NativeAddress)?;
                    return pixel(bytes, at, painter.premultiplied_argb, true);
                }

                let factor = if (accumulated as u32) >= 0xff00 { 256 } else { (accumulated >> 8) as u32 };
                direct_factor_pixel(bytes, layout, painter, row_offset, x, factor)
            }
            CoverageEvent::Span { x, count, level } => {
                if replace {
                    let factor = level.wrapping_add(1) as u32;
                    let source = painter.premultiplied_argb;
                    let ga = (((source >> 8) & 0x00ff00ff).wrapping_mul(factor)) & 0xff00ff00;
                    let rb = (((source & 0x00ff00ff).wrapping_mul(factor)) >> 8) & 0x00ff00ff;
                    let mut at = row_offset.checked_add(i64::from(x) * 4).ok_or(Error::NativeAddress)?;
                    for _ in 0..count {
                        pixel(bytes, at, ga | rb, true)?;
                        at = at.checked_add(4).ok_or(Error::NativeAddress)?;
                    }
                    return Ok(());
                }
                direct_factor_row(bytes, layout, painter, row_offset, x, count, level.wrapping_add(1) as u32)
            }
        };
        let mut emit=|event,_terminal|draw(event);
        super::edge_table::coverage::walk_row_ordered(row,&mut emit).map_err(EdgePaintCause::Coverage)?;
        progress.completed_rows+=1;
        if let Some(reason)=checkpoint() {return Ok(Some(reason));}
    }
    progress.step=EdgePaintStep::Complete;Ok(None)
    })();
    match result {
        Ok(None)=>Ok(ScheduledEdgePaint::Complete(progress)),
        Ok(Some(reason))=>Ok(ScheduledEdgePaint::Superseded {reason,progress}),
        Err(cause)=>Err(EdgePaintFailure {table,progress,cause}),
    }
}
