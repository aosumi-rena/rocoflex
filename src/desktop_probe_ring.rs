use super::*;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[allow(dead_code)]
#[path = "runtime/canvas/ring_numeric.rs"]
mod literal;

pub(super) enum ServiceError {
    Controls,
    Saturation(literal::InputError),
    Empty(source_math::EmptyRingFailure),
}

struct Services {

    empty: Option<source_math::ModeledEmptyRing>,
    colors: std::collections::HashMap<(u32, u32), u32>,

    fp: literal::FloatingPointBoundary,
}

impl literal::Services for Services {
    type Error = ServiceError;
    fn current_floating_point_boundary(&mut self)
        -> std::result::Result<literal::FloatingPointBoundary, Self::Error>
    {
        Ok(self.fp)
    }
    fn multiply_saturation(&mut self, argb: u32, multiplier: f32)
        -> std::result::Result<u32, Self::Error>
    {
        let key = (argb, multiplier.to_bits());
        if let Some(&color) = self.colors.get(&key) { return Ok(color); }
        let color = multiplied_source_saturation(argb, multiplier)?;
        if self.colors.len() >= 16_384 { self.colors.clear(); }
        self.colors.insert(key, color);
        Ok(color)
    }
    fn empty_average_rgb(&mut self) -> std::result::Result<[u8; 3], Self::Error> {
        if let Some(empty) = &self.empty { return Ok(empty.rgb); }
        let fp = floating_point().ok_or(ServiceError::Controls)?;
        let result = source_math::masked_empty_ring_rgb(fp.supplied_mxcsr)
            .map_err(ServiceError::Empty)?;
        let rgb = result.rgb;
        self.empty = Some(result);
        Ok(rgb)
    }
}

pub(super) fn multiplied_source_saturation(argb: u32, multiplier: f32)
    -> std::result::Result<u32, ServiceError>
{
    let fp = floating_point().ok_or(ServiceError::Controls)?;
    let saturation = literal::saturation_replacement_input(argb, multiplier, fp)
        .map_err(ServiceError::Saturation)?;

    let value = paint::math::with_saturation(argb, saturation);
    floating_point().ok_or(ServiceError::Controls)?;
    Ok(value)
}

fn floating_point() -> Option<literal::FloatingPointBoundary> {
    Some(literal::FloatingPointBoundary {
        supplied_mxcsr: layout::context::provider::check_current_thread().ok()?,
        separate_operations: true,
    })
}

#[derive(PartialEq, Eq, Hash)]
struct Key {
    diameter: u32,
    scale: u32,
    maximum_texture_side: usize,
    records: Vec<[u32; 4]>,
}

enum State {
    Pending { owner: literal::DisplayPrepared<Services>, pixels: Vec<Color32> },
    Ready(literal::DisplayFinished<Services>),
    Rejected { owner: literal::DisplayRejected<Services>, pixels: Vec<Color32> },
    Failed { owner: literal::DisplayStopped<Services>, pixels: Vec<Color32> },
    Allocation { input: literal::DisplayInput, error: std::collections::TryReserveError },
    TextureExtent { input: literal::DisplayInput },
    PlacementFailed { owner: literal::DisplayFinished<Services>, image: Arc<egui::ColorImage>, error: PlacementFailure },
}

enum PlacementFailure {
    Controls(layout::context::provider::Error),
    Source(bitmap_placement::Failure),
}

pub(super) struct Rendered {
    pub image: Arc<egui::ColorImage>,
    pub rect: Rect,
    texture: TextureHandle,

    placement: bitmap_placement::Placement,
}

