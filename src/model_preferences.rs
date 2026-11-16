use std::path::PathBuf;

pub(crate) fn directory() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Rocoflex")
}

pub(crate) fn configured_models() -> crate::Result<(PathBuf, PathBuf, bool)> {
    let settings = crate::offline::io::read_json(&directory().join("settings.json"), 1024 * 1024)
        .map_err(|_| "Set the model files in Rocoflex Settings.")?;
    let path = |key: &str| settings[key].as_str().map(str::trim).filter(|path| !path.is_empty());
    let analysis = path("model").ok_or("Choose an analysis model in Rocoflex Settings.")?;
    let legacy = settings["useLegacyVocoder"].as_bool().unwrap_or(false);
    let synthesis = if legacy {
        path("legacyVocoder").ok_or("Choose a legacy vocoder in Rocoflex Settings.")?
    } else {
        path("userdata").ok_or("Choose a morphing model in Rocoflex Settings.")?
    };
    Ok((analysis.into(), synthesis.into(), legacy))
}
