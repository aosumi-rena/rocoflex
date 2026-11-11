use crate::canvas::{Point, Probe, Ray, Scene};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Vec2, pos2, vec2,
};

pub const TEXT: Color32 = Color32::from_rgb(211, 212, 218);
pub const MUTED: Color32 = Color32::from_rgb(173, 174, 183);
pub const WASH: Color32 = Color32::from_rgba_premultiplied(54, 55, 57, 56);

#[derive(Clone)]
pub struct Backdrop {
    pub texture: egui::TextureHandle,
    pub rect: Rect,
}

pub fn frosted_canvas(
    scene: &Scene,
    probe: Probe,
    rays: &[Ray],
    polygons: &[[Point; 4]],
    full: Rect,
    camera: f64,
    native: Option<&super::paint::Frame>,
    ring: Option<&super::probe_ring::Rendered>,
    empty: super::empty_background::View<'_>,
) -> egui::ColorImage {
    let width = 360_usize;
    let height = (full.height() / full.width() * width as f32)
        .round()
        .max(2.0) as usize;
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
    let factor = width as f32 / full.width();
    let transform = |p| world_to_screen(p, rect, camera);
    let mut pixels = vec![[0.0_f32; 3]; width * height];
    if empty.active {
        if let Some(background) = empty.image {
            background.over_black(&mut pixels, width, height);
        }
    } else {
    for y in 0..height {
        for x in 0..width {
            let t = (x as f32 / width as f32 + y as f32 / height as f32) * 0.5;
            let c = if scene.anchors.is_empty() && scene.curves.is_empty() {
                [42.0, 42.0, 52.0]
            } else if t < 0.52 {
                let q = t / 0.52;
                [
                    52.0 + (45.0 - 52.0) * q,
                    55.0 + (48.0 - 55.0) * q,
                    68.0 + (59.0 - 68.0) * q,
                ]
            } else {
                let q = (t - 0.52) / 0.48;
                [45.0 - 7.0 * q, 48.0 - 7.0 * q, 59.0 - 7.0 * q]
            };
            pixels[y * width + x] = c;
        }
    }
    let mut lights = Vec::new();
    for curve in &scene.curves {
        if curve.nodes.is_empty() {
            continue;
        }
        let c = curve.nodes.iter().fold(Point::default(), |a, b| {
            Point::new(
                a.x + b.x / curve.nodes.len() as f64,
                a.y + b.y / curve.nodes.len() as f64,
            )
        });
        lights.push((transform(c), width as f32 * 0.24, curve.color, 0.12));
    }
    for anchor in scene.anchors.iter().filter(|a| a.curve.is_none()).take(256) {
        lights.push((
            transform(anchor.position),
            width as f32 * 0.16,
            anchor.color,
            0.16,
        ));
    }
    for (at, radius, color, alpha) in lights {
        let min_x = (at.x - radius).max(0.0) as usize;
        let max_x = (at.x + radius).ceil().max(0.0).min(width as f32) as usize;
        let min_y = (at.y - radius).max(0.0) as usize;
        let max_y = (at.y + radius).ceil().max(0.0).min(height as f32) as usize;
        for y in min_y..max_y {
            for x in min_x..max_x {
                let d = (x as f32 - at.x).hypot(y as f32 - at.y);
                if d < radius {
                    let a = (1.0 - d / radius) * alpha;
                    let pixel = &mut pixels[y * width + x];
                    for i in 0..3 {
                        pixel[i] = pixel[i] * (1.0 - a) + color[i] as f32 * a;
                    }
                }
            }
        }
    }

    let shadow = (!polygons.is_empty()).then(|| shadow_image(polygons, full, camera));
    for y in 0..height {
        for x in 0..width {
            let a = shadow.as_ref().map_or(0.0, |shadow| {
                let sx = x * shadow.size[0] / width;
                let sy = y * shadow.size[1] / height;
                shadow.pixels[sy * shadow.size[0] + sx].a() as f32 / 255.0
            });
            for i in 0..3 {
                pixels[y * width + x][i] =
                    pixels[y * width + x][i] * (1.0 - a) + [2.0, 4.0, 9.0][i] * a;
            }
        }
    }
    }
    if let Some(frame) = native {
        frame.raster(&mut pixels, width, height, full);
    } else if !empty.active {
        for ray in rays {
            let anchor = &scene.anchors[ray.source];
            let at = anchor.position;
            let d = at.distance(probe.position);
            if d < 1e-6 {
                continue;
            }
            let subtract = ray.shadow_strength > 0.01;
            if !subtract && (ray.blocked || ray.strength <= 0.002) {
                continue;
            }
            let length = if subtract {
                (0.16 + d * 0.36 + crate::canvas::connection_reach(probe.radius) * 0.12)
                    .clamp(0.16, 0.74)
            } else {
                0.5_f64.min(ray.edge_distance)
            };
            let direction = if subtract {
                Point::new((at.x - probe.position.x) / d, (at.y - probe.position.y) / d)
            } else {
                Point::new((probe.position.x - at.x) / d, (probe.position.y - at.y) / d)
            };
            for segment in 0..8 {
                let t = segment as f64 / 8.0;
                let z = (segment + 1) as f64 / 8.0;
                let a = Point::new(
                    at.x + direction.x * length * t,
                    at.y + direction.y * length * t,
                );
                let b = Point::new(
                    at.x + direction.x * length * z,
                    at.y + direction.y * length * z,
                );
                let alpha = if subtract {
                    0.082 * (1.0 - t)
                } else {
                    0.34 * (1.0 - (length / 0.5 * t).powf(2.15))
                };
                raster_line(
                    &mut pixels,
                    width,
                    height,
                    transform(a),
                    transform(b),
                    anchor.color,
                    alpha as f32,
                    (factor * 0.92).max(0.45),
                );
            }
        }
        for curve in &scene.curves {
            for pair in curve.path.windows(2) {
                raster_line(
                    &mut pixels,
                    width,
                    height,
                    transform(pair[0]),
                    transform(pair[1]),
                    curve.color,
                    0.56,
                    (factor * 1.8).max(0.6),
                );
            }
        }
        for anchor in &scene.anchors {
            let at = transform(anchor.position);
            raster_line(
                &mut pixels,
                width,
                height,
                at,
                at,
                anchor.color,
                1.0,
                factor * 12.4,
            );
            if anchor.curve.is_none() {
                for i in 0..8 {
                    let a = i as f32 / 8.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_8;
                    let b = a + std::f32::consts::TAU / 8.0;
                    raster_line(
                        &mut pixels,
                        width,
                        height,
                        at + vec2(a.cos(), a.sin()) * 14.0 * factor,
                        at + vec2(b.cos(), b.sin()) * 14.0 * factor,
                        anchor.color,
                        0.82,
                        (factor * 1.4).max(0.5),
                    );
                }
            }
        }
    }
    let center = transform(probe.position);
    let radius = probe.radius as f32 * width.min(height) as f32 * 0.5 * camera as f32;

    let underlay_radius = (radius - factor).max(0.0);
    raster_line(
        &mut pixels,
        width,
        height,
        center,
        center,
        [53, 54, 64],
        128.0 / 255.0,
        underlay_radius * 2.0,
    );
    if native.is_some() || empty.active {
        raster_line(&mut pixels, width, height, center, center,
            [202, 202, 202], 1.0, 12.0 * factor);
        if let Some(ring) = ring {
            ring.composite(&mut pixels, width, height, full);
        }
    } else {

        for i in 0..96 {
            let a = i as f64 / 96.0 * std::f64::consts::TAU;
            let b = (i + 1) as f64 / 96.0 * std::f64::consts::TAU;
            let c = ring_color(a, scene, rays, probe);
            raster_line(
                &mut pixels,
                width,
                height,
                center + vec2(a.cos() as f32, a.sin() as f32) * (radius - 2.5 * factor).max(0.0),
                center + vec2(b.cos() as f32, b.sin() as f32) * (radius - 2.5 * factor).max(0.0),
                [c.r(), c.g(), c.b()],
                0.96,
                (factor * 5.0).max(0.8),
            );
        }
        raster_line(
            &mut pixels,
            width,
            height,
            center,
            center,
            [202, 202, 202],
            1.0,
            12.0 * factor,
        );
    }
    let pixels = blur(pixels, width, height, (18.0 * factor).max(1.0));
    egui::ColorImage::new(
        [width, height],
        (0..width * height)
            .map(|i| {
                Color32::from_rgb(
                    pixels[i][0].round().clamp(0.0, 255.0) as u8,
                    pixels[i][1].round().clamp(0.0, 255.0) as u8,
                    pixels[i][2].round().clamp(0.0, 255.0) as u8,
                )
            })
            .collect(),
    )
}

