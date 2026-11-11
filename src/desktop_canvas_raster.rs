use super::empty_background::literal;
use eframe::egui::{Color32, ColorImage};
use std::{fmt, sync::Arc};

#[path = "desktop_canvas_math.rs"]
mod radial_math;

#[derive(Clone, Copy, Debug)]
pub(super) struct Fields {
    pub dimensions: [i32; 2],
    pub probe: [f32; 2],
    pub radius: f32,
    pub zoom: f32,
}

#[derive(Debug)]
pub(super) struct RasterError(&'static str);
impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.0) }
}

#[derive(Clone, Copy)]
struct AxisSample { low: usize, high: usize, fraction: u32, interior: bool }

struct Prepared {
    geometry: Option<Arc<literal::Geometry>>,
    dimensions: [i32; 2],
    zoom: u32,
    quarter: [usize; 2],
    roots: Vec<([f32; 4], u32)>,
    getters: Vec<literal::edge_table::EdgeTable>,
    segments: Vec<[f32; 4]>,
    x_samples: Vec<AxisSample>,
    y_samples: Vec<AxisSample>,
    mask_dimensions: [usize; 2],
    mask_x_samples: Vec<AxisSample>,
    mask_y_samples: Vec<AxisSample>,
}

#[derive(Default)]
pub(super) struct RendererCache {
    prepared: Option<Prepared>,
    quarter: Vec<u8>,
    blur_scratch: Vec<u8>,
    mask: Vec<u8>,
    mask_delta: Vec<i16>,
    colors: Vec<u32>,
    obstacles: Vec<[u32; 4]>,
    envelope: Vec<[f32; 4]>,
    polygon: Vec<[i32; 2]>,
    crossings: Vec<Vec<i32>>,
}

impl RendererCache {

    pub(super) fn render_into(&mut self, fields: Fields,
        geometry: Option<&Arc<literal::Geometry>>, image: &mut ColorImage)
        -> Result<(), RasterError>
    {
        validate(fields)?;
        let refresh = self.prepared.as_ref().is_none_or(|p|
            p.dimensions != fields.dimensions || p.zoom != fields.zoom.to_bits()
                || !same_geometry(p.geometry.as_ref(), geometry));
        if refresh { self.prepared = Some(Prepared::new(fields, geometry)?); }
        let p = self.prepared.as_ref().ok_or(RasterError("missing canvas geometry"))?;
        let [qw, qh] = p.quarter;
        let [width, height] = fields.dimensions.map(|v| v as usize);
        resize_bytes(&mut self.quarter, qw * qh * 4)?;
        let clear = literal::rectangle::premultiply_argb(0x98d0d0ff).to_le_bytes();
        for pixel in self.quarter.chunks_exact_mut(4) { pixel.copy_from_slice(&clear); }

        let frame = literal::prefix::QuarterProjection::new(
            [0, 0, width as i32, height as i32], fields.probe, fields.zoom)
            .map_err(|_| RasterError("canvas projection"))?;
        let pixel_radius = frame.pixel_radius(fields.radius)
            .map_err(|_| RasterError("canvas radius"))?;
        shade_radial(&mut self.quarter, p.quarter, &frame, fields.radius,
            pixel_radius, &mut self.colors)?;

        let layout = literal::image::BitmapLayout {
            width: qw, height: qh, row_stride: qw * 4, pixel_stride: 4,
        };
        let clip = [[0, 0, qw as i32, qh as i32]];

        for _ in 0..5 {
            for &(rectangle, argb) in &p.roots {
                paint_rectangle(&mut self.quarter, &layout, &clip, rectangle, argb)?;
            }
            for &(rectangle, argb) in p.roots.iter().rev() {
                paint_rectangle(&mut self.quarter, &layout, &clip, rectangle, argb)?;
            }
        }
        blur(&mut self.quarter, &mut self.blur_scratch, qw, qh, 37)?;
        for table in &p.getters {
            paint_ellipse(&mut self.quarter, &layout, table, 0x1a000000)?;
        }
        let probe = frame.probe_rectangle_from_saved_radius(pixel_radius)
            .map_err(|_| RasterError("canvas probe ellipse"))?;
        if let Some(table) = ellipse_table(probe, p.quarter)? {
            paint_ellipse(&mut self.quarter, &layout, &table, 0x40000000)?;
        }
        blur(&mut self.quarter, &mut self.blur_scratch, qw, qh, 1)?;

        let [mask_width, mask_height] = p.mask_dimensions;
        let mask_len = mask_width.checked_mul(mask_height).ok_or(RasterError("canvas mask extent"))?;
        resize_bytes(&mut self.mask, mask_len)?;
        let delta_len = mask_width.checked_add(1).and_then(|n| n.checked_mul(mask_height))
            .ok_or(RasterError("canvas mask extent"))?;
        if delta_len > self.mask_delta.len() {
            self.mask_delta.try_reserve(delta_len - self.mask_delta.len())
                .map_err(|_| RasterError("canvas coverage allocation"))?;
        }
        self.mask_delta.resize(delta_len, 0);
        self.mask_delta.fill(0);
        render_mask(fields, p.mask_dimensions, &p.segments, &mut self.mask, &mut self.mask_delta, &mut self.obstacles,
            &mut self.envelope, &mut self.polygon, &mut self.crossings)?;

        let count = width * height;
        if image.pixels.capacity() < count {
            image.pixels.try_reserve(count.saturating_sub(image.pixels.len()))
                .map_err(|_| RasterError("canvas output allocation"))?;
        }
        let mut pixels = std::mem::take(&mut image.pixels);
        pixels.resize(count, Color32::TRANSPARENT);
        for (y, sy) in p.y_samples.iter().enumerate() {
            for (x, sx) in p.x_samples.iter().enumerate() {
                let mut rgba = sample(&self.quarter, qw, *sx, *sy);

                if x + 1 == width {
                    for channel in &mut rgba { *channel = (u32::from(*channel) * 254 >> 8) as u8; }
                }
                let at = y * width + x;
                let coverage = sample_mask(&self.mask, mask_width,
                    p.mask_x_samples[x], p.mask_y_samples[y]);
                let factor = (u32::from(coverage) + 127).min(255);
                let product = u32::from(rgba[3]) * factor;
                let channel = |c: u8| ((u32::from(c) * product + 32767) >> 16) as u8;
                pixels[at] = Color32::from_rgba_premultiplied(
                    channel(rgba[2]), channel(rgba[1]), channel(rgba[0]),
                    (product >> 8) as u8);
            }
        }
        *image = ColorImage::new([width, height], pixels);
        Ok(())
    }
}

