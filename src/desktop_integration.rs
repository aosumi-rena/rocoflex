use super::*;
use crate::integration_wire::{self as wire, State};
use std::io::{BufReader, BufWriter, Write};

#[path = "desktop_integration_input.rs"]
mod input;

pub fn integration_editor() -> Result<()> {
    source_math::initialize_current_process();
    let mut reader = BufReader::new(std::io::stdin().lock());
    let mut writer = BufWriter::new(std::io::stdout().lock());
    let opening: wire::Open = read(&mut reader, wire::DOCUMENT_LIMIT + 4096)?
        .ok_or("Missing Rocoflex editor document")?;
    if opening.version != wire::VERSION { return Err("Unsupported Rocoflex editor version".into()); }
    let ctx = egui::Context::default();
    let mut app = DesktopApp::new_integration(&ctx, opening.state.is_none());
    let mut reported = None;
    let mut previous = None;
    let mut revision = None;
    let mut controls = None;
    let start = Instant::now();
    let mut focused = false;
    let opened = if let Some(state) = opening.state {
        Preset::parse(state.document).and_then(|preset| embedded_canvas::install_document(&mut app, preset))
            .map(|()| app.monitor_controls.set_values(state.pregain_db, state.pitch_semitones))
    } else {
        Ok(())
    };
    if let Err(error) = opened {
        let _ = send(&mut writer, wire::Frame { error: Some(error.clone()), ..Default::default() }, Vec::new());
        return Err(error);
    }
    while let Some(frame) = read::<wire::Input>(&mut reader, wire::INPUT_LIMIT)? {
        let size = frame.size;
        let raw = input::raw(frame, start.elapsed().as_secs_f64(), &mut focused)?;
        app.set_experimental_navigation_batch(focused);
        let mut output = ctx.run_ui(raw, |ui| app.show(ui));
        let values = (app.monitor_controls.pregain_db(), app.monitor_controls.pitch_semitones());
        let dirty = revision != Some(app.revision) || controls != Some(values);
        let layout_busy = app.layout_blocked() || app.layout.active();
        let busy = layout_busy || app.library_scan.busy() || app.model_inspection.busy()
            || app.userdata_load.busy() || app.generation_busy();
        #[cfg(windows)]
        let busy = busy || app.legacy_vocoder_load.busy();
        let mut next = wire::Frame { size, unsettled: dirty || layout_busy, busy, ..Default::default() };
        if dirty && !layout_busy && !ctx.input(|i| i.pointer.any_down()) {
            match app.prepare_saved_snapshot(None).and_then(|saved| {
                if saved.serialized.len() > wire::DOCUMENT_LIMIT { return Err("Rocoflex preset is too large".into()); }
                let document = serde_json::from_str(&saved.serialized).map_err(|e| e.to_string())?;
                Ok(State { document, pregain_db: values.0, pitch_semitones: values.1, has_target: embedded_canvas::has_target(&app) })
            }) {
                Ok(state) => {
                    revision = Some(app.revision);
                    controls = Some(values);
                    next.unsettled = false;
                    if reported.as_ref() != Some(&state) {
                        reported = Some(state.clone());
                        next.state = Some(state);
                    }
                }
                Err(error) => {
                    revision = Some(app.revision);
                    controls = Some(values);
                    app.error = Some(error);
                }
            }
        }
        next.cursor = egui::CursorIcon::ALL.iter().position(|v| *v == output.platform_output.cursor_icon).unwrap_or(0);
        next.ime = output.platform_output.ime.map(|ime| (rect(ime.rect), rect(ime.cursor_rect), ime.should_interrupt_composition));
        for command in output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command { next.clipboard = Some(text); }
        }
        let due = output.viewport_output.get(&egui::ViewportId::ROOT)
            .map_or(u128::MAX, |v| v.repaint_delay.as_millis());
        next.wait_ms = if busy {
            Some(due.clamp(33, 100) as u32)
        } else if due != u128::MAX && due < 2000 {
            Some(due.max(50) as u32)
        } else {
            None
        };
        let mut pixels = Vec::new();
        for (id, deltas) in output.textures_delta.set.drain() {
            use egui::epaint::textures::{TextureOptions, TextureFilter, TextureWrapMode};
            let filter = |v| u8::from(v == TextureFilter::Linear);
            for delta in deltas {
                let TextureOptions { magnification, minification, wrap_mode, mipmap_mode } = delta.options;
                let wrap = match wrap_mode { TextureWrapMode::ClampToEdge => 0, TextureWrapMode::Repeat => 1, TextureWrapMode::MirroredRepeat => 2 };
                let egui::ImageData::Color(image) = delta.image;
                reserve(&mut pixels, image.pixels.len(), 4)?;
                for color in &image.pixels { pixels.extend(color.to_array()); }
                next.textures.push(wire::Texture { id: texture_id(id)?, size: image.size, offset: delta.pos,
                    sampling: [filter(magnification), filter(minification), wrap, mipmap_mode.map_or(0, |v| filter(v) + 1)] });
            }
        }
        next.free = output.textures_delta.free.drain().map(texture_id).collect::<Result<_>>()?;
        let texture_bytes = pixels.len();
        for egui::epaint::ClippedPrimitive { clip_rect, primitive } in ctx.tessellate(output.shapes, output.pixels_per_point) {
            let egui::epaint::Primitive::Mesh(mesh) = primitive else { return Err("Unsupported Rocoflex paint callback".into()); };
            if mesh.is_empty() { continue; }
            reserve(&mut pixels, mesh.vertices.len(), 20)?;
            for v in &mesh.vertices {
                for value in [v.pos.x, v.pos.y, v.uv.x, v.uv.y] { pixels.extend(value.to_le_bytes()); }
                pixels.extend(v.color.to_array());
            }
            reserve(&mut pixels, mesh.indices.len(), 4)?;
            for &index in &mesh.indices { pixels.extend(index.to_le_bytes()); }
            let clip = Rect::from_min_size(Pos2::ZERO, vec2(size[0], size[1])).intersect(clip_rect);
            next.meshes.push(wire::Mesh { id: texture_id(mesh.texture_id)?, clip: rect(clip), vertices: mesh.vertices.len(), indices: mesh.indices.len() });
        }
        let mesh_bytes = &pixels[texture_bytes..];
        let reusable = next.free.is_empty() && previous.as_ref().is_some_and(
            |(old_size, geometry, old_bytes): &([f32; 2], Vec<wire::Mesh>, Vec<u8>)|
                old_size == &size && geometry == &next.meshes && old_bytes.as_slice() == mesh_bytes);
        if reusable {
            next.reuse_picture = true;
            next.meshes.clear();
            pixels.truncate(texture_bytes);
            if !busy && next.state.is_none() && next.textures.is_empty() && due == 0 {
                next.wait_ms = None;
            }
        } else {
            previous = Some((size, next.meshes.clone(), mesh_bytes.to_vec()));
        }
        send(&mut writer, next, pixels)?;
    }
    Ok(())
}