pub(super) fn raster_line(
    pixels: &mut [[f32; 3]],
    width: usize,
    height: usize,
    a: Pos2,
    b: Pos2,
    color: [u8; 3],
    alpha: f32,
    line_width: f32,
) {
    let radius = line_width * 0.5;
    let min_x = (a.x.min(b.x) - radius - 1.0).floor().max(0.0) as usize;
    let max_x = (a.x.max(b.x) + radius + 1.0)
        .ceil()
        .max(0.0)
        .min(width as f32) as usize;
    let min_y = (a.y.min(b.y) - radius - 1.0).floor().max(0.0) as usize;
    let max_y = (a.y.max(b.y) + radius + 1.0)
        .ceil()
        .max(0.0)
        .min(height as f32) as usize;
    let segment = b - a;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let p = pos2(x as f32 + 0.5, y as f32 + 0.5);
            let t = ((p - a).dot(segment) / segment.length_sq().max(1e-6)).clamp(0.0, 1.0);
            let distance = p.distance(a + segment * t);
            let coverage = (radius + 0.65 - distance).clamp(0.0, 1.0) * alpha;
            let pixel = &mut pixels[y * width + x];
            for i in 0..3 {
                pixel[i] = pixel[i] * (1.0 - coverage) + color[i] as f32 * coverage;
            }
        }
    }
}

