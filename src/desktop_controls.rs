use super::*;

const PROJECT_DESCRIPTION: &str = "A voice morphing engine and editor developed with Rust + egui.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceNameNavigation {
    Forward,
    Backward,
    Uncertain,
}

#[derive(Clone, Copy)]
struct SourceNameFocusPass {
    pass: u64,
    owned: bool,
    navigation: Option<SourceNameNavigation>,
}

fn settings_panel_rect(full: Rect) -> Rect {
    Rect::from_center_size(full.center(), vec2(540.0, 380.0))
}

pub(super) fn modal_panel<R>(
    ctx: &egui::Context, id: &str, rect: Rect, backdrop: Color32,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {

    egui::Modal::new(egui::Id::new(id))
        .area(egui::Area::new(egui::Id::new(id))
            .kind(egui::UiKind::Modal).order(egui::Order::Foreground)
            .fixed_pos(rect.min).sense(Sense::hover()))
        .frame(egui::Frame::NONE).backdrop_color(backdrop).show(ctx, content)
}

fn rail_button(
    ui: &mut egui::Ui, rect: Rect, id: &str, trajectory: bool, tip: &str,
    show_tooltips: bool,
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    let translated_tip = localization::tr(ui.ctx(), tip);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), translated_tip));
    let color = if !ui.is_enabled() { MUTED.gamma_multiply(0.5) }
        else if response.hovered() || response.has_focus() { Color32::from_rgb(224, 224, 226) }
        else { Color32::from_rgb(183, 184, 187) };
    icon(
        ui.painter(), rect,
        if trajectory { Icon::Audio } else { Icon::Point }, color,
    );
    optional_tooltip(response, show_tooltips, tip)
}

fn menu_item_semantics(response: &egui::Response, enabled: bool, label: &str, selected: Option<bool>) {
    response.widget_info(|| match selected {
        Some(selected) => egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, selected, label),
        None => egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label),
    });

    let _ = response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(if selected.is_some() {
            egui::accesskit::Role::MenuItemRadio
        } else {
            egui::accesskit::Role::MenuItem
        });
        if let Some(selected) = selected {
            node.set_toggled(if selected { egui::accesskit::Toggled::True } else { egui::accesskit::Toggled::False });
        }
    });
}

fn menu_trigger_semantics(response: &egui::Response, expanded: bool) {
    let _ = response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_has_popup(egui::accesskit::HasPopup::Menu);
        node.set_expanded(expanded);
    });
}

fn menu_container_semantics(ui: &egui::Ui, label: &str) {
    let translated_label = localization::tr(ui.ctx(), label);
    let _ = ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(egui::accesskit::Role::Menu);
        node.set_label(translated_label);
    });
}

fn preset_menu_row(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    text: &str,
) -> egui::Response {
    let text = localization::tr(ui.ctx(), text);
    preset_menu_row_labelled(ui, rect, id, text, text, None)
}

fn preset_menu_row_labelled(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    text: &str,
    label: &str,
    selected: Option<bool>,
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    menu_item_semantics(&response, ui.is_enabled(), label, selected);
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect.shrink2(vec2(3.0, 0.0)),
            3,
            Color32::from_rgba_unmultiplied(91, 126, 178, 65),
        );
    }
    ui.painter().with_clip_rect(ui.clip_rect().intersect(rect)).text(
        rect.left_center() + vec2(25.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        TEXT,
    );
    response
}

const SETTINGS_REFERENCE_LABEL: Color32 = Color32::from_rgb(183, 184, 187);
const SETTINGS_REFERENCE_HIGHLIGHT: Color32 = Color32::from_rgb(224, 224, 226);
const SETTINGS_SHORTCUT_COLUMN: Color32 = Color32::from_rgb(152, 153, 155);
const SETTINGS_SHORTCUT_VALUE: Color32 = Color32::from_rgb(189, 189, 189);

pub(super) fn settings_section(ui: &mut egui::Ui, width: f32, text: &str) {
    settings_section_colored(ui, width, text, TEXT);
}

fn settings_section_colored(ui: &mut egui::Ui, width: f32, text: &str, color: Color32) {
    let text = localization::tr(ui.ctx(), text);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 20.0), Sense::hover());
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(), FontId::proportional(14.0), color,
    );
    let right = rect.left() + galley.size().x + 10.0;
    ui.painter().galley(
        pos2(rect.left(), rect.center().y - galley.size().y * 0.5), galley, color,
    );
    if right < rect.right() {
        ui.painter().line_segment(
            [pos2(right, rect.center().y), rect.right_center()],
            Stroke::new(0.7, Color32::from_white_alpha(65)),
        );
    }
}

pub(super) fn settings_row(ui: &mut egui::Ui, width: f32, text: &str) -> Rect {
    settings_row_colored(ui, width, text, SETTINGS_REFERENCE_LABEL)
}

fn settings_row_colored(ui: &mut egui::Ui, width: f32, text: &str, color: Color32) -> Rect {
    let text = localization::tr(ui.ctx(), text);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    ui.painter().text(
        rect.left_center(), Align2::LEFT_CENTER, text,
        FontId::proportional(13.0), color,
    );
    rect
}

pub(super) fn settings_value_rect(row: Rect) -> Rect {
    Rect::from_center_size(
        pos2(row.right() - 151.5, row.center().y), vec2(303.0, 24.0),
    )
}

fn settings_shortcut_rects(row: Rect) -> [Rect; 2] {
    let value = settings_value_rect(row);
    [
        Rect::from_min_max(value.min, pos2(value.center().x - 4.0, value.bottom())),
        Rect::from_min_max(pos2(value.center().x + 4.0, value.top()), value.max),
    ]
}

#[derive(Clone, Copy)]
pub(super) enum SettingsValueStyle { Default, AudioSystem }

pub(super) fn setting_value(
    ui: &mut egui::Ui, row: Rect, label: &str, value: &str, explanation: &str, show_tooltips: bool,
    dropdown: bool, style: SettingsValueStyle,
) {
    let label = localization::tr(ui.ctx(), label);
    let value = localization::tr(ui.ctx(), value);
    let rect = settings_value_rect(row);
    ui.painter().rect_filled(rect, 12, Color32::from_black_alpha(38));
    let is_audio_system = matches!(style, SettingsValueStyle::AudioSystem);
    let inset = if is_audio_system { 6.0 } else { 12.0 };
    let text_rect = Rect::from_min_max(rect.min + vec2(inset, 0.0),
        rect.max - vec2(if dropdown { 26.0 } else { 12.0 }, 0.0));
    ui.painter().with_clip_rect(ui.clip_rect().intersect(text_rect)).text(
        rect.left_center() + vec2(inset, 0.0), Align2::LEFT_CENTER,
        value, FontId::proportional(13.0),
        if is_audio_system && ui.is_enabled() { SETTINGS_REFERENCE_LABEL } else { MUTED },
    );
    if dropdown {
        icon(ui.painter(), Rect::from_center_size(
            pos2(rect.right() - 12.0, rect.center().y), vec2(12.0, 12.0),
        ), Icon::Chevron, MUTED);
    }
    let response = ui.interact(rect, ui.id().with(("unavailable-setting", label)), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(
        egui::WidgetType::Label, ui.is_enabled(), format!("{label}: {value}"),
    ));
    optional_tooltip(response, show_tooltips, explanation);
}

pub(super) fn settings_field_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.interact_size.y = 24.0;
    style.spacing.button_padding = vec2(12.0, 3.0);
    style.spacing.icon_width = 12.0;
    style.spacing.icon_spacing = 6.0;
    style.spacing.menu_margin = egui::Margin::same(0);
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style.visuals.extreme_bg_color = Color32::from_black_alpha(38);
    style.visuals.window_fill = Color32::from_rgb(70, 70, 77);
    style.visuals.window_stroke = Stroke::NONE;
    style.visuals.menu_corner_radius = 3.into();
    fn field(visual: &mut egui::style::WidgetVisuals, alpha: u8) {
        visual.corner_radius = 12.into();
        visual.weak_bg_fill = Color32::from_black_alpha(alpha);
        visual.bg_fill = Color32::from_black_alpha(alpha);
        visual.bg_stroke = Stroke::NONE;
        visual.expansion = 0.0;
    }
    field(&mut style.visuals.widgets.inactive, 38);
    field(&mut style.visuals.widgets.hovered, 55);
    field(&mut style.visuals.widgets.active, 65);
    field(&mut style.visuals.widgets.open, 55);
    field(&mut style.visuals.widgets.noninteractive, 38);
}

