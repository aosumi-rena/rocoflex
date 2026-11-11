use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2, pos2, vec2};
use std::hash::{Hash, Hasher};

#[allow(dead_code)]
#[path = "desktop_paint_math.rs"]
pub mod math;

pub struct PointInput {
    pub index: usize,
    pub position: [f32; 2],
    pub fields: [f32; 4],
}
pub struct SourceInput {
    pub index: usize,
    pub argb: u32,
    pub trajectory: bool,
    pub selected: bool,
    pub points: Vec<PointInput>,
}
pub struct Node {
    pub source: usize,
    pub index: usize,
    pub selected: bool,
    pub command: math::PointCommands,
}
pub struct Segment {
    pub points: [[f32; 2]; 2],
    pub argb: u32,
}
#[derive(Default)]
pub struct Frame {
    pub segments: Vec<Segment>,
    pub nodes: Vec<Node>,
}

pub fn parse_argb(text: &str) -> u32 {
    let text = text.trim().trim_start_matches('#');
    match text.len() {
        6 => u32::from_str_radix(text, 16).map(|v| 0xff000000 | v),
        8 => u32::from_str_radix(text, 16),
        _ => return 0xff7fb235,
    }
    .unwrap_or(0xff7fb235)
}

fn rgba(argb: u32) -> [u8; 4] {
    [
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        (argb >> 24) as u8,
    ]
}
fn color(argb: u32) -> Color32 {
    let [r, g, b, a] = rgba(argb);
    Color32::from_rgba_unmultiplied(r, g, b, a)
}
fn finite(point: [f32; 2]) -> bool {
    point.iter().all(|v| v.is_finite())
}

impl Frame {
    pub fn new(sources: impl IntoIterator<Item = SourceInput>, probe: [f32; 2], zoom: f64) -> Self {
        let mut frame = Self {
            segments: Vec::new(),
            nodes: Vec::new(),
        };
        for source in sources {
            if source.trajectory {
                for pair in source.points.windows(2) {
                    if pair[0].position != pair[1].position
                        && pair
                            .iter()
                            .all(|p| finite(p.position) && p.fields[2].is_finite())
                    {
                        frame.segments.push(Segment {
                            points: [pair[0].position, pair[1].position],
                            argb: math::trajectory_color(
                                source.argb,
                                pair[0].fields[2],
                                pair[1].fields[2],
                            ),
                        });
                    }
                }
            }
            for point in source.points {
                if let Ok(command) = math::point_commands(
                    point.position,
                    probe,
                    source.argb,
                    point.fields,
                    source.trajectory,
                    zoom,
                ) {
                    frame.nodes.push(Node {
                        source: source.index,
                        index: point.index,
                        selected: source.selected,
                        command,
                    });
                }
            }
        }
        frame
    }