fn validate(f: Fields) -> Result<(), RasterError> {
    if f.dimensions.iter().any(|&v| v < 4)
        || !f.zoom.is_finite() || f.zoom <= 0.0
        || !f.radius.is_finite() || f.radius <= 0.0
        || f.probe.iter().any(|v| !v.is_finite()) {
        return Err(RasterError("invalid canvas dimensions or probe"));
    }
    (f.dimensions[0] as usize).checked_mul(f.dimensions[1] as usize)
        .and_then(|n| n.checked_mul(4)).ok_or(RasterError("canvas extent overflow"))?;
    Ok(())
}

fn same_geometry(a: Option<&Arc<literal::Geometry>>, b: Option<&Arc<literal::Geometry>>) -> bool {
    match (a, b) { (Some(a), Some(b)) => Arc::ptr_eq(a, b), (None, None) => true, _ => false }
}

fn resize_bytes(bytes: &mut Vec<u8>, count: usize) -> Result<(), RasterError> {
    if count > bytes.len() {
        bytes.try_reserve(count - bytes.len()).map_err(|_| RasterError("canvas buffer allocation"))?;
    }
    bytes.resize(count, 0);
    Ok(())
}

impl Prepared {
    fn new(f: Fields, geometry: Option<&Arc<literal::Geometry>>) -> Result<Self, RasterError> {
        let projection = literal::prefix::QuarterProjection::new(
            [0, 0, f.dimensions[0], f.dimensions[1]], [0.0; 2], f.zoom)
            .map_err(|_| RasterError("canvas geometry projection"))?;
        let quarter = projection.dimensions().map(|v| v as usize);
        let mut roots = Vec::new();
        let mut getters = Vec::new();
        let mut segments = Vec::new();
        if let Some(g) = geometry {
            roots.try_reserve(g.roots.len()).map_err(|_| RasterError("canvas root allocation"))?;
            getters.try_reserve(g.getters.len()).map_err(|_| RasterError("canvas getter allocation"))?;
            segments.try_reserve(g.segments.len()).map_err(|_| RasterError("canvas segment allocation"))?;
            for root in &g.roots {
                let r = projection.world_rectangle(root.xy.map(f32::from_bits), 8.0)
                    .map_err(|_| RasterError("canvas root coordinates"))?;
                roots.push((r, (root.argb & 0xffffff) | 0x33000000));
            }
            for xy in &g.getters {
                let r = projection.world_rectangle(xy.map(f32::from_bits), 2.0)
                    .map_err(|_| RasterError("canvas getter coordinates"))?;
                if let Some(table) = ellipse_table(r, quarter)? { getters.push(table); }
            }
            for words in &g.segments {
                let values = words.map(f32::from_bits);
                if values.iter().any(|v| !v.is_finite()) {
                    return Err(RasterError("nonfinite canvas segment"));
                }
                segments.push(values);
            }
        }
        let sx = f.dimensions[0] as f32 / quarter[0] as f32;
        let sy = f.dimensions[1] as f32 / quarter[1] as f32;
        let inverse = literal::rectangle::inverse_source([sx, 0.0, 0.0, 0.0, sy, 0.0]);
        let mask_dimensions = mask_dimensions(f.dimensions);
        let mask_inverse = literal::rectangle::inverse_source([
            f.dimensions[0] as f32 / mask_dimensions[0] as f32, 0.0, 0.0,
            0.0, f.dimensions[1] as f32 / mask_dimensions[1] as f32, 0.0,
        ]);
        Ok(Self { geometry: geometry.cloned(), dimensions: f.dimensions, zoom: f.zoom.to_bits(),
            quarter, roots, getters, segments,
            x_samples: sample_axis(f.dimensions[0] as usize, quarter[0], inverse[0], inverse[2])?,
            y_samples: sample_rows(f.dimensions[1] as usize, quarter[1], inverse[4], inverse[5])?,
            mask_dimensions,
            mask_x_samples: sample_axis(f.dimensions[0] as usize, mask_dimensions[0], mask_inverse[0], mask_inverse[2])?,
            mask_y_samples: sample_rows(f.dimensions[1] as usize, mask_dimensions[1], mask_inverse[4], mask_inverse[5])? })
    }
}