pub(super) use super::icons::{Icon, icon};

pub fn optional_tooltip(
    response: Response,
    enabled: bool,
    text: impl Into<egui::WidgetText>,
) -> Response {
    let text = text.into();
    if !enabled || !super::localization::original_tooltip(text.text()) {
        return response;
    }
    let translated = super::localization::tr(&response.ctx, text.text()).to_owned();
    let mut anchor = response.interact_rect;
    if let Some(transform) = response.ctx.layer_transform_to_global(response.layer_id) {
        anchor = transform * anchor;
    }
    let viewport = response.ctx.content_rect();
    let alignment = if anchor.center().y < viewport.top() + 70.0 {
        if anchor.center().x > viewport.center().x { egui::RectAlign::BOTTOM_END }
        else { egui::RectAlign::BOTTOM_START }
    } else if anchor.center().x < viewport.center().x {
        egui::RectAlign::RIGHT
    } else {
        egui::RectAlign::LEFT
    };
    let fill = Color32::from_rgb(63, 63, 70);
    let frame = egui::Frame::NONE.fill(fill).corner_radius(5)
        .inner_margin(egui::Margin::symmetric(15, 12))
        .shadow(egui::epaint::Shadow {
            offset: [0, 1], blur: 6, spread: 0, color: Color32::from_black_alpha(45),
        });
    let mut tooltip = egui::Tooltip::for_enabled(&response).gap(14.0).width(200.0);
    tooltip.popup = tooltip.popup.align(alignment).frame(frame);
    if let Some(popup) = tooltip.show(|ui| {
        ui.set_max_width(170.0);
        ui.add(egui::Label::new(egui::RichText::new(translated).size(13.0).color(TEXT)).wrap());
    }) {

        let rect = popup.response.rect;
        if rect.is_positive() && rect.width() > 20.0 && rect.height() > 20.0 {
            let x = anchor.center().x.clamp(rect.left() + 10.0, rect.right() - 10.0);
            let y = anchor.center().y.clamp(rect.top() + 10.0, rect.bottom() - 10.0);
            let points = if rect.left() >= anchor.right() {
                vec![pos2(rect.left(), y - 7.0), pos2(rect.left() - 8.0, y), pos2(rect.left(), y + 7.0)]
            } else if rect.right() <= anchor.left() {
                vec![pos2(rect.right(), y - 7.0), pos2(rect.right() + 8.0, y), pos2(rect.right(), y + 7.0)]
            } else if rect.top() >= anchor.bottom() {
                vec![pos2(x - 7.0, rect.top()), pos2(x, rect.top() - 8.0), pos2(x + 7.0, rect.top())]
            } else if rect.bottom() <= anchor.top() {
                vec![pos2(x - 7.0, rect.bottom()), pos2(x, rect.bottom() + 8.0), pos2(x + 7.0, rect.bottom())]
            } else { Vec::new() };
            if !points.is_empty() {
                response.ctx.layer_painter(popup.response.layer_id)
                    .add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));
            }
        }
    }
    response
}

