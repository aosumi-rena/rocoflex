use super::*;
use super::controls::{setting_value, settings_option, settings_reference_combo_icon,
    settings_reference_text, settings_row, settings_value_rect, SettingsValueStyle};
use crate::wasapi::Error;
use crate::wasapi_devices::{Catalog, Endpoint, Flow};
use crate::wasapi_render::{Channels, Conversion, Parts};
use std::{num::NonZeroU32, thread::JoinHandle};

#[derive(Clone, Copy, Debug)]
pub(super) enum Direction { Input, Output }

#[derive(Debug)]
pub(super) enum ConfigurationError {
    Invalid { direction: Direction, parts: Parts, cause: crate::wasapi_render::Error },
    EmptyCallback,
    InputChannel(usize),
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { direction, cause, .. } => write!(f, "{direction:?} requested format is invalid: {cause:?}"),
            Self::EmptyCallback => write!(f, "Buffer Size must be at least one sample."),
            Self::InputChannel(channel) => write!(f, "Stereo input channel {channel} is unavailable."),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Selection {

    pub output: Option<String>,
    pub input: Option<String>,
    pub sample_rate: u32,

    pub buffer_size_frames: u32,
    pub input_channel: usize,
}

impl Default for Selection {
    fn default() -> Self {
        Self { output: None, input: None, sample_rate: 48_000,
            buffer_size_frames: 480, input_channel: 0 }
    }
}

#[derive(Debug)]
pub(super) struct DuplexConfigurationError {
    pub selection: Selection,
    pub policy: crate::wasapi_duplex::Policy,
    pub cause: ConfigurationError,
}

impl std::fmt::Display for DuplexConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cause)
    }
}

impl Selection {
    fn parts(&self, endpoint: &Option<String>, direction: Direction) -> std::result::Result<Parts, ConfigurationError> {
        let parts = Parts {
            endpoint: match endpoint {
                Some(id) => crate::wasapi::EndpointChoice::Id(id.clone()),
                None => crate::wasapi::EndpointChoice::DefaultMultimedia,
            },
            sample_rate: self.sample_rate,

            channels: Channels::Stereo,
            conversion: Conversion::SystemRateAndChannelConversion,
        };

        if let Err(cause) = parts.format() {
            return Err(ConfigurationError::Invalid { direction, parts, cause });
        }
        Ok(parts)
    }
}

impl Selection {
    pub(super) fn from_preferences(value: &Value) -> Self {
        let defaults = Self::default();
        Self {
            output: value["output"].as_str().map(str::to_owned),
            input: value["input"].as_str().map(str::to_owned),
            sample_rate: value["sampleRate"].as_u64().and_then(|v| u32::try_from(v).ok())
                .unwrap_or(defaults.sample_rate),
            buffer_size_frames: value["bufferSizeFrames"].as_u64().and_then(|v| u32::try_from(v).ok())
                .unwrap_or(defaults.buffer_size_frames),
            input_channel: value["inputChannel"].as_u64().and_then(|v| usize::try_from(v).ok())
                .unwrap_or(defaults.input_channel),
        }
    }

    pub(super) fn to_preferences(&self) -> Value {
        json!({"output": self.output, "input": self.input, "sampleRate": self.sample_rate,
            "bufferSizeFrames": self.buffer_size_frames, "inputChannel": self.input_channel})
    }

    pub(super) fn duplex_configuration(&self, policy: crate::wasapi_duplex::Policy)
        -> std::result::Result<crate::wasapi_duplex::Configuration, DuplexConfigurationError> {
        let refused = |cause| DuplexConfigurationError { selection: self.clone(), policy, cause };
        let callback_frames = NonZeroU32::new(self.buffer_size_frames)
            .ok_or_else(|| refused(ConfigurationError::EmptyCallback))?;
        if self.input_channel > 1 {
            return Err(refused(ConfigurationError::InputChannel(self.input_channel)));
        }
        let capture = self.parts(&self.input, Direction::Input).map_err(&refused)?;
        let render = self.parts(&self.output, Direction::Output).map_err(refused)?;
        Ok(crate::wasapi_duplex::Configuration { capture, render, policy, callback_frames })
    }
}

enum QueryFailure {
    Spawn(std::io::Error),
    Native(Error),
    Panic(Box<dyn std::any::Any + Send>),
}