fn shade_radial(bytes: &mut [u8], dimensions: [usize; 2],
    frame: &literal::prefix::QuarterProjection, radius: f32, pixel_radius: f32,
    colors: &mut Vec<u32>) -> Result<(), RasterError>
{
    let commands = frame.initial_gradient(0xffffffff);
    let mut gradient = literal::gradient::Gradient::from_initial(&commands)
        .map_err(|_| RasterError("canvas gradient"))?;
    let mut walk = literal::prefix::RadialWalk::new(radius, pixel_radius)
        .map_err(|_| RasterError("canvas gradient radius"))?;
    while let Some(alpha) = walk.current_alpha_bits().map_err(|_| RasterError("canvas gradient alpha"))? {
        gradient.insert(walk.current_key(), 0x00ffffff | alpha)
            .map_err(|_| RasterError("canvas gradient stop"))?;
        walk.advance_after_insert().map_err(|_| RasterError("canvas gradient progress"))?;
    }
    let brush = literal::gradient::BrushPayload::from_gradient(&gradient)
        .map_err(|_| RasterError("canvas gradient brush"))?;
    let prepared = literal::gradient::prepare_render(&brush, &literal::rectangle::StateBoundary {
        clip_present: true, flags_28_29: [1, 0], integer_offset: [0, 0],
        affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
    }).map_err(|_| RasterError("canvas gradient preparation"))?;
    let norm = literal::lookup::source_norm_checked(literal::lookup::norm_operands(
        &prepared.gradient, prepared.affine)).map_err(|_| RasterError("canvas gradient norm"))?;
    let mut slot = literal::lookup::Slot::fresh();
    literal::lookup::rebuild_with_source_norm(&mut slot, &prepared.gradient, norm)
        .map_err(|_| RasterError("canvas gradient lookup"))?;
    let table = slot.table.ok_or(RasterError("missing canvas lookup"))?;
    colors.clear();
    colors.try_reserve(table.words.len()).map_err(|_| RasterError("canvas lookup allocation"))?;
    for word in table.words { colors.push(word.ok_or(RasterError("incomplete canvas lookup"))?); }
    let [sx, sy] = prepared.gradient.start;
    let [ex, ey] = prepared.gradient.end;
    let dx = sx - ex;
    let dy = sy - ey;
    let radius_squared = ((dx * dx) + (dy * dy)) as f64;
    if radius_squared <= 0.0 || !radius_squared.is_finite() {
        return Err(RasterError("invalid canvas radial extent"));
    }
    let scale = (colors.len() - 1) as f64 / radius_squared.sqrt();
    radial_math::shade_radial_argb(bytes, dimensions[0], dimensions[1],
        [sx as f64, sy as f64], radius_squared, scale, colors);
    Ok(())
}

fn paint_rectangle(bytes: &mut [u8], layout: &literal::image::BitmapLayout,
    clip: &[[i32; 4]], rectangle: [f32; 4], argb: u32) -> Result<(), RasterError>
{
    let mut painter = literal::solid::SpanPainter::new(literal::rectangle::premultiply_argb(argb));
    literal::solid::paint_float_clipped(bytes, layout, clip, rectangle, &mut painter)
        .map_err(|_| RasterError("canvas root paint"))
}

