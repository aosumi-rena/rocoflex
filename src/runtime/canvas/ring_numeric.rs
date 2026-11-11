#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub direction: [f32; 2],
    pub argb: u32,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BitmapBoundary {
    pub width: u32,
    pub height: u32,
    pub integer_origin: [i32; 2],
    pub transform_scale: f32,
    pub forward_affine: [f32; 6],
    pub inverse_affine: [f32; 6],
    pub format: BitmapFormat,
    pub allocation_was_cleared: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitmapFormat { SourceFormat2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyDefinedness { RefuseUndefined, RequireBackend }
#[derive(Clone, Copy, Debug)]
pub struct FloatingPointBoundary {
    pub supplied_mxcsr: u32,
    pub separate_operations: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SuppliedExtent {
    pub initial_probe_screen: [f32; 2],
    pub source_cvtt_extent: i32,
    pub transform_scale: f32,
    pub forward_affine: [f32; 6],
    pub inverse_affine: [f32; 6],
}
#[derive(Clone, Copy, Debug)]
pub struct AllocationRequest {
    pub integer_origin: [i32; 2],
    pub width: u32,
    pub height: u32,
    pub transform_scale: f32,
    pub forward_affine: [f32; 6],
    pub inverse_affine: [f32; 6],
}
pub struct Input<'a> {
    pub bitmap: BitmapBoundary,
    pub records: &'a [Record],
    pub empty: EmptyDefinedness,
    pub floating_point: FloatingPointBoundary,
}

pub struct DisplayInput {
    pub diameter: u32,
    pub integer_origin: [i32; 2],
    pub transform_scale: f32,
    pub allocation_was_cleared: bool,
    pub records: Vec<Record>,

    pub empty: EmptyDefinedness,
    pub floating_point: FloatingPointBoundary,
}

pub trait Services {
    type Error;

    fn current_floating_point_boundary(&mut self) -> Result<FloatingPointBoundary, Self::Error>;
    fn multiply_saturation(&mut self, opaque_argb: u32, multiplier: f32) -> Result<u32, Self::Error>;

    fn empty_average_rgb(&mut self) -> Result<[u8; 3], Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    FloatingPointBoundary,
    UnsupportedBitmapExtent,
    UnclearedAllocation,
    NonfiniteTransform,
    TransformIntegerOverflow,
    InvalidEmittedRecord { index: usize },
    InvalidSaturationMultiplier,
    AllocationMismatch,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    Input(InputError),
    PixelOutsideLoop,
    NonfiniteReduction,
    UndefinedEmptyReduction,
    RequiredService(E),
    AlreadyHalted,
    Returned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelOperation {
    Untouched,
    GrayImageSetter { straight_argb: u32 },
    RawFormat2 { premultiplied_argb: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pixel {
    pub x: u32,
    pub y: u32,
    pub operation: PixelOperation,
}
#[derive(Clone, Copy)]
struct LoopParameters {
    diameter: u32,
    center: f32,
    half_integer: f32,
    thickness: f32,
    gray_inner: f32,
    gray_outer: f32,
    ring_inner: f32,
    ring_inner_feather: f32,
    ring_outer: f32,
    ring_outer_feather: f32,
}

fn parameters(width: u32, height: u32, scale: f32, cleared: bool,
    records: &[Record], fp: FloatingPointBoundary,
    native_affines: Option<(&[f32; 6], &[f32; 6])>) -> Result<LoopParameters, InputError>
{
    if fp.supplied_mxcsr & 0xffc0 != 0x1f80 || !fp.separate_operations {
        return Err(InputError::FloatingPointBoundary);
    }

    if height < 2 || height > i32::MAX as u32 || width < height || width > i32::MAX as u32 {
        return Err(InputError::UnsupportedBitmapExtent);
    }
    if !cleared { return Err(InputError::UnclearedAllocation); }
    if !scale.is_finite() || scale <= 0.0
        || native_affines.is_some_and(|(forward, inverse)| forward.iter().chain(inverse).any(|v| !v.is_finite())) {
        return Err(InputError::NonfiniteTransform);
    }
    let twice = scale + scale;
    if !twice.is_finite() || twice < 0.0 || twice >= 2147483648.0 {
        return Err(InputError::TransformIntegerOverflow);
    }
    let t = twice as i32;
    let triple = t.checked_mul(3).ok_or(InputError::TransformIntegerOverflow)?;
    let triple_plus_one = triple.checked_add(1).ok_or(InputError::TransformIntegerOverflow)?;
    for (index, record) in records.iter().enumerate() {

        if record.direction.iter().any(|v| !v.is_finite()) || !record.weight.is_finite()
            || !(f64::from(record.weight) > 0.001) || record.weight > 1.0 {
            return Err(InputError::InvalidEmittedRecord { index });
        }
    }
    let center = height as f32 * 0.5;
    let half_integer = (height >> 1) as f32;
    let thickness = t as f32;
    let ring_inner = half_integer - thickness;
    let ring_outer = thickness + half_integer;
    Ok(LoopParameters {
        diameter: height, center, half_integer, thickness,
        gray_inner: triple as f32, gray_outer: triple_plus_one as f32,
        ring_inner, ring_inner_feather: ring_inner + -1.0,
        ring_outer, ring_outer_feather: ring_outer + -1.0,
    })
}

fn alpha_byte(alpha: f32) -> u8 {
    if 0.0 >= alpha { 0 } else if alpha >= 1.0 { 255 } else {
        let scaled = alpha * 255.0;
        (f64::from(scaled) + 6755399441055744.0).to_bits() as u8
    }
}
fn channels(argb: u32) -> [u8; 3] { [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8] }
fn argb(alpha: u8, rgb: [u8; 3]) -> u32 {
    (u32::from(alpha) << 24) | (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2])
}
fn current_controls<S: Services>(services: &mut S) -> Result<(), Error<S::Error>> {
    let fp = services.current_floating_point_boundary().map_err(Error::RequiredService)?;
    if fp.supplied_mxcsr & 0xffc0 != 0x1f80 || !fp.separate_operations {
        return Err(Error::Input(InputError::FloatingPointBoundary));
    }
    Ok(())
}

pub fn allocation_from_supplied_extent(
    input: SuppliedExtent, fp: FloatingPointBoundary,
) -> Result<AllocationRequest, InputError> {
    if fp.supplied_mxcsr & 0xffc0 != 0x1f80 || !fp.separate_operations {
        return Err(InputError::FloatingPointBoundary);
    }
    if input.initial_probe_screen.iter().chain(&input.forward_affine).chain(&input.inverse_affine)
        .any(|value| !value.is_finite()) || !input.transform_scale.is_finite() || input.transform_scale <= 0.0 {
        return Err(InputError::NonfiniteTransform);
    }
    let [x, y] = input.initial_probe_screen;
    let m = input.forward_affine;
    let yy = m[4] * y;
    let yx = m[3] * x;
    let y_sum = yx + yy;
    let transformed_y = y_sum + m[5];
    let xy = y * m[1];
    let xx = x * m[0];
    let x_sum = xx + xy;
    let transformed_x = x_sum + m[2];
    let rounded = |value: f32| -> Result<i32, InputError> {
        if !value.is_finite() || value < -2147483648.0 || value >= 2147483648.0 {
            return Err(InputError::TransformIntegerOverflow);
        }
        Ok((f64::from(value) + 6755399441055744.0).to_bits() as u32 as i32)
    };

    let cy = rounded(transformed_y)?;
    let cx = rounded(transformed_x)?;
    let r = input.source_cvtt_extent;
    let x_plus = cx.checked_add(r).ok_or(InputError::TransformIntegerOverflow)?;
    let y_plus = cy.checked_add(r).ok_or(InputError::TransformIntegerOverflow)?;
    let x_minus = cx.checked_sub(r).ok_or(InputError::TransformIntegerOverflow)?;
    let y_minus = cy.checked_sub(r).ok_or(InputError::TransformIntegerOverflow)?;
    let dx = x_plus.checked_sub(x_minus).ok_or(InputError::TransformIntegerOverflow)?;
    let dy = y_plus.checked_sub(y_minus).ok_or(InputError::TransformIntegerOverflow)?;
    let width = dx.checked_abs().ok_or(InputError::TransformIntegerOverflow)? as u32;
    let height = dy.checked_abs().ok_or(InputError::TransformIntegerOverflow)? as u32;
    Ok(AllocationRequest { integer_origin: [x_minus.min(x_plus), y_minus.min(y_plus)], width, height,
        transform_scale: input.transform_scale, forward_affine: input.forward_affine, inverse_affine: input.inverse_affine })
}
impl AllocationRequest {

    pub fn accept_format2_allocation(self, actual_width: u32, actual_height: u32, was_cleared: bool)
        -> Result<BitmapBoundary, InputError>
    {
        if actual_width != self.width || actual_height != self.height { return Err(InputError::AllocationMismatch); }
        if !was_cleared { return Err(InputError::UnclearedAllocation); }
        Ok(BitmapBoundary { width: actual_width, height: actual_height, integer_origin: self.integer_origin,
            transform_scale: self.transform_scale, forward_affine: self.forward_affine,
            inverse_affine: self.inverse_affine, format: BitmapFormat::SourceFormat2, allocation_was_cleared: true })
    }
}

pub fn saturation_replacement_input(
    original_argb: u32, multiplier: f32, floating_point: FloatingPointBoundary,
) -> Result<f32, InputError> {
    if floating_point.supplied_mxcsr & 0xffc0 != 0x1f80 || !floating_point.separate_operations {
        return Err(InputError::FloatingPointBoundary);
    }
    if !multiplier.is_finite() || multiplier < 0.0 { return Err(InputError::InvalidSaturationMultiplier); }
    let c = channels(original_argb).map(i32::from);
    let maximum = c[0].max(c[1]).max(c[2]);
    let saturation = if maximum == 0 { 0.0 } else {
        let minimum = c[0].min(c[1]).min(c[2]);
        let delta = maximum - minimum;
        delta as f32 / maximum as f32
    };
    Ok((saturation * multiplier).min(1.0))
}

fn rounded_extent_finite(value: f32) -> f32 {
    let raw = value.to_bits();
    let sign = raw & 0x8000_0000;
    let magnitude = raw & 0x7fff_ffff;
    let exponent = (magnitude >> 23) & 255;
    if exponent >= 150 { return value; }
    if exponent < 126 { return f32::from_bits(sign); }
    if exponent == 126 { return f32::from_bits(sign | 0x3f80_0000); }
    let fractional_bits = 150 - exponent;
    let mask = (1_u32 << fractional_bits) - 1;
    let half = 1_u32 << (fractional_bits - 1);
    let truncated = magnitude & !mask;
    let rounded = if magnitude & mask >= half { truncated + (1_u32 << fractional_bits) } else { truncated };
    f32::from_bits(sign | rounded)
}

pub fn source_extent(
    later_projected_positive_x_edge: f32, initial_screen_probe_x: f32,
    transform_scale: f32, fp: FloatingPointBoundary,
) -> Result<i32, InputError> {
    if fp.supplied_mxcsr & 0xffc0 != 0x1f80 || !fp.separate_operations {
        return Err(InputError::FloatingPointBoundary);
    }
    if !later_projected_positive_x_edge.is_finite() || !initial_screen_probe_x.is_finite()
        || !transform_scale.is_finite() || transform_scale <= 0.0 {
        return Err(InputError::NonfiniteTransform);
    }
    let delta = later_projected_positive_x_edge - initial_screen_probe_x;
    let scaled = delta * transform_scale;
    if !scaled.is_finite() { return Err(InputError::TransformIntegerOverflow); }
    let rounded = rounded_extent_finite(scaled);
    if rounded < -2147483648.0 || rounded >= 2147483648.0 { return Err(InputError::TransformIntegerOverflow); }
    Ok(rounded as i32)
}

pub fn gray_format2_word(straight_argb: u32) -> u32 {
    let alpha = (straight_argb >> 24) as u8;
    let rgb = channels(straight_argb);
    let premultiplied = match alpha {
        0 => [0; 3],
        255 => rgb,
        _ => rgb.map(|c| ((u32::from(alpha) * u32::from(c) + 127) >> 8) as u8),
    };
    argb(alpha, premultiplied)
}

fn pixel<S: Services>(records: &[Record], empty: EmptyDefinedness,
    p: LoopParameters, x: u32, y: u32, services: &mut S, work: &mut usize)
    -> Result<PixelOperation, Error<S::Error>>
{
    if x >= p.diameter || y >= p.diameter { return Err(Error::PixelOutsideLoop); }
    let dx = x as f32 - p.center;
    let nx = dx / p.half_integer;
    let dy = y as f32 - p.center;

    let dy64 = f64::from(dy);
    let dx64 = f64::from(dx);
    let second_square = dy64 * dy64;
    let first_square = dx64 * dx64;
    let squared = second_square + first_square;
    let radius = squared.sqrt() as f32;
    if radius < p.gray_outer {
        let alpha = if p.gray_inner < radius { p.gray_outer - radius } else { 1.0 };
        return Ok(PixelOperation::GrayImageSetter { straight_argb: argb(alpha_byte(alpha), [0xca; 3]) });
    }
    let shifted_radius = radius + p.thickness;
    if p.ring_inner_feather > shifted_radius || shifted_radius > p.ring_outer {
        return Ok(PixelOperation::Untouched);
    }
    let mut weight = 0.0_f32;
    let mut rgb = [0.0_f32; 3];
    let average = if records.is_empty() {
        match empty {
            EmptyDefinedness::RefuseUndefined => return Err(Error::UndefinedEmptyReduction),
            EmptyDefinedness::RequireBackend => {
                let rgb = services.empty_average_rgb().map_err(Error::RequiredService)?;
                current_controls(services)?;
                rgb
            }
        }
    } else {
        let ny = dy / p.half_integer;
        for record in records {

            *work += 1;
            let delta_y = ny - record.direction[1];
            let delta_x = nx - record.direction[0];
            let y_square = delta_y * delta_y;
            let x_square = delta_x * delta_x;
            let sum = x_square + y_square;
            let denominator = sum + f32::from_bits(0x3c23d70a);
            let contribution = record.weight / denominator;
            weight += contribution;
            let c = channels(record.argb);
            let red = f32::from(c[0]) * contribution;
            rgb[0] += red;
            let green = f32::from(c[1]) * contribution;
            rgb[1] += green;
            let blue = f32::from(c[2]) * contribution;
            rgb[2] += blue;
        }
        if !(weight > 0.0) || !weight.is_finite() || rgb.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonfiniteReduction);
        }

        let blue = rgb[2] / weight;
        let green = rgb[1] / weight;
        let red = rgb[0] / weight;
        if [red, green, blue].iter().any(|v| !v.is_finite() || *v < 0.0 || *v >= 2147483648.0) {
            return Err(Error::NonfiniteReduction);
        }
        [(red as i32) as u8, (green as i32) as u8, (blue as i32) as u8]
    };
    let inner = if shifted_radius < p.ring_inner {
        shifted_radius - p.ring_inner_feather
    } else { 1.0 };
    let outer = if p.ring_outer_feather < shifted_radius {
        p.ring_outer - shifted_radius
    } else { 1.0 };
    let alpha = outer * inner;

    let saturated = services.multiply_saturation(argb(255, average), f32::from_bits(0x3fb33333))
        .map_err(Error::RequiredService)?;
    current_controls(services)?;
    if 0.0 >= alpha { return Ok(PixelOperation::RawFormat2 { premultiplied_argb: 0 }); }
    let brightened = channels(saturated).map(|c| {
        let inverted = f32::from(255 - c);
        let multiplied = inverted * f32::from_bits(0xbf2aaaab);
        let result = multiplied + 255.0;
        (result as i32) as u8
    });
    let a = alpha_byte(alpha);
    let premultiplied = match a {
        0 => [0; 3],
        255 => brightened,
        _ => brightened.map(|c| ((u32::from(a) * u32::from(c) + 127) >> 8) as u8),
    };
    Ok(PixelOperation::RawFormat2 { premultiplied_argb: argb(a, premultiplied) })
}

pub struct Prepared<'a, S> {
    input: Input<'a>, cursor: Cursor<S>,
}
pub struct Rejected<'a, S: Services> { pub input: Input<'a>, pub services: S, pub error: Error<S::Error> }
pub struct Stopped<'a, S: Services> { pub pending: Prepared<'a, S>, pub error: Error<S::Error> }
pub struct Parts<'a, S> { pub input: Input<'a>, pub services: S, pub x: u32, pub y: u32, pub halted: bool }

