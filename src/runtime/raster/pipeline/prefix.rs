#[derive(Debug)]
pub enum Error { Domain, GradientNonProgress, PaletteReadCount, ArithmeticMode(u32),
    RequiredExpBackend, InternalState }

#[derive(Clone, Copy)]
pub struct PaletteReads<'a> {
    pub gradient_initial: u32, pub gradient_insertions: &'a [u32],
    pub getter_marker: u32, pub probe_marker: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Stop { pub position: f64, pub argb: u32 }
#[derive(Debug)]
pub struct Gradient {
    pub start: [f32; 2], pub end: [f32; 2], pub radial: bool,
    pub initial: [Stop; 2],

    pub insertions: Vec<Stop>,
}
#[derive(Debug)]
pub struct Root { pub argb: u32, pub displaced_position: [f32; 2] }
#[derive(Debug)]
pub struct Rectangle { pub argb: u32, pub rectangle: [f32; 4] }
#[derive(Debug)]
pub struct Commands {
    pub quarter_dimensions: [i32; 2], pub clear_argb: u32, pub gradient: Gradient,
    pub root_squares: Vec<Rectangle>, pub getter_rectangles: Vec<[f32; 4]>,
    pub getter_argb: u32, pub probe_ellipse: Rectangle,
}
#[link(name = "ucrt")]
unsafe extern "C" { fn expf(x: f32) -> f32; }
pub const RADIAL_EXP_UPPER_F64: u64 = 0x8000000080000000;
#[derive(Clone, Copy, Debug)]
pub struct ExpRequest { pub argument: u32, pub upper_f64: u64 }
pub trait ExpServices {

    fn radial_exp(&mut self, request: ExpRequest) -> Result<f32, ()>;
}

pub struct HostRadialExp;
impl ExpServices for HostRadialExp {
    fn radial_exp(&mut self, request: ExpRequest) -> Result<f32, ()> {

        Ok(unsafe { expf(f32::from_bits(request.argument)) })
    }
}
fn current_alpha_mode() -> Result<(), Error> {
    let mut mxcsr = 0u32;
    unsafe { std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr,
        options(nostack, preserves_flags)); }
    if mxcsr & 0xffc0 == 0x1f80 { Ok(()) } else { Err(Error::ArithmeticMode(mxcsr)) }
}
fn flip(v: f32) -> f32 { f32::from_bits(v.to_bits() ^ 0x80000000) }
fn minss(a: f32, b: f32) -> f32 { if a < b { a } else { b } }
fn source_rect(x: f32, y: f32, half: f32) -> Result<[f32; 4], Error> {
    let xp = x + half; let yp = y + half;
    let xm = x + flip(half); let ym = y + flip(half);
    let mut width = xm - xp; let mut height = ym - yp;

    if 0.0 > width { width = flip(width); }
    if 0.0 > height { height = flip(height); }
    let rectangle = [minss(xp, xm), minss(yp, ym), width, height];
    if rectangle.iter().any(|x| !x.is_finite()) { return Err(Error::Domain); }
    Ok(rectangle)
}
fn alpha_bits(alpha: f32) -> Result<u32, Error> {
    if !alpha.is_finite() { return Err(Error::Domain); }
    if alpha <= 0.0 { return Ok(0); }
    if alpha >= 1.0 { return Ok(0xff000000); }
    let scaled = alpha * 255.0;

    let rounded = (scaled as f64 + 6755399441055744.0).to_bits() as u32;
    Ok(rounded.wrapping_shl(24))
}

