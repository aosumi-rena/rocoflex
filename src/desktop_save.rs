use super::*;
use serde_json::Number;

pub(super) struct SavedSnapshot {
    preset: Preset,
    pub serialized: String,
}

impl AsRef<[u8]> for SavedSnapshot {
    fn as_ref(&self) -> &[u8] {
        self.serialized.as_bytes()
    }
}

fn snapshot_document(
    document: &Value,
    working_uv: Option<[f32; 8]>,
    probe: Probe,
) -> std::result::Result<Value, &'static str> {
    let Some(uv) = working_uv else {
        return Ok(document.clone());
    };
    if uv.iter().any(|value| !value.is_finite()) {
        return Err("The current transformation cannot be saved.");
    }
    if !probe.position.x.is_finite() || !probe.position.y.is_finite()
        || !probe.radius.is_finite() || probe.radius <= 0.0
    {
        return Err("The current probe cannot be saved.");
    }
    let mut result = document.clone();
    let voice = if result.get("voice").is_some() {
        result.get_mut("voice").expect("voice key is present")
    } else {
        &mut result
    };
    let object = voice
        .as_object_mut()
        .ok_or("The preset voice is not an object.")?;
    let values = uv
        .into_iter()
        .map(|value| {
            Value::Number(
                Number::from_f64(f64::from(value)).expect("finite f32 widens to finite f64"),
            )
        })
        .collect();
    object.insert("transformation".into(), Value::Array(values));

    for (key, value) in ["probeLocationX", "probeLocationY", "probeRadius"].into_iter()
        .zip([probe.position.x, probe.position.y, probe.radius])
    {
        object.insert(key.into(), Value::Number(
            Number::from_f64(value).expect("finite committed probe")));
    }
    Ok(result)
}

impl DesktopApp {
    pub(super) fn prepare_saved_snapshot(&self, name: Option<&str>) -> Result<SavedSnapshot> {
        let working_uv = self.layout.working_uv_snapshot();
        let mut document =
            snapshot_document(&self.preset.document, working_uv, self.probe).map_err(str::to_owned)?;
        if let Some(name) = name {
            document["presetName"] = json!(name);
        }

        let preset = Preset::parse(document).map_err(|_| {
            "The preset cannot be saved with its current inspection projection.".to_owned()
        })?;
        let serialized =
            if working_uv.is_none() && name.is_none() && !self.dirty && !self.original.is_empty() {
                self.original.clone()
            } else {
                serde_json::to_string_pretty(&preset.document).map_err(|e| e.to_string())?
            };
        Ok(SavedSnapshot { preset, serialized })
    }

    pub(super) fn commit_saved_snapshot(&mut self, snapshot: SavedSnapshot) {
        self.live.invalidate_canvas_condition();
        self.preset = snapshot.preset;
        self.original = snapshot.serialized;
        self.dirty = false;
    }
}