struct Cursor<S> {
    parameters: LoopParameters, services: S, x: u32, y: u32, halted: bool,
    last_work: usize,
}
impl<S: Services> Cursor<S> {
    fn next(&mut self, records: &[Record], empty: EmptyDefinedness)
        -> Result<Pixel, Error<S::Error>>
    {
        self.last_work = 0;
        if self.halted { return Err(Error::AlreadyHalted); }
        if self.finished() { return Err(Error::Returned); }

        self.last_work = 1;
        if let Err(error) = current_controls(&mut self.services) {
            self.halted = true;
            return Err(error);
        }
        match pixel(records, empty, self.parameters, self.x, self.y, &mut self.services, &mut self.last_work) {
            Ok(operation) => {
                let result = Pixel { x: self.x, y: self.y, operation };
                self.y += 1;
                if self.y == self.parameters.diameter { self.y = 0; self.x += 1; }
                Ok(result)
            }
            Err(error) => { self.halted = true; Err(error) }
        }
    }
    fn finished(&self) -> bool { !self.halted && self.x == self.parameters.diameter }
}

impl<'a, S: Services> Prepared<'a, S> {
    pub fn new(input: Input<'a>, mut services: S) -> Result<Self, Rejected<'a, S>> {
        if let Err(error) = current_controls(&mut services) { return Err(Rejected { input, services, error }); }
        let b = input.bitmap;
        match parameters(b.width, b.height, b.transform_scale, b.allocation_was_cleared,
            input.records, input.floating_point, Some((&b.forward_affine, &b.inverse_affine)))
        {
            Ok(parameters) => Ok(Self { input, cursor: Cursor { parameters, services, x: 0, y: 0, halted: false, last_work: 0 } }),
            Err(error) => Err(Rejected { input, services, error: Error::Input(error) }),
        }
    }
    pub fn bitmap_boundary(&self) -> BitmapBoundary { self.input.bitmap }
    pub fn next(mut self) -> Result<(Self, Pixel), Stopped<'a, S>> {
        match self.cursor.next(self.input.records, self.input.empty) {
            Ok(result) => Ok((self, result)),
            Err(error) => Err(Stopped { pending: self, error }),
        }
    }
    pub fn into_finished(self) -> Result<(BitmapBoundary, S), Self> {
        if self.cursor.finished() { Ok((self.input.bitmap, self.cursor.services)) } else { Err(self) }
    }
    pub fn into_parts(self) -> Parts<'a, S> {
        Parts { input: self.input, services: self.cursor.services,
            x: self.cursor.x, y: self.cursor.y, halted: self.cursor.halted }
    }
}

