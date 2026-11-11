#[path = "pipeline/pixels.rs"] pub mod image;
pub use self::image as pixels;
#[path = "pipeline/public_numeric.rs"] pub mod public_numeric;
#[path = "pipeline/prefix.rs"] pub mod prefix;
#[path = "pipeline/gradient.rs"] pub mod gradient;
#[path = "pipeline/path.rs"] pub mod path;
#[path = "pipeline/rectangle.rs"] pub mod rectangle;
#[path = "pipeline/edge_table.rs"] pub mod edge_table;
#[path = "pipeline/raster_numeric_binding.rs"] pub mod raster_numeric_binding;
#[path = "pipeline/region.rs"] pub mod region;
#[path = "pipeline/sort.rs"] pub mod sort;
#[path = "pipeline/lookup.rs"] pub mod lookup;
#[path = "pipeline/radial.rs"] pub mod radial;
#[path = "pipeline/solid.rs"] pub mod solid;
#[path = "pipeline/sampling.rs"] pub mod sampling;
#[path = "pipeline/translated.rs"] pub mod translated;
#[path = "pipeline/image_draw.rs"] pub mod image_draw;
#[path = "pipeline/geometry.rs"] pub mod geometry;
#[path = "pipeline/empty.rs"] pub mod empty;
#[path = "pipeline/obstacles.rs"] pub mod producer;
#[path = "pipeline/inserted.rs"] pub mod insertion;
#[path = "pipeline/projected.rs"] pub mod projection;
#[path = "triangle/core.rs"] pub mod raster;
#[path = "blur/blur.rs"] pub mod blur;

#[derive(Clone, Copy, Debug)]
pub enum ProbeMarker { Visible, Hidden }
#[derive(Clone, Copy, Debug)]
pub struct OwnPalette {
    pub gradient_initial_argb: u32, pub gradient_insert_rgb: u32,
    pub getter_rgb: u32, pub probe_rgb: u32,
}
impl OwnPalette {

    pub const STARTUP_WHITE_BLACK: Self = Self {
        gradient_initial_argb: 0xffffffff, gradient_insert_rgb: 0x00ffffff,
        getter_rgb: 0, probe_rgb: 0,
    };
}
#[derive(Clone, Copy, Debug)]
pub struct DisplayFields {

    pub dimensions: [i32; 2], pub probe: [f32; 2], pub radius: f32,
    pub saved_initial_zoom: f32, pub viewport_zooms: [f32; 16],
    pub palette: OwnPalette, pub probe_marker: ProbeMarker,

    pub line_pattern_seed: u32,
}
#[derive(Debug)]
pub struct OwnedRequest {

