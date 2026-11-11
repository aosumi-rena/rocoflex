use eframe::egui::{self, Color32, Pos2, Rect, Stroke, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Icon {
    Gear,
    Chevron,
    Reset,
    More,
    Edit,
    Trash,
    Close,
    Info,
    Point,
    Audio,
    Folder,
    Check,
    Back,
    Keyboard,
    TuningFork,
    Music,
    Magic,
    Globe,
    Help,
    Warning,
    Pin,
    PinAdd,
    Reroute,
    Eraser,
    Pen,
    Exclamation,
}

pub(super) fn icon(painter: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let extent = rect.width().min(rect.height());
    if !rect.is_finite() || extent <= 0.0 || color == Color32::TRANSPARENT {
        return;
    }
    let ink = Ink {
        painter,
        center: rect.center(),
        scale: extent / 24.0,
        color,
    };
    match kind {
        Icon::Gear => {

            let mut points = Vec::with_capacity(30);
            for tooth in 0..6 {
                for (offset, radius) in [(-0.5, 7.5), (-0.3, 7.5), (-0.22, 9.5),
                    (0.22, 9.5), (0.3, 7.5)] {
                    let angle = (tooth as f32 + offset) * std::f32::consts::FRAC_PI_3
                        - std::f32::consts::FRAC_PI_2;
                    points.push((angle.cos() * radius, angle.sin() * radius));
                }
            }
            ink.outline(&points);
            ink.ring((0.0, 0.0), 3.1);
        }
        Icon::Chevron => ink.path(&[(-4.6, -2.2), (0.0, 2.4), (4.6, -2.2)]),
        Icon::Edit => {
            ink.outline(&[(-8.7, 8.7), (-6.6, 2.6), (5.2, -9.1),
                (9.1, -5.2), (-2.6, 6.6)]);
            ink.line((2.2, -6.1), (6.1, -2.2));
            ink.line((-6.6, 2.6), (-2.6, 6.6));
        }
        Icon::Trash => {
            ink.line((-8.0, -5.8), (8.0, -5.8));
            ink.path(&[(-3.8, -5.8), (-2.4, -8.8), (2.4, -8.8), (3.8, -5.8)]);
            ink.path(&[(-6.2, -5.8), (-5.2, 8.5), (5.2, 8.5), (6.2, -5.8)]);
            ink.line((-2.0, -2.4), (-1.6, 5.2));
            ink.line((2.0, -2.4), (1.6, 5.2));
        }
        Icon::Close => {
            ink.line((-6.3, -6.3), (6.3, 6.3));
            ink.line((6.3, -6.3), (-6.3, 6.3));
        }
        Icon::Info => {

            ink.dot((0.0, 0.0), 8.8);
            let glyph = Ink { painter, center: ink.center, scale: ink.scale,
                color: Color32::from_rgb(70, 70, 77) };
            glyph.dot((0.0, -4.2), 1.0);
            glyph.path(&[(-1.4, -0.8), (0.5, -0.8), (0.5, 4.8)]);
            glyph.line((-1.6, 4.8), (2.4, 4.8));
        }
        Icon::Point => {

            let scale = extent / 28.0;
            let origin = rect.center() - vec2(14.0, 14.0) * scale;
            let at = |x: f32, y: f32| origin + vec2(x, y) * scale;
            let points = (0..8).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                at(14.0 + 9.0 * angle.sin(), 18.0 - 9.0 * angle.cos())
            }).collect();
            painter.add(egui::Shape::closed_line(points, Stroke::new(scale, color)));
            painter.circle_filled(at(14.0, 18.0), 2.0 * scale, color);
            painter.rect_filled(Rect::from_min_size(at(5.0, 3.0), vec2(1.0, 7.0) * scale), 0, color);
            painter.rect_filled(Rect::from_min_size(at(2.0, 6.0), vec2(7.0, 1.0) * scale), 0, color);
        }
        Icon::Audio => {
            let scale = extent / 28.0;
            let origin = rect.center() - vec2(14.0, 14.0) * scale;
            let at = |x: f32, y: f32| origin + vec2(x, y) * scale;
            let points = [(9.0, 10.0), (14.0, 7.0), (19.0, 10.0),
                (20.0, 16.0), (15.0, 21.0), (9.0, 22.0)];
            painter.add(egui::Shape::line(points.iter().map(|&(x, y)| at(x + 1.0, y + 1.0)).collect(),
                Stroke::new(scale, color)));
            for (x, y) in points { painter.circle_filled(at(x + 1.0, y + 1.0), 2.0 * scale, color); }
            painter.rect_filled(Rect::from_min_size(at(5.0, 2.0), vec2(1.0, 7.0) * scale), 0, color);
            painter.rect_filled(Rect::from_min_size(at(2.0, 5.0), vec2(7.0, 1.0) * scale), 0, color);
        }
        Icon::Folder => ink.outline(&[(-9.0, 7.2), (-9.0, -7.0),
            (-2.5, -7.0), (0.0, -4.5), (9.0, -4.5), (9.0, 7.2)]),
        Icon::Check => ink.path(&[(-7.4, -0.2), (-2.0, 5.3), (8.0, -5.7)]),
        Icon::Back => ink.path(&[(3.6, -6.6), (-3.0, 0.0), (3.6, 6.6)]),
        Icon::Keyboard => {
            ink.outline(&[(-9.6, -6.8), (9.6, -6.8), (9.6, 6.8), (-9.6, 6.8)]);
            for y in [-3.7, -0.4] {
                for x in [-6.5, -3.2, 0.1, 3.4, 6.7] {
                    ink.block((x - 0.7, y - 0.7), (x + 0.7, y + 0.7));
                }
            }
            ink.line((-4.2, 3.6), (4.2, 3.6));
            ink.dot((-6.8, 3.6), 0.7);
            ink.dot((6.8, 3.6), 0.7);
        }
        Icon::TuningFork => {

            let fork = Stroke::new(1.1 * ink.scale, color);
            let curve = |points: [(f32, f32); 4]| {
                painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
                    points.map(|point| ink.at(point)), false, Color32::TRANSPARENT, fork,
                )));
            };
            painter.line_segment([ink.at((6.76, -12.0)), ink.at((-3.71, -1.53))], fork);
            curve([(-3.71, -1.53), (-5.05, -0.08), (-5.03, 2.17), (-3.71, 3.56)]);
            curve([(-3.71, 3.56), (-2.34, 5.01), (0.02, 5.10), (1.53, 3.71)]);
            painter.line_segment([ink.at((1.53, 3.71)), ink.at((12.0, -6.76))], fork);
            painter.line_segment([ink.at((-3.76, 3.76)), ink.at((-9.02, 9.02))],
                Stroke::new(1.0 * ink.scale, color));
            painter.circle_stroke(ink.at((-10.25, 10.25)), 1.75 * ink.scale,
                Stroke::new(ink.scale, color));
        }
        Icon::Music => {
            ink.path(&[(-3.5, 5.6), (-3.5, -6.4), (7.1, -8.7), (7.1, 3.5)]);
            ink.line((-3.5, -2.8), (7.1, -5.1));
            ink.oval((-6.1, 6.1), (2.7, 1.9));
            ink.oval((4.5, 4.0), (2.7, 1.9));
        }
        Icon::Magic => {
            ink.outline(&[(-2.2, -4.7), (8.6, 6.1), (6.1, 8.6), (-4.7, -2.2)]);
            ink.line((-0.2, -2.7), (-2.7, -0.2));
            for (a, b) in [((-6.7, -7.0), (-8.2, -8.5)), ((-2.6, -7.5), (-2.6, -9.7)),
                ((1.0, -6.4), (2.6, -8.0)), ((-7.5, -2.6), (-9.7, -2.6)),
                ((-6.4, 1.0), (-8.0, 2.6))] {
                ink.line(a, b);
            }
        }
        Icon::Globe => {
            ink.ring((0.0, 0.0), 9.0);
            ink.curve([(0.0, -9.0), (-7.5, -4.0), (-7.5, 4.0), (0.0, 9.0)]);
            ink.curve([(0.0, -9.0), (7.5, -4.0), (7.5, 4.0), (0.0, 9.0)]);
            ink.line((-8.8, 0.0), (8.8, 0.0));
            ink.curve([(-7.2, -5.4), (-3.5, -3.7), (3.5, -3.7), (7.2, -5.4)]);
            ink.curve([(-7.2, 5.4), (-3.5, 3.7), (3.5, 3.7), (7.2, 5.4)]);
        }
        Icon::Help => {
            ink.curve([(-5.2, -4.7), (-5.0, -10.1), (6.7, -10.1), (5.6, -3.9)]);
            ink.curve([(5.6, -3.9), (5.0, -0.9), (0.0, -0.8), (0.0, 3.4)]);
            ink.dot((0.0, 7.4), 1.15);
        }
        Icon::Warning => {
            ink.outline(&[(0.0, -9.0), (9.4, 7.8), (-9.4, 7.8)]);
            ink.line((0.0, -3.4), (0.0, 1.7));
            ink.dot((0.0, 5.0), 0.95);
        }
        Icon::Pin | Icon::PinAdd | Icon::Reroute | Icon::More | Icon::Reset => {
            use super::original_glyphs::{self, Glyph};
            let glyph = match kind { Icon::Pin => Glyph::Waypoints, Icon::PinAdd => Glyph::AddWaypoint,
                Icon::Reroute => Glyph::RerouteWaypoints, Icon::More => Glyph::More, Icon::Reset => Glyph::Reset,
                _ => unreachable!("source glyph dispatch") };
            original_glyphs::paint(painter, rect, glyph, color);
        }
        Icon::Eraser => {
            ink.outline(&[(-8.6, 2.7), (2.4, -8.3), (8.5, -2.2),
                (-2.4, 8.7), (-5.0, 8.7), (-8.6, 5.1)]);
            ink.line((-4.3, -1.6), (1.8, 4.5));
        }
        Icon::Pen => {
            ink.outline(&[(-9.0, 9.0), (-5.4, -1.9), (5.6, -9.0),
                (9.0, -5.6), (1.9, 5.4)]);
            ink.line((-9.0, 9.0), (0.0, 0.0));
            ink.ring((0.6, -0.6), 1.0);
        }
        Icon::Exclamation => {
            ink.line((0.0, -8.4), (0.0, 2.7));
            ink.dot((0.0, 7.5), 1.3);
        }
    }
}

