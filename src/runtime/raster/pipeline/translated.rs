use super::{edge_table::{self, CoverageEvent, EdgeTable, Row}, image::BitmapLayout, radial};
#[derive(Debug)]
pub enum Error { Layout, Size, NativeAddress, Unwritten, EdgeCount }

fn address(layout: &BitmapLayout, y: i64, x: i32) -> Result<usize, Error> {
    if layout.pixel_stride != 4 { return Err(Error::Layout); }
    let stride = i64::try_from(layout.row_stride).map_err(|_| Error::Size)?;
    let offset = y.checked_mul(stride).and_then(|n| n.checked_add(i64::from(x) * 4))
        .ok_or(Error::NativeAddress)?;
    usize::try_from(offset).map_err(|_| Error::NativeAddress)
}
fn load(source: &[u8], address: usize) -> Result<u32, Error> {
    let end = address.checked_add(4).ok_or(Error::NativeAddress)?;
    let bytes = source.get(address..end).ok_or(Error::NativeAddress)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
fn scale(source: u32, factor: u32) -> u32 {
    let rb = ((source & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff;
    let ga = ((((source >> 8) & 0x00ff00ff).wrapping_mul(factor) >> 8) & 0x00ff00ff) << 8;
    rb | ga
}
fn one(source: &[u8], source_layout: &BitmapLayout, target: &mut [u8], target_layout: &BitmapLayout,
    target_y: i64, source_y: i32, x: i32, translation_x: i32, factor: Option<u32>)
    -> Result<(), Error> {

    let source_x = x.wrapping_sub(translation_x);
    let sample = load(source, address(source_layout, i64::from(source_y), source_x)?)?;
    let sample = factor.map(|factor| scale(sample, factor)).unwrap_or(sample);
    let at = address(target_layout, target_y, x)?;
    let end = at.checked_add(4).ok_or(Error::NativeAddress)?;
    let pixel = target.get_mut(at..end).ok_or(Error::NativeAddress)?;
    let old = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
    pixel.copy_from_slice(&radial::blend_full_coverage(sample, old).to_le_bytes());
    Ok(())
}
pub fn shade_source_edge_table(table: &EdgeTable, source: &[u8], source_layout: &BitmapLayout,
    target: &mut [u8], target_layout: &BitmapLayout, translation: [i32; 2], brush_alpha: u8)
    -> Result<(), Error> {
    if table.bounds[3] <= 0 { return Ok(()); }
    let alpha_plus_one = i32::from(brush_alpha) + 1;
    for ordinal in 0..table.bounds[3] {
        let row = table.rows.get(ordinal as usize).and_then(Option::as_ref).ok_or(Error::Unwritten)?;
        let count = match row { Row::Empty => 0, Row::Span { .. } => 2,
            Row::Edges { entries } => entries.len() };
        if count < 2 { continue; }
        if count > usize::try_from(table.source_capacity_edges).map_err(|_| Error::EdgeCount)? {
            return Err(Error::EdgeCount);
        }
        let target_y = i64::from(table.bounds[1]) + i64::from(ordinal);
        let source_y = (target_y as i32).wrapping_sub(translation[1]);
        let mut emit = |event| match event {
            CoverageEvent::Fringe { x, accumulated } => {
                if accumulated < 256 { return Ok(()); }

                let factor = if (accumulated as u32) >= 0xff00 { alpha_plus_one as u32 }
                    else { (((accumulated as u32) >> 8) as i32)
                        .wrapping_mul(alpha_plus_one).wrapping_shr(8) as u32 };
                one(source, source_layout, target, target_layout, target_y, source_y,
                    x, translation[0], Some(factor))
            }
            CoverageEvent::Span { x, count, level } => {
                let factor = level.wrapping_mul(alpha_plus_one) >> 8;

                for ordinal in 0..count {
                    one(source, source_layout, target, target_layout, target_y, source_y,
                        x.wrapping_add(ordinal), translation[0],
                        if factor > 253 { None } else { Some(factor as u32) })?;
                }
                Ok(())
            }
        };
        match row {
            Row::Empty => {},
            Row::Span { fixed_x, level } => edge_table::walk_coverage_row(
                &[[fixed_x[0], *level], [fixed_x[1], 0]], &mut emit)?,
            Row::Edges { entries } => edge_table::walk_coverage_row(entries, &mut emit)?,
        }
    }
    Ok(())
}