fn ellipse_table(rectangle: [f32; 4], dimensions: [usize; 2])
    -> Result<Option<literal::edge_table::EdgeTable>, RasterError>
{
    let path = literal::path::ellipse(rectangle).map_err(|_| RasterError("canvas ellipse path"))?;
    let clip = [0, 0, dimensions[0] as i32, dimensions[1] as i32];
    let Some(mut table) = literal::image::path_mask_source(clip, &path,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]).map_err(|_| RasterError("canvas ellipse raster"))?
        else { return Ok(None); };
    let region = literal::region::Rectangles::initial_clip(clip)
        .map_err(|_| RasterError("canvas ellipse clip"))?;
    if table.intersect_rectangles_mut_source(&region).map_err(|_| RasterError("canvas ellipse intersection"))? {
        Ok(Some(table))
    } else { Ok(None) }
}

fn paint_ellipse(bytes: &mut [u8], layout: &literal::image::BitmapLayout,
    table: &literal::edge_table::EdgeTable, argb: u32) -> Result<(), RasterError>
{
    let mut painter = literal::solid::SpanPainter::new(literal::rectangle::premultiply_argb(argb));
    literal::solid::shade_edge_table_format2_blend(bytes, layout, table, &mut painter)
        .map_err(|_| RasterError("canvas ellipse paint"))
}

fn blur(bytes: &mut [u8], scratch: &mut Vec<u8>, width: usize, height: usize, radius: i32)
    -> Result<(), RasterError>
{
    resize_bytes(scratch, bytes.len())?;
    let radius = literal::blur::effective_radius(radius);
    let multiply = literal::blur::MULTIPLY[radius];
    let shift = literal::blur::SHIFT[radius];
    let stride = width * 4;
    for y in 0..height {
        blur_line(bytes, scratch, y * stride, 4, width, radius, multiply, shift);
    }
    for x in 0..width {
        blur_line(scratch, bytes, x * 4, stride, height, radius, multiply, shift);
    }
    Ok(())
}

fn blur_line(source: &[u8], destination: &mut [u8], start: usize, stride: usize,
    length: usize, radius: usize, multiply: u32, shift: u32)
{
    let last = length - 1;
    let sample = |p: usize| {
        let at = start + p.min(last) * stride;
        [source[at] as u32, source[at+1] as u32, source[at+2] as u32, source[at+3] as u32]
    };
    let mut sum = [0u32; 4];
    for k in 0..2*radius+1 {
        let pixel = sample(k.saturating_sub(radius));
        let weight = (radius+1-k.abs_diff(radius)) as u32;
        for c in 0..4 { sum[c] += pixel[c] * weight; }
    }
    let first = sample(0);
    let mut lower = first.map(|v| v * (radius+1) as u32);
    let mut upper = [0u32; 4];
    for p in 1..=radius+1 { let pixel = sample(p);
        for c in 0..4 { upper[c] += pixel[c]; }
    }
    for x in 0..length {
        let at = start+x*stride;
        for c in 0..4 { destination[at+c] = (sum[c].wrapping_mul(multiply) >> shift) as u8; }
        if x == last { break; }
        let next = sample(x+1); let leaving = sample(x.saturating_sub(radius));
        let entering = sample(x.saturating_add(radius+2));
        for c in 0..4 {
            sum[c] = sum[c]-lower[c]+upper[c];
            lower[c] = lower[c]-leaving[c]+next[c];
            upper[c] = upper[c]-next[c]+entering[c];
        }
    }
}

fn sample_axis(count: usize, source: usize, scale: f32, translation: f32)
    -> Result<Vec<AxisSample>, RasterError>
{
    let mut samples = Vec::new();
    samples.try_reserve(count).map_err(|_| RasterError("canvas sample allocation"))?;

    append_x_span(&mut samples, 0, 1, source, scale, translation);
    if count > 2 { append_x_span(&mut samples, 1, count-2, source, scale, translation); }
    if count > 1 { append_x_span(&mut samples, count-1, 1, source, scale, translation); }
    Ok(samples)
}

fn append_x_span(samples: &mut Vec<AxisSample>, start: usize, count: usize,
    source: usize, scale: f32, translation: f32)
{
    let x0 = start as f32 + 0.5;
    let x1 = count as f32 + x0;
    let first = ((scale * x0 + translation) * 256.0) as i32;
    let end = ((scale * x1 + translation) * 256.0) as i32;
    let divisor = count as i32;
    let delta = end - first;
    let mut step = delta / divisor;
    let mut remainder = delta % divisor;
    if remainder <= 0 { remainder += divisor; step -= 1; }
    let mut current = first - 128;
    let mut error = remainder - divisor;
    let last = source as i32 - 1;
    for _ in 0..count {
        let coordinate = current;
        current += step;
        error += remainder;
        if error > 0 { error -= divisor; current += 1; }
        let low = coordinate >> 8;
        samples.push(AxisSample { low: low.clamp(0, last) as usize,
            high: (low + 1).clamp(0, last) as usize,
            fraction: (coordinate & 255) as u32, interior: low >= 0 && low < last });
    }
}

