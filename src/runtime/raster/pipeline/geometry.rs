use super::{producer, insertion, projection, radial, raster, sort};
#[path = "geometry_mask.rs"] pub mod snapshot;
pub use snapshot::{Mask16Completed, Mask16Phase, Mask16Progress, Mask16Renderer, Mask16Status};
#[derive(Debug)]
pub enum Error {
    Domain, Size, Allocation, RingTrigRequired, CompositeStopped, Ownership,
    Producer(producer::Error), ProducerMath(producer::MathFailure), Sort(sort::Error),
    Insertion(insertion::Error), InsertionMath(insertion::MathFailure),
    Projection(projection::Error), ProjectionMath(projection::MathFailure), Raster(raster::Error),
    Viewport { cause: ViewportCause, partial: ViewportEnvelope, pending: ViewportPending },
}
#[link(name = "ucrt")]
unsafe extern "C" { fn sinf(x: f32) -> f32; fn cosf(x: f32) -> f32; fn atan2(y: f64, x: f64) -> f64; }
fn angle(x: f32, y: f32) -> Result<f32, Error> {
    let v = unsafe { atan2(x as f64, y as f64) } as f32;
    if v.is_finite() { Ok(v) } else { Err(Error::Domain) }
}

pub trait MathServices: radial::SqrtServices {
    fn atan2(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()>;
    fn ring_sin_cos(&mut self, theta: f32) -> Result<[f32; 2], ()>;
    fn projection_sin(&mut self, angle: f32) -> Result<f32, ()>;
    fn projection_cos(&mut self, angle: f32) -> Result<f32, ()>;
    fn norm(&mut self, first: f32, second: f32) -> Result<f32, ()>;
}
struct HostMath;
impl radial::SqrtServices for HostMath {
    fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()> {
        Ok(f64::from_bits(operand).sqrt().to_bits())
    }
}
impl MathServices for HostMath {
    fn norm(&mut self, first: f32, second: f32) -> Result<f32, ()> { distance(first, second).map_err(|_| ()) }
    fn projection_sin(&mut self, angle: f32) -> Result<f32, ()> { Ok(unsafe { sinf(angle) }) }
    fn projection_cos(&mut self, angle: f32) -> Result<f32, ()> { Ok(unsafe { cosf(angle) }) }
    fn atan2(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()> {
        angle(numerator, denominator).map_err(|_| ())
    }
    fn ring_sin_cos(&mut self, theta: f32) -> Result<[f32; 2], ()> {
        let sin = unsafe { sinf(theta) }; let cos = unsafe { cosf(theta) };
        Ok([sin, cos])
    }
}
fn distance(x: f32, y: f32) -> Result<f32, Error> {
    let x = x as f64; let y = y as f64;
    let v = ((y * y) + (x * x)).sqrt() as f32;
    if v.is_finite() { Ok(v) } else { Err(Error::Domain) }
}
fn domain(rect: [i32; 4], zoom: f32) -> Result<(), Error> {
    if rect[2] <= 0 || rect[3] <= 0 || !zoom.is_finite() || zoom <= 0.0 {
        return Err(Error::Domain);
    }
    Ok(())
}
fn frame(rect: [i32; 4]) -> (f32, f32, f32) {
    ((rect[2] as f32) * 0.5 + rect[0] as f32,
     (rect[3] as f32) * 0.5 + rect[1] as f32,
     (rect[2].min(rect[3]) as f32) * 0.5)
}

fn clamp_pixel(v: f32, low: f32, high: f32) -> f32 {
    let upper = if high < v { high } else { v };
    if v < low { low } else { upper }
}

pub struct RingSample { sin: f32, cos: f32 }

pub fn ring_sample(pass: usize) -> Result<RingSample, Error> {
    ring_sample_with_math(pass, &mut HostMath)
}
fn ring_sample_with_math(pass: usize, math: &mut impl MathServices) -> Result<RingSample, Error> {
    if pass >= 16 { return Err(Error::Domain); }
    let theta = ((pass as f32) * 0.0625) * f32::from_bits(0x40c90fdb);
    let [sin, cos] = math.ring_sin_cos(theta).map_err(|_| Error::RingTrigRequired)?;
    if !sin.is_finite() || !cos.is_finite() { return Err(Error::Domain); }
    Ok(RingSample { sin, cos })
}
pub fn ring_origin(probe: [f32; 2], radius: f32, pass: usize,
                   rect: [i32; 4], saved_initial_zoom: f32) -> Result<[f32; 2], Error> {
    origin_from_sample(probe, radius, ring_sample(pass)?, rect, saved_initial_zoom)
}
pub fn origin_from_sample(probe: [f32; 2], radius: f32, sample: RingSample,
    rect: [i32; 4], saved_initial_zoom: f32) -> Result<[f32; 2], Error> {
    domain(rect, saved_initial_zoom)?;
    if probe.iter().any(|x| !x.is_finite()) || !radius.is_finite() {
        return Err(Error::Domain);
    }
    let wy = sample.sin * radius + probe[1];
    let wx = sample.cos * radius + probe[0];
    let (cx, cy, scale) = frame(rect);
    let px = (wx * scale) * saved_initial_zoom + cx;
    let py = cy - (wy * scale) * saved_initial_zoom;
    if !px.is_finite() || !py.is_finite() { return Err(Error::Domain); }

    let px = clamp_pixel(px, rect[0] as f32, rect[2] as f32 + rect[0] as f32);
    let py = clamp_pixel(py, rect[1] as f32, rect[3] as f32 + rect[1] as f32);
    let origin = [((px - cx) / scale) / saved_initial_zoom,
                  ((cy - py) / scale) / saved_initial_zoom];
    if origin.iter().any(|x| !x.is_finite()) { return Err(Error::Domain); }
    Ok(origin)
}
#[derive(Debug)]
pub struct ViewportEnvelope { pub records: Vec<insertion::Record>, pub capacity: usize }
#[derive(Debug)]
pub enum ViewportCause { AngleRequired, NormRequired, NonFinite, Allocation }
#[derive(Clone, Copy, Debug)]
pub enum ViewportPhase { AngleA, AngleB, NormA, NormB, Append }
#[derive(Debug)]
pub struct ViewportPending {
    pub angles: [[f32; 2]; 2], pub norms: [[f32; 2]; 2],
    pub values: [Option<f32>; 4], pub phase: ViewportPhase,
}
fn push(out: &mut ViewportEnvelope, a: [f32; 2], b: [f32; 2],
        da: [f32; 2], db: [f32; 2], math: &mut impl MathServices) -> Result<(), Error> {
    let mut pending = ViewportPending { angles: [a, b], norms: [da, db],
        values: [None; 4], phase: ViewportPhase::AngleA };
    let result = (|| -> Result<(), ViewportCause> {

        let start = math.atan2(a[0], a[1]).map_err(|_| ViewportCause::AngleRequired)?;
        pending.values[0] = Some(start);
        pending.phase = ViewportPhase::AngleB;
        let end = math.atan2(b[0], b[1]).map_err(|_| ViewportCause::AngleRequired)?;
        pending.values[1] = Some(end);
        if !start.is_finite() || !end.is_finite() { return Err(ViewportCause::NonFinite); }
        pending.phase = ViewportPhase::NormA;
        let ds = math.norm(da[0], da[1]).map_err(|_| ViewportCause::NormRequired)?;
        pending.values[2] = Some(ds);
        pending.phase = ViewportPhase::NormB;
        let de = math.norm(db[0], db[1]).map_err(|_| ViewportCause::NormRequired)?;
        pending.values[3] = Some(de);
        if !ds.is_finite() || !de.is_finite() { return Err(ViewportCause::NonFinite); }
        pending.phase = ViewportPhase::Append;
        out.records.try_reserve(1).map_err(|_| ViewportCause::Allocation)?;
        if out.records.len() == out.capacity {
            out.capacity = (out.capacity + out.capacity / 2).max(out.records.len() + 1);
        }
        out.records.push(insertion::Record([start, end, ds, de]));
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(cause) => {
            let partial = std::mem::replace(out, ViewportEnvelope { records: Vec::new(), capacity: 0 });
            Err(Error::Viewport { cause, partial, pending })
        }
    }
}

pub fn viewport_envelope(origin: [f32; 2], rect: [i32; 4], viewport_reread_zoom: f32,
    math: &mut impl MathServices)
    -> Result<ViewportEnvelope, Error> {
    domain(rect, viewport_reread_zoom)?;
    if origin.iter().any(|x| !x.is_finite()) { return Err(Error::Domain); }
    let (cx, cy, scale) = frame(rect);
    let left = ((rect[0] as f32 - cx) / scale) / viewport_reread_zoom;
    let top = ((cy - rect[1] as f32) / scale) / viewport_reread_zoom;

    let bottom_edge = rect[1].wrapping_add(rect[3]) as f32;
    let right_edge = rect[0].wrapping_add(rect[2]) as f32;
    let bottom = ((cy - bottom_edge) / scale) / viewport_reread_zoom;
    let right = ((right_edge - cx) / scale) / viewport_reread_zoom;
    let dx_left = left - origin[0]; let dx_right = right - origin[0];
    let nx_left = origin[0] - left; let nx_right = origin[0] - right;
    let dy_top = origin[1] - top; let dy_bottom = origin[1] - bottom;
    let same = origin[0] - origin[0];
    let mut out = ViewportEnvelope { records: Vec::new(), capacity: 0 };
    if origin[0] > left {
        push(&mut out, [same, dy_top], [dx_left, dy_top], [same, dy_top], [nx_left, dy_top], math)?;
        out.records[0].0[0] = f32::from_bits(0xc0490fdb);
    }
    push(&mut out, [dx_left, dy_top], [dx_left, dy_bottom], [nx_left, dy_top], [nx_left, dy_bottom], math)?;
    push(&mut out, [dx_left, dy_bottom], [dx_right, dy_bottom], [nx_left, dy_bottom], [nx_right, dy_bottom], math)?;
    push(&mut out, [dx_right, dy_bottom], [dx_right, dy_top], [nx_right, dy_bottom], [nx_right, dy_top], math)?;
    if right > origin[0] {
        push(&mut out, [dx_right, dy_top], [same, dy_top], [nx_right, dy_top], [same, dy_top], math)?;
        let last = out.records.len() - 1;
        out.records[last].0[1] = f32::from_bits(0x40490fdb);
    }
    Ok(out)
}

pub struct PassBoundary<'a> {

    pub segments: &'a [[f32; 4]],
    pub viewport_reread_zoom: f32,
}
#[derive(Debug)]
pub struct Mask { pub bytes: Vec<u8>, pub width: usize, pub height: usize, pub pattern: u32 }
pub fn allocate_zero_mask(rect: [i32; 4], initial_line_pattern: u32) -> Result<Mask, Error> {
    if rect[2] <= 0 || rect[3] <= 0 { return Err(Error::Domain); }
    let width = rect[2] as usize; let height = rect[3] as usize;
    let count = width.checked_mul(height).ok_or(Error::Size)?;
    if count as u64 >= 0x400000001 { return Err(Error::Size); }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| Error::Allocation)?;
    bytes.resize(count, 0);
    Ok(Mask { bytes, width, height, pattern: initial_line_pattern })
}

pub fn produce_records(origin: [f32; 2], segments: &[[f32; 4]]) -> Result<Vec<[u32; 4]>, Error> {
    producer::produce_checked(origin, segments, segments.len())
        .map(|output| output.records).map_err(Error::Producer)
}
pub fn sort_records(records: &mut [[u32; 4]]) -> Result<(), Error> {
    sort::sort_source(records).map_err(Error::Sort)
}
#[derive(Clone, Copy, Debug)]
pub enum CompositePhase {
    Inputs, OwnedRecordBridge, Viewport461b1d, Insertion461f83, OwnedEnvelopeBridge,
    ReleaseObstacles462640, Projection462684, OwnedVertexBridge,
    Raster4628b5, ReleasePlanar463c46, ReleaseEnvelope463c54,
}

pub struct CompositePending {
    pub sorted: Option<Vec<[u32; 4]>>, pub obstacles: Vec<insertion::Record>,
    pub viewport: Option<ViewportEnvelope>, pub inserted: Option<insertion::Output>,
    pub envelope: Vec<projection::Record>, pub projected: Option<projection::Output>,
    pub vertices: Vec<[f32; 2]>,
}
#[must_use = "retain pending source-pass buffers and advanced math/mask; never retry"]
pub struct CompositeFailure {
    pub cause: Error, pub phase: CompositePhase,
    pub origin: [f32; 2], pub saved_initial_zoom: f32, pub viewport_reread_zoom: f32,
    pub pending: CompositePending,
}
impl std::fmt::Debug for CompositeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeFailure").field("cause", &self.cause).field("phase", &self.phase)
            .field("origin", &self.origin).field("saved_initial_zoom", &self.saved_initial_zoom)
            .field("viewport_reread_zoom", &self.viewport_reread_zoom)
            .field("sorted", &self.pending.sorted.as_ref().map(Vec::len))
            .field("obstacles", &self.pending.obstacles.len())
            .field("viewport", &self.pending.viewport.as_ref().map(|v| v.records.len()))
            .field("inserted", &self.pending.inserted.as_ref().map(|v| v.records.len()))
            .field("envelope", &self.pending.envelope.len())
            .field("planar", &self.pending.projected.as_ref().map(|v| v.planar.len()))
            .field("vertices", &self.pending.vertices.len()).finish()
    }
}
pub fn composite_sorted_pass(mask: &mut Mask, rect: [i32; 4], origin: [f32; 2],
    saved_initial_zoom: f32, viewport_reread_zoom: f32, sorted: Vec<[u32; 4]>,
    math: &mut impl MathServices) -> Result<(), CompositeFailure> {
    let mut phase = CompositePhase::Inputs;
    let mut pending = CompositePending { sorted: Some(sorted), obstacles: Vec::new(),
        viewport: None, inserted: None, envelope: Vec::new(), projected: None, vertices: Vec::new() };
    let result = (|| -> Result<(), Error> {
        if rect[2] <= 0 || rect[3] <= 0 || mask.width != rect[2] as usize
            || mask.height != rect[3] as usize { return Err(Error::Domain); }
        phase = CompositePhase::OwnedRecordBridge;
        let sorted = pending.sorted.as_ref().ok_or(Error::Ownership)?;
        pending.obstacles.try_reserve_exact(sorted.len()).map_err(|_| Error::Allocation)?;
        for r in sorted { pending.obstacles.push(insertion::Record(r.map(f32::from_bits))); }
        phase = CompositePhase::Viewport461b1d;
        pending.viewport = Some(viewport_envelope(origin, rect, viewport_reread_zoom, math)?);
        phase = CompositePhase::Insertion461f83;
        let viewport = pending.viewport.as_ref().ok_or(Error::Ownership)?;
        pending.inserted = Some(insertion::insert_with_math(&viewport.records, &pending.obstacles,
            viewport.capacity, math).map_err(Error::InsertionMath)?);
        phase = CompositePhase::ReleaseObstacles462640;

        drop(pending.sorted.take());
        drop(core::mem::take(&mut pending.obstacles));
        drop(pending.viewport.take());
        phase = CompositePhase::OwnedEnvelopeBridge;
        let inserted = pending.inserted.as_ref().ok_or(Error::Ownership)?;
        pending.envelope.try_reserve_exact(inserted.records.len()).map_err(|_| Error::Allocation)?;
        for r in &inserted.records { pending.envelope.push(projection::Record {
            start: r.0[0], end: r.0[1], start_distance: r.0[2], end_distance: r.0[3],
        }); }
        drop(pending.inserted.take());
        phase = CompositePhase::Projection462684;
        pending.projected = Some(projection::project_with_math(&pending.envelope, origin, rect,
            saved_initial_zoom, math).map_err(Error::ProjectionMath)?);
        let projected = pending.projected.as_ref().ok_or(Error::Ownership)?;
        if !projected.planar.is_empty() {
            phase = CompositePhase::OwnedVertexBridge;
            let n = projected.planar.len() / 2;
            pending.vertices.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
            for i in 0..n { pending.vertices.push([projected.planar[i], projected.planar[n + i]]); }
            phase = CompositePhase::Raster4628b5;
            raster::rasterize_checked(&mut mask.bytes, raster::Dimensions {
                width: mask.width, height: mask.height, depth: 1, channels: 1,
            }, &pending.vertices, &[255], &mut mask.pattern).map_err(Error::Raster)?;
        }
        phase = CompositePhase::ReleasePlanar463c46;
        drop(core::mem::take(&mut pending.vertices));
        drop(pending.projected.take());
        phase = CompositePhase::ReleaseEnvelope463c54;
        drop(core::mem::take(&mut pending.envelope));
        Ok(())
    })();
    match result { Ok(()) => Ok(()), Err(cause) => Err(CompositeFailure {
        cause, phase, origin, saved_initial_zoom, viewport_reread_zoom, pending }) }
}

