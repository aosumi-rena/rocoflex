#[allow(dead_code)]
pub(super) struct HueRandom {
    state: u64,
}

#[allow(dead_code)]
impl HueRandom {
    pub fn from_state(state: u64) -> Self {
        Self { state }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_hue(&mut self) -> f32 {
        let next = self.state.wrapping_mul(0x5deece66d).wrapping_add(11);
        self.state = next & 0xffffffffffff;
        let value = (next >> 16) as u32;
        ((value as f32) * f32::from_bits(0x2f800000))
            .min(f32::from_bits(0x3f7ffffe))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ToneFit {
    pub rgb: [u8; 3],
    pub hue: f64,
    pub x: f64,
    pub y: f64,
}

pub fn rgb_at(x: f64, y: f64, hue: f64) -> [u8; 3] {
    let x = x.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let saturation = bilinear(0.252, 0.648, 0.821, 0.818, x, y);
    let dip = (bilinear(0.252, 0.648, 0.821, 0.818, 0.5, 0.5) - 0.482).max(0.0);
    let center_distance = (x - 0.5).hypot(y - 0.5) / 0.5_f64.hypot(0.5);
    let blend = (1.0 - center_distance).clamp(0.0, 1.0).powf(1.5);
    hsl_to_rgb(
        hue,
        (saturation - dip * blend).clamp(0.18, 0.9),
        bilinear(0.32, 0.722, 0.22, 0.496, x, y).clamp(0.12, 0.86),
    )
}

fn bilinear(a: f64, b: f64, c: f64, d: f64, x: f64, y: f64) -> f64 {
    let top = a + (b - a) * x;
    let bottom = c + (d - c) * x;
    top + (bottom - top) * y
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let raw = if hp < 1.0 {
        [c, x, 0.0]
    } else if hp < 2.0 {
        [x, c, 0.0]
    } else if hp < 3.0 {
        [0.0, c, x]
    } else if hp < 4.0 {
        [0.0, x, c]
    } else if hp < 5.0 {
        [x, 0.0, c]
    } else {
        [c, 0.0, x]
    };
    raw.map(|v| ((v + l - c / 2.0) * 255.0).round().clamp(0.0, 255.0) as u8)
}

fn rgb_to_hsl(rgb: [u8; 3]) -> (f64, f64, f64) {
    let [r, g, b] = rgb.map(|v| v as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta) % 6.0
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    let l = (max + min) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    ((h * 60.0 + 360.0) % 360.0, s, l)
}

pub fn fit(target: [u8; 3], fallback_hue: f64) -> ToneFit {
    let (base, saturation, _) = rgb_to_hsl(target);
    let mut hues = Vec::new();
    if saturation < 0.02 {
        hues.push(fallback_hue.rem_euclid(360.0));
    } else {
        for offset in (-28..=28).step_by(4) {
            hues.push(((base + offset as f64).rem_euclid(360.0) * 1000.0).round() / 1000.0);
        }
        let fallback = (fallback_hue.rem_euclid(360.0) * 1000.0).round() / 1000.0;
        if !hues.contains(&fallback) {
            hues.push(fallback);
        }
    }
    let mut best = ToneFit {
        rgb: rgb_at(0.5, 0.5, base),
        hue: base,
        x: 0.5,
        y: 0.5,
    };
    let mut distance = f64::INFINITY;
    for hue in hues {
        for yi in 0..=40 {
            for xi in 0..=40 {
                let x = xi as f64 / 40.0;
                let y = yi as f64 / 40.0;
                let rgb = rgb_at(x, y, hue);
                let d = (rgb[0] as f64 - target[0] as f64)
                    .hypot(rgb[1] as f64 - target[1] as f64)
                    .hypot(rgb[2] as f64 - target[2] as f64);
                if d < distance {
                    distance = d;
                    best = ToneFit { rgb, hue, x, y };
                }
            }
        }
    }
    best
}