fn sample_rows(count: usize, source: usize, scale: f32, translation: f32)
    -> Result<Vec<AxisSample>, RasterError>
{
    let last = source as i32 - 1;
    let mut samples = Vec::new();
    samples.try_reserve(count).map_err(|_| RasterError("canvas sample allocation"))?;
    for row in 0..count {
        let y = row as f32 + 0.5;
        let start_y = y * scale + translation;
        let coordinate = (start_y * 256.0) as i32 - 128;
        let low = coordinate >> 8;
        samples.push(AxisSample { low: low.clamp(0, last) as usize,
            high: (low + 1).clamp(0, last) as usize,
            fraction: (coordinate & 255) as u32, interior: low >= 0 && low < last });
    }
    Ok(samples)
}

#[inline]
fn sample(bytes: &[u8], width: usize, x: AxisSample, y: AxisSample) -> [u8; 4] {
    let read = |xx, yy| { let at = (yy * width + xx) * 4;
        [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]] };
    let a = read(x.low, y.low);
    let mut out = [0; 4];
    if x.interior && y.interior {
        let b = read(x.high, y.low); let c = read(x.high, y.high); let d = read(x.low, y.high);
        let weights = [(256-y.fraction)*(256-x.fraction), (256-y.fraction)*x.fraction,
            y.fraction*x.fraction, y.fraction*(256-x.fraction)];
        for k in 0..4 { out[k] = ((u32::from(a[k])*weights[0] + u32::from(b[k])*weights[1]
            + u32::from(c[k])*weights[2] + u32::from(d[k])*weights[3] + 32768) >> 16) as u8; }
    } else if x.interior || y.interior {
        let (b, fraction) = if x.interior { (read(x.high, y.low), x.fraction) }
            else { (read(x.low, y.high), y.fraction) };
        for k in 0..4 { out[k] = ((u32::from(a[k])*(256-fraction)
            + u32::from(b[k])*fraction + 128) >> 8) as u8; }
    } else { out = a; }
    out
}

struct GeometryMath;
impl literal::radial::SqrtServices for GeometryMath {
    fn sqrt_f64_bits(&mut self, bits: u64) -> Result<u64, ()> { Ok(f64::from_bits(bits).sqrt().to_bits()) }
}
impl literal::geometry::MathServices for GeometryMath {
    fn atan2(&mut self, a: f32, b: f32) -> Result<f32, ()> { Ok((a as f64).atan2(b as f64) as f32) }
    fn norm(&mut self, a: f32, b: f32) -> Result<f32, ()> {
        Ok(((b as f64 * b as f64) + (a as f64 * a as f64)).sqrt() as f32)
    }
    fn ring_sin_cos(&mut self, a: f32) -> Result<[f32; 2], ()> { Ok([a.sin(), a.cos()]) }
    fn projection_sin(&mut self, a: f32) -> Result<f32, ()> { Ok(a.sin()) }
    fn projection_cos(&mut self, a: f32) -> Result<f32, ()> { Ok(a.cos()) }
}

fn mask_dimensions(dimensions: [i32; 2]) -> [usize; 2] {
    dimensions.map(|n| (n / 2 + n % 2).max(4) as usize)
}

fn mask_vertex(logical: [f32; 2], dimensions: [i32; 2], mask: [usize; 2])
    -> Result<[i32; 2], RasterError>
{

    if logical.iter().any(|v| !v.is_finite() || v.abs() >= 2147483000.0) {
        return Err(RasterError("canvas shadow projection"));
    }
    let ratios = [mask[0] as f32 / dimensions[0] as f32,
        mask[1] as f32 / dimensions[1] as f32];
    let grid = [(logical[0] + 0.5) * ratios[0], (logical[1] + 0.5) * ratios[1]];
    if grid.iter().any(|v| !v.is_finite() || v.abs() >= 2147483000.0) {
        return Err(RasterError("canvas shadow projection"));
    }
    Ok(grid.map(|v| v.floor() as i32))
}

fn sample_mask(bytes: &[u8], width: usize, x: AxisSample, y: AxisSample) -> u8 {
    let read = |xx, yy| u32::from(bytes[yy * width + xx]);
    let weights = [(256-y.fraction)*(256-x.fraction), (256-y.fraction)*x.fraction,
        y.fraction*x.fraction, y.fraction*(256-x.fraction)];
    ((read(x.low, y.low)*weights[0] + read(x.high, y.low)*weights[1]
        + read(x.high, y.high)*weights[2] + read(x.low, y.high)*weights[3] + 32768) >> 16) as u8
}