pub(super) fn settings_combo_icon(ui: &egui::Ui, rect: Rect,
    visuals: &egui::style::WidgetVisuals, _open: bool)
{
    let extent = rect.width().min(rect.height());
    if !rect.is_finite() || extent <= 0.0 { return; }
    let half = (extent * 0.32).min(4.0);
    let center = rect.center();
    ui.painter().add(egui::Shape::line(vec![
        center + vec2(-half, -half * 0.5),
        center + vec2(0.0, half * 0.45),
        center + vec2(half, -half * 0.5),
    ], Stroke::new((extent * 0.12).clamp(0.8, 1.5), visuals.text_color())));
}

pub(super) fn settings_reference_combo_icon(ui: &egui::Ui, rect: Rect,
    visuals: &egui::style::WidgetVisuals, open: bool)
{
    let mut visuals: egui::style::WidgetVisuals = (*visuals).clone();
    if ui.is_enabled() && !open && visuals.text_color() == ui.visuals().widgets.inactive.text_color() {
        visuals.fg_stroke.color = SETTINGS_REFERENCE_HIGHLIGHT;
    }

    settings_combo_icon(ui, rect.translate(vec2(-2.0, 0.0)), &visuals, open);
}

pub(super) fn settings_reference_text(text: impl Into<String>, enabled: bool) -> egui::WidgetText {
    let text = text.into();
    if enabled { egui::RichText::new(text).color(SETTINGS_REFERENCE_LABEL).into() }
    else { text.into() }
}

pub(super) fn settings_option(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let translated = localization::tr(ui.ctx(), text).to_owned();
    settings_option_raw(ui, &translated, selected)
}

pub(super) fn settings_option_raw(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let translated = text.to_owned();
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 2.0, Color32::from_white_alpha(if response.hovered() { 22 } else { 12 }));
    }
    if selected {
        icon(ui.painter(), Rect::from_center_size(rect.left_center() + vec2(12.0, 0.0), vec2(14.0, 14.0)),
            Icon::Check, TEXT);
    }
    ui.painter().text(rect.left_center() + vec2(24.0, 0.0), Align2::LEFT_CENTER,
        &translated, FontId::proportional(13.0), TEXT);
    response.widget_info(|| egui::WidgetInfo::selected(
        egui::WidgetType::SelectableLabel, ui.is_enabled(), selected, &translated));
    if response.clicked() {
        ui.close_kind(egui::UiKind::Menu);
    }
    response
}

pub(super) fn settings_button(ui: &mut egui::Ui, rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug, text: &str, enabled: bool) -> egui::Response
{
    let translated = localization::tr(ui.ctx(), text).to_owned();
    settings_button_raw(ui, rect, id, &translated, enabled)
}

pub(super) fn settings_button_raw(ui: &mut egui::Ui, rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug, text: &str, enabled: bool) -> egui::Response
{
    let active = enabled && ui.is_enabled();
    let response = ui.interact(rect, ui.id().with(id),
        if active { Sense::click() } else { Sense::hover() });
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, active, text));
    ui.painter().rect_filled(rect, 12.0,
        Color32::from_black_alpha(if active && response.hovered() { 55 } else { 38 }));
    ui.painter().with_clip_rect(ui.clip_rect().intersect(rect)).text(rect.center(), Align2::CENTER_CENTER,
        text, FontId::proportional(13.0), if active { TEXT } else { MUTED });
    response
}

pub(super) fn settings_check(ui: &mut egui::Ui, rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug, label: &str, checked: &mut bool, enabled: bool,
) -> egui::Response {
    let translated_label = localization::tr(ui.ctx(), label).to_owned();
    settings_check_raw(ui, rect, id, &translated_label, checked, enabled)
}

pub(super) fn settings_check_raw(ui: &mut egui::Ui, rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug, label: &str, checked: &mut bool, enabled: bool,
) -> egui::Response {
    settings_check_with_mark(ui, rect, id, label, checked, enabled, 3.0, TEXT)
}

fn settings_check_with_mark(ui: &mut egui::Ui, rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug, label: &str, checked: &mut bool, enabled: bool,
    mark_inset: f32, mark_color: Color32,
) -> egui::Response {
    let active = enabled && ui.is_enabled();
    let mut response = ui.interact(rect, ui.id().with(id),
        if active { Sense::click() } else { Sense::hover() });
    if active && response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(
        egui::WidgetType::Checkbox, active, *checked, label));
    ui.painter().circle_filled(rect.center(), 12.0, Color32::from_black_alpha(43));
    if *checked {
        icon(ui.painter(), rect.shrink(mark_inset), Icon::Check, if active { mark_color } else { MUTED });
    }
    response
}

fn settings_toggle(
    ui: &mut egui::Ui, row: Rect, id: &str, label: &str, checked: &mut bool, enabled: bool,
) -> egui::Response {
    let rect = Rect::from_center_size(
        pos2(row.right() - 12.0, row.center().y), vec2(24.0, 24.0),
    );
    let translated_label = localization::tr(ui.ctx(), label).to_owned();
    let response = settings_check_with_mark(ui, rect, id, &translated_label, checked, enabled,
        6.0, SETTINGS_REFERENCE_HIGHLIGHT);
    if !enabled {
        ui.painter().text(
            rect.left_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER,
            localization::tr(ui.ctx(), "Unavailable"), FontId::proportional(12.0), MUTED,
        );
    }
    response
}

fn editor_name_allowed(edit: Edit, input: &str) -> bool {
    if matches!(edit, Edit::SourceName(_)) {
        return input.as_bytes().first().is_some_and(|b| *b != 0);
    }
    let name = input.trim();
    edit == Edit::DeletePreset || (!name.is_empty() && name != "My Preset")
}

