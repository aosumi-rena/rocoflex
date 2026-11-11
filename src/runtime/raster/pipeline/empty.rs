use super::{edge_table, geometry, gradient, image, image_draw, lookup, path, prefix,
    radial, rectangle, region, sampling, solid, translated};

#[derive(Debug)]
pub enum Error {
    NotEmpty, Prefix(prefix::Error), PrefixMath(prefix::CommandFailure), Gradient(gradient::Error),
    Lookup(lookup::Error), Path(path::Error), EdgeTable(edge_table::Error),
    Geometry(geometry::Error), Region(region::Error),
}
#[derive(Debug)]
pub enum RunError<E> {
    Recipe(Error), Service(E), Mask(geometry::MaskFailure), Radial(RadialFailure<E>),
    Multiply { error: E, mask: geometry::Mask },
}

pub enum ScheduledPrefix {
    Palette { projection: prefix::QuarterProjection, pixel_radius: f32,
        walk: prefix::RadialWalk, insertion_reads: Vec<u32> },
    Recipe { commands: prefix::Commands, resize: Resize, insertion_reads: Vec<u32>,
        roots: Vec<prefix::Root>, getters: Vec<[f32; 2]>, segments: Vec<[f32; 4]> },
    RootRectangle { ordinal: usize, rectangle: [f32; 4], argb: u32 },
    Path { stage: ScheduledPath, path: path::Path, painted: bool },
    SolidPath { stage: ScheduledPath, path: path::Path, partial: SolidPathPrefix },
    SolidPathSetup { stage: ScheduledPath, path: path::Path, partial: SolidPathSetupPrefix },
    Gradient { gradient: gradient::Gradient, next_insertion: usize },
    PreparedRadial { prepared: gradient::PreparedRender },
    Lookup { prepared: gradient::PreparedRender, lookup: lookup::Slot, progress: lookup::LookupProgress },
    Radial { progress: radial::rectangles::RectangleProgress,
        prepared: gradient::PreparedRender, lookup: lookup::Slot },
    Mask { partial: Option<geometry::Mask>, progress: geometry::Mask16Progress },
    Blur { source_argument: i32, partial: super::blur::FilterPartial },
    Resize { renderer: sampling::EdgeRenderer, table: edge_table::EdgeTable,
        clip: region::Rectangles, path: path::Path, next_ordinal: i32 },
    Multiply { mask: geometry::Mask, progress: image::mask_multiply::MultiplyProgress },
}
#[derive(Clone, Copy, Debug)]
pub enum ScheduledPath { Getter(usize), Probe }
pub enum ScheduledOutcome<C> {
    Complete,
    Superseded { reason: C, prefix: ScheduledPrefix },
}
pub enum RadialOutcome<C> {
    Complete,
    Superseded { reason: C, progress: radial::rectangles::RectangleProgress },
}
pub enum RadialPreparation<C> {
    Complete(gradient::PreparedRender),
    Superseded { reason: C, gradient: gradient::Gradient, next_insertion: usize },
}
pub struct SolidPathPrefix {
    pub table: edge_table::EdgeTable, pub clip: region::Rectangles,
    pub painter: solid::SpanPainter, pub progress: solid::EdgePaintProgress,
}
pub enum SolidPathSetupPrefix {
    Mask { table: edge_table::EdgeTable },
    Clip { table: edge_table::EdgeTable, clip: region::Rectangles },
    Intersection { table: edge_table::EdgeTable, clip: region::Rectangles, intersects: bool },
}
pub enum SolidPathOutcome<C> {
    Complete,
    Superseded { reason: C, partial: SolidPathPrefix },
    SetupSuperseded { reason: C, partial: SolidPathSetupPrefix },
}
pub enum MultiplyOutcome<C> {
    Complete,
    Superseded { reason: C, progress: image::mask_multiply::MultiplyProgress },
}
#[derive(Debug)]
pub enum RadialCause<E> { NormRequired([f32; 2]), Lookup(lookup::Error), Service(E) }
#[must_use = "retain prepared gradient and partial lookup alongside advanced tensors"]
pub struct RadialFailure<E> {
    pub cause: RadialCause<E>, pub prepared: gradient::PreparedRender, pub lookup: lookup::Slot,
}
impl<E: std::fmt::Debug> std::fmt::Debug for RadialFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadialFailure").field("cause", &self.cause)
            .field("lookup_size", &self.lookup.table.as_ref().map(|table| table.source_size))
            .finish_non_exhaustive()
    }
}

