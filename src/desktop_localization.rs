use eframe::egui::{Context, Id};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Locale {
    #[default]
    English, SimplifiedChinese, TraditionalChinese, Japanese, Korean,
    German, LatinAmericanSpanish, French, BrazilianPortuguese, Russian, Vietnamese,
}

impl Locale {
    pub const ALL: [Self; 11] = [Self::English, Self::SimplifiedChinese,
        Self::TraditionalChinese, Self::Japanese, Self::Korean, Self::German,
        Self::LatinAmericanSpanish, Self::French, Self::BrazilianPortuguese,
        Self::Russian, Self::Vietnamese];

    pub fn id(self) -> &'static str {
        match self {
            Self::English => "en-us", Self::SimplifiedChinese => "zh-cn",
            Self::TraditionalChinese => "zh-tw", Self::Japanese => "ja-jp",
            Self::Korean => "ko-kr", Self::German => "de-de",
            Self::LatinAmericanSpanish => "es-la", Self::French => "fr-fr",
            Self::BrazilianPortuguese => "pt-br", Self::Russian => "ru-ru",
            Self::Vietnamese => "vi-vn",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English", Self::SimplifiedChinese => "简体中文",
            Self::TraditionalChinese => "繁體中文", Self::Japanese => "日本語",
            Self::Korean => "한국어", Self::German => "Deutsch",
            Self::LatinAmericanSpanish => "Español (Latinoamérica)",
            Self::French => "Français", Self::BrazilianPortuguese => "Português (Brasil)",
            Self::Russian => "Русский", Self::Vietnamese => "Tiếng Việt",
        }
    }

    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|locale| locale.id() == id).unwrap_or_default()
    }
}

type Catalog = HashMap<String, HashMap<String, String>>;

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("../assets/locales.json"))
        .expect("embedded locale catalog is valid"))
}

pub(super) fn set(ctx: &Context, locale: Locale) {
    ctx.data_mut(|data| data.insert_temp(Id::new("rocoflex-ui-locale"), locale));
}

pub(super) fn text<'a>(locale: Locale, english: &'a str) -> &'a str {
    catalog().get(locale.id()).and_then(|table| table.get(english))
        .map(String::as_str).unwrap_or(english)
}

pub(super) fn tr<'a>(ctx: &Context, english: &'a str) -> &'a str {
    let locale = ctx.data(|data| data.get_temp::<Locale>(Id::new("rocoflex-ui-locale")))
        .unwrap_or_default();
    text(locale, english)
}

pub(super) fn input_value(ctx: &Context, template: &str, value: &str) -> String {
    tr(ctx, template).replace("{value}", value)
}

pub(super) fn midi_binding(ctx: &Context, binding: super::midi_match::MidiBinding) -> String {
    let template = match binding.kind {
        super::midi_match::MidiKind::Note => "Note {number} / {channel}",
        super::midi_match::MidiKind::Controller => "CC {number} / {channel}",
    };
    tr(ctx, template).replace("{number}", &binding.number.to_string())
        .replace("{channel}", &(u16::from(binding.channel) + 1).to_string())
}

pub(super) fn input_message(ctx: &Context, message: &str) -> String {
    for (prefix, suffix, template) in [
        ("Invalid input shortcuts: ", "", "Invalid input shortcuts: {value}"),
        ("Invalid MIDI input selection: ", "", "Invalid MIDI input selection: {value}"),
        ("Could not list MIDI devices: ", ". Click Refresh MIDI devices to retry.", "Could not list MIDI devices: {value}. Click Refresh MIDI devices to retry."),
        ("Could not list MIDI devices: ", "", "Could not list MIDI devices: {value}"),
        ("MIDI close is retrying (Windows error ", ").", "MIDI close is retrying (Windows error {value})."),
        ("MIDI device open failed (Windows error ", ").", "MIDI device open failed (Windows error {value})."),
        ("MIDI device start failed (Windows error ", ").", "MIDI device start failed (Windows error {value})."),
        ("Could not start MIDI input: ", "", "Could not start MIDI input: {value}"),
        ("Could not read input bindings: ", "", "Could not read input bindings: {value}"),
        ("Could not inspect input bindings: ", "", "Could not inspect input bindings: {value}"),
        ("Input bindings file is invalid: ", "", "Input bindings file is invalid: {value}"),
        ("Could not create input bindings directory: ", "", "Could not create input bindings directory: {value}"),
        ("Could not serialize input bindings: ", "", "Could not serialize input bindings: {value}"),
        ("Could not create temporary input bindings: ", "", "Could not create temporary input bindings: {value}"),
        ("Could not save input bindings: ", "", "Could not save input bindings: {value}"),
        ("Could not replace input bindings: ", "", "Could not replace input bindings: {value}"),
        ("Could not open input bindings lock: ", "", "Could not open input bindings lock: {value}"),
        ("Input bindings are busy or locked: ", ". Try again.", "Input bindings are busy or locked: {value}. Try again."),
    ] {
        if let Some(value) = message.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix)) {
            return input_value(ctx, template, value);
        }
    }
    tr(ctx, message).to_owned()
}

pub(super) fn status(ctx: &Context, message: &str) -> String {
    for heading in ["Valid analysis model", "Invalid analysis model",
        "Valid morphing model", "Invalid morphing model"] {
        if let Some(rest) = message.strip_prefix(heading) {
            if let Some(numbers) = rest.strip_prefix("\nFormat ").and_then(|rest| rest.strip_suffix(" nodes"))
                && let Some((version, nodes)) = numbers.split_once(" · ")
                && !version.is_empty() && version.chars().all(|c| c.is_ascii_digit())
                && !nodes.is_empty() && nodes.chars().all(|c| c.is_ascii_digit())
            {
                let detail = tr(ctx, "Format _ · _ nodes").replacen('_', version, 1).replacen('_', nodes, 1);
                return format!("{}\n{detail}", tr(ctx, heading));
            }
            return format!("{}{rest}", tr(ctx, heading));
        }
    }
    tr(ctx, message).to_owned()
}

pub(super) fn original_tooltip(text: &str) -> bool {
    matches!(text,
        "Generate a new voice timbre from scratch."
        | "Import a voice timbre from an audio file."
        | "Pick a random hue for the voice timbre."
        | "Save the current voice as a new item and return to the main interface."
        | "Click to manually enter the color hex code."
        | "Turn off Realtime Mode to save CPU for applications where low latency is not required."
        | "Turn on Realtime Mode to optimize Vocoflex for live performance, which minimizes latency at the expense of CPU usage."
        | "Adjust how the algorithm balances latency with audio quality."
        | "Save and re-open the workspace as a preset, or load a predefined voice."
        | "Open settings."
        | "Adjust the pre-gain."
        | "Apply pitch shifting to the voice."
        | "Setup waypoints for quickly switching voices."
        | "Add a new waypoint at the cursor position."
        | "Reroute the waypoints by selecting them in the desired order."
        | "Assign a key shortcut for the selected waypoint."
        | "Assign a MIDI event for the selected waypoint."
        | "Press a key to change the assignment."
        | "Press a key to assign a shortcut."
        | "Send a MIDI event to change the assignment."
        | "Send a MIDI event to assign a shortcut."
        | "Assign a MIDI event for the action."
        | "Assign a key shortcut for the action.")
}

