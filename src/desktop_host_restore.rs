use super::*;
use crate::prototype_controls::Parameters;

#[derive(Clone)]
pub(crate) enum ExpectedCondition {
    Empty,
    Canvas { embedding: [f32; 128], parameters: Parameters },
    Tone { controls: [f32; 3], parameters: Parameters },
    Unavailable,
}

pub(super) fn same_canvas_sources(a: &Preset, b: &Preset) -> bool {
    fn same_values(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(a,b)| a.to_bits() == b.to_bits())
    }
    a.transformation.map(f32::to_bits) == b.transformation.map(f32::to_bits)
        && a.sources.len() == b.sources.len() && a.sources.iter().zip(&b.sources).all(|(a,b)|
        a.kind == b.kind && a.enabled == b.enabled && same_values(&a.data,&b.data)
        && a.points.len() == b.points.len() && a.points.iter().zip(&b.points).all(|(a,b)|
            a.enabled == b.enabled && same_values(&a.data,&b.data) && same_values(&a.reduced,&b.reduced)))
}

impl ExpectedCondition {
    pub(crate) fn validate(&self) -> Result<()> {
        let (values, parameters): (&[f32], Option<Parameters>) = match self {
            Self::Empty => (&[], None),
            Self::Canvas { embedding, parameters } => (embedding, Some(*parameters)),
            Self::Tone { controls, parameters } => (controls, Some(*parameters)),
            Self::Unavailable => return Err("The saved voice selection is unavailable; it cannot be reconstructed as a new selection.".into()),
        };
        if values.iter().any(|v| !v.is_finite()) || parameters.is_some_and(|p|
            !p.pregain_db.is_finite() || !p.pitch_semitones.is_finite()) {
            return Err("The saved voice condition is invalid.".into());
        }
        Ok(())
    }

    pub(super) fn verify(&self, preset: &Preset, view: Option<&transactions::LayoutView>) -> Result<()> {
        self.validate()?;
        let enabled = preset.sources.iter().any(|source|
            source.enabled && source.points.iter().any(|point| point.enabled));
        match self {
            Self::Empty if enabled => Err("The saved empty selection disagrees with the restored canvas.".into()),
            Self::Canvas { embedding, parameters } => {
                if !enabled { return Err("The saved voice target has no enabled canvas source.".into()); }
                let experimental=crate::preset::experimental_canvas_metadata(&preset.document)?;
                let mix = if experimental.is_some_and(|value|value.enabled) {
                    // Validate the same complete 128D source payload and control ranges.
                    // The separately retained host embedding remains authoritative.
                    let mut selected = false;
                    let mut weights = Vec::new();
                    let mut blends = Vec::new();
                    for source in &preset.sources {
                        for point in &source.points {
                            let take = !selected && source.enabled && point.enabled;
                            if take { selected = true; }
                            weights.push([if take {1.0} else {0.0}, 0.0]);
                        }
                        blends.push((source.kind == "trajectory").then_some(0.0));
                    }
                    Some(crate::prototype_controls::ControllerMix { point_weights: weights,
                        positive_scale:1.0, negative_scale:0.0, trajectory_blends:blends })
                } else { None };
                let mix = mix.as_ref().or_else(|| view.and_then(|v| v.mix.as_deref()))
                    .ok_or("The restored canvas has no completed voice mix.")?;
                let _ = embedding;
                let candidate = crate::prototype_controls::from_controller_mix(preset, mix, *parameters)?;
                if candidate.parameters.pregain_db.to_bits() != parameters.pregain_db.to_bits()
                    || candidate.parameters.pitch_semitones.to_bits() != parameters.pitch_semitones.to_bits() {
                    return Err("The saved voice controls are outside the supported range.".into());
                }
                Ok(())
            }
            Self::Empty | Self::Tone { .. } => Ok(()),
            Self::Unavailable => unreachable!("validated above"),
        }
    }

    pub(super) fn verify_current(&self, app: &mut DesktopApp) -> Result<()> {
        self.verify(&app.preset, app.layout_view.as_ref())?;
        if let Self::Canvas { embedding, parameters } = self {
            let current = match app.live.restored_canvas() {
                Some(current) => current,
                None if app.experimental.enabled => {
                    let mix = app.experimental_mix()?;
                    crate::prototype_controls::from_controller_mix(&app.preset, &mix, *parameters)?.embedding
                }
                None => {
                    let mix = app.layout_view.as_ref().and_then(|v| v.mix.as_deref())
                        .ok_or("The restored canvas has no completed voice mix.")?;
                    crate::prototype_controls::from_controller_mix(&app.preset, mix, *parameters)?.embedding
                }
            };
            if current.map(f32::to_bits) != embedding.map(f32::to_bits) {
                return Err("The host replaced the accepted canvas voice target.".into());
            }
        }
        Ok(())
    }

    pub(super) fn apply_controls(&self, app: &mut DesktopApp) {
        app.close_overlays();
        app.live.set_restored_canvas(match self {
            Self::Canvas { embedding, .. } => Some(*embedding),
            _ => None,
        });
        let current = match self {
            Self::Canvas { embedding, parameters } => Some(crate::current_condition::CurrentCondition {
                revision: 0, target: crate::current_condition::ConditionTarget::Canvas128 {
                    embedding: *embedding, targets: Vec::new() },
                pregain_db: parameters.pregain_db, pitch_semitones: parameters.pitch_semitones,
            }),
            Self::Tone { controls, parameters } => Some(crate::current_condition::CurrentCondition {
                revision: 0, target: crate::current_condition::ConditionTarget::TonePreview { controls: *controls },
                pregain_db: parameters.pregain_db, pitch_semitones: parameters.pitch_semitones,
            }),
            Self::Empty => {
                app.monitor_controls.set_values(0.0, 0.0);
                None
            }
            Self::Unavailable => None,
        };
        if let Some(current) = current {
            debug_assert!(!app.layout_blocked());
            app.restore_plugin_condition(&current);
        }
    }
}