pub struct Inputs<'a> {
    pub rectangle: [i32; 4],

    pub probe: [f32; 2], pub radius: f32, pub saved_initial_zoom: f32,
    pub copied_roots: &'a [prefix::Root], pub copied_getters: &'a [[f32; 2]],
    pub passes: [geometry::PassBoundary<'a>; 16],
    pub palettes: prefix::PaletteReads<'a>,

    pub outer_voice_byte_f0: u8,
    pub initial_line_pattern: u32,
}

pub struct Resize {
    pub source_dimensions: [i32; 2], pub target_dimensions: [i32; 2],
    pub quality: i32, pub affine: [f32; 6],
}

pub struct PublishedFields { pub probe: [f32; 2], pub radius: f32, pub zoom: f32 }
impl PublishedFields {
    pub fn after_publication_reads(radius: f64, probe_pair: [Option<f32>; 2],
        zoom_pair: [Option<f32>; 2]) -> Self {
        Self { probe: super::public_numeric::source_probe(probe_pair), radius: radius as f32,
            zoom: super::public_numeric::source_zoom(zoom_pair) }
    }
}
pub struct Recipe<'a> {
    rectangle: [i32; 4], probe: [f32; 2], radius: f32,
    saved_initial_zoom: f32, passes: [geometry::PassBoundary<'a>; 16],
    initial_line_pattern: u32,
    commands: prefix::Commands,
    resize: Resize,
    outer_voice_byte_f0: u8,
}
impl<'a> Recipe<'a> {
    pub fn commands(&self) -> &prefix::Commands { &self.commands }
    pub fn resize(&self) -> &Resize { &self.resize }
    pub fn outer_voice_byte_f0(&self) -> u8 { self.outer_voice_byte_f0 }

    pub(super) fn into_schedule_prefix(self) -> (prefix::Commands, Resize) {
        (self.commands, self.resize)
    }
    pub fn new(inputs: Inputs<'a>) -> Result<Self, Error> {
        if !inputs.copied_roots.is_empty() || !inputs.copied_getters.is_empty()
            || inputs.passes.iter().any(|pass| !pass.segments.is_empty()) {
            return Err(Error::NotEmpty);
        }
        Self::from_published_values(inputs)
    }

    pub fn from_published_values(inputs: Inputs<'a>) -> Result<Self, Error> {

        let commands = prefix::commands(inputs.rectangle, inputs.probe, inputs.radius,
            inputs.saved_initial_zoom, inputs.copied_roots, inputs.copied_getters,
            inputs.palettes).map_err(Error::Prefix)?;
        Self::from_commands(inputs, commands)
    }

    pub fn from_published_values_with_exp(inputs: Inputs<'a>, exp: &mut impl prefix::ExpServices)
        -> Result<Self, Error> {
        let commands = prefix::commands_with_exp(inputs.rectangle, inputs.probe, inputs.radius,
            inputs.saved_initial_zoom, inputs.copied_roots, inputs.copied_getters,
            inputs.palettes, exp).map_err(Error::PrefixMath)?;
        Self::from_commands(inputs, commands)
    }

    fn from_commands(inputs: Inputs<'a>, commands: prefix::Commands) -> Result<Self, Error> {
        let target = [inputs.rectangle[2], inputs.rectangle[3]];
        let source = commands.quarter_dimensions;

        let sy = target[1] as f32 / source[1] as f32;
        let sx = target[0] as f32 / source[0] as f32;
        Ok(Self { rectangle: inputs.rectangle, probe: inputs.probe,
            radius: inputs.radius, saved_initial_zoom: inputs.saved_initial_zoom,
            passes: inputs.passes, initial_line_pattern: inputs.initial_line_pattern,
            resize: Resize { source_dimensions: source, target_dimensions: target,
                quality: 1, affine: [sx, 0.0, 0.0, 0.0, sy, 0.0] },
            commands, outer_voice_byte_f0: inputs.outer_voice_byte_f0 })
    }

    pub fn prepare_radial(&self) -> Result<gradient::PreparedRender, Error> {
        match self.prepare_radial_scheduled(&mut || None::<std::convert::Infallible>)? {
            RadialPreparation::Complete(prepared) => Ok(prepared),
            RadialPreparation::Superseded { reason, .. } => match reason {},
        }
    }

    pub fn prepare_radial_scheduled<C>(&self, checkpoint: &mut impl FnMut() -> Option<C>)
        -> Result<RadialPreparation<C>, Error> {
        let mut initial = gradient::Gradient::from_initial(&self.commands.gradient)
            .map_err(Error::Gradient)?;
        if let Some(reason) = checkpoint() {
            return Ok(RadialPreparation::Superseded { reason, gradient: initial, next_insertion: 0 });
        }
        for (index, stop) in self.commands.gradient.insertions.iter().enumerate() {
            initial.insert(stop.position, stop.argb).map_err(Error::Gradient)?;
            if let Some(reason) = checkpoint() {
                return Ok(RadialPreparation::Superseded {
                    reason, gradient: initial, next_insertion: index + 1,
                });
            }
        }
        let temporary = gradient::BrushPayload::from_gradient(&initial).map_err(Error::Gradient)?;
        let mut current = gradient::BrushPayload::solid(0xff000000);
        current.assign_null_image_source(&temporary).map_err(Error::Gradient)?;
        drop(temporary);
        gradient::prepare_render(&current, &rectangle::StateBoundary {
            clip_present: true, flags_28_29: [1, 0], integer_offset: [0, 0],
            affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }).map(RadialPreparation::Complete).map_err(Error::Gradient)
    }

    pub fn probe_path(&self) -> Result<Option<path::Path>, Error> {
        if self.outer_voice_byte_f0 != 0 { return Ok(None); }
        path::ellipse(self.commands.probe_ellipse.rectangle).map(Some).map_err(Error::Path)
    }
    pub fn probe_edge_table(&self) -> Result<Option<edge_table::EdgeTable>, Error> {
        let Some(path) = self.probe_path()? else { return Ok(None); };
        let [w, h] = self.commands.quarter_dimensions;
        let Some(mut table) = image::path_mask_source([0, 0, w, h], &path,
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]).map_err(Error::EdgeTable)? else { return Ok(None); };
        let clip = region::Rectangles::initial_clip([0, 0, w, h]).map_err(Error::Region)?;
        if table.intersect_rectangles_mut_source(&clip).map_err(Error::EdgeTable)? {
            Ok(Some(table))
        } else { Ok(None) }
    }

