use super::uniform::GaussianPairCache;

pub trait UniformSource { fn next_uniform(&mut self) -> Result<f32, String>; }
pub trait GaussianMath {
    fn log_radius(&mut self, value: f32) -> f32;
    fn binary_exponent(&mut self, value: f32) -> i32;
    fn scale_power_two(&mut self, value: f32, exponent: i32) -> f32;
    fn exceptional_sqrt(&mut self, value: f32) -> f32;
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters { pub mean: f32, pub scale: f32 }

pub fn next(
    cache: &mut GaussianPairCache,
    uniform: &mut impl UniformSource,
    parameters: Parameters,
    cache_allowed: bool,
    math: &mut impl GaussianMath,
    attempt_limit: usize,
) -> Result<f32, String> {
    if !parameters.mean.is_finite() || !parameters.scale.is_finite() {
        return Err("Gaussian adapter requires finite parameters".into());
    }
    if cache_allowed && cache.pending {
        let value = f32::from_bits(cache.partner_bits);
        if !value.is_finite() { return Err("Gaussian cached partner must be finite".into()); }
        cache.pending = false;
        return Ok(value * parameters.scale + parameters.mean);
    }
    for _ in 0..attempt_limit {
        let a = uniform.next_uniform()?;
        let b = uniform.next_uniform()?;
        if !a.is_finite() || !b.is_finite() {
            return Err("Gaussian uniform input must be finite".into());
        }
        let mut x = (a + a) + f32::from_bits(0xbf80_0000);
        let mut y = (b + b) + f32::from_bits(0xbf80_0000);
        if x == 0.0 || y == 0.0 { continue; }
        let mut radius = x * x + y * y;
        if !(radius < 1.0) { continue; }
        let log = if radius <= f32::from_bits(0x38d1_b717) {
            let exponent = math.binary_exponent(y.abs().max(x.abs()));
            x = math.scale_power_two(x, -exponent);
            y = math.scale_power_two(y, -exponent);
            radius = x * x + y * y;
            let logarithm = math.log_radius(radius);
            exponent as f32 * f32::from_bits(0x3fb1_7218) + logarithm
        } else {
            math.log_radius(radius)
        };
        let quotient = (log * f32::from_bits(0xc000_0000)) / radius;
        let factor = if quotient.is_nan() || quotient < 0.0 {
            math.exceptional_sqrt(quotient)
        } else { quotient.sqrt() };
        if cache_allowed {
            cache.partner_bits = (y * factor).to_bits();
            cache.pending = true;
        }
        let standard = x * factor;
        return Ok(standard * parameters.scale + parameters.mean);
    }
    Err("Gaussian attempt limit reached; native loop has no such bound".into())
}