impl std::fmt::Display for QueryFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(error) => write!(f, "Device query could not start: {error}"),
            Self::Native(error) => write!(f, "Device query failed: {error}"),
            Self::Panic(payload) => {
                let text = payload.downcast_ref::<String>().map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied());
                write!(f, "Device query stopped unexpectedly{}", text.map(|s| format!(": {s}")).unwrap_or_default())
            }
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    pub selection: Selection,
    pending: Option<JoinHandle<std::result::Result<Catalog, Error>>>,
    catalog: Option<std::result::Result<Catalog, QueryFailure>>,
}

impl State {
    fn start(&mut self) {
        if self.pending.is_some() { return; }
        match crate::wasapi_devices::spawn_catalog_query() {
            Ok(job) => self.pending = Some(job),
            Err(error) => self.catalog = Some(Err(QueryFailure::Spawn(error))),
        }
    }

    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        if !self.pending.as_ref().is_some_and(JoinHandle::is_finished) {
            if self.pending.is_some() { ctx.request_repaint_after(Duration::from_millis(50)); }
            return;
        }
        let Some(job) = self.pending.take() else { return; };
        self.catalog = Some(match job.join() {
            Ok(result) => result.map_err(QueryFailure::Native),
            Err(payload) => Err(QueryFailure::Panic(payload)),
        });
        ctx.request_repaint();
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, width: f32, tooltips: bool) {
        if self.catalog.is_none() && self.pending.is_none() { self.start(); }
        let row = settings_row(ui, width, "Audio System");
        setting_value(ui, row, "Audio System", "Windows Audio", "Windows shared audio.", tooltips,
            false, SettingsValueStyle::AudioSystem);
        let catalog = self.catalog.as_ref().and_then(|result| result.as_ref().ok());
        let idle = self.pending.is_none();
        endpoint_selector(ui, width, "Output", &mut self.selection.output,
            catalog.map(|catalog| &catalog.render), idle, tooltips);
        endpoint_selector(ui, width, "Input", &mut self.selection.input,
            catalog.map(|catalog| &catalog.capture), idle, tooltips);
        setting_choice(ui, width, "Sample Rate", &mut self.selection.sample_rate,
            |rate| format!("{rate} Hz"), &[48_000]);
        let rate = self.selection.sample_rate;
        let duration_format = localization::tr(ui.ctx(), "_ samples (_ ms)");
        let samples_format = localization::tr(ui.ctx(), "_ samples");
        setting_choice(ui, width, "Buffer Size", &mut self.selection.buffer_size_frames,
            |frames| format_buffer_size(frames, rate, duration_format, samples_format),
            &[144, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 480, 512,
                576, 640, 704, 768, 832, 896, 960, 1024, 1152, 1280, 1408, 1536,
                1664, 1792, 1920, 2048]);
        if let Some(Err(error)) = &self.catalog {
            ui.label(egui::RichText::new(error.to_string()).size(12.0).color(MUTED));
        }
    }

    pub(super) fn stereo_ui(&mut self, ui: &mut egui::Ui, width: f32) {
        setting_choice(ui, width, "When the input is stereo", &mut self.selection.input_channel,
            |channel| match channel {
                0 => "Use the left channel".into(),
                1 => "Use the right channel".into(),
                _ => "Unavailable channel".into(),
            }, &[0, 1]);
    }
}

pub(super) fn format_buffer_size(frames: u32, rate: u32, duration_format: &str,
    samples_format: &str) -> String
{
    let frame_text = frames.to_string();
    if rate == 0 {
        return samples_format.replacen('_', &frame_text, 1);
    }
    let milliseconds = format!("{:.0}", f64::from(frames) * 1000.0 / f64::from(rate));
    duration_format.replacen('_', &frame_text, 1).replacen('_', &milliseconds, 1)
}

fn setting_choice<T: Copy + PartialEq>(ui: &mut egui::Ui, width: f32, label: &str,
    selected: &mut T, text: impl Fn(T) -> String, choices: &[T]) {
    let row = settings_row(ui, width, label);
    let value = settings_value_rect(row);
    let enabled = ui.is_enabled();
    let mut field_ui = ui.new_child(egui::UiBuilder::new().max_rect(value));
    field_ui.spacing_mut().interact_size.y = 24.0;
    field_ui.spacing_mut().button_padding.y = 3.0;
    field_ui.spacing_mut().button_padding.x = 6.0;
    field_ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.0));
    let selected_text = settings_reference_text(localization::tr(ui.ctx(), &text(*selected)).to_owned(), enabled);
    let response = egui::ComboBox::from_id_salt(("audio-setting", label))
        .width(value.width()).height(200.0).truncate().icon(settings_reference_combo_icon)
        .selected_text(selected_text)
        .show_ui(&mut field_ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_enabled_ui(enabled, |ui| {
                for &choice in choices {
                    let label = text(choice);
                    if settings_option(ui, &label, *selected == choice).clicked() { *selected = choice; }
                }
            });
        });
    let translated_label = localization::tr(ui.ctx(), label);
    let _ = response.response.ctx.accesskit_node_builder(response.response.id, |node| {
        node.set_label(translated_label);
    });
}