    pub fn mask(&self) -> Result<geometry::Mask, Error> {
        geometry::render_mask16(self.rectangle, self.probe, self.radius,
            self.saved_initial_zoom, &self.passes, self.initial_line_pattern)
            .map_err(Error::Geometry)
    }

    pub fn run_numeric<S: NumericServices>(&self, service: &mut S,
        math: &mut impl geometry::MathServices) -> Result<(), RunError<S::Failure>> {
        match self.run_numeric_scheduled(service, math,
            &mut || None::<std::convert::Infallible>)? {
            ScheduledOutcome::Complete => Ok(()),
            ScheduledOutcome::Superseded { reason, .. } => match reason {},
        }
    }

    pub fn run_numeric_scheduled<S: NumericServices, C>(&self, service: &mut S,
        math: &mut impl geometry::MathServices, checkpoint: &mut impl FnMut() -> Option<C>)
        -> Result<ScheduledOutcome<C>, RunError<S::Failure>> {
        service.begin_cleared_format2_quarter(self.commands.quarter_dimensions)
            .map_err(RunError::Service)?;
        service.clear_full_integer_clip(self.commands.clear_argb, false).map_err(RunError::Service)?;
        let prepared = match self.prepare_radial_scheduled(checkpoint).map_err(RunError::Recipe)? {
            RadialPreparation::Complete(prepared) => prepared,
            RadialPreparation::Superseded { reason, gradient, next_insertion } => return Ok(
                ScheduledOutcome::Superseded { reason,
                    prefix: ScheduledPrefix::Gradient { gradient, next_insertion } }),
        };
        if let Some(reason) = checkpoint() {
            return Ok(ScheduledOutcome::Superseded { reason,
                prefix: ScheduledPrefix::PreparedRadial { prepared } });
        }
        let operands = lookup::norm_operands(&prepared.gradient, prepared.affine);
        let norm = match math.norm(operands[0], operands[1]) {
            Ok(value) => value,
            Err(()) => return Err(RunError::Radial(RadialFailure {
                cause: RadialCause::NormRequired(operands), prepared, lookup: lookup::Slot::fresh(),
            })),
        };
        let mut slot = lookup::Slot::fresh();
        match lookup::rebuild_with_source_norm_scheduled(&mut slot, &prepared.gradient, norm, checkpoint) {
            Err(error) => return Err(RunError::Radial(RadialFailure {
                cause: RadialCause::Lookup(error), prepared, lookup: slot,
            })),
            Ok(lookup::ScheduledRebuild::Superseded { reason, progress }) => return Ok(
                ScheduledOutcome::Superseded { reason,
                    prefix: ScheduledPrefix::Lookup { prepared, lookup: slot, progress } }),
            Ok(lookup::ScheduledRebuild::Complete(_)) => {},
        }
        let rendered = match slot.table.as_ref() {
            Some(table) => service.radial_full_rect_region_scheduled(&prepared, table, math, checkpoint),
            None => return Err(RunError::Radial(RadialFailure {
                cause: RadialCause::Lookup(lookup::Error::Unwritten), prepared, lookup: slot,
            })),
        };
        match rendered {
            Err(error) => return Err(RunError::Radial(RadialFailure {
                cause: RadialCause::Service(error), prepared, lookup: slot,
            })),
            Ok(RadialOutcome::Superseded { reason, progress }) => return Ok(
                ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Radial {
                    progress, prepared, lookup: slot,
                } }),
            Ok(RadialOutcome::Complete) => {},
        }
        drop(slot); drop(prepared);