fn render_mask(f: Fields, dimensions: [usize; 2], segments: &[[f32; 4]], mask: &mut [u8], delta: &mut [i16],
    obstacles: &mut Vec<[u32; 4]>, envelope: &mut Vec<[f32; 4]>,
    polygon: &mut Vec<[i32; 2]>, crossings: &mut Vec<Vec<i32>>) -> Result<(), RasterError>
{
    let rect = [0, 0, f.dimensions[0], f.dimensions[1]];
    let [width, height] = dimensions;
    let mut table = [0u8; 256];
    for (i, value) in table.iter_mut().enumerate() {
        *value = ((i as f32 * f32::from_bits(0x3f7de00d)
            + 255.0 * f32::from_bits(0x3c07fcb9)) as i32) as u8;
    }
    for pass in 0..16 {
        let origin = literal::geometry::ring_origin(f.probe, f.radius, pass, rect, f.zoom)
            .map_err(|_| RasterError("canvas shadow origin"))?;
        produce_obstacles(origin, segments, obstacles)?;
        literal::sort::sort_source(obstacles).map_err(|_| RasterError("canvas shadow sort"))?;
        let viewport = literal::geometry::viewport_envelope(origin, rect, f.zoom, &mut GeometryMath)
            .map_err(|_| RasterError("canvas shadow viewport"))?;
        envelope.clear(); envelope.extend(viewport.records.iter().map(|r| r.0));
        insert_obstacles(envelope, obstacles)?;
        polygon.clear();
        polygon.try_reserve(envelope.len() * 2).map_err(|_| RasterError("canvas polygon allocation"))?;
        let scale = f.dimensions[0].min(f.dimensions[1]) as f32 * 0.5;
        let cx = f.dimensions[0] as f32 * 0.5; let cy = f.dimensions[1] as f32 * 0.5;
        for r in envelope.iter() {
            for (angle, distance) in [(r[0], r[2]), (r[1], r[3])] {
                let angle = angle + f32::from_bits(0xbfc90fdb);
                let wx = angle.cos() * distance + origin[0];
                let wy = angle.sin() * distance + origin[1];
                let x = ((wx * scale) * f.zoom + cx) + -0.5;
                let y = (cy - (wy * scale) * f.zoom) + -0.5;
                polygon.push(mask_vertex([x, y], f.dimensions, dimensions)?);
            }
        }
        raster_polygon(delta, width, height, polygon, crossings)?;
    }

    let mut coverage_values = [0u8; 17];
    for n in 1..coverage_values.len() {
        coverage_values[n] = table[coverage_values[n-1] as usize];
    }
    for (row, output) in delta.chunks_exact(width+1).zip(mask.chunks_exact_mut(width)) {
        let mut count = 0i32;
        for (edge, pixel) in row.iter().zip(output) {
            count += i32::from(*edge);
            if !(0..=16).contains(&count) { return Err(RasterError("canvas coverage count")); }
            *pixel = coverage_values[count as usize];
        }
    }
    Ok(())
}

fn produce_obstacles(origin: [f32; 2], segments: &[[f32; 4]], output: &mut Vec<[u32; 4]>)
    -> Result<(), RasterError>
{
    fn norm(a: f32, b: f32) -> f32 {
        ((b as f64 * b as f64) + (a as f64 * a as f64)).sqrt() as f32
    }
    output.clear();
    output.try_reserve(segments.len().saturating_mul(2))
        .map_err(|_| RasterError("canvas obstacle allocation"))?;
    let far = [origin[0] + 0.0, origin[1] + 100000.0];

    let mut previous_end: Option<([u32; 2], f32, f32)> = None;
    for &[ax, ay, bx, by] in segments {
        let cached = previous_end.filter(|(xy, _, _)| *xy == [ax.to_bits(), ay.to_bits()]);
        let aa = if let Some((_, angle, _)) = cached { angle }
            else { ((ax-origin[0]) as f64).atan2((origin[1]-ay) as f64) as f32 };
        let ab = ((bx-origin[0]) as f64).atan2((origin[1]-by) as f64) as f32;
        let da = if let Some((_, _, distance)) = cached { distance }
            else { norm(origin[0]-ax, origin[1]-ay) };
        let db = norm(origin[0]-bx, origin[1]-by);
        previous_end = (ab.is_finite() && db.is_finite())
            .then_some(([bx.to_bits(), by.to_bits()], ab, db));
        let (start, end, ds, de) = if aa > ab { (ab, aa, db, da) } else { (aa, ab, da, db) };
        let pi = f32::from_bits(0x40490fdb);
        if end-start > pi {
            let point = if far == [ax, ay] { far } else {
                let dx = far[0]-origin[0]; let dy = far[1]-origin[1];
                let ex = bx-ax; let ey = by-ay; let den = dx*ey-dy*ex;
                if dx != 0.0 || !dy.is_finite() || dy <= 0.0 || !den.is_finite() {
                    return Err(RasterError("canvas obstacle seam"));
                }
                if den.abs() <= f32::MIN_POSITIVE { [(far[0]+ax)*0.5, (far[1]+ay)*0.5] }
                else { let t = (ex*(origin[1]-ay)-ey*(origin[0]-ax))/den;
                    [dx*t+origin[0], dy*t+origin[1]] }
            };
            let distance = norm(origin[0]-point[0], origin[1]-point[1]);
            if !distance.is_finite() { return Err(RasterError("canvas obstacle distance")); }
            output.push([-pi, start, distance, ds].map(f32::to_bits));
            output.push([end, pi, de, distance].map(f32::to_bits));
        } else { output.push([start, end, ds, de].map(f32::to_bits)); }
    }
    Ok(())
}