fn endpoint_name(endpoint: &Endpoint) -> &str {
    endpoint.friendly_name.as_ref().ok().and_then(Option::as_deref)
        .or_else(|| endpoint.id.as_ref().ok().map(String::as_str))
        .unwrap_or("Unnamed endpoint")
}

fn selected_endpoint<'a>(flow: &'a Flow, selected: &Option<String>) -> Option<&'a Endpoint> {
    let id = match selected {
        Some(id) => Some(id.as_str()),
        None => flow.default_multimedia_endpoint.as_ref().ok().and_then(Option::as_deref),
    }?;
    flow.endpoints.iter().filter_map(|entry| entry.endpoint.as_ref().ok())
        .find(|endpoint| endpoint.id.as_ref().is_ok_and(|actual| actual == id))
}

fn endpoint_selector(ui: &mut egui::Ui, width: f32, label: &str, selected: &mut Option<String>,
    result: Option<&std::result::Result<Flow, Error>>, idle: bool, tooltips: bool)
{
    let row = settings_row(ui, width, label);
    let value = settings_value_rect(row);
    let flow = result.and_then(|result| result.as_ref().ok());
    let text = match (selected.as_ref(), flow.and_then(|flow| selected_endpoint(flow, selected))) {
        (None, Some(endpoint)) => format!("{} — {}", localization::tr(ui.ctx(), "System default"), endpoint_name(endpoint)),
        (None, None) => localization::tr(ui.ctx(), "System default").into(),
        (Some(_), Some(endpoint)) => endpoint_name(endpoint).into(),
        (Some(id), None) if flow.is_none() => format!("Saved — {id}"),
        (Some(id), None) => format!("{} — {id}", localization::tr(ui.ctx(), "Unavailable")),
    };
    let enabled = ui.is_enabled() && idle && flow.is_some();
    let selected_text = settings_reference_text(text, enabled);

    let mut value_ui = ui.new_child(egui::UiBuilder::new().max_rect(value));
    value_ui.spacing_mut().interact_size.y = 24.0;
    value_ui.spacing_mut().button_padding.y = 3.0;
    value_ui.spacing_mut().button_padding.x = 6.0;
    value_ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.0));
    value_ui.add_enabled_ui(enabled, |ui| {
            let response = egui::ComboBox::from_id_salt(("audio-endpoint", label))
                .width(value.width()).height(200.0).truncate().icon(settings_reference_combo_icon).selected_text(selected_text)
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;

                    ui.add_enabled_ui(enabled, |ui| {
                        if settings_option(ui, "System default", selected.is_none()).clicked() { *selected = None; }
                        if let Some(flow) = flow {
                            for entry in &flow.endpoints {
                                match &entry.endpoint {
                                    Ok(endpoint) => {
                                        if let Ok(id) = &endpoint.id {
                                            let response = settings_option(ui, endpoint_name(endpoint), selected.as_deref() == Some(id.as_str()));
                                            if response.clicked() { *selected = Some(id.clone()); }
                                            optional_tooltip(response, tooltips, id);
                                        } else {
                                            ui.add_enabled(false, egui::Label::new(format!("{} (ID unavailable)", endpoint_name(endpoint))));
                                        }
                                    }
                                    Err(error) => { ui.add_enabled(false, egui::Label::new(format!("Endpoint {}: {error}", entry.ordinal))); }
                                }
                            }
                        }
                    });
                });
            let translated_label = localization::tr(ui.ctx(), label);
            let _ = response.response.ctx.accesskit_node_builder(response.response.id, |node| {
                node.set_label(translated_label);
            });
            if let Some(Err(error)) = result {
                optional_tooltip(response.response, tooltips, &error.to_string());
            }
    });
    if let Some(Err(error)) = result {
        ui.label(egui::RichText::new(format!("{label}: {error}")).size(12.0).color(MUTED));
    } else if let Some(flow) = flow && let Err(error) = &flow.default_multimedia_endpoint {
        ui.label(egui::RichText::new(format!("{label} default: {error}")).size(12.0).color(MUTED));
    }
}

