use super::*;
use native_tone::edges::pipeline::{self as pad, controls as pad_controls};

const PANEL_WIDTH: i32 = 320;
const PANEL_HEIGHT: i32 = 384;
const SURFACE_ALPHA: u8 = 255;

fn panel_rect(origin: Pos2, rect: pad::Rect) -> Rect {
    Rect::from_min_size(origin + vec2(rect.x, rect.y), vec2(rect.width, rect.height))
}

fn tone_button(ui: &mut egui::Ui, rect: Rect, id: &str, text: &str, enabled: bool,
    tip: &str, show_tooltips: bool) -> egui::Response {
    let text = localization::tr(ui.ctx(), text);
    let response = ui.interact(rect, ui.id().with(id),
        if enabled { Sense::click() } else { Sense::hover() });
    response.widget_info(|| egui::WidgetInfo::labeled(
        egui::WidgetType::Button, enabled && ui.is_enabled(), text));
    ui.painter().rect_filled(rect, 12,
        Color32::from_black_alpha(if enabled && response.hovered() { 75 } else { 45 }));
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text,
        FontId::proportional(13.0), if enabled { TEXT } else { MUTED });
    optional_tooltip(response, show_tooltips, tip)
}

impl DesktopApp {
    fn native_color_preview(&mut self) {
        if let Ok([red, green, blue]) = crate::tone::TonePoint::display_color(self.tone_controls())
        {
            self.color = [red, green, blue];
        }
    }

    pub(super) fn sync_native_color_label(&mut self) {
        self.native_color_preview();
        self.native_tone_hex.sync_from_controls(self.color);
    }

    fn commit_native_rgb(&mut self, rgb: [u8; 3]) {
        let controls = pad_controls::controls_from_rgb(rgb);
        self.native_tone_controls = Some([controls.gender, controls.tone, controls.hue]);
        self.tone_position = Point::new(controls.tone as f64, (1.0 - controls.gender) as f64);
        self.tone_hue = controls.hue as f64 * 360.0;
        self.native_color_preview();
    }

    pub(super) fn commit_tone_hex(&mut self) {
        if let Some(rgb) = self.native_tone_hex.commit() {
            self.commit_native_rgb(rgb);
        }
    }