fn read<T: serde::de::DeserializeOwned>(reader: &mut impl std::io::Read, limit: usize) -> Result<Option<T>> {
    wire::read(reader, limit).map_err(|e| e.to_string())?.map(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string())).transpose()
}

fn send(writer: &mut impl Write, frame: wire::Frame, payload: Vec<u8>) -> Result<()> {
    let header = serde_json::to_vec(&frame).map_err(|e| e.to_string())?;
    if header.len() + payload.len() > wire::FRAME_LIMIT { return Err("Rocoflex frame is too large".into()); }
    wire::write(writer, &header, wire::FRAME_LIMIT).and_then(|()| wire::write(writer, &payload, wire::FRAME_LIMIT))
        .and_then(|()| writer.flush()).map_err(|e| e.to_string())
}

fn rect(rect: Rect) -> [f32; 4] { [rect.min.x, rect.min.y, rect.max.x, rect.max.y] }

fn texture_id(id: egui::TextureId) -> Result<u64> {
    match id {
        egui::TextureId::Managed(index) => Ok(index),
        egui::TextureId::User(_) => Err("Unsupported Rocoflex texture".into()),
    }
}

fn reserve(bytes: &mut Vec<u8>, count: usize, stride: usize) -> Result<()> {
    let extra = count.checked_mul(stride).filter(|&len| len <= wire::FRAME_LIMIT - bytes.len())
        .ok_or("Rocoflex frame is too large")?;
    bytes.try_reserve(extra).map_err(|e| e.to_string())
}
