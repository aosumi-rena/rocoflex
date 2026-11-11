use super::prefix::{Gradient as Commands, Stop};

#[derive(Debug)]
pub enum Error { Count, Capacity, Allocation, MissingGradient }

pub struct Gradient {
    pub start: [f32; 2], pub end: [f32; 2], pub radial: bool,
    stops: Vec<Stop>, source_capacity: i32,
}
impl Gradient {

    pub fn from_initial(commands: &Commands) -> Result<Self, Error> {
        let mut stops = Vec::new();
        stops.try_reserve_exact(8).map_err(|_| Error::Allocation)?;
        stops.extend_from_slice(&commands.initial);
        Ok(Self { start: commands.start, end: commands.end,
            radial: commands.radial, stops, source_capacity: 8 })
    }
    pub fn stops(&self) -> &[Stop] { &self.stops }
    pub fn source_capacity(&self) -> i32 { self.source_capacity }

    fn ensure_slot(&mut self) -> Result<(), Error> {
        let count = i32::try_from(self.stops.len()).map_err(|_| Error::Count)?;
        if self.source_capacity > count { return Ok(()); }

        let mut rounded = count.wrapping_add(1);
        let carry = ((rounded as u32) >> 31) as i32;
        rounded = rounded.wrapping_add(carry) >> 1;
        rounded = rounded.wrapping_add(count).wrapping_add(1) & !7;
        let capacity = rounded.wrapping_add(8);
        if rounded < -7 || capacity <= count || capacity <= 0 {
            return Err(Error::Capacity);
        }
        let capacity = usize::try_from(capacity).map_err(|_| Error::Capacity)?;
        if capacity.checked_mul(16).is_none() { return Err(Error::Capacity); }
        self.stops.try_reserve_exact(capacity - self.stops.len())
            .map_err(|_| Error::Allocation)?;
        self.source_capacity = capacity as i32;
        Ok(())
    }

    pub fn insert(&mut self, position: f64, argb: u32) -> Result<usize, Error> {

        if 0.0 >= position {
            if let Some(first) = self.stops.first_mut() {
                first.position = 0.0;
                first.argb = argb;
                return Ok(0);
            }
            self.ensure_slot()?;
            self.stops.push(Stop { position: 0.0, argb });
            return Ok(0);
        }

        let normalized = if position < 1.0 { position } else { 1.0 };
        let mut at = 0;

        while at < self.stops.len() && !(self.stops[at].position > normalized) { at += 1; }
        self.ensure_slot()?;
        self.stops.insert(at, Stop { position: normalized, argb });
        Ok(at)
    }

    pub fn copy_for_brush(&self) -> Result<Self, Error> {

        let count = i32::try_from(self.stops.len()).map_err(|_| Error::Count)?;
        let source_capacity = if count == 0 { 0 } else {
            let rounded = (count as u32).wrapping_add((count as u32) >> 1) & 0x7ffffff8;
            let capacity = rounded.wrapping_add(8);
            if capacity > i32::MAX as u32 || capacity < count as u32 {
                return Err(Error::Capacity);
            }
            capacity as i32
        };
        let mut stops = Vec::new();
        stops.try_reserve_exact(source_capacity as usize).map_err(|_| Error::Allocation)?;
        stops.extend_from_slice(&self.stops);
        Ok(Self { start: self.start, end: self.end, radial: self.radial,
            stops, source_capacity })
    }
}

pub struct BrushPayload {
    pub argb: u32, pub gradient: Option<Gradient>, pub affine: [f32; 6],
}
impl BrushPayload {
    pub fn from_gradient(gradient: &Gradient) -> Result<Self, Error> {
        Ok(Self { argb: 0xff000000, gradient: Some(gradient.copy_for_brush()?),
            affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] })
    }

    pub fn solid(argb: u32) -> Self {
        Self { argb, gradient: None, affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] }
    }

    pub fn assign_null_image_source(&mut self, source: &Self) -> Result<(), Error> {
        if core::ptr::eq(self, source) { return Ok(()); }
        self.argb = source.argb;
        let replacement = source.gradient.as_ref().map(Gradient::copy_for_brush).transpose()?;
        let previous = core::mem::replace(&mut self.gradient, replacement);
        drop(previous);
        self.affine = source.affine;
        Ok(())
    }
}

pub struct PreparedRender {
    pub gradient: Gradient, pub affine: [f32; 6], pub source_translation_flag: bool,
}

pub fn prepare_render(brush: &BrushPayload, state: &super::rectangle::StateBoundary)
    -> Result<PreparedRender, Error> {
    let mut gradient = brush.gradient.as_ref().ok_or(Error::MissingGradient)?.copy_for_brush()?;
    let factor = (brush.argb >> 24) as f32 / 255.0;

    for stop in &mut gradient.stops {
        let scaled = (stop.argb >> 24) as f32 * factor;
        let rounded = (scaled as f64 + 6755399441055744.0).to_bits() as u32 as i32;
        let alpha = rounded.min(255) as u8;
        stop.argb = (stop.argb & 0x00ffffff) | ((alpha as u32) << 24);
    }
    let [ba, bb, btx, bc, bd, bty] = brush.affine;
    let [a, b, mut tx, c, d, mut ty] = if state.flags_28_29[0] != 0 {
        [ba, bb, state.integer_offset[0] as f32 + btx,
         bc, bd, state.integer_offset[1] as f32 + bty]
    } else {
        let [sa, sb, stx, sc, sd, sty] = state.affine;
        [(sa * ba) + (sb * bc), (sa * bb) + (sb * bd),
         ((sa * btx) + (sb * bty)) + stx,
         (ba * sc) + (bc * sd), (bb * sc) + (bd * sd),
         ((sc * btx) + (sd * bty)) + sty]
    };
    tx = tx + -0.5;
    ty = ty + -0.5;
    let translation = c == 0.0 && b == 0.0 && a == 1.0 && d == 1.0;
    let affine;
    if translation {
        let [sx, sy] = gradient.start; let [ex, ey] = gradient.end;
        gradient.start = [((b * sy) + sx) + tx, ((sx * c) + sy) + ty];
        gradient.end = [((b * ey) + ex) + tx, ((c * ex) + ey) + ty];
        affine = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    } else { affine = [a, b, tx, c, d, ty]; }
    Ok(PreparedRender { gradient, affine, source_translation_flag: translation })
}