struct Ink<'a> {
    painter: &'a egui::Painter,
    center: Pos2,
    scale: f32,
    color: Color32,
}

impl Ink<'_> {
    fn at(&self, point: (f32, f32)) -> Pos2 {
        self.center + vec2(point.0, point.1) * self.scale
    }

    fn stroke(&self) -> Stroke {
        Stroke::new(1.6 * self.scale, self.color)
    }

    fn line(&self, a: (f32, f32), b: (f32, f32)) {
        self.path(&[a, b]);
    }

    fn path(&self, points: &[(f32, f32)]) {
        self.painter.add(egui::Shape::line(
            points.iter().map(|&point| self.at(point)).collect(),
            self.stroke(),
        ));
    }

    fn outline(&self, points: &[(f32, f32)]) {
        self.painter.add(egui::Shape::closed_line(
            points.iter().map(|&point| self.at(point)).collect(),
            self.stroke(),
        ));
    }

    fn curve(&self, points: [(f32, f32); 4]) {
        self.painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
            points.map(|point| self.at(point)), false, Color32::TRANSPARENT, self.stroke(),
        )));
    }

    fn ring(&self, center: (f32, f32), radius: f32) {
        self.painter.circle_stroke(self.at(center), radius * self.scale, self.stroke());
    }

    fn dot(&self, center: (f32, f32), radius: f32) {
        self.painter.circle_filled(self.at(center), radius * self.scale, self.color);
    }

    fn block(&self, min: (f32, f32), max: (f32, f32)) {
        self.painter.rect_filled(Rect::from_min_max(self.at(min), self.at(max)), 0.0, self.color);
    }

    fn oval(&self, center: (f32, f32), radius: (f32, f32)) {
        let points = (0..24).map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 24.0;
            self.at((center.0 + radius.0 * angle.cos(), center.1 + radius.1 * angle.sin()))
        }).collect();
        self.painter.add(egui::Shape::convex_polygon(points, self.color, Stroke::NONE));
    }
}