pub fn icon_button(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    kind: Icon,
    tip: &str,
    show_tooltips: bool,
) -> Response {
    icon_button_layout(ui, rect, rect.shrink(5.0), id, kind, tip, show_tooltips)
}

pub fn icon_button_layout(
    ui: &mut egui::Ui,
    hit: Rect,
    glyph: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    kind: Icon,
    tip: &str,
    show_tooltips: bool,
) -> Response {
    let response = ui.interact(hit, ui.id().with(id), Sense::click());
    let accessible_label = super::localization::tr(ui.ctx(), tip);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), accessible_label));
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(hit, 20, Color32::from_white_alpha(15));
    }
    icon(
        ui.painter(),
        glyph,
        kind,
        if !ui.is_enabled() {
            MUTED.gamma_multiply(0.5)
        } else if response.hovered() || response.has_focus() {
            Color32::from_gray(240)
        } else {
            TEXT
        },
    );
    optional_tooltip(response, show_tooltips, tip)
}

pub fn text_button(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    text: &str,
    enabled: bool,
    centered: bool,
) -> Response {
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && response.hovered() {
        ui.painter()
            .rect_filled(rect, 5, Color32::from_black_alpha(28));
    }
    ui.painter().text(
        if centered {
            rect.center()
        } else {
            rect.left_center() + vec2(17.0, 0.0)
        },
        if centered {
            Align2::CENTER_CENTER
        } else {
            Align2::LEFT_CENTER
        },
        text,
        FontId::proportional(15.0),
        if enabled {
            TEXT
        } else {
            Color32::from_gray(131)
        },
    );
    response
}

pub fn pill_button(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    text: &str,
    enabled: bool,
) -> Response {
    let text = super::localization::tr(ui.ctx(), text);
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled && ui.is_enabled(), text));
    let fill = if enabled && response.hovered() {
        Color32::from_black_alpha(88)
    } else {
        Color32::from_black_alpha(52)
    };
    ui.painter().rect_filled(rect, 24, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(15.0),
        if enabled {
            TEXT
        } else {
            Color32::from_gray(148)
        },
    );
    response
}