pub struct DisplayPrepared<S> { input: DisplayInput, cursor: Cursor<S> }
pub struct DisplayRejected<S: Services> { pub input: DisplayInput, pub services: S, pub error: Error<S::Error> }
pub struct DisplayStopped<S: Services> { pub pending: DisplayPrepared<S>, pub error: Error<S::Error> }
pub struct DisplayFinished<S> { pub input: DisplayInput, pub services: S }
pub enum DisplayPoll<S: Services> {
    Pending(DisplayPrepared<S>), Ready(DisplayFinished<S>), Failed(DisplayStopped<S>),
}

pub struct DisplayBatch<S: Services> {
    pub state: DisplayPoll<S>, pub pixels: Vec<Pixel>, pub steps: usize,

    pub work_spent: u128,
}

#[derive(Clone, Copy)]
enum DisplayBudget { Pixels(usize), Work(usize) }

impl<S: Services> DisplayPrepared<S> {
    pub fn new(input: DisplayInput, mut services: S) -> Result<Self, DisplayRejected<S>> {
        if let Err(error) = current_controls(&mut services) {
            return Err(DisplayRejected { input, services, error });
        }
        match parameters(input.diameter, input.diameter, input.transform_scale,
            input.allocation_was_cleared, &input.records, input.floating_point, None)
        {
            Ok(parameters) => Ok(Self { input, cursor: Cursor { parameters, services, x: 0, y: 0, halted: false, last_work: 0 } }),
            Err(error) => Err(DisplayRejected { input, services, error: Error::Input(error) }),
        }
    }
    pub fn input(&self) -> &DisplayInput { &self.input }
    pub fn position(&self) -> [u32; 2] { [self.cursor.x, self.cursor.y] }
    pub fn services(&self) -> &S { &self.cursor.services }
    pub fn next(mut self) -> Result<(Self, Pixel), DisplayStopped<S>> {
        match self.cursor.next(&self.input.records, self.input.empty) {
            Ok(result) => Ok((self, result)),
            Err(error) => Err(DisplayStopped { pending: self, error }),
        }
    }
    pub fn into_finished(self) -> Result<DisplayFinished<S>, Self> {
        if self.cursor.finished() { Ok(DisplayFinished { input: self.input, services: self.cursor.services }) }
        else { Err(self) }
    }