        for (ordinal, square) in self.commands.root_squares.iter().enumerate() {
            service.solid_root_rectangle(square.rectangle, square.argb).map_err(RunError::Service)?;
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::RootRectangle {
                    ordinal, rectangle: square.rectangle, argb: square.argb,
                } });
            }
        }
        match service.blur_format2_scheduled(37, checkpoint).map_err(RunError::Service)? {
            ScheduledOutcome::Complete => {},
            stopped @ ScheduledOutcome::Superseded { .. } => return Ok(stopped),
        }

        service.install_unused_getter_colour(self.commands.getter_argb).map_err(RunError::Service)?;
        for (ordinal, rectangle) in self.commands.getter_rectangles.iter().enumerate() {
            let path = path::ellipse(*rectangle).map_err(|e| RunError::Recipe(Error::Path(e)))?;
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Path {
                    stage: ScheduledPath::Getter(ordinal), path, painted: false,
                } });
            }
            match service.solid_probe_path_scheduled(&path, self.commands.getter_argb, checkpoint)
                .map_err(RunError::Service)? {
                SolidPathOutcome::Complete => {},
                SolidPathOutcome::SetupSuperseded { reason, partial } => return Ok(
                    ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::SolidPathSetup {
                        stage: ScheduledPath::Getter(ordinal), path, partial,
                    } }),
                SolidPathOutcome::Superseded { reason, partial } => return Ok(
                    ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::SolidPath {
                        stage: ScheduledPath::Getter(ordinal), path, partial,
                    } }),
            }
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Path {
                    stage: ScheduledPath::Getter(ordinal), path, painted: true,
                } });
            }
        }
        if let Some(path) = self.probe_path().map_err(RunError::Recipe)? {
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Path {
                    stage: ScheduledPath::Probe, path, painted: false,
                } });
            }
            match service.solid_probe_path_scheduled(&path, self.commands.probe_ellipse.argb, checkpoint)
                .map_err(RunError::Service)? {
                SolidPathOutcome::Complete => {},
                SolidPathOutcome::SetupSuperseded { reason, partial } => return Ok(
                    ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::SolidPathSetup {
                        stage: ScheduledPath::Probe, path, partial,
                    } }),
                SolidPathOutcome::Superseded { reason, partial } => return Ok(
                    ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::SolidPath {
                        stage: ScheduledPath::Probe, path, partial,
                    } }),
            }
            if let Some(reason) = checkpoint() {
                return Ok(ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Path {
                    stage: ScheduledPath::Probe, path, painted: true,
                } });
            }
        }

        match service.blur_format2_scheduled(1, checkpoint).map_err(RunError::Service)? {
            ScheduledOutcome::Complete => {},
            stopped @ ScheduledOutcome::Superseded { .. } => return Ok(stopped),
        }
        match service.resize_quality1_format2_scheduled(&self.resize, checkpoint).map_err(RunError::Service)? {
            ScheduledOutcome::Complete => {},
            stopped @ ScheduledOutcome::Superseded { .. } => return Ok(stopped),
        }
        let mut renderer = geometry::Mask16Renderer::new(self.rectangle, self.probe, self.radius,
            self.saved_initial_zoom, &self.passes, self.initial_line_pattern, math).map_err(RunError::Mask)?;
        let mask = loop {
            match renderer.advance(std::num::NonZeroUsize::MIN) {
                geometry::Mask16Status::Pending => {
                    if let Some(reason) = checkpoint() {
                        match renderer.into_partial_parts() {
                            Ok((partial, progress, _math)) => return Ok(ScheduledOutcome::Superseded {
                                reason, prefix: ScheduledPrefix::Mask { partial, progress },
                            }),

                            Err(owner) => renderer = owner,
                        }
                    }
                }
                geometry::Mask16Status::Complete | geometry::Mask16Status::Stopped => {
                    let progress = renderer.progress();

                    let mask = renderer.finish().map_err(RunError::Mask)?;
                    if let Some(reason) = checkpoint() {
                        return Ok(ScheduledOutcome::Superseded { reason,
                            prefix: ScheduledPrefix::Mask { partial: Some(mask), progress } });
                    }
                    break mask;
                }
            }
        };
        match service.multiply_full_mask_scheduled(&mask, checkpoint) {
            Ok(MultiplyOutcome::Complete) => Ok(ScheduledOutcome::Complete),
            Ok(MultiplyOutcome::Superseded { reason, progress }) => Ok(ScheduledOutcome::Superseded {
                reason, prefix: ScheduledPrefix::Multiply { mask, progress },
            }),
            Err(error) => Err(RunError::Multiply { error, mask }),
        }
    }
}