impl Rendered {
    pub fn paint(&self, painter: &egui::Painter) {
        painter.image(self.texture.id(), self.rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }

    pub fn composite(&self, target: &mut [[f32; 3]], width: usize, height: usize, full: Rect) {
        let sx = width as f32 / full.width();
        let sy = height as f32 / full.height();
        let first_x = ((self.rect.left() - full.left()) * sx).floor().clamp(0.0, width as f32) as usize;
        let last_x = ((self.rect.right() - full.left()) * sx).ceil().clamp(0.0, width as f32) as usize;
        let first_y = ((self.rect.top() - full.top()) * sy).floor().clamp(0.0, height as f32) as usize;
        let last_y = ((self.rect.bottom() - full.top()) * sy).ceil().clamp(0.0, height as f32) as usize;
        for y in first_y..last_y {
            for x in first_x..last_x {
                let at = pos2(
                    full.left() + (x as f32 + 0.5) * full.width() / width as f32,
                    full.top() + (y as f32 + 0.5) * full.height() / height as f32,
                );
                if !self.rect.contains(at) { continue; }
                let sx = (((at.x - self.rect.left()) / self.rect.width())
                    * self.image.size[0] as f32) as usize;
                let sy = (((at.y - self.rect.top()) / self.rect.height())
                    * self.image.size[1] as f32) as usize;
                if sx >= self.image.size[0] || sy >= self.image.size[1] { continue; }
                let source = self.image.pixels[sy * self.image.size[0] + sx];
                let inverse_alpha = 1.0 - f32::from(source.a()) / 255.0;
                let destination = &mut target[y * width + x];
                for (channel, value) in [source.r(), source.g(), source.b()].into_iter().enumerate() {
                    destination[channel] = f32::from(value) + destination[channel] * inverse_alpha;
                }
            }
        }
    }
}

pub(super) struct Cached {
    key: Key,
    rect: Rect,
    state: Option<State>,
    rendered: Option<Rendered>,
    admitted: bool,
    fingerprint: u64,
    texture: Option<TextureHandle>,
    placement_failure: Option<PlacementFailure>,
}

impl Cached {
    pub fn rendered(&self) -> Option<&Rendered> {
        if self.admitted { self.rendered.as_ref() } else { None }
    }

    pub fn publication_fingerprint(&self) -> u64 {
        self.fingerprint ^ if self.rendered().is_some() { 0x5049_5845_4c52_494e } else { 0 }
    }

    pub fn update(
        slot: &mut Option<Self>, ctx: &egui::Context,
        frame: &paint::Frame, center: Pos2, radius: f32,
    ) {
        let Some(fp) = floating_point() else {

            if let Some(cache) = slot.as_mut() { cache.admitted = false; }
            return;
        };
        let scale = ctx.pixels_per_point();
        let extent = (radius * scale).round();
        let rounded_center = source_menu::rounded_display_center(
            [center.x * scale, center.y * scale], fp.supplied_mxcsr,
        );
        if !scale.is_finite() || scale <= 0.0 || !extent.is_finite()
            || extent < 1.0 || extent >= 1_073_741_824.0 || rounded_center.is_none()
        {
            *slot = None;
            return;
        }
        let extent = extent as i32;
        let center = rounded_center.unwrap();
        let (Some(x), Some(y)) = (center[0].checked_sub(extent), center[1].checked_sub(extent))
            else { *slot = None; return; };
        let diameter = (extent as u32) * 2;
        let maximum_texture_side = ctx.input(|input| input.max_texture_side);
        let same_raster = slot.as_ref().is_some_and(|cache|
            cache.key.diameter == diameter && cache.key.scale == scale.to_bits()
                && cache.key.maximum_texture_side == maximum_texture_side
                && cache.key.records.iter().copied().eq(frame.nodes.iter()
                    .filter_map(|node| node.command.ring_record)
                    .map(|record| [record.direction[0].to_bits(), record.direction[1].to_bits(),
                        record.argb, record.weight.to_bits()])));
        let rect = Rect::from_min_size(pos2(x as f32 / scale, y as f32 / scale),
            vec2(diameter as f32 / scale, diameter as f32 / scale));
        if !same_raster {
            if !rect.min.x.is_finite() || !rect.min.y.is_finite()
                || !rect.max.x.is_finite() || !rect.max.y.is_finite()
                || rect.width() <= 0.0 || rect.height() <= 0.0
            {
                *slot = None;
                return;
            }
            let records: Vec<_> = frame.nodes.iter().filter_map(|node| node.command.ring_record)
                .map(|record| literal::Record {
                    direction: record.direction, argb: record.argb, weight: record.weight,
                }).collect();
            let key = Key { diameter, scale: scale.to_bits(), maximum_texture_side,
                records: records.iter().map(|record| [record.direction[0].to_bits(),
                    record.direction[1].to_bits(), record.argb, record.weight.to_bits()]).collect() };
            let previous = slot.as_mut().and_then(|cache| cache.rendered.take());
            let colors = match slot.as_mut().and_then(|cache| cache.state.take()) {
                Some(State::Ready(owner)) => owner.services.colors,
                _ => std::collections::HashMap::new(),
            };
            let (mut pixels, texture) = match previous {
                Some(rendered) => (Arc::try_unwrap(rendered.image)
                    .map(|image| image.pixels).unwrap_or_default(), Some(rendered.texture)),
                None => (Vec::new(), slot.as_mut().and_then(|cache| cache.texture.take())),
            };
            let mut input = literal::DisplayInput {
                diameter, integer_origin: [x, y], transform_scale: scale,
                allocation_was_cleared: false, records,

                empty: literal::EmptyDefinedness::RequireBackend, floating_point: fp,
            };
            let state = if diameter as usize > key.maximum_texture_side {
                State::TextureExtent { input }
            } else {
                let area = (diameter as usize).checked_mul(diameter as usize);
                pixels.clear();
                match pixels.try_reserve_exact(area.unwrap_or(usize::MAX)) {
                    Err(error) => State::Allocation { input, error },
                    Ok(()) => {
                        pixels.resize(area.unwrap(), Color32::TRANSPARENT);
                        input.allocation_was_cleared = true;
                        match literal::DisplayPrepared::new(input, Services { empty: None,
                            colors, fp }) {
                            Ok(owner) => State::Pending { owner, pixels },
                            Err(owner) => State::Rejected { owner, pixels },
                        }
                    }
                }
            };
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hash);
            let fingerprint = hash.finish();
            *slot = Some(Self {
                key, rect, state: Some(state), rendered: None, admitted: true, fingerprint,
                texture, placement_failure: None,
            });
        }
        let Some(cache) = slot.as_mut() else { return; };
        cache.admitted = true;
        if cache.rect != rect {
            cache.rect = rect;
            if let Some(rendered) = &mut cache.rendered {
                match bitmap_placement::for_image(rendered.image.size, rect, fp.supplied_mxcsr, 0,
                    crate::source_scalar::ScalarF32Architecture::IntelSdm) {
                    Ok(placement) => {
                        rendered.rect = placement.rect();
                        rendered.placement = placement;
                        cache.placement_failure = None;
                    }
                    Err(error) => cache.placement_failure = Some(PlacementFailure::Source(error)),
                }
            }
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            cache.key.hash(&mut hash);
            x.hash(&mut hash); y.hash(&mut hash);
            cache.fingerprint = hash.finish();
        }
        if cache.placement_failure.is_some() { cache.admitted = false; return; }
        if !matches!(cache.state.as_ref(), Some(State::Pending { .. })) { return; }
        let Some(State::Pending { mut owner, mut pixels }) = cache.state.take() else { return; };

