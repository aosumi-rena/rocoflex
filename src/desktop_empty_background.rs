use super::*;
use std::{hash::{Hash, Hasher}, sync::{Arc, atomic::{AtomicBool, Ordering}}};

#[allow(dead_code)]
#[path = "runtime/raster/empty_display.rs"]
pub(super) mod literal;

#[derive(Clone)]
pub(super) struct Lineage(Arc<AtomicBool>);
impl Default for Lineage {
    fn default() -> Self { Self(Arc::new(AtomicBool::new(true))) }
}
impl PartialEq for Lineage {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) }
}
impl Eq for Lineage {}
impl Hash for Lineage {
    fn hash<H: Hasher>(&self, state: &mut H) { Arc::as_ptr(&self.0).hash(state); }
}
impl Lineage {
    pub fn retire(&self) { self.0.store(false, Ordering::Release); }
    fn current(&self) -> bool { self.0.load(Ordering::Acquire) }
}

#[derive(Clone)]
struct Key {
    lineage: Lineage,
    dimensions: [i32; 2],
    probe: [u32; 2],
    radius: u32,
    zoom: u32,
    geometry: Option<Arc<literal::Geometry>>,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.lineage == other.lineage && self.dimensions == other.dimensions
            && self.probe == other.probe && self.radius == other.radius && self.zoom == other.zoom
            && match (&self.geometry, &other.geometry) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}
impl Eq for Key {}

pub(super) struct Rendered {
    key: Key,
    image: Arc<egui::ColorImage>,
    texture: egui::TextureId,
}
impl Rendered {
    pub fn paint(&self, painter: &egui::Painter, rect: Rect) {
        painter.image(self.texture, rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }

    pub fn over_black(&self, target: &mut [[f32; 3]], width: usize, height: usize) {
        for y in 0..height {
            let sy = y * self.image.size[1] / height;
            for x in 0..width {
                let sx = x * self.image.size[0] / width;
                let pixel = self.image.pixels[sy * self.image.size[0] + sx];
                target[y * width + x] = [pixel.r(), pixel.g(), pixel.b()].map(f32::from);
            }
        }
    }
}

pub(super) struct View<'a> {
    pub active: bool,
    pub image: Option<&'a Rendered>,
    pub fingerprint: u64,
}

pub(super) struct EmptyBackground {
    active: bool,
    desired: Option<Key>,
    rendered: Option<Rendered>,
    renderer: canvas_raster::RendererCache,
    working: egui::ColorImage,

    retired: [Option<Arc<egui::ColorImage>>; 2],
    texture: Option<TextureHandle>,
    failure: Option<(Key, canvas_raster::RasterError)>,
    revision: u64,
}
impl Default for EmptyBackground {
    fn default() -> Self {
        Self { active: false, desired: None, rendered: None,
            renderer: canvas_raster::RendererCache::default(),
            working: egui::ColorImage::new([0, 0], Vec::new()), retired: [None, None], texture: None,
            failure: None, revision: 0 }
    }
}

impl EmptyBackground {
    pub fn failure_status(&self) -> Option<&'static str> {
        self.failure.as_ref().filter(|(key, _)| self.active
            && self.desired.as_ref() == Some(key))
            .map(|_| "Background rendering stopped")
    }

    pub fn view(&self) -> View<'_> {
        let image = self.rendered.as_ref().filter(|image| self.active
            && image.key.lineage.current() && self.desired.as_ref() == Some(&image.key));
        View { active: self.active, image,
            fingerprint: self.revision ^ if self.active { 0x4341_4e56_4153 } else { 0 } }
    }

    pub fn update(&mut self, ctx: &egui::Context, rect: Rect, probe: Probe, camera: f64,
        active: bool, known_empty: bool, geometry: Option<Arc<literal::Geometry>>, lineage: Lineage) {
        self.active = active;

        self.desired = if active && lineage.current() && (known_empty || geometry.is_some()) {
            let dimensions = [rect.width().round(), rect.height().round()];
            let maximum = ctx.input(|input| input.max_texture_side) as f32;
            let xy = [probe.position.x as f32, probe.position.y as f32];
            let radius = probe.radius as f32;
            let zoom = camera as f32;
            if dimensions.iter().any(|n| !n.is_finite() || *n < 4.0 || *n > maximum || *n >= 2_147_483_648.0)
                || xy.iter().any(|n| !n.is_finite()) || !radius.is_finite() || radius <= 0.0
                || !zoom.is_finite() || zoom <= 0.0 {
                None
            } else {
                Some(Key { lineage, dimensions: dimensions.map(|n| n as i32),
                    probe: xy.map(f32::to_bits), radius: radius.to_bits(), zoom: zoom.to_bits(), geometry })
            }
        } else { None };
        self.submit_current(ctx);
    }

    pub fn submit_current(&mut self, ctx: &egui::Context) {
        let Some(key) = self.desired.clone() else { return; };
        if !key.lineage.current()
            || self.rendered.as_ref().is_some_and(|image| image.key == key)
            || self.failure.as_ref().is_some_and(|(failed, _)| failed == &key) {
            return;
        }

        self.reclaim_back_buffer();
        let fields = canvas_raster::Fields { dimensions: key.dimensions,
            probe: key.probe.map(f32::from_bits), radius: f32::from_bits(key.radius),
            zoom: f32::from_bits(key.zoom) };
        if let Err(error) = self.renderer.render_into(fields, key.geometry.as_ref(), &mut self.working) {
            self.failure = Some((key, error));
            return;
        }

        let image = Arc::new(std::mem::replace(&mut self.working,
            egui::ColorImage::new([0, 0], Vec::new())));
        let texture = match &mut self.texture {
            Some(texture) => {
                if texture.size() == image.size {
                    texture.set_partial([0, 0], image.clone(), egui::TextureOptions::LINEAR);
                } else { texture.set(image.clone(), egui::TextureOptions::LINEAR); }
                texture.id()
            }
            None => {
                let texture = ctx.load_texture("source-background", image.clone(), egui::TextureOptions::LINEAR);
                let id = texture.id();
                self.texture = Some(texture);
                id
            }
        };
        self.revision = self.revision.wrapping_add(1);
        self.failure = None;
        let previous = self.rendered.replace(Rendered { key, image, texture });
        if let Some(previous) = previous { self.retire_image(previous.image); }
    }

    fn reclaim_back_buffer(&mut self) {
        for slot in &mut self.retired {
            let Some(candidate) = slot.take() else { continue; };
            match Arc::try_unwrap(candidate) {
                Ok(image) => {

                    if image.pixels.capacity() > self.working.pixels.capacity() {
                        self.working = image;
                    }
                }
                Err(shared) => *slot = Some(shared),
            }
        }
    }

    fn retire_image(&mut self, image: Arc<egui::ColorImage>) {

        if self.retired[0].is_none() { self.retired[0] = self.retired[1].take(); }
        if self.retired[1].is_some() { self.retired[0] = self.retired[1].take(); }
        self.retired[1] = Some(image);
    }
}