impl DesktopApp {
    pub(super) fn header(&mut self, ctx: &egui::Context, full: Rect) {
        if self.layout_blocked() {
            return;
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let name_editor_open = self.editor.is_some();
        if !self.settings && !message_open && !name_editor_open {
            let id = egui::Id::new("settings-modal");
            if egui::AreaState::load(ctx, id).is_none_or(|state| state.size.is_none()) {

                let rect = settings_panel_rect(full);
                let _ = egui::Area::new(id)
                    .order(egui::Order::Foreground)
                    .fixed_pos(rect.min).sense(Sense::hover())
                    .interactable(false).enabled(false)
                    .show(ctx, |ui| {
                        ui.set_invisible();
                        ui.set_min_size(rect.size());
                    });
            }
        }
        let default_preset_name = if self.path.is_none() && self.preset.name == "My Preset" {
            Some(localization::tr(ctx, "My Preset"))
        } else { None };
        let mut title = clipped_title(default_preset_name.unwrap_or(&self.preset.name), 24);
        let font = FontId::proportional(13.0);

        let dimensions = [full.width() as i32, full.height() as i32];
        let mapped = |rect: super::header_glyph_layout::RectI32| Rect::from_min_size(
            full.min + vec2(rect.x as f32, rect.y as f32), vec2(rect.width as f32, rect.height as f32));
        let bar = mapped(super::header_glyph_layout::parent_rect(dimensions[0], dimensions[1]));
        let settings = mapped(super::header_glyph_layout::settings_rect(dimensions[0], dimensions[1]));
        let width = bar.width();
        while ctx.fonts_mut(|f| f.layout_no_wrap(title.clone(), font.clone(), TEXT).size().x)
            > width - 146.0
            && title.chars().count() > 7
        {
            let n = title.chars().count();
            title = format!("{}...", title.chars().take(n - 4).collect::<String>());
        }
        self.overlay_rects.push(bar);
        egui::Area::new(egui::Id::new("header"))
            .order(egui::Order::Foreground)
            .fixed_pos(full.min)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!message_open && !name_editor_open, |ui| {
                let available = |app: &Self| !app.layout_blocked() && app.editor.is_none()
                    && app.error.is_none() && app.notice.is_none();
                ui.set_min_size(vec2(full.width(), 45.0));
                brand(ui.painter(), full.min + vec2(19.0, 20.0), 17.0);
                if bar.width() > 0.0 && bar.height() > 0.0 {
                let header_painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(bar));
                let hovered = ctx.pointer_hover_pos().is_some_and(|p| bar.contains(p))
                    || matches!(self.menu, Some(Menu::Library | Menu::Actions));
                if hovered {
                    header_painter.rect_filled(
                        bar,
                        24,
                        Color32::from_rgba_unmultiplied(79, 80, 92, 150),
                    );
                }
                let title_rect =
                    Rect::from_center_size(bar.center(), vec2((width - 136.0).max(105.0), 24.0));
                let response =
                    ui.interact(title_rect.intersect(bar), ui.id().with("preset-title"), Sense::click());
                optional_tooltip(response.clone(), self.show_tooltips,
                    "Save and re-open the workspace as a preset, or load a predefined voice.");
                response.widget_info(|| egui::WidgetInfo::labeled(
                    egui::WidgetType::Button, ui.is_enabled(),
                    default_preset_name.unwrap_or(&self.preset.name),
                ));
                header_painter.text(
                    title_rect.center(),
                    Align2::CENTER_CENTER,
                    &title,
                    font,
                    TEXT,
                );
                if response.clicked() {
                    self.toggle_menu(Menu::Library);
                }
                menu_trigger_semantics(&response, self.menu == Some(Menu::Library));
                if hovered {
                    icon(
                        &header_painter,
                        Rect::from_center_size(
                            pos2(title_rect.right() + 9.0, bar.center().y + 1.0),
                            vec2(18.0, 18.0),
                        ),
                        Icon::Chevron,
                        TEXT,
                    );

                    let glyphs = super::header_glyph_layout::layout(bar.width() as i32, bar.height() as i32);
                    let mapped = |r: super::header_glyph_layout::RectI32| Rect::from_min_size(
                        bar.min + vec2(r.x as f32, r.y as f32), vec2(r.width as f32, r.height as f32));
                    if icon_button_layout(ui, mapped(glyphs.reset_hit), mapped(glyphs.reset_fit),
                        "new-preset", Icon::Reset, "Clear canvas", self.show_tooltips).clicked() {
                        self.clear();
                    }
                    if !available(self) {
                        ui.disable();
                    }
                    let response = icon_button_layout(ui, mapped(glyphs.more_hit), mapped(glyphs.more_fit),
                        "preset-actions", Icon::More, "Preset actions", self.show_tooltips);
                    if response.clicked() {
                        self.toggle_menu(Menu::Actions);
                    }
                    menu_trigger_semantics(&response, self.menu == Some(Menu::Actions));
                }
                }
                let right = full.right();
                if settings.width() > 0.0 && settings.height() > 0.0 {
                self.overlay_rects.push(settings);
                if !available(self) {
                    ui.disable();
                }
                let settings_response = ui.interact(settings, ui.id().with("settings"), Sense::click());
                let settings_label = localization::tr(ctx, "Settings");
                settings_response.widget_info(|| egui::WidgetInfo::labeled(
                    egui::WidgetType::Button, ui.is_enabled(), settings_label,
                ));
                let settings_painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(settings));
                if settings_response.hovered() {
                    settings_painter.rect_filled(settings, 20, Color32::from_white_alpha(0x10));
                }
                icon(
                    &settings_painter, Rect::from_center_size(settings.center(), vec2(18.0, 18.0)),
                    Icon::Gear,
                    if settings_response.hovered() { Color32::from_rgb(224, 224, 226) }
                        else { Color32::from_rgb(183, 184, 187) },
                );
                if optional_tooltip(settings_response, self.show_tooltips, "Open settings.").clicked() {
                    if !self.settings {
                        if let Err(error) = self.refresh_input_preferences_at(&preferences_dir()) { self.error = Some(error); }
                        if self.color_picker && self.color_source.is_none() {

                            self.native_tone_hex.cancel();
                        } else {
                            self.color_picker = false;
                        }
                        self.settings_resources = false;
                        self.settings_scroll_reset = true;
                    }
                    self.settings = !self.settings;
                    self.menu = None;
                    self.voices_open = false;
                    self.drag = None;
                }
                }
                let realtime =
                    Rect::from_min_size(pos2(right - 147.0, bar.center().y - 12.0), vec2(96.0, 24.0));
                self.overlay_rects.push(realtime);
                let on = self.realtime();
                if !available(self) {
                    ui.disable();
                }
                let mut response = ui.interact(realtime, ui.id().with("realtime"), Sense::click());
                if on {
                    ui.painter()
                        .rect_filled(realtime, 20, Color32::from_rgb(193, 194, 201));
                } else {
                    ui.painter().rect_stroke(
                        realtime,
                        20,
                        Stroke::new(1.25, Color32::from_gray(186)),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.painter().text(
                    realtime.center(),
                    Align2::CENTER_CENTER,
                    localization::tr(ctx, "Realtime"),
                    FontId::proportional(13.0),
                    if on {
                        Color32::from_rgb(48, 49, 57)
                    } else {
                        TEXT
                    },
                );
                if response.clicked() {
                    self.remember();
                    self.preset.document["useRealtimeLatency"] = json!(!on);
                    if voice(&self.preset.document)
                        .get("useRealtimeLatency")
                        .is_some()
                    {
                        voice_mut(&mut self.preset.document)["useRealtimeLatency"] = json!(!on);
                    }
                    self.changed();
                    self.menu = None;
                    response.mark_changed();
                }
                let realtime_label = localization::tr(ctx, "Realtime");
                response.widget_info(|| egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox, ui.is_enabled(), self.realtime(),
                    realtime_label,
                ));
                optional_tooltip(response, self.show_tooltips, if on {
                    "Turn off Realtime Mode to save CPU for applications where low latency is not required."
                } else {
                    "Turn on Realtime Mode to optimize Vocoflex for live performance, which minimizes latency at the expense of CPU usage."
                });
                let on = self.realtime();
                let quality =
                    Rect::from_min_size(pos2(right - 288.0, bar.center().y - 14.0), vec2(124.0, 28.0));
                self.overlay_rects.push(quality);
                if !available(self) {
                    ui.disable();
                }
                let quality_response = ui.interact(
                    quality, ui.id().with("quality"),
                    if on { Sense::click() } else { Sense::hover() },
                );
                let quality_label = localization::tr(ctx,
                    "Adjust how the algorithm balances latency with audio quality.");
                let translated_quality = localization::tr(ctx,
                    match crate::prepare_quality::Preferences::from_document(&self.preset.document) {
                        Ok(_)=>quality_names()[self.quality()], Err(_)=>"Invalid quality",
                    });
                quality_response.widget_info(|| {
                    let mut info = egui::WidgetInfo::labeled(
                        egui::WidgetType::ComboBox, ui.is_enabled() && on, quality_label,
                    );
                    info.current_text_value = Some(translated_quality.to_owned());
                    info
                });
                if on && quality_response.hovered() {
                    ui.painter().rect_filled(quality, 5, Color32::from_black_alpha(28));
                }
                ui.painter().text(
                    quality.left_center() + vec2(17.0, 0.0), Align2::LEFT_CENTER,
                    translated_quality, FontId::proportional(13.0),
                    if on { TEXT } else { Color32::from_gray(131) },
                );
                optional_tooltip(quality_response.clone(), self.show_tooltips,
                    "Adjust how the algorithm balances latency with audio quality.");
                if quality_response.clicked() && on {
                    self.toggle_menu(Menu::Quality);
                }
                menu_trigger_semantics(&quality_response, self.menu == Some(Menu::Quality));
                icon(
                    ui.painter(),
                    Rect::from_center_size(
                            pos2(quality.right() - 8.0, quality.center().y),
                        vec2(18.0, 18.0),
                    ),
                    Icon::Chevron,
                    if on { TEXT } else { MUTED },
                );
                if matches!(self.menu, Some(Menu::Library | Menu::Actions)) {
                    self.menu_rects.push(bar);
                } else if self.menu == Some(Menu::Quality) {
                    self.menu_rects.push(quality);
                }
                });
            });
        if !message_open && !self.layout_blocked() && self.editor.is_none()
            && self.error.is_none() && self.notice.is_none()
            && let Some(menu) = self.menu
        {
            let position = match menu {
                Menu::Library => pos2(full.center().x - 99.0, full.top() + 37.0),
                Menu::Actions => pos2(full.center().x + 98.0, full.top() + 37.0),
                Menu::Quality => pos2(full.right() - 283.0, full.top() + 41.0),
            };
            match menu {
                Menu::Library => self.library_menu(ctx, full, position),
                Menu::Actions => self.actions_menu(ctx, position),
                Menu::Quality => self.quality_menu(ctx, position),
            }
        }
    }
    fn toggle_menu(&mut self, menu: Menu) {
        if self.menu.is_none() && menu == Menu::Library && !self.library_scan.busy() {
            if let Err(error) = self.library_scan.request(&self.preset_paths) { self.error = Some(error); }
        }
        self.menu = if self.menu.is_some() {
            None
        } else {
            Some(menu)
        };
        self.voices_open = false;
        self.color_picker = false;
    }

    fn library_menu(&mut self, ctx: &egui::Context, full: Rect, position: Pos2) {
        let height = (45.0 + self.library.len().max(1) as f32 * 27.0)
            .min((full.bottom() - position.y - 12.0).max(45.0));
        let rect = Rect::from_min_size(position, vec2(252.0, height));
        self.overlay_rects.push(rect);
        self.menu_rects.push(rect);

        let pointer_activity = ctx.input(|input|
            input.pointer.delta() != egui::Vec2::ZERO || input.pointer.any_pressed());
        if pointer_activity && ctx
            .pointer_hover_pos()
            .is_some_and(|p| rect.contains(p) && p.y >= rect.top() + 37.0)
        {
            self.voices_open = false;
        }
        let lights = self.ambient_lights(full);
        let entries = &self.library;
        let scanning = self.library_scan.busy();
        let voices_open = &mut self.voices_open;
        let mut load = None;
        egui::Area::new(egui::Id::new("preset-library"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ctx, |ui| {
                menu_container_semantics(ui, "Presets");
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &lights, true);
                let voices = Rect::from_min_size(position + vec2(0.0, 1.0), vec2(rect.width(), 35.0));
                let response = preset_menu_row(ui, voices, "voices", "Voices");
                let arrow = pos2(rect.right() - 15.0, voices.center().y);
                ui.painter().add(egui::Shape::line(
                    vec![arrow + vec2(-2.0, -4.0), arrow + vec2(2.0, 0.0), arrow + vec2(-2.0, 4.0)],
                    Stroke::new(1.0, TEXT),
                ));
                if response.hovered() || response.clicked() {
                    *voices_open = true;
                }
                let expanded = *voices_open;
                let _ = ctx.accesskit_node_builder(response.id, |node| {
                    node.set_has_popup(egui::accesskit::HasPopup::Menu);
                    node.set_expanded(expanded);
                });
                ui.painter().line_segment(
                    [
                        pos2(rect.left(), voices.bottom()),
                        pos2(rect.right(), voices.bottom()),
                    ],
                    Stroke::new(0.7, Color32::from_white_alpha(30)),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                            .max_rect(Rect::from_min_max(position + vec2(0.0, 37.0), rect.max - vec2(0.0, 8.0))),
                    |ui| {
                        let scroll = egui::ScrollArea::vertical()
                            .id_salt("library-scroll")
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .min_scrolled_height(0.0)
                            .max_height(height - 45.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_width(rect.width() - 4.0);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                if entries.is_empty() {
                                    let (row, _) = ui.allocate_exact_size(vec2(rect.width(), 27.0), Sense::hover());
                                    ui.painter().text(row.left_center() + vec2(25.0, 0.0), Align2::LEFT_CENTER,
                                        if scanning { "…" } else { localization::tr(ctx, "No Saved Preset") },
                                        FontId::proportional(13.0), MUTED);
                                }
                                for (i, entry) in entries.iter().enumerate() {
                                    let (row, _) = ui.allocate_exact_size(
                                        vec2(rect.width(), 27.0),
                                        Sense::hover(),
                                    );
                                    if load.is_some() {
                                        ui.disable();
                                    }
                                    let name = clipped_title(&entry.name, 28);
                                    let response = preset_menu_row_labelled(
                                        ui,
                                        row,
                                        format!("library-{i}"),
                                        &name,
                                        &entry.name,
                                        None,
                                    );
                                    if response.gained_focus() {
                                        response.scroll_to_me(None);
                                    }
                                    if response.clicked() && load.is_none()
                                    {
                                        response.surrender_focus();
                                        load = Some(entry.path.clone());
                                    }
                                }
                            });
                        hover_scroll(ui, &scroll);
                    },
                );
            });
        if self.voices_open && load.is_none() {
            let top = (rect.top() - 18.0).max(full.top() + 8.0);
            let available = (full.bottom() - top - 12.0).max(29.0);
            let imported_rows = if self.library_voices.is_empty() { 0 } else { self.library_voices.len() + 1 };
            let side_height = (2.0 + (self.factory_voices.len() + imported_rows).max(1) as f32 * 27.0)
                .min(available);
            let side = Rect::from_min_size(pos2(rect.right() + 2.0, top), vec2(213.0, side_height));
            self.overlay_rects.push(side);
            self.menu_rects
                .push(Rect::from_min_max(pos2(rect.right(), side.top()), side.max));
            let factory_status = self.factory_status();
            let factory_failed = self.factory_error.is_some();
            let catalogs = [
                (true, "Factory Voices", self.factory_voices.as_slice()),
                (false, "Imported Voices", self.library_voices.as_slice()),
            ];
            let mut chosen = None;
            egui::Area::new(egui::Id::new("preset-voices"))
                .order(egui::Order::Foreground)
                .fixed_pos(side.min)
                .show(ctx, |ui| {
                    menu_container_semantics(ui, "Voices");
                    ui.set_min_size(side.size());
                    glass(ui.painter(), side, &lights, true);
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(side.shrink2(vec2(0.0, 1.0))),
                        |ui| {
                            let scroll = egui::ScrollArea::vertical()
                                .id_salt("voices-scroll")
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .min_scrolled_height(0.0)
                                .max_height(side.height() - 2.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_min_width(side.width());
                                    ui.set_max_width(side.width());
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    for (factory, heading, catalog) in catalogs {
                                        if !factory && !catalog.is_empty() {
                                            let (row, _) = ui.allocate_exact_size(vec2(side.width(), 27.0), Sense::hover());
                                            ui.painter().text(
                                                row.left_center() + vec2(25.0, 0.0), Align2::LEFT_CENTER,
                                                localization::tr(ctx, heading), FontId::proportional(12.0), MUTED,
                                            );
                                        }
                                        if catalog.is_empty() {
                                            if !factory { continue; }
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(if factory {
                                                        &factory_status
                                                    } else {
                                                        "Import presets to add voices."
                                                    })
                                                    .color(if factory && factory_failed {
                                                        Color32::from_rgb(232, 166, 166)
                                                    } else {
                                                        MUTED
                                                    })
                                                    .size(13.0),
                                                )
                                                .wrap(),
                                            );
                                        }
                                        for (index, entry) in catalog.iter().enumerate() {
                                            let (row, _) = ui.allocate_exact_size(
                                                vec2(side.width(), 27.0),
                                                Sense::hover(),
                                            );
                                            if chosen.is_some() {
                                                ui.disable();
                                            }
                                            let name = clipped_title(&entry.name, 28);
                                            let response = preset_menu_row_labelled(
                                                ui,
                                                row,
                                                ("voice", factory, index),
                                                &name,
                                                &entry.name,
                                                None,
                                            );
                                            if response.gained_focus() {
                                                response.scroll_to_me(None);
                                            }
                                            if response.clicked() && chosen.is_none()
                                            {
                                                response.surrender_focus();
                                                chosen = Some((factory, index));
                                            }
                                        }
                                    }
                                });
                            hover_scroll(ui, &scroll);
                        },
                    );
                });
            if let Some((factory, index)) = chosen {
                let entry = if factory {
                    &self.factory_voices
                } else {
                    &self.library_voices
                }
                .get(index)
                .cloned();
                if let Some(entry) = entry {
                    self.append_voice(&entry);
                }
            }
        }
        if let Some(path) = load {
            if let Err(e) = self.load(path) {
                self.error = Some(e);
            }
        }
    }
    fn actions_menu(&mut self, ctx: &egui::Context, position: Pos2) {
        let items = [
            "Save Preset",
            "Save As a New Preset",
            "Rename Preset",
            "Delete Preset",
        ];
        let text_width = items.into_iter().chain(["Browse Presets Folder"]).map(|label| {
            let text = localization::tr(ctx, label).to_owned();
            ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, FontId::proportional(13.0), TEXT).size().x)
        }).fold(0.0_f32, f32::max);
        let width = (text_width + 40.0).max(177.0)
            .min((ctx.content_rect().right() - position.x - 8.0).max(80.0));
        let rect = Rect::from_min_size(position, vec2(width, 153.0));
        self.overlay_rects.push(rect);
        self.menu_rects.push(rect);
        let can_save = !self.preset_needs_name() || self.experimental_save_requires_copy();
        egui::Area::new(egui::Id::new("preset-actions"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ctx, |ui| {
                menu_container_semantics(ui, "Preset actions");
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &[], true);
                for (index, label) in items.iter().enumerate() {
                    let row = Rect::from_min_size(
                        pos2(rect.left(), rect.top() + 4.0 + index as f32 * 27.0), vec2(rect.width(), 27.0),
                    );
                    let available = self.menu == Some(Menu::Actions) && !self.layout_blocked()
                        && !self.settings && self.editor.is_none() && !self.color_picker
                        && self.error.is_none() && self.notice.is_none();
                    if !available {
                        ui.disable();
                    }
                    if index == 0 && !can_save {
                        let response = ui.interact(row, ui.id().with(index), Sense::hover());
                        menu_item_semantics(&response, false, localization::tr(ctx, label), None);
                        ui.painter().text(row.left_center() + vec2(25.0, 0.0), Align2::LEFT_CENTER,
                            localization::tr(ctx, label), FontId::proportional(13.0), Color32::from_gray(131));
                        continue;
                    }
                    let response = preset_menu_row(ui, row, index, label);
                    if response.clicked() && available {
                        match index {
                            0 => self.save_current(),
                            1 => self.begin_edit(Edit::SaveAs),
                            2 => self.begin_edit(Edit::PresetName),
                            _ => self.begin_edit(Edit::DeletePreset),
                        }
                        if self.menu != Some(Menu::Actions) {
                            response.surrender_focus();
                        }
                    }
                }
                ui.painter().line_segment(
                    [pos2(rect.left(), rect.top() + 116.0), pos2(rect.right(), rect.top() + 116.0)],
                    Stroke::new(0.7, Color32::from_white_alpha(30)),
                );
                let browse = Rect::from_min_size(rect.min + vec2(0.0, 120.0), vec2(rect.width(), 27.0));
                let browse_available = self.menu == Some(Menu::Actions) && !self.layout_blocked()
                    && !self.settings && self.editor.is_none() && !self.color_picker
                    && self.error.is_none() && self.notice.is_none();
                if !browse_available {
                    ui.disable();
                }
                let response = preset_menu_row(ui, browse, "browse-presets", "Browse Presets Folder");
                if response.clicked() && browse_available {
                    self.browse_presets_folder();
                    if self.menu != Some(Menu::Actions) {
                        response.surrender_focus();
                    }
                }
            });
    }

    fn browse_presets_folder(&mut self) {
        let result = (|| -> Result<()> {
            fs::create_dir_all(&self.library_dir)
                .map_err(|_| "The presets folder could not be created.".to_owned())?;
            let directory = self.path.as_ref().and_then(|path| path.parent())
                .filter(|path| path.is_dir()).unwrap_or(&self.library_dir);
            let mut command = if cfg!(windows) {
                std::process::Command::new("explorer.exe")
            } else if cfg!(target_os = "macos") {
                std::process::Command::new("open")
            } else {
                std::process::Command::new("xdg-open")
            };
            command.arg(directory).spawn()
                .map_err(|_| "The presets folder could not be opened.".to_owned())?;
            Ok(())
        })();
        self.menu = None;
        if let Err(error) = result { self.error = Some(error); }
    }
    fn quality_menu(&mut self, ctx: &egui::Context, position: Pos2) {
        let text_width = quality_names().into_iter().map(|label| {
            let text = localization::tr(ctx, label).to_owned();
            ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, FontId::proportional(13.0), TEXT).size().x)
        }).fold(0.0_f32, f32::max);
        let width = (text_width + 40.0).max(141.0)
            .min((ctx.content_rect().right() - position.x - 8.0).max(80.0));
        let rect = Rect::from_min_size(position, vec2(width, 116.0));
        self.overlay_rects.push(rect);
        self.menu_rects.push(rect);
        egui::Area::new(egui::Id::new("quality-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ctx, |ui| {
                menu_container_semantics(ui, "Adjust how the algorithm balances latency with audio quality.");
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &[], true);
                for (row_index, index) in (0..quality_names().len()).rev().enumerate() {
                    let available = self.menu == Some(Menu::Quality) && self.realtime()
                        && !self.layout_blocked() && !self.settings
                        && self.editor.is_none() && !self.color_picker
                        && self.error.is_none() && self.notice.is_none();
                    if !available {
                        ui.disable();
                    }
                    let row = Rect::from_min_size(
                        position + vec2(0.0, 4.0 + row_index as f32 * 27.0),
                        vec2(rect.width(), 27.0),
                    );
                    let selected = self.quality() == index;
                    if selected {
                        icon(ui.painter(), Rect::from_center_size(
                            pos2(row.left() + 15.0, row.center().y), vec2(12.0, 12.0),
                        ), Icon::Check, TEXT);
                    }
                    let response = ui.add_enabled_ui(true, |ui| {
                        preset_menu_row_labelled(ui,row,index,
                            localization::tr(ctx, quality_names()[index]),
                            localization::tr(ctx, quality_names()[index]),Some(selected))
                    }).inner;
                    if response.clicked() && available {
                        self.set_quality(index);
                        if self.menu != Some(Menu::Quality) {
                            response.surrender_focus();
                        }
                    }
                }
            });
    }

    pub(super) fn rail(&mut self, ctx: &egui::Context, full: Rect) {
        if self.layout_blocked() {
            return;
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let name_editor_open = self.editor.is_some();
        let rect = super::monitor::parent_bounds(full).voice_group;

        let first = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + 20.0), vec2(28.0, 28.0));
        let second = first.translate(vec2(0.0, 40.0));
        self.overlay_rects.push(rect);
        egui::Area::new(egui::Id::new("canvas-tools"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!message_open && !name_editor_open, |ui| {
                ui.set_min_size(rect.size());
                if rail_button(
                    ui,
                    first,
                    "color-point",
                    false,
                    "Generate a new voice timbre from scratch.",
                    self.show_tooltips,
                )
                .clicked()
                {
                    self.begin_color(None);
                    if self.color_picker {
                        return;
                    }
                }
                if rail_button(
                    ui,
                    second,
                    "audio",
                    true,
                    "Import a voice timbre from an audio file.",
                    self.show_tooltips,
                )
                .clicked()
                {
                    self.choose_audio(ctx);
                }
                });
            });
    }
    pub(super) fn source_actions(&mut self, ctx: &egui::Context, full: Rect) {
        if self.layout_blocked() {
            return;
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let Some(selected) = self.selected else {
            return;
        };
        if selected >= self.preset.sources.len() {
            self.selected = None;
            return;
        }
        let Some(rect) = self.source_actions_rect(full) else {
            return;
        };
        self.overlay_rects.push(rect);
        egui::Area::new(egui::Id::new("source-actions"))
            .order(egui::Order::Foreground)
            .constrain(false)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!message_open, |ui| {
                ui.set_min_size(rect.size());
                ui.painter().rect_filled(
                    rect,
                    14,
                    Color32::from_rgba_unmultiplied(160, 160, 165, 95),
                );
                for (slot, action, kind, tip) in [
                    (0, 0, Icon::Edit, "Rename"),
                    (1, 2, Icon::Trash, "Delete"),
                ] {
                    let r = Rect::from_min_size(
                        rect.min + vec2(2.0 + slot as f32 * 30.0, 0.0),
                        vec2(30.0, 28.0),
                    );
                    let available = self.selected == Some(selected)
                        && selected < self.preset.sources.len()
                        && !self.layout_blocked() && !self.settings
                        && self.editor.is_none() && !self.color_picker
                        && self.error.is_none() && self.notice.is_none();
                    if !available {
                        ui.disable();
                    }
                    let response = icon_button(ui, r, action, kind, tip, self.show_tooltips);
                    if response.clicked() && available {
                        match action {
                            0 => self.begin_edit(Edit::SourceName(selected)),
                            _ => self.delete_source(selected),
                        }
                        if self.selected != Some(selected) || self.editor.is_some() || self.layout_blocked() {
                            response.surrender_focus();
                        }
                    }
                }
                });
            });
    }

    pub(super) fn settings_ui(&mut self, ctx: &egui::Context, full: Rect) {
        if self.layout_blocked() {
            return;
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let assignment_open = self.shortcuts.editing();
        let rect = settings_panel_rect(full);
        self.overlay_rects.push(rect);
        let lights = self.ambient_lights(full);
        let paths_before = self.model_paths.clone();
        let ffmpeg_before = self.ffmpeg_path.clone();
        let before_tooltips = self.show_tooltips;
        let before_multithread = self.live.multithread();
        let before_locale = self.locale;
        #[cfg(windows)]
        let audio_before = self.audio_devices.selection.clone();
        #[cfg(windows)]
        let midi_before = self.midi_input.preference();
        let resources_page = self.settings_resources;
        let reset_scroll = std::mem::take(&mut self.settings_scroll_reset);
        let modal = modal_panel(ctx, "settings-modal", rect, Color32::from_black_alpha(25), |ui| {
                ui.add_enabled_ui(!message_open && !assignment_open, |ui| {
                    ui.set_min_size(rect.size());
                    glass(ui.painter(), rect, &lights, false);
                    if resources_page {
                        let response = icon_button(ui,
                            Rect::from_center_size(rect.min + vec2(30.0, 28.0), vec2(24.0, 24.0)),
                            "settings-back", Icon::Back, "Back to Settings", self.show_tooltips,
                        );
                        if response.clicked() {
                            response.surrender_focus();
                            self.settings_resources = false;
                            self.settings_scroll_reset = true;
                        }
                    }
                    ui.painter().text(
                        rect.min + vec2(if resources_page { 52.0 } else { 24.0 }, 28.0), Align2::LEFT_CENTER,
                        localization::tr(ctx, if resources_page { "Resources & Workspace" } else { "Settings" }),
                        FontId::proportional(14.0),
                        if resources_page { TEXT } else { SETTINGS_REFERENCE_HIGHLIGHT },
                    );
                    if !self.settings || self.error.is_some() || self.notice.is_some() {
                        ui.disable();
                    }
                    if !resources_page {
                        let response = icon_button(ui,
                            Rect::from_center_size(rect.min + vec2(461.0, 28.0), vec2(24.0, 24.0)),
                            "settings-resources", Icon::Folder, "Resources & Workspace", self.show_tooltips,
                        );
                        if response.clicked() {
                            response.surrender_focus();
                            self.settings_resources = true;
                            self.settings_scroll_reset = true;
                        }
                    }
                    if icon_button(
                        ui, Rect::from_center_size(rect.min + vec2(485.0, 28.0), vec2(24.0, 24.0)),
                        "settings-info", Icon::Info, "Project information", self.show_tooltips,
                    ).clicked() {
                        self.notice = Some(PROJECT_DESCRIPTION.into());
                    }
                    if !self.settings || self.error.is_some() || self.notice.is_some() {
                        ui.disable();
                    }
                    let close_response = icon_button(
                        ui, Rect::from_center_size(rect.min + vec2(509.0, 28.0), vec2(24.0, 24.0)),
                        "settings-close", Icon::Close, "Close", self.show_tooltips,
                    );
                    if close_response.clicked() {
                        close_response.surrender_focus();
                        self.settings = false;
                    }
                    if !self.settings || self.error.is_some() || self.notice.is_some()
                        || resources_page != self.settings_resources
                    {
                        ui.disable();
                    }
                    let body = Rect::from_min_max(rect.min + vec2(24.0, 58.0), rect.max - vec2(14.0, 16.0));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                        settings_field_style(ui);
                        ui.style_mut().spacing.scroll.bar_width = 5.0;
                        ui.style_mut().spacing.scroll.floating_width = 4.0;
                        ui.style_mut().spacing.scroll.dormant_handle_opacity = 0.5;
                        let scroll = egui::ScrollArea::vertical().id_salt(("settings-scroll", resources_page))
                            .max_height(body.height()).auto_shrink([false, false]);
                        let scroll = if reset_scroll { scroll.vertical_scroll_offset(0.0) } else { scroll };
                        scroll.show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let width = 492.0;
                                ui.set_max_width(width);
                                if resources_page {
                                    self.resources_ui(ui, message_open);
                                    return;
                                }
                                settings_section_colored(ui, width, "General", SETTINGS_REFERENCE_HIGHLIGHT);
                                let row = settings_row_colored(ui, width, "Language", SETTINGS_REFERENCE_LABEL);
                                let language_rect = settings_value_rect(row);

                                let mut language_ui = ui.new_child(egui::UiBuilder::new().max_rect(language_rect));
                                language_ui.spacing_mut().button_padding.x = 6.0;
                                let selected = settings_reference_text(self.locale.name(), language_ui.is_enabled());
                                egui::ComboBox::from_id_salt("interface-language")
                                    .width(language_rect.width()).truncate().icon(settings_reference_combo_icon)
                                    .selected_text(selected)
                                    .show_ui(&mut language_ui, |ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        ui.add_enabled_ui(!message_open && self.settings, |ui| {
                                            for locale in localization::Locale::ALL {
                                                if settings_option(ui, locale.name(), self.locale == locale).clicked() {
                                                    self.locale = locale;
                                                }
                                            }
                                        });
                                    });
                                drop(language_ui);
                                if self.locale != before_locale {
                                    localization::set(ctx, self.locale);
                                    ctx.request_repaint();
                                }
                                let row = settings_row_colored(ui, width, "Multithread Processing", SETTINGS_REFERENCE_LABEL);
                                let mut multithread = self.live.multithread();
                                if settings_toggle(ui, row, "multithread-processing", "Multithread Processing", &mut multithread, true).changed() {
                                    self.live.edit_multithread(multithread, self.plugin_hosted);
                                }
                                let row = settings_row_colored(ui, width, "CUDA Acceleration", SETTINGS_REFERENCE_LABEL);
                                let mut cuda_acceleration = false;
                                let _ = settings_toggle(ui, row, "cuda-acceleration", "CUDA Acceleration", &mut cuda_acceleration, false);
                                let row = settings_row_colored(ui, width, "Show Tooltips", SETTINGS_REFERENCE_LABEL);
                                let _ = settings_toggle(ui, row, "show-tooltips", "Show Tooltips", &mut self.show_tooltips, true);
                                ui.add_space(7.0);
                                settings_section_colored(ui, width, "Audio", SETTINGS_REFERENCE_HIGHLIGHT);
                                #[cfg(windows)]
                                if self.plugin_hosted {
                                    for label in ["Audio System", "Output", "Input", "Sample Rate", "Buffer Size", "When the input is stereo"] {
                                        let row = settings_row(ui, width, label);
                                        setting_value(ui, row, label, "Managed by host", "The plugin host controls audio routing and processing buffers.", self.show_tooltips, false, SettingsValueStyle::Default);
                                    }
                                    let row = settings_row(ui, width, "MIDI Control Device");
                                    setting_value(ui, row, "MIDI Control Device", "Host input unavailable", "VST host MIDI input is not connected yet.", self.show_tooltips, false, SettingsValueStyle::Default);
                                } else {
                                    self.audio_devices.ui(ui, width, self.show_tooltips);
                                    self.midi_input.ui(ui, width);
                                    self.audio_devices.stereo_ui(ui, width);
                                }
                                #[cfg(not(windows))]
                                for label in ["Audio System", "Output", "Input", "Sample Rate", "Buffer Size", "MIDI Control Device", "When the input is stereo"] {
                                    let row = settings_row(ui, width, label);
                                    setting_value(ui, row, label, "Unavailable", "Realtime audio devices and MIDI control are not connected yet.", self.show_tooltips, false, SettingsValueStyle::Default);
                                }
                                ui.add_space(7.0);
                                settings_section_colored(ui, width, "Shortcuts", SETTINGS_REFERENCE_HIGHLIGHT);
                                let (head, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                                let headings = settings_shortcut_rects(head);
                                let action_heading = Rect::from_min_max(head.min, pos2(headings[0].left(), head.bottom()));
                                for (region, text) in [(action_heading, "ACTION"), (headings[0], "KEYBOARD"), (headings[1], "MIDI")] {
                                    ui.painter().text(region.center(), Align2::CENTER_CENTER,
                                        localization::tr(ctx, text), FontId::proportional(11.0), SETTINGS_SHORTCUT_COLUMN);
                                }
                                for (index, action) in shortcuts::ACTIONS.into_iter().enumerate() {
                                    let (row, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
                                    let buttons = settings_shortcut_rects(row);
                                    ui.painter().with_clip_rect(Rect::from_min_max(row.min, pos2(buttons[0].left() - 6.0, row.bottom())))
                                        .text(row.left_center(), Align2::LEFT_CENTER, localization::tr(ctx, action), FontId::proportional(13.0), SETTINGS_REFERENCE_LABEL);
                                    for (column, button) in buttons.into_iter().enumerate() {
                                        self.shortcut_cell(ui, button, index, column);
                                    }
                                }
                                ui.add_space(7.0);
                                settings_section(ui, width, "About");

                                let mut about_job = egui::text::LayoutJob::simple(
                                    format!("Rocoflex {}\n{}\n{}: {}", env!("CARGO_PKG_VERSION"), localization::tr(ctx, PROJECT_DESCRIPTION), localization::tr(ctx, "License"), env!("CARGO_PKG_LICENSE")),
                                    FontId::proportional(13.0), MUTED, width - 145.0);
                                about_job.sections[0].format.line_height = Some(16.0);
                                let about_text = ui.painter().layout_job(about_job);
                                let (about, _) = ui.allocate_exact_size(
                                    vec2(width, (about_text.size().y + 16.0).max(110.0)), Sense::hover());
                                ui.painter().galley(about.min + vec2(0.0, 8.0), about_text, MUTED);
                                brand(ui.painter(), pos2(about.right() - 128.0, about.top() + 19.0), 20.0);
                            });
                    });
                });
            });
        if !message_open && !assignment_open && self.error.is_none() && self.notice.is_none()
            && modal.backdrop_response.clicked()
        {
            self.settings = false;
        }
        if !self.settings || self.settings_resources { self.shortcuts.cancel_edit(); }
        else if self.shortcuts.editing() { self.shortcut_assignment_dialog(ctx, full); }
        let multithread_changed = before_multithread != self.live.multithread();
        let changed = paths_before != self.model_paths || ffmpeg_before != self.ffmpeg_path
            || before_tooltips != self.show_tooltips || before_locale != self.locale;
        #[cfg(windows)]
        let changed = changed || audio_before != self.audio_devices.selection;
        #[cfg(windows)]
        if midi_before != self.midi_input.preference() {
            let selected = self.midi_input.preference();
            if let Err(error) = input_preferences::edit(&preferences_dir(), |value| { value["midiInput"] = selected; Ok(()) }) {
                if let Ok(previous) = midi::State::decode_preference(&midi_before) { self.midi_input.selected = previous; }
                self.error = Some(error);
            }
        }
        if multithread_changed { self.save_multithread_edit(before_multithread); }
        if changed { self.save_preferences(); }
    }

    pub(super) fn editor_ui(&mut self, ctx: &egui::Context, full: Rect) {
        if matches!(self.editor, Some(Edit::WaypointKey(_))) {
            self.waypoint_key_dialog(ctx, full); return;
        }
        if matches!(self.editor, Some(Edit::WaypointMidi(_))) {
            self.waypoint_midi_dialog(ctx, full); return;
        }
        let Some(edit) = self.editor else {
            return;
        };
        let message_open = self.error.is_some() || self.notice.is_some();
        let deletion = edit == Edit::DeletePreset;
        let size = if deletion {
            vec2(570.0, 195.0)
        } else {
            vec2(320.0, 169.0)
        };
        let rect = Rect::from_center_size(full.center(), size);
        self.overlay_rects.push(full);
        let lights = self.ambient_lights(full);
        modal_panel(ctx, "preset-editor", rect,
            Color32::from_black_alpha(if deletion { 35 } else { 0 }), |ui| {
                ui.add_enabled_ui(!message_open, |ui| {
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &lights, false);
                let left = rect.left() + if deletion { 84.0 } else { 25.0 };
                let right = rect.right() - if deletion { 31.0 } else { 24.0 };
                let title = match edit {
                    Edit::SourceName(_) => "Rename Voice",
                    Edit::PresetName => "Rename Preset",
                    Edit::SaveAs => "Save As a New Preset",
                    Edit::DeletePreset => "Delete Preset",
                    Edit::WaypointKey(_) | Edit::WaypointMidi(_) => unreachable!("dispatched to binding dialog"),
                };
                ui.painter().text(
                    pos2(left, rect.top() + if deletion { 34.0 } else { 28.0 }),
                    Align2::LEFT_CENTER,
                    localization::tr(ctx, title),
                    FontId::proportional(if deletion { 18.0 } else { 14.0 }),
                    TEXT,
                );
                if deletion {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, vec2(55.0, rect.height())),
                        10,
                        Color32::from_rgba_unmultiplied(43, 90, 96, 85),
                    );
                    ui.painter().text(
                        pos2(rect.left() + 28.0, rect.center().y),
                        Align2::CENTER_CENTER,
                        "?",
                        FontId::proportional(36.0),
                        Color32::from_rgb(51, 169, 176),
                    );
                    ui.painter().text(
                        pos2(left, rect.top() + 91.0),
                        Align2::LEFT_CENTER,
                        localization::tr(ctx, "Are you sure to delete \"_\"?")
                            .replacen('_', &clipped_title(&self.preset.name, 32), 1),
                        FontId::proportional(15.0),
                        TEXT,
                    );
                } else {
                    ui.painter().text(
                        pos2(left, rect.top() + 64.0),
                        Align2::LEFT_CENTER,
                        localization::tr(ctx, if matches!(edit, Edit::SourceName(_)) { "New Name" } else { "Name" }),
                        FontId::proportional(12.0),
                        MUTED,
                    );
                    let previous_widgets = matches!(edit, Edit::SaveAs | Edit::SourceName(_) | Edit::PresetName)
                        .then(|| ui.visuals().widgets.clone());
                    if previous_widgets.is_some() {
                        let widgets = &mut ui.style_mut().visuals.widgets;
                        widgets.noninteractive.corner_radius = 12u8.into();
                        widgets.inactive.corner_radius = 12u8.into();
                        widgets.hovered.corner_radius = 12u8.into();
                        widgets.active.corner_radius = 12u8.into();
                        widgets.open.corner_radius = 12u8.into();
                    }
                    let focus_before_input = ui.memory(|mem| mem.focused());
                    let source_name_pass = matches!(edit, Edit::SourceName(_))
                        .then(|| ctx.cumulative_pass_nr());
                    let input_rect = Rect::from_min_max(
                        pos2(left, rect.top() + 77.0),
                        pos2(right, rect.top() + 101.0),
                    );
                    let response = ui.put(input_rect, |field_ui: &mut egui::Ui| {

                        let input_id = field_ui.next_auto_id();
                        let previous_source_name_owner = source_name_pass.is_some_and(|pass| {
                            ctx.data_mut(|data| data.get_temp::<SourceNameFocusPass>(input_id))
                                .is_some_and(|previous| previous.pass.checked_add(1) == Some(pass)
                                    && previous.owned
                                    && matches!(previous.navigation, None | Some(SourceNameNavigation::Forward)))
                        });
                        if matches!(edit, Edit::SourceName(_)) && !self.focus_editor
                            && focus_before_input.is_none() && previous_source_name_owner
                            && field_ui.is_enabled() && field_ui.is_visible()
                            && field_ui.is_rect_visible(input_rect) && self.editor == Some(edit)
                            && !self.layout_blocked() && self.error.is_none() && self.notice.is_none()
                            && field_ui.memory(|mem| mem.had_focus_last_frame(input_id)
                                && mem.focused().is_none() && mem.allows_interaction(field_ui.layer_id()))
                            && field_ui.input(|input| input.focused && !input.pointer.any_pressed()
                                && !input.pointer.any_released()
                                && input.events.iter().all(|event| match event {
                                    egui::Event::Key { key, pressed: true, modifiers, .. } => match key {
                                        egui::Key::Escape => !modifiers.any(),
                                        egui::Key::Enter | egui::Key::Tab
                                            | egui::Key::ArrowUp | egui::Key::ArrowRight
                                            | egui::Key::ArrowDown | egui::Key::ArrowLeft => false,
                                        _ => true,
                                    },
                                    egui::Event::Ime(_) | egui::Event::Cut
                                        | egui::Event::AccessKitActionRequest(_) => false,
                                    _ => true,
                                })
                                && input.events.iter().any(|event| matches!(event,
                                    egui::Event::Key { key: egui::Key::Escape, pressed: true, modifiers, .. }
                                        if !modifiers.any())))
                        {

                            field_ui.memory_mut(|mem| mem.request_focus(input_id));
                            ctx.request_repaint();
                        }
                        field_ui.add(egui::TextEdit::singleline(&mut self.input)
                            .interactive(!message_open)
                            .event_filter(egui::EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                tab: false,
                                escape: matches!(edit, Edit::SourceName(_)),
                            })
                            .font(FontId::proportional(13.0))
                            .margin(vec2(8.0, 3.0)))
                    });
                    if let Some(widgets) = previous_widgets {
                        ui.style_mut().visuals.widgets = widgets;
                    }
                    if self.focus_editor && !message_open {
                        response.request_focus();
                        self.focus_editor = false;
                        if matches!(edit, Edit::SourceName(_)) {
                            ctx.request_repaint();
                        }
                    }
                    if response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))

                        && ui.memory(|mem| mem.had_focus_last_frame(response.id)
                            && (mem.focused().is_none() || mem.has_focus(response.id)))
                        && ui.is_enabled() && self.editor == Some(edit)
                        && !self.layout_blocked() && self.error.is_none() && self.notice.is_none()
                    {
                        if editor_name_allowed(edit, &self.input) {
                            self.confirm_edit();
                        } else {
                            response.request_focus();
                            ctx.request_repaint();
                        }
                    }
                    if let Some(pass) = source_name_pass {
                        let owned = ui.is_enabled() && self.editor == Some(edit)
                            && !self.layout_blocked() && self.error.is_none() && self.notice.is_none()
                            && ui.memory(|mem| mem.has_focus(response.id));
                        let navigation = if focus_before_input == Some(response.id) {
                            ui.input(|input| input.events.iter().filter_map(|event| match event {
                                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. }
                                    if modifiers.shift_only() => Some(SourceNameNavigation::Backward),
                                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. }
                                    if !modifiers.any() => Some(SourceNameNavigation::Forward),

                                egui::Event::Key { key, pressed: true, modifiers, .. }
                                    if !modifiers.any() && matches!(*key,
                                        egui::Key::ArrowUp | egui::Key::ArrowRight
                                        | egui::Key::ArrowDown | egui::Key::ArrowLeft)
                                    => Some(SourceNameNavigation::Uncertain),
                                _ => None,
                            }).last())
                        } else { None };
                        ctx.data_mut(|data| data.insert_temp(response.id,
                            SourceNameFocusPass { pass, owned, navigation }));
                    }
                }
                let y = if deletion { rect.bottom() - 62.0 } else { rect.top() + 121.0 };
                let gap = if deletion { 11.0 } else { 8.0 };
                let button_height = if deletion { 34.0 } else { 24.0 };
                let width = (right - left - gap) / 2.0;
                let editor_available = self.editor == Some(edit) && !self.layout_blocked()
                    && self.error.is_none() && self.notice.is_none();
                let enabled = editor_available && editor_name_allowed(edit, &self.input);
                let ok_response = pill_button(
                    ui,
                    Rect::from_min_size(pos2(left, y), vec2(width, button_height)),
                    "edit-ok",
                    if deletion { "Yes" } else { "OK" },
                    enabled,
                );
                if ok_response.clicked() && enabled {
                    self.confirm_edit();
                    if self.editor != Some(edit) {
                        ok_response.surrender_focus();
                    }
                }
                let cancel_enabled = self.editor == Some(edit) && !self.layout_blocked()
                    && self.error.is_none() && self.notice.is_none();
                let cancel_response = pill_button(
                    ui,
                    Rect::from_min_size(pos2(left + width + gap, y), vec2(width, button_height)),
                    "edit-cancel",
                    "Cancel",
                    cancel_enabled,
                );
                if cancel_response.clicked() && cancel_enabled {
                    cancel_response.surrender_focus();
                    self.editor = None;
                }
                });
            });
    }

    pub(super) fn color_ui(&mut self, ctx: &egui::Context, full: Rect) {
        if self.layout_blocked() {
            return;
        }
        if self.color_source.is_none() {
            self.native_color_ui(ctx, full);
            return;
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let rect = Rect::from_center_size(full.center(), vec2(380.0, 450.0));
        self.overlay_rects.push(rect);
        self.picker_rect = Some(rect);
        let lights = self.ambient_lights(full);
        let texture_hue = self.tone_hue.round().rem_euclid(360.0) as i32;
        if self.tone_texture_hue != Some(texture_hue) {
            let size = 176;
            let pixels = (0..size * size)
                .map(|i| {
                    let c = tone::rgb_at(
                        (i % size) as f64 / (size - 1) as f64,
                        (i / size) as f64 / (size - 1) as f64,
                        texture_hue as f64,
                    );
                    Color32::from_rgb(c[0], c[1], c[2])
                })
                .collect();
            let image = egui::ColorImage::new([size, size], pixels);
            if let Some(texture) = &mut self.tone_texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.tone_texture =
                    Some(ctx.load_texture("tone-surface", image, egui::TextureOptions::LINEAR));
            }
            self.tone_texture_hue = Some(texture_hue);
        }
        egui::Area::new(egui::Id::new("color-picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!message_open, |ui| {
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &lights, false);
                let back_response = icon_button(
                    ui,
                    Rect::from_min_size(rect.min + vec2(7.0, 7.0), vec2(30.0, 30.0)),
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
                let pad = Rect::from_min_size(rect.min + vec2(19.0, 45.0), vec2(342.0, 342.0));
                if let Some(texture) = &self.tone_texture {
                    ui.painter().image(
                        texture.id(),
                        pad,
                        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                let response = ui.interact(pad, ui.id().with("color-pad"), Sense::click_and_drag());
                if (response.clicked() || response.dragged())
                    && let Some(p) = response.interact_pointer_pos()
                {
                    self.tone_position = Point::new(
                        ((p.x - pad.left()) / pad.width()).clamp(0.0, 1.0) as f64,
                        ((p.y - pad.top()) / pad.height()).clamp(0.0, 1.0) as f64,
                    );
                    let c = tone::rgb_at(self.tone_position.x, self.tone_position.y, self.tone_hue);
                    self.set_picker_color(Color32::from_rgb(c[0], c[1], c[2]));
                }
                let thumb = pad.min
                    + vec2(
                        self.tone_position.x as f32 * pad.width(),
                        self.tone_position.y as f32 * pad.height(),
                    );
                ui.painter()
                    .circle_stroke(thumb, 17.5, Stroke::new(1.5, Color32::WHITE));
                ui.painter().circle_filled(thumb, 2.0, Color32::WHITE);
                let galley =
                    ui.painter()
                        .layout_no_wrap("TONE".into(), FontId::proportional(15.0), TEXT);
                ui.painter().add(
                    egui::epaint::TextShape::new(
                        pos2(pad.right() - 9.0, pad.top() + 10.0),
                        galley,
                        TEXT,
                    )
                    .with_angle(std::f32::consts::FRAC_PI_2),
                );
                ui.painter().text(
                    pad.left_bottom() + vec2(10.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    "MASCULINE",
                    FontId::proportional(10.0),
                    TEXT,
                );
                ui.painter().text(
                    pad.right_bottom() + vec2(-10.0, -10.0),
                    Align2::RIGHT_BOTTOM,
                    "FEMININE",
                    FontId::proportional(10.0),
                    TEXT,
                );
                let y = pad.bottom() + 8.0;
                if pill_button(
                    ui,
                    Rect::from_min_size(pos2(pad.left(), y), vec2(95.0, 32.0)),
                    "random-tone",
                    "Random",
                    true,
                )
                .clicked()
                {
                    let entropy = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_nanos())
                        .unwrap_or(0);
                    self.tone_hue = entropy as f64 / 1_000_000_000.0 * 360.0;
                    let c = tone::rgb_at(self.tone_position.x, self.tone_position.y, self.tone_hue);
                    self.set_picker_color(Color32::from_rgb(c[0], c[1], c[2]));
                }
                let response = ui.put(
                    Rect::from_min_size(pos2(pad.left() + 103.0, y), vec2(136.0, 32.0)),
                    egui::TextEdit::singleline(&mut self.color_hex)
                        .interactive(!message_open)
                        .char_limit(7)
                        .font(FontId::proportional(15.0))
                        .horizontal_align(egui::Align::Center)
                        .vertical_align(egui::Align::Center)
                        .min_size(vec2(136.0, 32.0))
                        .margin(vec2(10.0, 0.0)),
                );
                if !message_open && (response.lost_focus()
                    || (response.has_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))))
                    && self.color_hex.trim_start_matches('#').len() == 6
                {
                    if let Ok(n) = u32::from_str_radix(self.color_hex.trim_start_matches('#'), 16) {
                        let fitted =
                            tone::fit([(n >> 16) as u8, (n >> 8) as u8, n as u8], self.tone_hue);
                        self.tone_hue = fitted.hue;
                        self.tone_position = Point::new(fitted.x, fitted.y);
                        self.set_picker_color(Color32::from_rgb(
                            fitted.rgb[0],
                            fitted.rgb[1],
                            fitted.rgb[2],
                        ));
                    }
                }
                let apply_response = pill_button(
                    ui,
                    Rect::from_min_size(pos2(pad.right() - 95.0, y), vec2(95.0, 32.0)),
                    "apply-color",
                    if self.color_source.is_none() && self.generation_busy() {
                        "Working..."
                    } else {
                        "Apply"
                    },
                    self.color_source.is_some() || !self.generation_busy(),
                );
                if apply_response.clicked() {
                    if self.color_source.is_some() {
                        self.apply_color();
                        if !self.color_picker {
                            apply_response.surrender_focus();
                        }
                    } else {
                        self.start_tone_generation(ctx);
                    }
                }
                });
            });
    }
    pub(super) fn set_picker_color(&mut self, c: Color32) {
        self.color = [c.r(), c.g(), c.b()];
        self.color_hex = format!(
            "{:02X}{:02X}{:02X}",
            self.color[0], self.color[1], self.color[2]
        );
    }
    pub(super) fn message_ui(&mut self, ctx: &egui::Context, full: Rect) {
        let error = self.error.is_some();
        let Some(message) = self.error.as_deref().or(self.notice.as_deref()) else {
            return;
        };

        let message = clipped_title(&localization::input_message(ctx, message), 16 * 1024);
        let rect = Rect::from_center_size(full.center(), vec2(470.0, 210.0));
        self.overlay_rects.push(full);
        modal_panel(ctx, "message", rect, Color32::from_black_alpha(25), |ui| {
                ui.set_min_size(rect.size());
                glass(ui.painter(), rect, &[], false);
                ui.painter().text(
                    rect.min + vec2(28.0, 33.0),
                    Align2::LEFT_CENTER,
                    localization::tr(ctx, if error {
                        "Could not complete the operation"
                    } else {
                        "Rocoflex"
                    }),
                    FontId::proportional(18.0),
                    TEXT,
                );
                let body = Rect::from_min_max(
                    rect.min + vec2(28.0, 69.0),
                    rect.max - vec2(28.0, 66.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("message-body")
                        .max_height(body.height())
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width().min(body.width()));
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(message).size(14.0).color(TEXT),
                                )
                                .wrap(),
                            );
                        });
                });
                let close_response = pill_button(
                    ui,
                    Rect::from_min_size(
                        pos2(rect.right() - 147.0, rect.bottom() - 53.0),
                        vec2(119.0, 32.0),
                    ),
                    "message-close",
                    "Close",
                    true,
                );
                if close_response.clicked() {
                    close_response.surrender_focus();
                    if error {
                        self.error = None;
                    } else {
                        self.notice = None;
                    }
                }
            });
    }
}