pub fn render_mask16(rect: [i32; 4], probe: [f32; 2], radius: f32,
                     saved_initial_zoom: f32, passes: &[PassBoundary<'_>; 16],
                     initial_line_pattern: u32) -> Result<Mask, Error> {
    render_mask16_checked(rect, probe, radius, saved_initial_zoom, passes, initial_line_pattern)

        .map_err(|failure| match failure.composite_failure {
            Some(stopped) => stopped.cause, None => failure.cause,
        })
}
#[must_use = "retain partially rasterized mask and failing pass; do not publish or retry"]
#[derive(Debug)]
pub struct MaskFailure { pub cause: Error, pub partial: Option<Mask>, pub ordinal: usize,
    pub composite_failure: Option<CompositeFailure> }

pub fn render_mask16_checked(rect: [i32; 4], probe: [f32; 2], radius: f32,
    saved_initial_zoom: f32, passes: &[PassBoundary<'_>; 16], initial_line_pattern: u32)
    -> Result<Mask, MaskFailure> {

    render_mask16_with_math(rect, probe, radius, saved_initial_zoom, passes,
        initial_line_pattern, &mut HostMath)
}
pub fn render_mask16_with_math(rect: [i32; 4], probe: [f32; 2], radius: f32,
    saved_initial_zoom: f32, passes: &[PassBoundary<'_>; 16], initial_line_pattern: u32,
    math: &mut impl MathServices) -> Result<Mask, MaskFailure> {

    Mask16Renderer::new(rect, probe, radius, saved_initial_zoom, passes,
        initial_line_pattern, math)?.finish()
}

pub trait MaskOwner {
    type Guard;
    type Failure;
    type Math: MathServices;

    fn mask_math(&mut self) -> &mut Self::Math;
    fn enter_section_360(&mut self) -> Result<Self::Guard, Self::Failure>;

    fn leave_section_360(&mut self, guard: &Self::Guard) -> Result<(), Self::Failure>;
    fn ordered_segments_260<'a>(&'a self, guard: &Self::Guard)
        -> Result<&'a [[f32; 4]], Self::Failure>;

    fn ordered_segments_and_math_260<'a>(&'a mut self, guard: &Self::Guard)
        -> Result<(&'a [[f32; 4]], &'a mut Self::Math), Self::Failure>;
    fn reread_zoom_27c(&mut self) -> Result<f32, Self::Failure>;
}
#[derive(Debug)]
pub enum OwnerMaskError<E> { Binding(E), Numeric(Error), Composite(CompositeFailure),
    NumericAndUnlock(Error, E), BindingAndUnlock(E, E) }
enum PassProductionError<E> { Binding(E), Numeric(Error) }
#[derive(Clone, Copy, Debug)]
pub enum OwnerPassPhase { Unlock, Sort, RereadZoom }
#[derive(Debug)]
#[must_use = "completed/partially sorted records are the stopped pass, not a new input replay"]
pub struct OwnerPass {
    pub origin: [f32; 2], pub records: Vec<[u32; 4]>, pub phase: OwnerPassPhase,
}
pub struct OwnerMaskFailure<H: MaskOwner> {
    pub cause: OwnerMaskError<H::Failure>,
    pub partial: Option<Mask>, pub ordinal: usize,
    pub held_section: Option<H::Guard>,

    pub pending_pass: Option<OwnerPass>,
}

pub fn render_mask16_from_owner<H: MaskOwner>(host: &mut H, rect: [i32; 4],
    probe: [f32; 2], radius: f32, saved_initial_zoom: f32, initial_line_pattern: u32)
    -> Result<Mask, OwnerMaskFailure<H>> {
    if let Err(error) = domain(rect, saved_initial_zoom) {
        return Err(OwnerMaskFailure { cause: OwnerMaskError::Numeric(error),
            partial: None, ordinal: 0, held_section: None, pending_pass: None });
    }
    let mut mask = match allocate_zero_mask(rect, initial_line_pattern) {
        Ok(mask) => mask,
        Err(error) => return Err(OwnerMaskFailure { cause: OwnerMaskError::Numeric(error),
            partial: None, ordinal: 0, held_section: None, pending_pass: None }),
    };
    for ordinal in 0..16 {
        let sample = match ring_sample_with_math(ordinal, host.mask_math()) {
            Ok(sample) => sample,
            Err(error) => return Err(OwnerMaskFailure { cause: OwnerMaskError::Numeric(error),
                partial: Some(mask), ordinal, held_section: None, pending_pass: None }),
        };
        let guard = match host.enter_section_360() {
            Ok(guard) => guard,
            Err(error) => return Err(OwnerMaskFailure { cause: OwnerMaskError::Binding(error),
                partial: Some(mask), ordinal, held_section: None, pending_pass: None }),
        };
        let produced = match origin_from_sample(probe, radius, sample, rect, saved_initial_zoom) {
            Ok(origin) => match host.ordered_segments_and_math_260(&guard) {
                Ok((segments, math)) => producer::produce_with_math(origin, segments, segments.len(), math)
                    .map(|output| (origin, output.records)).map_err(|error|
                        PassProductionError::Numeric(Error::ProducerMath(error))),
                Err(error) => Err(PassProductionError::Binding(error)),
            },
            Err(error) => Err(PassProductionError::Numeric(error)),
        };
        if let Err(unlock) = host.leave_section_360(&guard) {
            let (cause, pending_pass) = match produced {
                Ok((origin, records)) => (OwnerMaskError::Binding(unlock), Some(OwnerPass {
                    origin, records, phase: OwnerPassPhase::Unlock })),
                Err(PassProductionError::Numeric(error)) => (OwnerMaskError::NumericAndUnlock(error, unlock), None),
                Err(PassProductionError::Binding(error)) => (OwnerMaskError::BindingAndUnlock(error, unlock), None),
            };
            return Err(OwnerMaskFailure { cause, partial: Some(mask), ordinal,
                held_section: Some(guard), pending_pass });
        }
        drop(guard);
        let (origin, mut obstacles) = match produced {
            Ok(value) => value,
            Err(error) => { let cause = match error {
                PassProductionError::Numeric(error) => OwnerMaskError::Numeric(error),
                PassProductionError::Binding(error) => OwnerMaskError::Binding(error),
            }; return Err(OwnerMaskFailure { cause, partial: Some(mask), ordinal,
                held_section: None, pending_pass: None }); },
        };
        if let Err(error) = sort_records(&mut obstacles) {
            return Err(OwnerMaskFailure { cause: OwnerMaskError::Numeric(error),
                partial: Some(mask), ordinal, held_section: None, pending_pass: Some(OwnerPass {
                    origin, records: obstacles, phase: OwnerPassPhase::Sort }) });
        }
        let zoom = match host.reread_zoom_27c() {
            Ok(zoom) => zoom,
            Err(error) => return Err(OwnerMaskFailure { cause: OwnerMaskError::Binding(error),
                partial: Some(mask), ordinal, held_section: None, pending_pass: Some(OwnerPass {
                    origin, records: obstacles, phase: OwnerPassPhase::RereadZoom }) }),
        };
        if let Err(error) = composite_sorted_pass(&mut mask, rect, origin, saved_initial_zoom, zoom, obstacles,
            host.mask_math()) {
            return Err(OwnerMaskFailure { cause: OwnerMaskError::Composite(error),
                partial: Some(mask), ordinal, held_section: None, pending_pass: None });
        }
    }
    Ok(mask)
}