pub fn polygon(p: &egui::Painter, center: Pos2, radius: f32, color: Color32, width: f32) {
    let points = (0..8)
        .map(|i| {
            let a = i as f32 / 8.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_8;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    p.add(egui::Shape::closed_line(points, Stroke::new(width, color)));
}

pub fn rgba(c: [u8; 3], a: f64) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}
pub fn parse_color(text: &str) -> [u8; 3] {
    let text = text.trim_start_matches('#');
    let text = if text.len() == 8 {
        text.get(2..).unwrap_or("")
    } else {
        text
    };
    if text.len() == 6
        && let Ok(n) = u32::from_str_radix(text, 16)
    {
        [(n >> 16) as u8, (n >> 8) as u8, n as u8]
    } else {
        [127, 178, 53]
    }
}
pub fn quad(p: &egui::Painter, positions: [Pos2; 4], colors: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    for i in 0..4 {
        mesh.colored_vertex(positions[i], colors[i]);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}
pub fn glow(p: &egui::Painter, center: Pos2, radius: f32, color: [u8; 3], alpha: f64) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, rgba(color, alpha));
    for i in 0..=64 {
        let a = i as f32 / 64.0 * std::f32::consts::TAU;
        mesh.colored_vertex(center + vec2(a.cos(), a.sin()) * radius, rgba(color, 0.0));
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    p.add(mesh);
}

pub fn fading_line(
    p: &egui::Painter,
    a: Pos2,
    b: Pos2,
    color: [u8; 3],
    alpha: f64,
    width: f32,
    normalized: f64,
    subtract: bool,
) {
    if a.distance_sq(b) < 0.01 {
        return;
    }
    let d = (b - a).normalized();
    let normal = vec2(-d.y, d.x) * width * 0.5;
    let mut mesh = egui::Mesh::default();
    for i in 0..=12 {
        let q = i as f32 / 12.0;
        let fade = if subtract {
            if q <= 0.72 {
                1.0 - q as f64 / 0.72 * 0.55
            } else {
                (1.0 - q as f64) / 0.28 * 0.45
            }
        } else {
            (1.0 - (normalized * q as f64).clamp(0.0, 1.0).powf(2.15)).max(0.0)
        };
        let c = rgba(color, alpha * fade);
        let at = a.lerp(b, q);
        mesh.colored_vertex(at - normal, c);
        mesh.colored_vertex(at + normal, c);
        if i > 0 {
            let n = i * 2;
            mesh.add_triangle(n - 2, n - 1, n);
            mesh.add_triangle(n - 1, n + 1, n);
        }
    }
    p.add(mesh);
}

pub fn ring_color(angle: f64, scene: &Scene, rays: &[Ray], probe: Probe) -> Color32 {
    let visible: Vec<_> = rays
        .iter()
        .filter(|r| !r.blocked && r.strength > 0.005)
        .take(56)
        .collect();
    let sampled: Vec<_> = if visible.is_empty() {
        rays.iter().take(56).collect()
    } else {
        visible
    };
    let mut sum = [0.0; 3];
    let mut total = 0.0;
    for ray in sampled {
        let a = &scene.anchors[ray.source];
        let theta = (-(a.position.y - probe.position.y)).atan2(a.position.x - probe.position.x);
        let delta = (angle - theta).abs() % std::f64::consts::TAU;
        let delta = delta.min(std::f64::consts::TAU - delta);
        let strength =
            (0.06 + ray.strength * 1.2 * if a.curve.is_some() { 1.0 } else { 1.9 }).clamp(0.0, 1.4);
        let weight = (-delta * delta / (2.0 * 0.36 * 0.36)).exp() * strength;
        for (v, c) in sum.iter_mut().zip(a.color) {
            *v += c as f64 * weight;
        }
        total += weight;
    }
    let fallback = scene
        .anchors
        .iter()
        .min_by(|a, b| {
            a.position
                .distance(probe.position)
                .total_cmp(&b.position.distance(probe.position))
        })
        .map(|a| a.color)
        .unwrap_or([220, 222, 228]);
    rgba(
        if total > 1e-6 {
            sum.map(|v| (v / total).round().clamp(0.0, 255.0) as u8)
        } else {
            fallback
        },
        0.96,
    )
}

pub fn smooth_path(nodes: &[Point]) -> Vec<Point> {
    if nodes.len() < 3 {
        return nodes.to_vec();
    }
    let mut path = Vec::with_capacity(nodes.len() * 6);
    for i in 0..nodes.len() - 1 {
        let a = nodes[i.saturating_sub(1)];
        let b = nodes[i];
        let c = nodes[i + 1];
        let d = nodes[(i + 2).min(nodes.len() - 1)];
        for j in 0..6 {
            let t = j as f64 / 6.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let eval = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * ((2.0 * b)
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            path.push(Point::new(
                eval(a.x, b.x, c.x, d.x),
                eval(a.y, b.y, c.y, d.y),
            ));
        }
    }
    path.push(*nodes.last().unwrap());
    path
}

pub fn world_to_screen(p: Point, rect: Rect, camera: f64) -> Pos2 {
    let half = rect.width().min(rect.height()) * 0.5;
    pos2(
        (p.x as f32 * half) * camera as f32 + rect.center().x,
        rect.center().y - (half * p.y as f32) * camera as f32,
    )
}

pub fn shadow_image(polygons: &[[Point; 4]], rect: Rect, camera: f64) -> egui::ColorImage {
    let factor = (640.0 / rect.width()).min(0.75);
    let width = (rect.width() * factor).ceil().max(2.0) as usize;
    let height = (rect.height() * factor).ceil().max(2.0) as usize;
    let mut mask = vec![0.0_f32; width * height];
    for poly in polygons {
        let points = poly.map(|p| {
            let at = world_to_screen(p, rect, camera);
            (at - rect.min) * factor
        });
        let ymin = points
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let ymax = points
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(height as f32) as usize;
        for y in ymin..ymax {
            let scan = y as f32 + 0.5;
            let mut intersections = Vec::with_capacity(4);
            for i in 0..4 {
                let a = points[i];
                let b = points[(i + 1) % 4];
                if (a.y <= scan && b.y > scan) || (b.y <= scan && a.y > scan) {
                    intersections.push(a.x + (scan - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            intersections.sort_by(f32::total_cmp);
            for pair in intersections.chunks_exact(2) {
                let left = pair[0].ceil().max(0.0) as usize;
                let right = pair[1].ceil().max(0.0).min(width as f32) as usize;
                for x in left.min(width)..right {
                    mask[y * width + x] = 1.0;
                }
            }
        }
    }
    let blurred = blur(mask, width, height, 7.0 * factor);
    egui::ColorImage::new(
        [width, height],
        blurred
            .into_iter()
            .map(|a| rgba([2, 4, 9], a as f64 * 0.24))
            .collect(),
    )
}

trait BlurPixel: Copy {
    fn zero() -> Self;
    fn add_scaled(&mut self, source: Self, weight: f32);
}

impl BlurPixel for f32 {
    fn zero() -> Self { 0.0 }
    fn add_scaled(&mut self, source: Self, weight: f32) {
        *self += source * weight;
    }
}

impl BlurPixel for [f32; 3] {
    fn zero() -> Self { [0.0; 3] }
    fn add_scaled(&mut self, source: Self, weight: f32) {
        for channel in 0..3 {
            self[channel] += source[channel] * weight;
        }
    }
}

fn blur<P: BlurPixel>(mut input: Vec<P>, width: usize, height: usize, sigma: f32) -> Vec<P> {
    let radius = (sigma * 2.5).ceil().max(1.0) as isize;
    let mut kernel: Vec<f32> = (-radius..=radius)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    for x in &mut kernel {
        *x /= sum;
    }
    let mut temp = vec![P::zero(); width * height];
    for y in 0..height {
        for x in 0..width {
            let mut v = P::zero();
            for (k, w) in kernel.iter().enumerate() {
                let xx = (x as isize + k as isize - radius).clamp(0, width as isize - 1) as usize;
                v.add_scaled(input[y * width + xx], *w);
            }
            temp[y * width + x] = v;
        }
    }

    for y in 0..height {
        for x in 0..width {
            let mut v = P::zero();
            for (k, w) in kernel.iter().enumerate() {
                let yy = (y as isize + k as isize - radius).clamp(0, height as isize - 1) as usize;
                v.add_scaled(temp[yy * width + x], *w);
            }
            input[y * width + x] = v;
        }
    }
    input
}

pub fn brand(p: &egui::Painter, at: Pos2, size: f32) {
    let s = size / 30.0;
    let id = egui::Id::new("rocoflex-brand-mark");
    let cached = p
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = cached.unwrap_or_else(|| {
        let pixels = include_bytes!("../assets/brand-mark.alpha")
            .iter()
            .map(|alpha| Color32::from_white_alpha(*alpha))
            .collect();
        let texture = p.ctx().load_texture(
            "brand-mark",
            egui::ColorImage::new([72, 106], pixels),
            egui::TextureOptions::LINEAR,
        );
        p.ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    let height = 33.0 * s;
    p.image(
        texture.id(),
        Rect::from_min_size(
            at + vec2(-3.0, -13.0) * s,
            vec2(height * 72.0 / 106.0, height),
        ),
        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
        TEXT,
    );
    p.text(
        at + vec2(29.0, 2.0) * s,
        Align2::LEFT_CENTER,
        "Rocoflex",
        FontId::proportional(size),
        TEXT,
    );
}

pub fn clipped_title(name: &str, max: usize) -> String {
    let mut chars = name.chars();
    let prefix: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!(
            "{}...",
            prefix.chars().take(max.saturating_sub(3)).collect::<String>()
        )
    } else {
        prefix
    }
}

pub fn glass(p: &egui::Painter, rect: Rect, lights: &[(Pos2, [u8; 3])], strong: bool) {
    if let Some(backdrop) = p
        .ctx()
        .data_mut(|d| d.get_temp::<Backdrop>(egui::Id::new("rocoflex-glass-backdrop")))
    {
        let mut mesh = egui::Mesh::with_texture(backdrop.texture.id());
        let uv = |at: Pos2| {
            pos2(
                (at.x - backdrop.rect.left()) / backdrop.rect.width(),
                (at.y - backdrop.rect.top()) / backdrop.rect.height(),
            )
        };
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rect.center(),
            uv: uv(rect.center()),
            color: Color32::WHITE,
        });
        for (corner, start) in [
            (
                rect.right_top() + vec2(-10.0, 10.0),
                -std::f32::consts::FRAC_PI_2,
            ),
            (rect.right_bottom() + vec2(-10.0, -10.0), 0.0),
            (
                rect.left_bottom() + vec2(10.0, -10.0),
                std::f32::consts::FRAC_PI_2,
            ),
            (rect.left_top() + vec2(10.0, 10.0), std::f32::consts::PI),
        ] {
            for i in 0..=8 {
                let a = start + i as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                let at = corner + vec2(a.cos(), a.sin()) * 10.0;
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: at,
                    uv: uv(at),
                    color: Color32::WHITE,
                });
            }
        }
        for i in 1..mesh.vertices.len() - 1 {
            mesh.add_triangle(0, i as u32, i as u32 + 1);
        }
        mesh.add_triangle(0, mesh.vertices.len() as u32 - 1, 1);
        p.add(mesh);
    }
    p.rect_filled(
        rect,
        10,
        if strong {
            Color32::from_rgba_unmultiplied(86, 87, 96, 228)
        } else {
            Color32::from_rgba_unmultiplied(200, 201, 208, 49)
        },
    );
    let clipped = p.with_clip_rect(rect.shrink(1.0));
    for (at, color) in lights {
        glow(&clipped, *at, rect.width() * 0.8, *color, 0.08);
    }
    p.rect_stroke(
        rect,
        10,
        Stroke::new(0.6, Color32::from_white_alpha(15)),
        egui::StrokeKind::Inside,
    );
}

pub fn section(ui: &mut egui::Ui, rect: Rect, text: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(16.0), TEXT);
    let end = rect.left() + galley.size().x + 12.0;
    ui.painter().galley(rect.left_top(), galley, TEXT);
    ui.painter().line_segment(
        [
            pos2(end, rect.center().y),
            pos2(rect.right(), rect.center().y),
        ],
        Stroke::new(1.0, Color32::from_white_alpha(70)),
    );
}

pub fn field_label(ui: &mut egui::Ui, rect: Rect, text: &str) {
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(15.0),
        MUTED,
    );
}

pub fn source_menu_position(at: Pos2, full: Rect, occupied: &[Pos2], size: Vec2) -> Rect {
    let candidates = [
        at + vec2(0.0, 16.0),
        at + vec2(-size.x, 16.0),
        at + vec2(0.0, -size.y - 16.0),
        at + vec2(-size.x, -size.y - 16.0),
    ];
    candidates
        .into_iter()
        .map(|p| {
            let r = Rect::from_min_size(
                pos2(
                    p.x.clamp(full.left() + 12.0, full.right() - size.x - 12.0),
                    p.y.clamp(full.top() + 76.0, full.bottom() - size.y - 12.0),
                ),
                size,
            );
            let overlap = occupied
                .iter()
                .filter(|p| r.expand(10.0).contains(**p))
                .count();
            (r, overlap)
        })
        .min_by_key(|(_, n)| *n)
        .unwrap()
        .0
}

pub fn hover_scroll<R>(ui: &egui::Ui, output: &egui::scroll_area::ScrollAreaOutput<R>) {
    let limit = (output.content_size.y - output.inner_rect.height()).max(0.0);
    let mut state = output.state;
    for (top, visible) in [
        (true, state.offset.y > 1.0),
        (false, state.offset.y < limit - 1.0),
    ] {
        if !visible {
            continue;
        }
        let frame = output.inner_rect;
        let rect = Rect::from_min_size(
            pos2(
                frame.left(),
                if top {
                    frame.top()
                } else {
                    frame.bottom() - 20.0
                },
            ),
            vec2(frame.width(), 20.0),
        );
        let shade = Color32::from_rgba_unmultiplied(37, 38, 45, 110);
        quad(
            ui.painter(),
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            if top {
                [shade, shade, Color32::TRANSPARENT, Color32::TRANSPARENT]
            } else {
                [Color32::TRANSPARENT, Color32::TRANSPARENT, shade, shade]
            },
        );
        let center = rect.center();
        let sign = if top { -1.0 } else { 1.0 };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                center + vec2(-5.0, -2.5 * sign),
                center + vec2(5.0, -2.5 * sign),
                center + vec2(0.0, 3.0 * sign),
            ],
            MUTED,
            Stroke::NONE,
        ));
        if ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| rect.contains(p))
        {
            state.offset.y = (state.offset.y + sign * ui.input(|i| i.stable_dt).min(0.05) * 70.0)
                .clamp(0.0, limit);
            state.store(ui.ctx(), output.id);
            ui.ctx().request_repaint();
        }
    }
}
