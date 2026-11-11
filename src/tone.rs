use crate::{
    Result, codec,
    conditional_head::{ConditionalHead128, ConditionalHeadStages, map_tone_control},
    reduction::TimbreReduction,
};
use serde_json::{Value, json};

pub struct ToneGenerator {
    head: ConditionalHead128,
    reducer: TimbreReduction,
}

pub struct TonePoint {
    pub data: [f32; 128],
    pub reduced: [f32; 4],
}

pub struct ToneStages {
    pub mapped_p0: f32,
    pub normalized_p0: f32,
    pub noise: [f32; 128],
    pub point: TonePoint,
}

impl ToneGenerator {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        Ok(Self {
            head: ConditionalHead128::from_dnni(model)?,
            reducer: TimbreReduction::from_dnni(model)?,
        })
    }

    pub fn generate(&self, controls: [f32; 3]) -> Result<TonePoint> {
        Ok(self.generate_stages(controls)?.point)
    }

    pub(crate) fn generate_embedding(&self, controls: [f32; 3]) -> Result<[f32; 128]> {
        let (_, _, head) = self.generate_head(controls)?;
        if head.output128.iter().any(|value| !value.is_finite()) {
            return Err("Tone model produced nonfinite values".into());
        }
        head.output128.try_into().map_err(|_| "Tone head width mismatch".into())
    }

    fn generate_head(&self, controls: [f32; 3])
        -> Result<(f32, [f32; 128], ConditionalHeadStages)> {
        if controls.iter().any(|v| !v.is_finite()) {
            return Err("Tone controls must be finite".into());
        }
        let mapped_p0 = map_tone_control(controls[0])?;
        if !mapped_p0.is_finite()
            || controls
                .iter()
                .any(|v| !((v + 1.0) * std::f32::consts::TAU).is_finite())
        {
            return Err("Tone control exceeds the supported numeric range".into());
        }
        let noise = crate::tone_noise::mixed_noise(controls);
        let head = self.head.forward(mapped_p0, controls[1], &noise, 1.0)?;
        Ok((mapped_p0, noise, head))
    }

    pub fn generate_stages(&self, controls: [f32; 3]) -> Result<ToneStages> {
        let (mapped_p0, noise, head) = self.generate_head(controls)?;
        let reduced = self.reducer.forward(&head.output128)?;
        if head
            .output128
            .iter()
            .chain(&reduced)
            .any(|v| !v.is_finite())
        {
            return Err("Tone model produced nonfinite values".into());
        }
        Ok(ToneStages {
            mapped_p0,
            normalized_p0: head.normalized_p0,
            noise,
            point: TonePoint {
                data: head
                    .output128
                    .try_into()
                    .map_err(|_| "Tone head width mismatch")?,
                reduced: reduced
                    .try_into()
                    .map_err(|_| "Tone reducer width mismatch")?,
            },
        })
    }
}

impl TonePoint {
    pub fn source_document(&self, name: String, color: String) -> Result<Value> {
        Ok(json!({
            "type": "point",
            "enabled": true,
            "name": name,
            "color": color,
            "data": codec::encode(&self.data)?,
            "reducedData": codec::encode(&self.reduced)?,
        }))
    }

    pub fn display_color(controls: [f32; 3]) -> Result<[u8; 3]> {
        if controls.iter().any(|v| !v.is_finite()) {
            return Err("Tone controls must be finite".into());
        }
        let saturation = ((1.0 - controls[0]) * 0.5) + 0.4;
        let value = ((controls[1] * 0.5) + 0.4) * 255.0;
        let value = value.clamp(0.0, 255.0);
        let byte = |v: f32| v.round_ties_even() as u8;
        if saturation <= 0.0 {
            return Ok([byte(value); 3]);
        }
        let saturation = saturation.min(1.0);
        let hue = ((controls[2] - controls[2].floor()) * 360.0) / 60.0;
        let fraction = hue - hue.floor();
        let low = (1.0 - saturation) * value;
        let falling = value * (1.0 - saturation * fraction);
        let rising = value * (1.0 - saturation * (1.0 - fraction));
        let rgb = if hue < 1.0 {
            [value, rising, low]
        } else if hue < 2.0 {
            [falling, value, low]
        } else if hue < 3.0 {
            [low, value, rising]
        } else if hue < 4.0 {
            [low, falling, value]
        } else if hue < 5.0 {
            [rising, low, value]
        } else {
            [value, low, falling]
        };
        Ok(rgb.map(byte))
    }
}