pub trait NumericServices {
    type Failure;
    fn begin_cleared_format2_quarter(&mut self, dimensions: [i32; 2]) -> Result<(), Self::Failure>;
    fn clear_full_integer_clip(&mut self, straight_argb: u32, replace: bool) -> Result<(), Self::Failure>;
    fn radial_full_rect_region(&mut self, prepared: &gradient::PreparedRender, table: &lookup::Table,
        math: &mut impl radial::SqrtServices)
        -> Result<(), Self::Failure>;

    fn radial_full_rect_region_scheduled<C>(&mut self, prepared: &gradient::PreparedRender,
        table: &lookup::Table, math: &mut impl radial::SqrtServices,
        _checkpoint: &mut impl FnMut() -> Option<C>) -> Result<RadialOutcome<C>, Self::Failure> {
        self.radial_full_rect_region(prepared, table, math)?;
        Ok(RadialOutcome::Complete)
    }
    fn solid_root_rectangle(&mut self, rectangle: [f32; 4], straight_argb: u32) -> Result<(), Self::Failure>;
    fn blur_format2(&mut self, source_argument: i32) -> Result<(), Self::Failure>;
    fn blur_format2_scheduled<C>(&mut self, source_argument: i32,
        _checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledOutcome<C>, Self::Failure> {
        self.blur_format2(source_argument)?;
        Ok(ScheduledOutcome::Complete)
    }
    fn install_unused_getter_colour(&mut self, straight_argb: u32) -> Result<(), Self::Failure>;
    fn solid_probe_path(&mut self, path: &path::Path, straight_argb: u32) -> Result<(), Self::Failure>;
    fn solid_probe_path_scheduled<C>(&mut self, path: &path::Path, straight_argb: u32,
        _checkpoint: &mut impl FnMut() -> Option<C>) -> Result<SolidPathOutcome<C>, Self::Failure> {
        self.solid_probe_path(path, straight_argb)?;
        Ok(SolidPathOutcome::Complete)
    }
    fn resize_quality1_format2(&mut self, resize: &Resize) -> Result<(), Self::Failure>;
    fn resize_quality1_format2_scheduled<C>(&mut self, resize: &Resize,
        _checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledOutcome<C>, Self::Failure> {
        self.resize_quality1_format2(resize)?;
        Ok(ScheduledOutcome::Complete)
    }
    fn multiply_full_mask(&mut self, mask: &geometry::Mask) -> Result<(), Self::Failure>;
    fn multiply_full_mask_scheduled<C>(&mut self, mask: &geometry::Mask,
        _checkpoint: &mut impl FnMut() -> Option<C>) -> Result<MultiplyOutcome<C>, Self::Failure> {
        self.multiply_full_mask(mask)?;
        Ok(MultiplyOutcome::Complete)
    }
}

#[derive(Debug)]
pub enum TensorError {
    Phase, Dimensions, Pixels(image::PixelError), Region(region::Error),
    Edge(edge_table::Error), Path(path::Error), Solid(solid::Error),
    Radial(radial::Error), Sampling(sampling::Error), Translated(translated::Error),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase { Fresh, Quarter, Resizing, Full, Complete }

pub struct TensorServices {
    phase: Phase,
    quarter: Option<image::SoftwareStorage>,
    full: Option<image::SoftwareStorage>,
    pub installed_unused_getter_colour: Option<u32>,
}
impl TensorServices {
    pub fn fresh() -> Self { Self { phase: Phase::Fresh, quarter: None, full: None,
        installed_unused_getter_colour: None } }
    pub fn partial_quarter(&self) -> Option<&image::SoftwareStorage> { self.quarter.as_ref() }
    pub fn partial_full(&self) -> Option<&image::SoftwareStorage> { self.full.as_ref() }
    pub fn completed(&self) -> Option<&image::SoftwareStorage> {
        if self.phase == Phase::Complete { self.full.as_ref() } else { None }
    }

    pub fn into_completed(mut self) -> Result<image::SoftwareStorage, Self> {
        if self.phase != Phase::Complete { return Err(self); }
        match self.full.take() { Some(full) => Ok(full), None => Err(self) }
    }
    fn quarter_mut(&mut self) -> Result<&mut image::SoftwareStorage, TensorError> {
        if self.phase != Phase::Quarter { return Err(TensorError::Phase); }
        self.quarter.as_mut().ok_or(TensorError::Phase)
    }
}
fn tensor_layout(tensor: &image::SoftwareStorage) -> image::BitmapLayout {
    image::BitmapLayout { width: tensor.dimensions[0] as usize,
        height: tensor.dimensions[1] as usize, row_stride: tensor.row_stride, pixel_stride: 4 }
}
fn initial_numeric_state() -> rectangle::StateBoundary {
    rectangle::StateBoundary { clip_present: true, flags_28_29: [1, 0],
        integer_offset: [0, 0], affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] }
}
impl NumericServices for TensorServices {
    type Failure = TensorError;
    fn begin_cleared_format2_quarter(&mut self, dimensions: [i32; 2]) -> Result<(), TensorError> {
        if self.phase != Phase::Fresh { return Err(TensorError::Phase); }
        if dimensions.iter().any(|&n| n <= 0) { return Err(TensorError::Dimensions); }
        self.phase = Phase::Quarter;
        self.quarter = Some(image::SoftwareStorage::create_cleared_format2(dimensions)
            .map_err(TensorError::Pixels)?);
        Ok(())
    }
    fn clear_full_integer_clip(&mut self, straight_argb: u32, replace: bool) -> Result<(), TensorError> {
        let tensor = self.quarter_mut()?;
        let layout = tensor_layout(tensor);
        let rectangle = [0, 0, tensor.dimensions[0], tensor.dimensions[1]];
        solid::paint_integer_clipped(&mut tensor.bytes, &layout, &[rectangle], rectangle,
            rectangle::premultiply_argb(straight_argb), replace).map_err(TensorError::Solid)
    }
    fn radial_full_rect_region(&mut self, prepared: &gradient::PreparedRender,
        table: &lookup::Table, math: &mut impl radial::SqrtServices) -> Result<(), TensorError> {
        match self.radial_full_rect_region_scheduled(prepared, table, math,
            &mut || None::<std::convert::Infallible>)? {
            RadialOutcome::Complete => Ok(()),
            RadialOutcome::Superseded { reason, .. } => match reason {},
        }
    }
    fn radial_full_rect_region_scheduled<C>(&mut self, prepared: &gradient::PreparedRender,
        table: &lookup::Table, math: &mut impl radial::SqrtServices,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<RadialOutcome<C>, TensorError> {
        let tensor = self.quarter_mut()?;
        let layout = tensor_layout(tensor);
        let rectangle = [0, 0, tensor.dimensions[0], tensor.dimensions[1]];
        let rectangles = [rectangle];
        let budget = std::num::NonZeroUsize::new(layout.width).unwrap_or(std::num::NonZeroUsize::MIN);
        let mut shader = radial::RectangleShader::new(&mut tensor.bytes, layout,
            &rectangles, prepared, table, math);
        loop {
            match shader.advance(budget) {
                radial::RectangleStatus::Pending => if let Some(reason) = checkpoint() {
                    return Ok(RadialOutcome::Superseded { reason, progress: shader.progress() });
                },
                radial::RectangleStatus::Complete | radial::RectangleStatus::Stopped => {
                    let progress = shader.progress();
                    shader.finish().map_err(TensorError::Radial)?;
                    if let Some(reason) = checkpoint() {
                        return Ok(RadialOutcome::Superseded { reason, progress });
                    }
                    return Ok(RadialOutcome::Complete);
                }
            }
        }
    }
    fn blur_format2(&mut self, source_argument: i32) -> Result<(), TensorError> {
        match self.blur_format2_scheduled(source_argument, &mut || None::<std::convert::Infallible>)? {
            ScheduledOutcome::Complete => Ok(()),
            ScheduledOutcome::Superseded { reason, .. } => match reason {},
        }
    }
    fn blur_format2_scheduled<C>(&mut self, source_argument: i32,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledOutcome<C>, TensorError> {
        let tensor = self.quarter_mut()?;
        let layout = tensor_layout(tensor);
        match image::blur_mapped_scheduled(&mut tensor.bytes, layout, source_argument, checkpoint)
            .map_err(TensorError::Pixels)? {
            None => Ok(ScheduledOutcome::Complete),
            Some((reason, partial)) => Ok(ScheduledOutcome::Superseded { reason,
                prefix: ScheduledPrefix::Blur { source_argument, partial } }),
        }
    }
    fn solid_root_rectangle(&mut self, rectangle: [f32; 4], straight_argb: u32) -> Result<(), TensorError> {
        match rectangle::dispatch(&initial_numeric_state(), rectangle).map_err(TensorError::Path)? {
            rectangle::Dispatch::EmptyClip => Ok(()),
            rectangle::Dispatch::AxisAligned(rectangle) => {
                let tensor = self.quarter_mut()?;
                let bounds = [0, 0, tensor.dimensions[0], tensor.dimensions[1]];
                let layout = tensor_layout(tensor);
                let mut painter = solid::SpanPainter::new(rectangle::premultiply_argb(straight_argb));
                solid::paint_float_clipped(&mut tensor.bytes, &layout, &[bounds], rectangle, &mut painter)
                    .map_err(TensorError::Solid)
            }
            rectangle::Dispatch::Path(path) => self.solid_probe_path(&path, straight_argb),
        }
    }
    fn install_unused_getter_colour(&mut self, straight_argb: u32) -> Result<(), TensorError> {
        if self.phase != Phase::Quarter { return Err(TensorError::Phase); }
        self.installed_unused_getter_colour = Some(straight_argb);
        Ok(())
    }
    fn solid_probe_path(&mut self, path: &path::Path, straight_argb: u32) -> Result<(), TensorError> {
        match self.solid_probe_path_scheduled(path, straight_argb,
            &mut || None::<std::convert::Infallible>)? {
            SolidPathOutcome::Complete => Ok(()),
            SolidPathOutcome::Superseded { reason, .. }
                | SolidPathOutcome::SetupSuperseded { reason, .. } => match reason {},
        }
    }
    fn solid_probe_path_scheduled<C>(&mut self, path: &path::Path, straight_argb: u32,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<SolidPathOutcome<C>, TensorError> {

        let tensor = self.quarter_mut()?;
        let bounds = [0, 0, tensor.dimensions[0], tensor.dimensions[1]];
        let Some(mut table) = image::path_mask_source(bounds, path, initial_numeric_state().affine)
            .map_err(TensorError::Edge)? else { return Ok(SolidPathOutcome::Complete); };
        if let Some(reason) = checkpoint() {
            return Ok(SolidPathOutcome::SetupSuperseded { reason,
                partial: SolidPathSetupPrefix::Mask { table } });
        }
        let clip = region::Rectangles::initial_clip(bounds).map_err(TensorError::Region)?;
        if let Some(reason) = checkpoint() {
            return Ok(SolidPathOutcome::SetupSuperseded { reason,
                partial: SolidPathSetupPrefix::Clip { table, clip } });
        }
        let intersects = table.intersect_rectangles_mut_source(&clip).map_err(TensorError::Edge)?;
        if let Some(reason) = checkpoint() {
            return Ok(SolidPathOutcome::SetupSuperseded { reason,
                partial: SolidPathSetupPrefix::Intersection { table, clip, intersects } });
        }
        if !intersects { return Ok(SolidPathOutcome::Complete); }
        let layout = tensor_layout(tensor);
        let mut painter = solid::SpanPainter::new(rectangle::premultiply_argb(straight_argb));
        match solid::shade_edge_table_format2_retaining_scheduled(&mut tensor.bytes, &layout,
            &table, &mut painter, false, checkpoint).map_err(|failure| TensorError::Solid(
                match failure.cause {
                    solid::EdgePaintCause::Source(error) => error,
                    solid::EdgePaintCause::Coverage(failure) => failure.error,
                }))? {
            solid::ScheduledEdgePaint::Complete(_) => Ok(SolidPathOutcome::Complete),
            solid::ScheduledEdgePaint::Superseded { reason, progress } => Ok(
                SolidPathOutcome::Superseded { reason,
                    partial: SolidPathPrefix { table, clip, painter, progress } }),
        }
    }
    fn resize_quality1_format2(&mut self, resize: &Resize) -> Result<(), TensorError> {
        match self.resize_quality1_format2_scheduled(resize, &mut || None::<std::convert::Infallible>)? {
            ScheduledOutcome::Complete => Ok(()),
            ScheduledOutcome::Superseded { reason, .. } => match reason {},
        }
    }
    fn resize_quality1_format2_scheduled<C>(&mut self, resize: &Resize,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<ScheduledOutcome<C>, TensorError> {
        if self.phase != Phase::Quarter { return Err(TensorError::Phase); }
        let source = self.quarter.as_ref().ok_or(TensorError::Phase)?;
        if source.dimensions != resize.source_dimensions || resize.quality != 1
            || resize.target_dimensions.iter().any(|&n| n <= 0) {
            return Err(TensorError::Dimensions);
        }

        if source.dimensions == resize.target_dimensions { return Err(TensorError::Dimensions); }
        self.phase = Phase::Resizing;
        self.full = Some(image::SoftwareStorage::create_cleared_format2(resize.target_dimensions)
            .map_err(TensorError::Pixels)?);
        let target = self.full.as_mut().ok_or(TensorError::Phase)?;
        let source_layout = tensor_layout(source); let target_layout = tensor_layout(target);
        let bounds = [0, 0, target.dimensions[0], target.dimensions[1]];
        let clip = region::Rectangles::initial_clip(bounds).map_err(TensorError::Region)?;
        match image_draw::dispatch(&initial_numeric_state(), 255, resize.affine, resize.quality) {
            image_draw::Route::NoOp => {},
            image_draw::Route::Affine(combined) => {
                let path = path::rectangle([0.0, 0.0, source.dimensions[0] as f32,
                    source.dimensions[1] as f32]).map_err(TensorError::Path)?;
                let copied = clip.copy_source().map_err(TensorError::Region)?;
                let clipped = copied.path_intersection_source(&path, combined).map_err(TensorError::Edge);
                drop(copied);
                if let Some(table) = clipped? {
                    let mut renderer = sampling::EdgeRenderer::new_source(combined, resize.quality, 255)
                        .map_err(TensorError::Sampling)?;
                    match renderer.shade_source_edge_table_scheduled(&table, &source.bytes, &source_layout,
                        &mut target.bytes, &target_layout, checkpoint).map_err(TensorError::Sampling)? {
                        sampling::ScheduledRows::Complete => {},
                        sampling::ScheduledRows::Superseded { reason, next_ordinal } => return Ok(
                            ScheduledOutcome::Superseded { reason, prefix: ScheduledPrefix::Resize {
                                renderer, table, clip, path, next_ordinal,
                            } }),
                    }
                }
            },
            image_draw::Route::Translation(translation) => {
                if let Some(rectangle) = image_draw::translated_framebuffer_rectangle(
                    source.dimensions, target.dimensions, translation) {
                    let mut table = edge_table::from_image_translation_rectangle(rectangle)
                        .map_err(TensorError::Edge)?;
                    if table.intersect_rectangles_mut_source(&clip).map_err(TensorError::Edge)? {
                        translated::shade_source_edge_table(&table, &source.bytes, &source_layout,
                            &mut target.bytes, &target_layout, translation, 255).map_err(TensorError::Translated)?;
                    }
                }
            },
        }
        self.phase = Phase::Full;
        Ok(ScheduledOutcome::Complete)
    }
    fn multiply_full_mask(&mut self, mask: &geometry::Mask) -> Result<(), TensorError> {
        match self.multiply_full_mask_scheduled(mask, &mut || None::<std::convert::Infallible>)? {
            MultiplyOutcome::Complete => Ok(()),
            MultiplyOutcome::Superseded { reason, .. } => match reason {},
        }
    }
    fn multiply_full_mask_scheduled<C>(&mut self, mask: &geometry::Mask,
        checkpoint: &mut impl FnMut() -> Option<C>) -> Result<MultiplyOutcome<C>, TensorError> {
        if self.phase != Phase::Full { return Err(TensorError::Phase); }
        let target = self.full.as_mut().ok_or(TensorError::Phase)?;
        let layout = tensor_layout(target);
        if let Some((reason, progress)) = image::multiply_mask_scheduled(&mut target.bytes, layout.width,
            layout.height, layout.row_stride, layout.pixel_stride, mask, checkpoint).map_err(TensorError::Pixels)? {
            return Ok(MultiplyOutcome::Superseded { reason, progress });
        }
        self.phase = Phase::Complete;
        Ok(MultiplyOutcome::Complete)
    }
}