    pub fn cache_key(&self) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        let point = |p: [f32; 2], h: &mut std::collections::hash_map::DefaultHasher| {
            p.map(f32::to_bits).hash(h)
        };
        for segment in &self.segments {
            for p in segment.points {
                point(p, &mut hash);
            }
            segment.argb.hash(&mut hash);
        }
        for node in &self.nodes {
            node.source.hash(&mut hash);
            node.index.hash(&mut hash);
            node.selected.hash(&mut hash);
            let c = &node.command;
            point(c.center, &mut hash);
            c.radius.to_bits().hash(&mut hash);
            c.argb.hash(&mut hash);
            c.ring_record.is_some().hash(&mut hash);
            if let Some(record) = c.ring_record {
                point(record.direction, &mut hash);
                record.argb.hash(&mut hash);
                record.weight.to_bits().hash(&mut hash);
            }
            for line in [&c.positive, &c.opposite] {
                line.is_some().hash(&mut hash);
                if let Some(line) = line {
                    point(line.from, &mut hash);
                    point(line.to, &mut hash);
                    point(line.gradient_end, &mut hash);
                    line.width.map(f32::to_bits).hash(&mut hash);
                    for stop in &line.stops {
                        stop.position.to_bits().hash(&mut hash);
                        stop.argb.hash(&mut hash);
                    }
                }
            }
            if let Some((r, w, c)) = c.octagon {
                r.to_bits().hash(&mut hash);
                w.to_bits().hash(&mut hash);
                c.hash(&mut hash);
            }
        }
        hash.finish()
    }

    pub fn paint(&self, painter: &egui::Painter) {
        for segment in &self.segments {
            painter.line_segment(
                segment.points.map(|p| pos2(p[0], p[1])),
                Stroke::new(1.5, color(segment.argb)),
            );
        }
        for node in &self.nodes {
            let c = &node.command;
            for line in [&c.positive, &c.opposite].into_iter().flatten() {
                paint_line(painter, line);
            }
            let center = pos2(c.center[0], c.center[1]);
            if c.radius.is_finite() && c.radius > 0.0 {
                painter.circle_filled(center, c.radius, color(c.argb));
                if node.selected {
                    painter.circle_filled(center, c.radius, super::look::WASH);
                    painter.circle_stroke(
                        center,
                        selection_radius(c),
                        Stroke::new(1.8, Color32::from_rgb(244, 247, 252)),
                    );
                }
            }
            if let Some((radius, width, argb)) = c.octagon.filter(|(r, w, _)| valid_stroke(*r, *w))
            {
                painter.add(egui::Shape::closed_line(
                    octagon(center, radius).to_vec(),
                    Stroke::new(width, color(argb)),
                ));
            }
        }
    }

    pub fn raster(&self, pixels: &mut [[f32; 3]], width: usize, height: usize, full: Rect) {
        if width == 0
            || height == 0
            || pixels.len() != width.saturating_mul(height)
            || !full.is_finite()
            || full.width() <= 0.0
            || full.height() <= 0.0
        {
            return;
        }
        let scale = width as f32 / full.width();
        let at = |p: [f32; 2]| pos2((p[0] - full.left()) * scale, (p[1] - full.top()) * scale);
        let solid = |pixels: &mut [[f32; 3]], a: Pos2, b: Pos2, argb: u32, w: f32| {
            let [r, g, b0, alpha] = rgba(argb);
            super::look::raster_line(
                pixels,
                width,
                height,
                a,
                b,
                [r, g, b0],
                f32::from(alpha) / 255.0,
                w * scale,
            );
        };
        for segment in &self.segments {
            solid(
                pixels,
                at(segment.points[0]),
                at(segment.points[1]),
                segment.argb,
                1.5,
            );
        }
        for node in &self.nodes {
            let c = &node.command;
            for line in [&c.positive, &c.opposite].into_iter().flatten() {
                raster_gradient(pixels, width, height, line, full, scale);
            }
            let center = at(c.center);
            if c.radius.is_finite() && c.radius > 0.0 {
                solid(pixels, center, center, c.argb, c.radius * 2.0);
                if node.selected {
                    let [r, g, b, a] = super::look::WASH.to_srgba_unmultiplied();
                    solid(
                        pixels,
                        center,
                        center,
                        u32::from_be_bytes([a, r, g, b]),
                        c.radius * 2.0,
                    );
                    let radius = selection_radius(c) * scale;
                    for i in 0..32 {
                        let a = i as f32 * std::f32::consts::TAU / 32.0;
                        let b = (i + 1) as f32 * std::f32::consts::TAU / 32.0;
                        super::look::raster_line(
                            pixels,
                            width,
                            height,
                            center + vec2(a.cos(), a.sin()) * radius,
                            center + vec2(b.cos(), b.sin()) * radius,
                            [244, 247, 252],
                            1.0,
                            1.8 * scale,
                        );
                    }
                }
            }
            if let Some((radius, w, argb)) = c.octagon.filter(|(r, w, _)| valid_stroke(*r, *w)) {
                let vertices = octagon(center, radius * scale);
                for i in 0..8 {
                    let [r, g, b, a] = rgba(argb);
                    super::look::raster_line(
                        pixels,
                        width,
                        height,
                        vertices[i],
                        vertices[(i + 1) % 8],
                        [r, g, b],
                        f32::from(a) / 255.0,
                        w * scale,
                    );
                }
            }
        }
    }
}