        while owner.position()[0] < cache.key.diameter {
            let pixel = match owner.next() {
                Ok((next, pixel)) => { owner = next; pixel }
                Err(owner) => { cache.state = Some(State::Failed { owner, pixels }); return; }
            };
            if let Some([r, g, b, a]) = pixel.operation.display_premultiplied_rgba() {
                pixels[pixel.y as usize * cache.key.diameter as usize + pixel.x as usize] =
                    Color32::from_rgba_premultiplied(r, g, b, a);
            }
        }
        cache.state = Some(match owner.into_finished() {
            Err(owner) => State::Pending { owner, pixels },
            Ok(owner) => {
                let side = cache.key.diameter as usize;
                let image = Arc::new(egui::ColorImage::new([side, side], pixels));

                let placement = layout::context::provider::check_current_thread()
                    .map_err(PlacementFailure::Controls)
                    .and_then(|mxcsr| bitmap_placement::for_image(image.size, cache.rect, mxcsr, 0,
                        crate::source_scalar::ScalarF32Architecture::IntelSdm).map_err(PlacementFailure::Source));
                match placement {
                    Ok(placement) => {
                        let rect = placement.rect();
                        let texture = match cache.texture.take() {
                            Some(mut texture) => {
                                if texture.size() == image.size {
                                    texture.set_partial([0, 0], image.clone(), egui::TextureOptions::NEAREST);
                                } else { texture.set(image.clone(), egui::TextureOptions::NEAREST); }
                                texture
                            }
                            None => ctx.load_texture("probe-ring", image.clone(), egui::TextureOptions::NEAREST),
                        };
                        cache.rendered = Some(Rendered { image, texture, rect, placement });
                        State::Ready(owner)
                    }
                    Err(error) => {
                        cache.rendered = None;
                        State::PlacementFailed { owner, image, error }
                    }
                }
            }
        });
    }
}
