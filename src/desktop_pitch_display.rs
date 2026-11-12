use super::*;

impl DesktopApp {
    pub(super) fn paint_pitch_capsule(&self, painter: &egui::Painter, index: usize, label: Rect) {
        if !self.monitor_controls.pitch_open() { return; }
        let Some(source) = self.preset.sources.get(index).filter(|s| s.enabled && s.kind == "trajectory") else { return; };
        let document = self.preset.document.get("voice").unwrap_or(&self.preset.document);
        let Some(row) = document.get("sources").and_then(Value::as_array).and_then(|v| v.get(index)) else { return; };
        let Some(stats) = crate::pitch_display::Statistics::from_source(row) else { return; };
        let bounds = Rect::from_center_size(pos2(label.center().x, label.bottom() + 5.0), vec2(60.0, 9.0));
        let paint = painter.with_clip_rect(painter.clip_rect());
        paint.rect_filled(bounds, 4.5, Color32::from_white_alpha(27));
        let [r,g,b] = parse_color(&source.color);
        let left = bounds.left() + stats.position(stats.0[1]).unwrap_or(0.0) * bounds.width();
        let right = bounds.left() + stats.position(stats.0[3]).unwrap_or(1.0) * bounds.width();
        if right > left {
            paint.rect_filled(Rect::from_min_max(pos2(left,bounds.top()+1.0),pos2(right,bounds.bottom()-1.0)),
                3.5, Color32::from_rgba_unmultiplied(r,g,b,128));
        }
        if let Some(note) = crate::pitch_display::midi(self.monitor_controls.pitch_sample()) {
            let x = bounds.left() + stats.position(note).unwrap_or(0.5) * bounds.width();
            paint.line_segment([pos2(x,bounds.top()+1.0),pos2(x,bounds.bottom()-1.0)],
                Stroke::new(1.5, Color32::from_rgb(217,217,218)));
        }
    }
}