fn selection_radius(c: &math::PointCommands) -> f32 {
    c.radius + if c.octagon.is_some() { 2.95 } else { 1.25 }
}
fn valid_stroke(radius: f32, width: f32) -> bool {
    radius.is_finite() && radius > 0.0 && width.is_finite() && width > 0.0
}
fn octagon(center: Pos2, radius: f32) -> [Pos2; 8] {
    std::array::from_fn(|i| {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        center + vec2(a.cos(), a.sin()) * radius
    })
}

fn sample(line: &math::Line, t: f32) -> [f32; 4] {
    let stops = &line.stops;
    if t <= stops[0].position {
        return rgba(stops[0].argb).map(f32::from);
    }
    for pair in stops.windows(2) {
        if t <= pair[1].position {
            let q =
                ((t - pair[0].position) / (pair[1].position - pair[0].position)).clamp(0.0, 1.0);
            let a = rgba(pair[0].argb);
            let b = rgba(pair[1].argb);
            return std::array::from_fn(|i| {
                f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * q
            });
        }
    }
    rgba(stops.last().expect("gradient stops").argb).map(f32::from)
}

fn line_geometry(line: &math::Line) -> Option<(Pos2, Vec2, Vec2, f32, f32)> {
    if ![line.from, line.to, line.gradient_end]
        .into_iter()
        .all(finite)
        || line.stops.len() < 2
    {
        return None;
    }
    let from = pos2(line.from[0], line.from[1]);
    let path = pos2(line.to[0], line.to[1]) - from;
    let gradient = pos2(line.gradient_end[0], line.gradient_end[1]) - from;
    let length = path.length();
    let g2 = gradient.length_sq();

    let width = line.width.unwrap_or(1.0);
    if !length.is_finite()
        || length <= 0.0
        || !g2.is_finite()
        || g2 <= 0.0
        || !width.is_finite()
        || width <= 0.0
    {
        return None;
    }
    Some((from, path, gradient, g2, width))
}

fn paint_line(painter: &egui::Painter, line: &math::Line) {
    let Some((from, path, gradient, g2, width)) = line_geometry(line) else {
        return;
    };
    let slope = path.dot(gradient) / g2;
    if !slope.is_finite() || slope <= 0.0 {
        return;
    }
    let mut positions = vec![0.0, 1.0];
    for stop in &line.stops {
        let q = stop.position / slope;
        if q > 0.0 && q < 1.0 {
            positions.push(q);
        }
    }
    positions.sort_by(f32::total_cmp);
    positions.dedup();
    let normal = vec2(-path.y, path.x) / path.length() * (width * 0.5);
    let mut mesh = egui::Mesh::default();
    for (i, q) in positions.into_iter().enumerate() {
        let at = from + path * q;
        let rgba = sample(line, q * slope).map(|v| v.round().clamp(0.0, 255.0) as u8);
        let c = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
        mesh.colored_vertex(at - normal, c);
        mesh.colored_vertex(at + normal, c);
        if i > 0 {
            let n = i as u32 * 2;
            mesh.add_triangle(n - 2, n - 1, n);
            mesh.add_triangle(n - 1, n + 1, n);
        }
    }
    painter.add(mesh);
}

fn raster_gradient(
    pixels: &mut [[f32; 3]],
    width: usize,
    height: usize,
    line: &math::Line,
    full: Rect,
    scale: f32,
) {
    let Some((from, path, gradient, g2, line_width)) = line_geometry(line) else {
        return;
    };
    let a = (from - full.min) * scale;
    let b = a + path * scale;
    let radius = line_width * scale * 0.5;
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
            let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
            let q = ((p - a).dot(segment) / segment.length_sq()).clamp(0.0, 1.0);
            let distance = (p - (a + segment * q)).length();
            let coverage = (radius + 0.65 - distance).clamp(0.0, 1.0);
            let world = full.min + p / scale;
            let rgba = sample(line, (world - from).dot(gradient) / g2);
            let alpha = coverage * rgba[3] / 255.0;
            let pixel = &mut pixels[y * width + x];
            for i in 0..3 {
                pixel[i] = pixel[i] * (1.0 - alpha) + rgba[i] * alpha;
            }
        }
    }
}