fn insert_obstacles(envelope: &mut Vec<[f32; 4]>, obstacles: &[[u32; 4]]) -> Result<(), RasterError> {
    const SHIFT: f32 = f32::from_bits(0xbfc90fdb);
    const EPS: f32 = f32::from_bits(0xb727c5ac);
    fn le(a: f32, b: f32) -> bool { a.is_nan() || b.is_nan() || a <= b }
    fn at_angle(r: [f32; 4], angle: f32) -> Result<f32, RasterError> {
        let end = r[1] + SHIFT; let start = r[0] + SHIFT;
        let ye = end.sin() * r[3]; let xe = end.cos() * r[3];
        let ys = start.sin() * r[2]; let xs = start.cos() * r[2];
        let dx = xe - xs; let dy = ye - ys;
        let numerator = dx * ys - dy * xs;
        let denominator = angle.sin() * dx - angle.cos() * dy;
        let value = numerator / denominator;
        if value.is_infinite() { Err(RasterError("canvas shadow intersection")) } else { Ok(value) }
    }
    fn insert(e: &mut Vec<[f32; 4]>, at: usize, r: [f32; 4]) -> Result<(), RasterError> {
        e.try_reserve(1).map_err(|_| RasterError("canvas shadow envelope allocation"))?;
        e.insert(at, r); Ok(())
    }
    for incoming in obstacles {
        let mut r = incoming.map(f32::from_bits);
        while r[1] > r[0] {
            let Some(index) = envelope.iter().rposition(|e| r[0] >= e[0] && e[1] > r[0]) else { break; };
            let e = envelope[index];
            if e[1] < r[1] {
                let d = at_angle(r, e[1] + SHIFT)?;
                if le(e[3] + EPS, d) { r[0] = e[1]; r[2] = d; }
                else {
                    let ed = at_angle(e, r[0] + SHIFT)?;
                    let next = [r[0], e[1], r[2], d];
                    r[0] = e[1]; r[2] = d;
                    envelope[index][1] = next[0]; envelope[index][3] = ed;
                    insert(envelope, index + 1, next)?;
                }
            } else {
                let d = at_angle(e, r[0] + SHIFT)?;
                if le(d + EPS, r[2]) { r[0] = r[1]; }
                else {
                    let distance = at_angle(e, r[1] + SHIFT)?;
                    envelope[index][1] = r[0]; envelope[index][3] = d;
                    if e[1] > r[1] { insert(envelope, index + 1, [r[1], e[1], distance, e[3]])?; }
                    insert(envelope, index + 1, r)?;
                    r[0] = r[1];
                }
            }
            if envelope[index][0] >= envelope[index][1] { envelope.remove(index); }
        }
    }
    Ok(())
}

fn span(delta: &mut [i16], width: usize, y: i64, left: i64, right: i64) {
    let left = left.max(0); let right = right.min(width as i64 - 1);
    if right < left { return; }
    let start = y as usize * (width+1);
    delta[start+left as usize] += 1;
    delta[start+right as usize+1] -= 1;
}