    pub request_id: u64, pub fields: DisplayFields,
    pub geometry: Option<std::sync::Arc<Geometry>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RootRecord { pub argb: u32, pub xy: [u32; 2] }
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Geometry {
    pub roots: Vec<RootRecord>, pub getters: Vec<[u32; 2]>, pub segments: Vec<[u32; 4]>,
}
#[derive(Debug)]
pub enum FailureCause {
    DisplayDomain, PaletteAllocation, Prefix(prefix::Error),
    Recipe(empty::Error), Numeric(empty::RunError<empty::TensorError>),
    IncompleteTensor,
}
#[must_use = "failure retains request and advanced tensors; never publish as complete"]
pub struct Failure {
    pub request: OwnedRequest, pub cause: FailureCause,
    pub tensors: empty::TensorServices,
}
#[must_use = "completed owned BGRA tensor; caller owns upload/result lifetime"]
pub struct Completed {
    pub request: OwnedRequest, pub tensor: image::SoftwareStorage,
}
#[must_use = "scheduler retirement retains partial values; never publish as a completed image"]
pub struct Superseded<C> {
    pub request: OwnedRequest,
    pub tensors: empty::TensorServices,
    pub reason: C,
    pub prefix: empty::ScheduledPrefix,
}
pub enum ScheduledOutput<C> { Completed(Completed), Superseded(Superseded<C>) }
impl Completed {
    pub fn dimensions(&self) -> [i32; 2] { self.tensor.dimensions }
    pub fn row_stride(&self) -> usize { self.tensor.row_stride }
    pub fn premultiplied_bgra(&self) -> &[u8] { &self.tensor.bytes }
    pub fn into_parts(self) -> (OwnedRequest, image::SoftwareStorage) { (self.request, self.tensor) }
}

pub fn render_owned(request: OwnedRequest,
    services: &mut (impl geometry::MathServices + prefix::ExpServices))
    -> Result<Completed, Failure> {
    match render_owned_scheduled(request, services, &mut || None::<std::convert::Infallible>)? {
        ScheduledOutput::Completed(image) => Ok(image),
        ScheduledOutput::Superseded(stopped) => match stopped.reason {},
    }
}

pub fn render_owned_scheduled<C>(request: OwnedRequest,
    services: &mut (impl geometry::MathServices + prefix::ExpServices),
    checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledOutput<C>, Failure> {
    let mut tensors = empty::TensorServices::fresh();
    let result = (|| -> Result<empty::ScheduledOutcome<C>, FailureCause> {
        let fields = &request.fields;
        let [width, height] = fields.dimensions;
        if width < 4 || height < 4 || !fields.radius.is_finite() || fields.radius <= 0.0
            || !fields.saved_initial_zoom.is_finite() || fields.saved_initial_zoom <= 0.0
            || fields.probe.iter().any(|v| !v.is_finite())
            || fields.viewport_zooms.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(FailureCause::DisplayDomain);
        }
        let rectangle = [0, 0, width, height];
        let frame = prefix::QuarterProjection::new(rectangle, fields.probe, fields.saved_initial_zoom)
            .map_err(FailureCause::Prefix)?;
        let radius = frame.pixel_radius(fields.radius).map_err(FailureCause::Prefix)?;
        let mut walk = prefix::RadialWalk::new(fields.radius, radius).map_err(FailureCause::Prefix)?;
        let mut insertion_reads = Vec::new();

        while walk.pending_position().is_some() {
            insertion_reads.try_reserve(1).map_err(|_| FailureCause::PaletteAllocation)?;
            insertion_reads.push(fields.palette.gradient_insert_rgb);
            walk.advance_after_insert().map_err(FailureCause::Prefix)?;
            if let Some(reason) = checkpoint() {
                return Ok(empty::ScheduledOutcome::Superseded { reason,
                    prefix: empty::ScheduledPrefix::Palette { projection: frame, pixel_radius: radius,
                        walk, insertion_reads } });
            }
        }
        let roots: Vec<_> = request.geometry.iter().flat_map(|geometry| &geometry.roots)
            .map(|root| prefix::Root { argb: root.argb, displaced_position: root.xy.map(f32::from_bits) }).collect();
        let getters: Vec<_> = request.geometry.iter().flat_map(|geometry| &geometry.getters)
            .map(|xy| xy.map(f32::from_bits)).collect();
        let segments: Vec<_> = request.geometry.iter().flat_map(|geometry| &geometry.segments)
            .map(|edge| edge.map(f32::from_bits)).collect();
        let passes = std::array::from_fn(|ordinal| geometry::PassBoundary {
            segments: &segments, viewport_reread_zoom: fields.viewport_zooms[ordinal],
        });

        let numeric_probe_guard = match fields.probe_marker { ProbeMarker::Visible => 0, ProbeMarker::Hidden => 1 };
        let recipe = empty::Recipe::from_published_values_with_exp(empty::Inputs {
            rectangle, probe: fields.probe, radius: fields.radius,
            saved_initial_zoom: fields.saved_initial_zoom, copied_roots: &roots, copied_getters: &getters,
            passes, palettes: prefix::PaletteReads {
                gradient_initial: fields.palette.gradient_initial_argb,
                gradient_insertions: &insertion_reads, getter_marker: fields.palette.getter_rgb,
                probe_marker: fields.palette.probe_rgb,
            }, outer_voice_byte_f0: numeric_probe_guard,
            initial_line_pattern: fields.line_pattern_seed,
        }, services).map_err(FailureCause::Recipe)?;
        if let Some(reason) = checkpoint() {
            let (commands, resize) = recipe.into_schedule_prefix();
            return Ok(empty::ScheduledOutcome::Superseded { reason,
                prefix: empty::ScheduledPrefix::Recipe { commands, resize, insertion_reads,
                    roots, getters, segments } });
        }
        recipe.run_numeric_scheduled(&mut tensors, services, checkpoint).map_err(FailureCause::Numeric)
    })();
    match result {
        Err(cause) => return Err(Failure { request, cause, tensors }),
        Ok(empty::ScheduledOutcome::Superseded { reason, prefix }) => return Ok(
            ScheduledOutput::Superseded(Superseded { request, tensors, reason, prefix })),
        Ok(empty::ScheduledOutcome::Complete) => {},
    }
    match tensors.into_completed() {
        Ok(tensor) => Ok(ScheduledOutput::Completed(Completed { request, tensor })),
        Err(tensors) => Err(Failure { request, cause: FailureCause::IncompleteTensor, tensors }),
    }
}
