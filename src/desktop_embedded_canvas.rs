use super::*;
use crate::prototype_controls::{self, Parameters, Snapshot};

pub fn offline_snapshot(preset: Preset, parameters: Parameters) -> Result<Option<Snapshot>> {
    source_math::initialize_current_process();
    let mut app = DesktopApp::new_canvas_state(PathBuf::new(), Vec::new(), None);
    app.plugin_hosted = true;
    install_document(&mut app, preset)?;
    if !has_target(&app) {
        return Ok(None);
    }
    let mix = if app.experimental.enabled {
        app.experimental_mix()?
    } else {
        app.layout_view.as_ref().and_then(|view| view.mix.clone())
            .ok_or("Canvas has no completed controller mix.")?
    };
    prototype_controls::from_controller_mix(&app.preset, &mix, parameters).map(Some)
}

pub(super) fn has_target(app: &DesktopApp) -> bool {
    app.preset.sources.iter().any(|source| source.enabled && source.points.iter().any(|point| point.enabled))
}

pub(super) fn install_document(app: &mut DesktopApp, preset: Preset) -> Result<()> {
    let [x, y, radius] = preset.probe;
    if !app.install(preset) {
        return Err(app.error.clone().or_else(|| app.layout_status.clone())
            .unwrap_or_else(|| "The preset could not be opened".into()));
    }
    if app.experimental.enabled && (-2.0..=2.0).contains(&x) && (-2.0..=2.0).contains(&y)
        && (0.04..=0.15).contains(&radius) && app.probe.position != Point::new(x, y)
        && !app.probe_changed(Probe { position: Point::new(x, y), radius })
    {
        return Err("The preset probe could not be restored".into());
    }
    app.settle_embedded_layout()
}

pub fn offline_model_paths() -> Result<(PathBuf, PathBuf)> {
    let settings = crate::offline::io::read_json(&preferences_dir().join("settings.json"), 1024 * 1024)
        .map_err(|_| "Set the model and userdata in Rocoflex Settings")?;
    let path = |key: &str| -> Result<PathBuf> {
        settings[key].as_str().filter(|p| !p.trim().is_empty()).map(PathBuf::from)
            .ok_or_else(|| "Set the model and userdata in Rocoflex Settings".into())
    };
    Ok((path("model")?, path("userdata")?))
}

