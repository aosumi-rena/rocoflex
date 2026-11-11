use crate::Result;

pub struct MinStd {
    state: u32,
}

impl MinStd {
    pub fn from_state(state: u32) -> Result<Self> {
        if state == 0 || state >= 2_147_483_647 {
            return Err("Invalid noise generator state".into());
        }
        Ok(Self { state })
    }

    pub fn state(&self) -> u32 {
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = ((u64::from(self.state) * 48_271) % 2_147_483_647) as u32;
        self.state
    }

    pub fn uniform(&mut self) -> f32 {
        let value = self.next_u32() as f32;
        let shifted = value - 1.0;
        shifted * f32::from_bits(0x30000000)
    }
}

#[derive(Default)]
pub struct NormalDistribution {
    spare: Option<f32>,
}

impl NormalDistribution {
    pub fn spare(&self) -> Option<f32> {
        self.spare
    }

    pub fn sample(
        &mut self,
        rng: &mut MinStd,
        mean: f32,
        scale: f32,
        cache_second: bool,
    ) -> Result<f32> {
        if !mean.is_finite() || !scale.is_finite() {
            return Err("Invalid noise distribution parameters".into());
        }
        if cache_second && let Some(value) = self.spare.take() {
            return Ok(value * scale + mean);
        }
        let (mut x, mut y, mut radius) = loop {
            let a = rng.uniform();
            let b = rng.uniform();
            let x = (a + a) - 1.0;
            let y = (b + b) - 1.0;
            if x == 0.0 || y == 0.0 {
                continue;
            }
            let radius = x * x + y * y;
            if radius < 1.0 {
                break (x, y, radius);
            }
        };
        let logarithm = if radius <= f32::from_bits(0x38d1b717) {
            let maximum = x.abs().max(y.abs());
            let exponent = ((maximum.to_bits() >> 23) & 255) as i32 - 127;
            let factor = 2.0_f64.powi(-exponent) as f32;
            x *= factor;
            y *= factor;
            radius = x * x + y * y;
            let logarithm = radius.ln();
            (exponent as f32 * f32::from_bits(0x3fb17218)) + logarithm
        } else {
            radius.ln()
        };
        let factor = ((logarithm * -2.0) / radius).sqrt();
        if cache_second {
            self.spare = Some(y * factor);
        }
        Ok((x * factor) * scale + mean)
    }
}