    fn native_hex_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let hex = self.native_tone_hex.show(ui, rect);
        if hex.began {

            self.tone_epoch = self.tone_epoch.wrapping_add(1);
            ui.ctx().request_repaint();
        }
        if let Some(rgb) = hex.rgb {
            self.commit_native_rgb(rgb);
            ui.ctx().request_repaint();
        }
    }

    pub(super) fn native_color_ui(&mut self, ctx: &egui::Context, full: Rect) {
        let message_open = self.settings || self.error.is_some() || self.notice.is_some();
        let layout =
            pad::Layout::new(PANEL_WIDTH, PANEL_HEIGHT).expect("positive panel dimensions");
        let rect =
            Rect::from_center_size(full.center(), vec2(PANEL_WIDTH as f32, PANEL_HEIGHT as f32));
        self.overlay_rects.push(rect);
        self.picker_rect = Some(rect);
        let display_scale = ctx.pixels_per_point();
        let hue = self.tone_controls()[2];
        let size = (PANEL_WIDTH as f32 * display_scale) as i32;
        let key = [
            hue.to_bits(),
            size as u32,
            display_scale.to_bits(),
            SURFACE_ALPHA as u32,
        ];
        if self.native_tone_texture_key != Some(key) {
            let Some(surface) = native_tone::render(hue, PANEL_WIDTH, display_scale, SURFACE_ALPHA)
            else {
                self.error =
                    Some("Could not create the Tone surface at this display scale.".into());
                self.color_picker = false;
                return;
            };
            let pixels = surface
                .final_bgra
                .into_iter()
                .map(|[blue, green, red, alpha]| {
                    Color32::from_rgba_premultiplied(red, green, blue, alpha)
                })
                .collect();
            let image = egui::ColorImage::new([surface.size as usize; 2], pixels);
            if let Some(texture) = &mut self.native_tone_texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.native_tone_texture = Some(ctx.load_texture(
                    "native-tone-surface",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            self.native_tone_texture_key = Some(key);
        }
        self.native_color_preview();
        egui::Area::new(egui::Id::new("color-picker"))
            .order(egui::Order::Foreground)
            .interactable(!self.settings)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!message_open, |ui| {
                ui.set_min_size(rect.size());
                let image_rect = panel_rect(rect.min, layout.image);
                if let Some(texture) = &self.native_tone_texture {
                    ui.painter().image(
                        texture.id(),
                        image_rect,
                        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                let response =
                    ui.interact(rect, ui.id().with("color-pad"), Sense::click_and_drag());
                if (response.clicked()
                    || response.dragged()
                    || response.is_pointer_button_down_on())
                    && let Some(current) = response.interact_pointer_pos()
                    && let Some(down) = ctx.input(|input| input.pointer.press_origin())
                    && let Some(controls) = layout.drag(
                        [down.x - rect.left(), down.y - rect.top()],
                        [current.x - rect.left(), current.y - rect.top()],
                        self.tone_controls()[2],
                    )
                {
                    self.native_tone_controls =
                        Some([controls.gender, controls.tone, controls.hue]);
                    self.tone_position =
                        Point::new(controls.tone as f64, (1.0 - controls.gender) as f64);
                    self.sync_native_color_label();
                }
                let [gender, tone, hue] = self.tone_controls();
                let thumb = layout.thumb(pad_controls::Controls { gender, tone, hue });
                let thumb = rect.min + vec2(thumb[0], thumb[1]);
                ui.painter().circle_stroke(
                    thumb,
                    pad::THUMB_RING_DIAMETER * 0.5,
                    Stroke::new(pad::THUMB_RING_STROKE, Color32::WHITE),
                );
                ui.painter()
                    .circle_filled(thumb, pad::THUMB_DOT_DIAMETER * 0.5, Color32::WHITE);
                let label = panel_rect(rect.min, layout.label);
                let label_color = Color32::from_white_alpha(pad::LABEL_ALPHA);
                let font = FontId::proportional(10.0);
                ui.painter().text(
                    label.left_center(),
                    Align2::LEFT_CENTER,
                    pad::LEFT_LABEL,
                    font.clone(),
                    label_color,
                );
                ui.painter().text(
                    label.right_center(),
                    Align2::RIGHT_CENTER,
                    pad::RIGHT_LABEL,
                    font.clone(),
                    label_color,
                );
                let galley = ui
                    .painter()
                    .layout_no_wrap("TONE".into(), font, label_color);
                let text_origin = pos2(image_rect.right() - 20.0, image_rect.top() + 36.0);
                ui.painter().add(
                    egui::epaint::TextShape::new(text_origin, galley, label_color)
                        .with_angle(-std::f32::consts::FRAC_PI_2),
                );
                let back_response = icon_button(
                    ui,
                    Rect::from_min_size(rect.min, vec2(24.0, 24.0)),
                    "color-close",
                    Icon::Back,
                    "Close",
                    self.show_tooltips,
                );
                if back_response.clicked() {
                    back_response.surrender_focus();
                    self.color_picker = false;
                    return;
                }
                if tone_button(
                    ui,
                    panel_rect(rect.min, layout.random),
                    "random-tone",
                    "Random",
                    true,
                    "Pick a random hue for the voice timbre.",
                    self.show_tooltips,
                )
                .clicked()
                {
                    let entropy = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.subsec_nanos())
                        .unwrap_or(0);
                    self.tone_hue = entropy as f64 / 1_000_000_000.0 * 360.0;
                    let mut controls = self.tone_controls();
                    controls[2] = (self.tone_hue / 360.0) as f32;
                    self.native_tone_controls = Some(controls);
                    self.sync_native_color_label();
                    ctx.request_repaint();
                }
                self.native_hex_ui(ui, panel_rect(rect.min, layout.hex));
                if tone_button(
                    ui,
                    panel_rect(rect.min, layout.apply),
                    "apply-color",
                    if self.generation_busy() {
                        "Working..."
                    } else {
                        "Apply"
                    },
                    !self.generation_busy(),
                    "Save the current voice as a new item and return to the main interface.",
                    self.show_tooltips,
                )
                .clicked()
                {
                    self.start_tone_generation(ctx);
                }
                });
            });
    }
}