fn raster_polygon(delta: &mut [i16], width: usize, height: usize, v: &[[i32; 2]],
    crossings: &mut Vec<Vec<i32>>) -> Result<(), RasterError>
{
    if v.len() < 2 { return Ok(()); }
    let xmin = v.iter().map(|p| p[0]).min().unwrap(); let xmax = v.iter().map(|p| p[0]).max().unwrap();
    let ymin = v.iter().map(|p| p[1]).min().unwrap(); let ymax = v.iter().map(|p| p[1]).max().unwrap();
    if xmax < 0 || xmin >= width as i32 || ymax < 0 || ymin >= height as i32 { return Ok(()); }
    if v.len() == 2 {
        raster_line(delta, width, height, v[0], v[1]); return Ok(());
    }
    if v.len() == 3 {
        let (lo, hi) = if v[0][1] > v[1][1] { (v[1], v[0]) } else { (v[0], v[1]) };
        let (low, mid, high) = if lo[1] > v[2][1] {
            if hi[1] > lo[1] { (v[2], lo, hi) } else { (v[2], hi, lo) }
        }
            else if hi[1] > v[2][1] { (lo, v[2], hi) } else { (lo, hi, v[2]) };
        let [xa, ya] = low.map(i64::from); let [xb, yb] = mid.map(i64::from); let [xz, yz] = high.map(i64::from);
        let ab = (yb-ya).max(1); let az = (yz-ya).max(1); let bz = (yz-yb).max(1);
        for y in ya.max(0)..=yz.min(height as i64-1) {
            let edge = if y < yb { ((y-ya)*(xb-xa)+(xb-xa).signum()*ab/2)/ab+xa }
                else { ((y-yb)*(xz-xb)+(xz-xb).signum()*bz/2)/bz+xb };
            let chord = ((y-ya)*(xz-xa)+(xz-xa).signum()*az/2)/az+xa;
            span(delta, width, y, edge.min(chord), edge.max(chord));
        }
        return Ok(());
    }
    if ymin == ymax {
        raster_line(delta, width, height, [xmin, ymin], [xmax, ymax]); return Ok(());
    }
    let first = ymin.max(0); let last = ymax.min(height as i32-1);
    let rows = (last-first+1) as usize;
    crossings.resize_with(rows, Vec::new);
    for row in crossings.iter_mut() { row.clear(); }
    let n = v.len(); let mut i = 0; let mut next = 1;
    loop {
        let original = i; let [x0, y0] = v[i]; let mut end = next;
        let mut y1 = v[end][1]; let mut following = (end+1)%n; let mut after = v[following][1];
        if y1 == y0 && after == y0 {
            while after == y0 { end = following; following = (end+1)%n; after = v[following][1]; }
            y1 = v[end][1];
        }
        i = end; next = following; let x1 = v[i][0];
        if after == y1 { let mut look = following;
            while after == y1 { look = (look+1)%n; after = v[look][1]; }
        }
        let dy = i64::from(y1)-i64::from(y0);
        if dy != 0 {
            let direction = dy.signum(); let denominator = dy.abs(); let dx = i64::from(x1)-i64::from(x0);
            let bias = dx.signum()*denominator/2;
            let steps = denominator-i64::from(direction == (i64::from(after)-i64::from(y1)).signum())+1;

            let begin = if direction > 0 { (i64::from(first)-i64::from(y0)).max(0) }
                else { (i64::from(y0)-i64::from(last)).max(0) };
            let stop = if direction > 0 { (i64::from(last)-i64::from(y0)+1).min(steps) }
                else { (i64::from(y0)-i64::from(first)+1).min(steps) };
            for k in begin..stop {
                let row = (i64::from(y0)-i64::from(first)+direction*k) as usize;
                let x = ((k*dx+bias)/denominator)+i64::from(x0);
                crossings[row].push(x as i32);
            }
        }
        if i <= original { break; }
    }
    for (row, values) in crossings[..rows].iter_mut().enumerate() {
        if values.is_empty() || values.len()%2 != 0 { return Err(RasterError("canvas polygon crossings")); }
        values.sort_unstable();
        let mut previous_right = width as i32;
        for pair in values.chunks_exact(2) {
            let left = i64::from(pair[0])+i64::from(pair[0] == previous_right);
            previous_right = pair[1];
            span(delta, width, i64::from(first)+row as i64, left, i64::from(pair[1]));
        }
    }
    Ok(())
}

fn raster_line(delta: &mut [i16], width: usize, height: usize, a: [i32; 2], b: [i32; 2]) {
    let dx = i64::from(b[0])-i64::from(a[0]); let dy = i64::from(b[1])-i64::from(a[1]);
    let x_major = dx.abs() > dy.abs();
    let (a0, b0, a1, b1, limit, minor_limit) = if x_major {
        (a[0], a[1], b[0], b[1], width, height)
    } else { (a[1], a[0], b[1], b[0], height, width) };
    let (start, origin, end, minor_delta) = if a0 <= a1 { (a0, b0, a1, i64::from(b1)-i64::from(b0)) }
        else { (a1, b1, a0, i64::from(b0)-i64::from(b1)) };
    let major_delta = i64::from(end)-i64::from(start); let divisor = major_delta.max(1);
    for major in i64::from(start).max(0)..=i64::from(end).min(limit as i64-1) {
        let minor = ((major-i64::from(start))*minor_delta+minor_delta.signum()*major_delta/2)/divisor+i64::from(origin);
        if minor >= 0 && minor < minor_limit as i64 {
            let (x, y) = if x_major { (major, minor) } else { (minor, major) };
            span(delta, width, y, x, x);
        }
    }
}