#[derive(Clone, Copy, Debug)]
pub struct QuarterProjection {
    dimensions: [i32; 2], center: [f32; 2], scale: f32,
    probe: [f32; 2], pixel_probe: [f32; 2], saved_zoom: f32,
}
impl QuarterProjection {
    pub fn new(rectangle: [i32; 4], probe: [f32; 2], saved_zoom: f32) -> Result<Self, Error> {
        if rectangle[2] <= 0 || rectangle[3] <= 0 || !saved_zoom.is_finite()
            || saved_zoom <= 0.0 || probe.iter().any(|x| !x.is_finite()) { return Err(Error::Domain); }
        let dimensions = [(rectangle[2] / 4).max(0), (rectangle[3] / 4).max(0)];
        if dimensions.contains(&0) { return Err(Error::Domain); }
        let center = [dimensions[0] as f32 * 0.5 + rectangle[0] as f32,
            dimensions[1] as f32 * 0.5 + rectangle[1] as f32];
        let scale = dimensions[0].min(dimensions[1]) as f32 * 0.5;
        let pixel_probe = [(scale * probe[0]) * saved_zoom + center[0],
            center[1] - (probe[1] * scale) * saved_zoom];
        if pixel_probe.iter().any(|x| !x.is_finite()) { return Err(Error::Domain); }
        Ok(Self { dimensions, center, scale, probe, pixel_probe, saved_zoom })
    }
    pub fn dimensions(&self) -> [i32; 2] { self.dimensions }
    pub fn initial_gradient(&self, argb: u32) -> Gradient {
        let [px, py] = self.pixel_probe; let rgb = argb & 0xffffff;
        Gradient { start: [px, py], end: [px + 250.0, py + 0.0], radial: true,
            initial: [Stop { position: 0.0, argb: rgb | 0x80000000 },
                Stop { position: 1.0, argb: rgb | 0x05000000 }], insertions: Vec::new() }
    }
    pub fn pixel_radius(&self, radius: f32) -> Result<f32, Error> {
        let projected = (((radius + self.probe[0]) * self.scale) * self.saved_zoom
            + self.center[0]) - self.pixel_probe[0];
        if !radius.is_finite() || !projected.is_finite() { return Err(Error::Domain); }
        Ok(projected)
    }
    pub fn world_rectangle(&self, xy: [f32; 2], half: f32) -> Result<[f32; 4], Error> {
        let x = (xy[0] * self.scale) * self.saved_zoom + self.center[0];
        let y = (xy[1] * flip(self.scale)) * self.saved_zoom + self.center[1];
        source_rect(x, y, half)
    }
    pub fn probe_rectangle(&self, radius: f32) -> Result<[f32; 4], Error> {
        self.probe_rectangle_from_saved_radius(self.pixel_radius(radius)?)
    }
    pub fn probe_rectangle_from_saved_radius(&self, pixel_radius: f32) -> Result<[f32; 4], Error> {
        source_rect(self.pixel_probe[0], self.pixel_probe[1], pixel_radius)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RadialWalk { denominator: f32, reference: f32, position: f32, refused: bool }
#[derive(Clone, Copy, Debug)]
pub enum AlphaPhase { Inputs, FirstExp, SecondExp, Compose, AlphaBits, Complete }
#[derive(Debug)]
pub struct AlphaProgress {
    pub phase: AlphaPhase,
    pub ratios: Option<[f32; 2]>,
    pub requests: [Option<ExpRequest>; 2],
    pub results: [Option<f32>; 2],
    pub alpha: Option<f32>,
    pub bits: Option<u32>,
}
#[derive(Debug)]
pub enum AlphaCause { Source(Error), RequiredExp(ExpRequest) }
#[derive(Debug)]
#[must_use = "retain current walk and first/second exp progress; no retry"]
pub struct AlphaFailure { pub cause: AlphaCause, pub walk: RadialWalk, pub partial: AlphaProgress }
impl AlphaFailure {
    fn into_host_error(self) -> Error {
        match self.cause {
            AlphaCause::Source(error) => error,
            AlphaCause::RequiredExp(_) => Error::RequiredExpBackend,
        }
    }
}
impl RadialWalk {

    pub fn pending_position(&self) -> Option<f32> {
        if !self.refused && 1000.0 > self.position { Some(self.position) } else { None }
    }
    pub fn new(radius: f32, pixel_radius: f32) -> Result<Self, Error> {
        if !radius.is_finite() || !pixel_radius.is_finite() { return Err(Error::Domain); }
        let denominator = 1000.0 * radius + 10.0;
        let reference = 4.0 * pixel_radius;
        Ok(Self { denominator, reference, position: reference, refused: false })
    }
    pub fn current_alpha_bits(&self) -> Result<Option<u32>, Error> {
        self.current_alpha_with_exp(&mut HostRadialExp).map_err(AlphaFailure::into_host_error)
    }
    pub fn current_alpha_with_exp(&self, service: &mut impl ExpServices)
        -> Result<Option<u32>, AlphaFailure>
    {
        let mut partial = AlphaProgress { phase: AlphaPhase::Inputs, ratios: None,
            requests: [None; 2], results: [None; 2], alpha: None, bits: None };
        let result = (|| -> Result<Option<u32>, AlphaCause> {
            if self.refused { return Err(AlphaCause::Source(Error::GradientNonProgress)); }
            current_alpha_mode().map_err(AlphaCause::Source)?;
            if !(1000.0 > self.position) {
                partial.phase = AlphaPhase::Complete;
                return Ok(None);
            }

            let first = (self.reference - self.position) / self.denominator;
            let second = (50.0 - self.position) / 50.0;
            partial.ratios = Some([first, second]);
            let first_request = ExpRequest { argument: (flip(first) * first).to_bits(),
                upper_f64: RADIAL_EXP_UPPER_F64 };
            partial.phase = AlphaPhase::FirstExp;
            partial.requests[0] = Some(first_request);
            let first_exp = service.radial_exp(first_request)
                .map_err(|_| AlphaCause::RequiredExp(first_request))?;
            partial.results[0] = Some(first_exp);
            current_alpha_mode().map_err(AlphaCause::Source)?;
            let second_request = ExpRequest { argument: (flip(second) * second).to_bits(),
                upper_f64: RADIAL_EXP_UPPER_F64 };
            partial.phase = AlphaPhase::SecondExp;
            partial.requests[1] = Some(second_request);
            let second_exp = service.radial_exp(second_request)
                .map_err(|_| AlphaCause::RequiredExp(second_request))?;
            partial.results[1] = Some(second_exp);
            current_alpha_mode().map_err(AlphaCause::Source)?;
            partial.phase = AlphaPhase::Compose;
            let alpha = (first_exp * f32::from_bits(0x3e19999a)
                + second_exp * f32::from_bits(0x3ea8f5c3)) + f32::from_bits(0x3ca3d70a);
            partial.alpha = Some(alpha);
            partial.phase = AlphaPhase::AlphaBits;
            let bits = alpha_bits(alpha).map_err(AlphaCause::Source)?;
            partial.bits = Some(bits);
            partial.phase = AlphaPhase::Complete;
            Ok(Some(bits))
        })();
        match result {
            Ok(value) => Ok(value),
            Err(cause) => Err(AlphaFailure { cause, walk: *self, partial }),
        }
    }
    pub fn current_key(&self) -> f64 { (self.position / 1000.0) as f64 }
    pub fn advance_after_insert(&mut self) -> Result<(), Error> {
        let next = self.position * 1.25;
        if !next.is_finite() || !(next > self.position) {
            self.refused = true; return Err(Error::GradientNonProgress);
        }
        self.position = next;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub enum CommandPhase { Inputs, Projection, InitialGradient, PixelRadius, Walk,
    Alpha, Palette, Insert, Advance, PaletteCount, RootForward { pass: usize, index: usize },
    RootReverse { pass: usize, index: usize }, Getter(usize), Probe, Complete }
#[derive(Debug)]
pub struct CommandProgress {
    pub phase: CommandPhase,
    pub frame: Option<QuarterProjection>,
    pub pixel_radius: Option<f32>,
    pub walk: Option<RadialWalk>,
    pub gradient: Option<Gradient>,
    pub root_squares: Vec<Rectangle>,
    pub getter_rectangles: Vec<[f32; 4]>,
    pub probe_ellipse: Option<Rectangle>,
}
#[derive(Debug)]
pub enum CommandCause { Source(Error), Alpha(AlphaFailure), MissingState }
#[derive(Debug)]
#[must_use = "retain the advanced command/gradient prefix and required exp owner"]
pub struct CommandFailure { pub cause: CommandCause, pub partial: CommandProgress }
impl CommandFailure {
    fn into_host_error(self) -> Error {
        match self.cause {
            CommandCause::Source(error) => error,
            CommandCause::Alpha(failure) => failure.into_host_error(),
            CommandCause::MissingState => Error::InternalState,
        }
    }
}

pub fn commands(rectangle: [i32; 4], probe: [f32; 2], radius: f32,
                saved_initial_zoom: f32, roots: &[Root], getters: &[[f32; 2]],
                palettes: PaletteReads<'_>) -> Result<Commands, Error> {
    commands_with_exp(rectangle, probe, radius, saved_initial_zoom, roots, getters,
        palettes, &mut HostRadialExp).map_err(CommandFailure::into_host_error)
}

pub fn commands_with_exp(rectangle: [i32; 4], probe: [f32; 2], radius: f32,
    saved_initial_zoom: f32, roots: &[Root], getters: &[[f32; 2]],
    palettes: PaletteReads<'_>, service: &mut impl ExpServices)
    -> Result<Commands, CommandFailure>
{
    let mut partial = CommandProgress { phase: CommandPhase::Inputs, frame: None,
        pixel_radius: None, walk: None, gradient: None, root_squares: Vec::new(),
        getter_rectangles: Vec::new(), probe_ellipse: None };
    let result = (|| -> Result<Commands, CommandCause> {
        if rectangle[2] <= 0 || rectangle[3] <= 0 || !saved_initial_zoom.is_finite()
            || saved_initial_zoom <= 0.0 || !radius.is_finite()
            || probe.iter().any(|x| !x.is_finite())
            || roots.iter().any(|r| r.displaced_position.iter().any(|x| !x.is_finite()))
            || getters.iter().flatten().any(|x| !x.is_finite()) {
            return Err(CommandCause::Source(Error::Domain));
        }
        partial.phase = CommandPhase::Projection;
        let frame = QuarterProjection::new(rectangle, probe, saved_initial_zoom).map_err(CommandCause::Source)?;
        partial.frame = Some(frame);
        partial.phase = CommandPhase::InitialGradient;
        partial.gradient = Some(frame.initial_gradient(palettes.gradient_initial));
        partial.phase = CommandPhase::PixelRadius;
        let pixel_radius = frame.pixel_radius(radius).map_err(CommandCause::Source)?;
        partial.pixel_radius = Some(pixel_radius);
        partial.phase = CommandPhase::Walk;
        partial.walk = Some(RadialWalk::new(radius, pixel_radius).map_err(CommandCause::Source)?);
        loop {
            partial.phase = CommandPhase::Alpha;
            let alpha = partial.walk.as_ref().ok_or(CommandCause::MissingState)?
                .current_alpha_with_exp(service).map_err(CommandCause::Alpha)?;
            let Some(alpha) = alpha else { break; };
            partial.phase = CommandPhase::Palette;
            let insertion_index = partial.gradient.as_ref().ok_or(CommandCause::MissingState)?.insertions.len();
            let rgb = *palettes.gradient_insertions.get(insertion_index)
                .ok_or(CommandCause::Source(Error::PaletteReadCount))? & 0xffffff;

            let position = partial.walk.as_ref().ok_or(CommandCause::MissingState)?.current_key();
            partial.phase = CommandPhase::Insert;
            partial.gradient.as_mut().ok_or(CommandCause::MissingState)?.insertions
                .push(Stop { position, argb: rgb | alpha });
            partial.phase = CommandPhase::Advance;
            partial.walk.as_mut().ok_or(CommandCause::MissingState)?
                .advance_after_insert().map_err(CommandCause::Source)?;
        }
        partial.phase = CommandPhase::PaletteCount;
        if partial.gradient.as_ref().ok_or(CommandCause::MissingState)?.insertions.len()
            != palettes.gradient_insertions.len() {
            return Err(CommandCause::Source(Error::PaletteReadCount));
        }
        for pass in 0..5 {
            for (index, root) in roots.iter().enumerate() {
                partial.phase = CommandPhase::RootForward { pass, index };
                let rectangle = frame.world_rectangle(root.displaced_position, 8.0).map_err(CommandCause::Source)?;
                partial.root_squares.push(Rectangle { argb: (root.argb & 0xffffff) | 0x33000000, rectangle });
            }
            for (index, root) in roots.iter().enumerate().rev() {
                partial.phase = CommandPhase::RootReverse { pass, index };
                let rectangle = frame.world_rectangle(root.displaced_position, 8.0).map_err(CommandCause::Source)?;
                partial.root_squares.push(Rectangle { argb: (root.argb & 0xffffff) | 0x33000000, rectangle });
            }
        }
        for (index, &xy) in getters.iter().enumerate() {
            partial.phase = CommandPhase::Getter(index);
            partial.getter_rectangles.push(frame.world_rectangle(xy, 2.0).map_err(CommandCause::Source)?);
        }
        partial.phase = CommandPhase::Probe;
        partial.probe_ellipse = Some(Rectangle {
            argb: (palettes.probe_marker & 0xffffff) | 0x40000000,
            rectangle: frame.probe_rectangle_from_saved_radius(pixel_radius).map_err(CommandCause::Source)?,
        });
        let gradient = partial.gradient.take().ok_or(CommandCause::MissingState)?;
        let probe_ellipse = partial.probe_ellipse.take().ok_or(CommandCause::MissingState)?;
        partial.phase = CommandPhase::Complete;
        Ok(Commands {
            quarter_dimensions: frame.dimensions(), clear_argb: 0x98d0d0ff, gradient,
            root_squares: std::mem::take(&mut partial.root_squares),
            getter_rectangles: std::mem::take(&mut partial.getter_rectangles),
            getter_argb: (palettes.getter_marker & 0xffffff) | 0x1a000000,
            probe_ellipse,
        })
    })();
    match result {
        Ok(commands) => Ok(commands),
        Err(cause) => Err(CommandFailure { cause, partial }),
    }
}