    pub fn poll(self, pixel_budget: usize) -> DisplayBatch<S> {
        self.poll_budget(DisplayBudget::Pixels(pixel_budget))
    }

    pub fn poll_work(self, work_budget: usize) -> DisplayBatch<S> {
        self.poll_budget(DisplayBudget::Work(work_budget))
    }
    fn poll_budget(mut self, budget: DisplayBudget) -> DisplayBatch<S> {
        let mut pixels = Vec::new();
        let mut steps = 0;
        let mut work_spent = 0_u128;
        if self.cursor.halted {
            return DisplayBatch { state: DisplayPoll::Failed(DisplayStopped {
                pending: self, error: Error::AlreadyHalted }), pixels, steps, work_spent };
        }
        while !self.cursor.finished() && match budget {
            DisplayBudget::Pixels(limit) => steps < limit,
            DisplayBudget::Work(limit) => work_spent < limit as u128,
        } {
            steps += 1;
            let result = self.cursor.next(&self.input.records, self.input.empty);
            work_spent += self.cursor.last_work as u128;
            match result {
                Ok(pixel) => pixels.push(pixel),
                Err(error) => return DisplayBatch { state: DisplayPoll::Failed(DisplayStopped { pending: self, error }), pixels, steps, work_spent },
            }
        }
        let state = if self.cursor.finished() {
            DisplayPoll::Ready(DisplayFinished { input: self.input, services: self.cursor.services })
        } else { DisplayPoll::Pending(self) };
        DisplayBatch { state, pixels, steps, work_spent }
    }
}

impl PixelOperation {

    pub fn display_premultiplied_rgba(self) -> Option<[u8; 4]> {
        let word = match self {
            Self::Untouched => return None,
            Self::GrayImageSetter { straight_argb } => gray_format2_word(straight_argb),
            Self::RawFormat2 { premultiplied_argb } => premultiplied_argb,
        };
        let [red, green, blue] = channels(word);
        Some([red, green, blue, (word >> 24) as u8])
    }
}
