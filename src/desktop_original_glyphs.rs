use super::empty_background::literal as raster;
use super::{glyph_fit, glyph_math, glyph_transform_values as transform_values};
use crate::source_scalar::{ExplicitExceptionState, ScalarF32Architecture};
use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, pos2};
use serde::Deserialize;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Glyph { AddWaypoint, RerouteWaypoints, Waypoints, More, Reset }
impl Glyph {
    fn binding(self) -> usize {
        match self {
            Self::AddWaypoint => 5,
            Self::RerouteWaypoints => 6,
            Self::Waypoints => 9,
            Self::More => 3,
            Self::Reset => 4,
        }
    }
    fn ordinal(self) -> usize { match self { Self::AddWaypoint => 0, Self::RerouteWaypoints => 1,
        Self::Waypoints => 2, Self::More => 3, Self::Reset => 4 } }
}
#[derive(Deserialize)]
struct Record { commands: Vec<Command> }
#[derive(Deserialize)]
struct Command { opcode: String, #[serde(default)] float32_bits: Vec<u32> }
enum Error {
    Catalog(serde_json::Error), Record { index: usize, command: Option<usize>, reason: &'static str },
    Dimensions, Storage(raster::image::PixelError), Mask(raster::edge_table::Error),
    EmptyMask, Paint(raster::solid::Error),
    Controls(super::layout::context::provider::Error), State { mxcsr: u32 },
    FitBinding(glyph_math::BindingFailure),
    Fit { failure: glyph_fit::Failure<glyph_math::Stopped>, math: glyph_math::Provider },
    TransformBinding { failure: transform_values::StartFailure, fit: glyph_fit::Completed },
    Transform { failure: transform_values::Failure, fit: glyph_fit::Completed },
    AfterTransform { cause: Box<Error>, fit: glyph_fit::Completed, transform: transform_values::Completed },
}
struct Catalog { paths: [raster::path::Path; 5] }
static CATALOG: OnceLock<Result<Catalog, Error>> = OnceLock::new();
const SOURCE: &str = include_str!("../assets/glyph_paths.json");

fn load() -> Result<Catalog, Error> {
    let records: Vec<Record> = serde_json::from_str(SOURCE).map_err(Error::Catalog)?;
    Ok(Catalog { paths: [decode(&records, Glyph::AddWaypoint)?,
        decode(&records, Glyph::RerouteWaypoints)?, decode(&records, Glyph::Waypoints)?,
        decode(&records, Glyph::More)?, decode(&records, Glyph::Reset)?] })
}
fn decode(records: &[Record], glyph: Glyph) -> Result<raster::path::Path, Error> {
    let index = glyph.binding();
    let record = records.get(index).ok_or(Error::Record { index, command: None, reason: "missing glyph record" })?;
    let mut words = Vec::new();
    let mut bounds = [f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY];
    let mut nonzero = false;
    let mut open = false;
    for (ordinal, command) in record.commands.iter().enumerate() {
        let error = |reason| Error::Record { index, command: Some(ordinal), reason };
        let (marker, coordinates) = match command.opcode.as_str() {
            "n" if ordinal == 0 && command.float32_bits.is_empty() => { nonzero = true; continue; }
            "m" if !open => { open = true; (0x47c35100, 2) }
            "l" if open => (0x47c35080, 2),
            "b" if open => (0x47c35200, 6),
            "c" if open => { open = false; (0x47c35280, 0) }
            _ => return Err(error("unsupported or unbalanced path command")),
        };
        if command.float32_bits.len() != coordinates { return Err(error("raw coordinate count changed")); }
        for pair in command.float32_bits.chunks_exact(2) {
            let [x, y] = [f32::from_bits(pair[0]), f32::from_bits(pair[1])];
            if !x.is_finite() || !y.is_finite() { return Err(error("nonfinite glyph coordinate")); }
            bounds[0] = bounds[0].min(x); bounds[1] = bounds[1].max(x);
            bounds[2] = bounds[2].min(y); bounds[3] = bounds[3].max(y);
        }
        words.push(marker);
        words.extend_from_slice(&command.float32_bits);
    }
    if open || !nonzero || words.is_empty() || bounds[0] >= bounds[1] || bounds[2] >= bounds[3] {
        return Err(Error::Record { index, command: None, reason: "closed nonzero compound glyph required" });
    }
    let source_capacity = i32::try_from(words.len()).map_err(|_| Error::Dimensions)?;
    Ok(raster::path::Path { words, source_capacity, bounds, source_flag_32: 1 })
}
struct PreparedImage { image: egui::ColorImage, fit: glyph_fit::Completed, transform: transform_values::Completed }
fn image(glyph: Glyph, dimensions: [i32; 2]) -> Result<PreparedImage, Error> {
    let catalog = CATALOG.get_or_init(load).as_ref().map_err(|_| Error::Record {
        index: glyph.binding(), command: None, reason: "retained catalog failure" })?;
    let path = &catalog.paths[glyph.ordinal()];
    let width = dimensions[0] as f32;
    let height = dimensions[1] as f32;

    let observed = super::layout::context::provider::check_current_thread().map_err(Error::Controls)?;
    let state = ExplicitExceptionState::from_explicit_parts(observed, 0).map_err(|_| Error::State { mxcsr: observed })?;
    let mut math = glyph_math::Provider::from_explicit_parts(ScalarF32Architecture::IntelSdm, state).map_err(Error::FitBinding)?;
    let input = glyph_fit::Input { bounds: path.bounds.map(f32::to_bits),
        target: [0.0f32.to_bits(), 0.0f32.to_bits(), width.to_bits(), height.to_bits()],
        preserve: 1, justification: 0x24 };
    let fit = match glyph_fit::prepare(input, &mut math) {
        Ok(fit) => fit, Err(failure) => return Err(Error::Fit { failure, math }),
    };
    let prepared = match transform_values::Prepared::from_values(path, fit.words, math) {
        Ok(prepared) => prepared, Err(failure) => return Err(Error::TransformBinding { failure, fit }),
    };
    let transform = match prepared.run() {
        Ok(transform) => transform, Err(failure) => return Err(Error::Transform { failure, fit }),
    };
    let result = (|| {

        let table = raster::image::path_mask_source([0, 0, dimensions[0], dimensions[1]],
            transform.path(), [1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
            .map_err(Error::Mask)?.ok_or(Error::EmptyMask)?;
        let mut storage = raster::image::SoftwareStorage::create_cleared_format2(dimensions).map_err(Error::Storage)?;
        let layout = raster::image::BitmapLayout { width: dimensions[0] as usize, height: dimensions[1] as usize,
            row_stride: storage.row_stride, pixel_stride: 4 };
        let mut painter = raster::solid::SpanPainter::new(0xffff_ffff);
        raster::solid::shade_edge_table_format2_blend(&mut storage.bytes, &layout, &table, &mut painter).map_err(Error::Paint)?;
        let mut pixels = Vec::with_capacity(layout.width * layout.height);
        for row in storage.bytes.chunks_exact(storage.row_stride).take(layout.height) {
            for pixel in row.chunks_exact(4).take(layout.width) {
                pixels.push(Color32::from_rgba_premultiplied(pixel[2], pixel[1], pixel[0], pixel[3]));
            }
        }
        Ok(egui::ColorImage::new([layout.width, layout.height], pixels))
    })();
    match result {
        Ok(image) => Ok(PreparedImage { image, fit, transform }),
        Err(cause) => Err(Error::AfterTransform { cause: Box::new(cause), fit, transform }),
    }
}
struct Rendered { texture: TextureHandle, fit: glyph_fit::Completed, transform: transform_values::Completed }
type Cached = Result<Arc<Rendered>, Arc<Error>>;
pub(super) fn paint(painter: &egui::Painter, rect: Rect, glyph: Glyph, tint: Color32) {
    if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 || tint == Color32::TRANSPARENT { return; }
    let context = painter.ctx();
    let density = context.pixels_per_point();
    let maximum = context.input(|input| input.max_texture_side) as f32;
    let physical = [(rect.width() * density).ceil(), (rect.height() * density).ceil()];
    if physical.iter().any(|value| !value.is_finite() || *value < 1.0 || *value > maximum || *value >= 2_147_483_648.0) { return; }
    let dimensions = physical.map(|value| value as i32);
    let id = egui::Id::new(("source-monitor-glyph", glyph, dimensions));
    let cached = context.data_mut(|data| data.get_temp::<Cached>(id));
    let cached = cached.unwrap_or_else(|| {
        let result = image(glyph, dimensions).map(|image| Arc::new(Rendered {
            texture: context.load_texture(format!("source-monitor-glyph-{glyph:?}"), image.image, egui::TextureOptions::LINEAR),
            fit: image.fit, transform: image.transform,
        })).map_err(Arc::new);
        context.data_mut(|data| data.insert_temp(id, result.clone()));
        result
    });
    if let Ok(rendered) = cached {
        painter.image(rendered.texture.id(), rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), tint);
    }
}
