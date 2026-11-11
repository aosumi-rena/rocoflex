const TERMS: usize = 64;
const WIDTH: usize = 128;
pub const UI_TERM_STRIDE: u32 = 16;

struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut state = [0; 624];
        state[0] = seed;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1812433253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn next(&mut self) -> u32 {
        if self.index == 624 {
            for i in 0..624 {
                let value = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (value >> 1)
                    ^ if value & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c5680;
        value ^= (value << 15) & 0xefc60000;
        value ^ (value >> 18)
    }

    fn signed_unit(&mut self) -> f32 {
        let value = self.next() as f32 * f32::from_bits(0x2f800000);
        (value + value) - 1.0
    }
}

#[derive(Default)]
struct Normal {
    cached: Option<f32>,
}

impl Normal {
    fn next(&mut self, rng: &mut Mt19937) -> f32 {
        if let Some(value) = self.cached.take() {
            return value * 1.0 + 0.0;
        }
        loop {
            let mut x = rng.signed_unit();
            let mut y = rng.signed_unit();
            if x == 0.0 || y == 0.0 {
                continue;
            }
            let mut sum = x * x + y * y;
            if sum >= 1.0 {
                continue;
            }
            let logarithm = if sum > 0.0001 {
                sum.ln()
            } else {

                let largest = x.abs().max(y.abs());
                let exponent = ((largest.to_bits() >> 23) & 255) as i32 - 127;
                let multiplier = f32::from_bits(((127 - exponent) as u32) << 23);
                x *= multiplier;
                y *= multiplier;
                sum = x * x + y * y;
                (exponent as f32 * f32::from_bits(0x3fb17218)) + sum.ln()
            };
            let factor = ((logarithm * -2.0) / sum).sqrt();
            self.cached = Some(y * factor);
            return (x * factor) * 1.0 + 0.0;
        }
    }
}

pub fn term_seeds_with_stride(input: f32, base: u32, stride: u32) -> [u32; TERMS] {
    assert!(stride > 0 && stride <= 0x1000000);
    let amplitude = stride as f32;
    std::array::from_fn(|i| {
        let angle = ((i as f32 / (TERMS - 1) as f32) + input) * f32::from_bits(0x40490fdb);
        let doubled = angle + angle;
        let quantized = ((doubled.cos() * amplitude) + 0.5)
            .round()
            .clamp(-amplitude, amplitude) as i32;
        base.wrapping_add(stride.wrapping_mul(i as u32 + 1))
            .wrapping_add(quantized as u32)
            .wrapping_sub(1)
    })
}

pub fn basis_noise_with_stride(input: f32, base: u32, stride: u32) -> [f32; WIDTH] {
    let mut output = [0.0; WIDTH];
    for seed in term_seeds_with_stride(input, base, stride) {
        let mut rng = Mt19937::new(seed);
        let mut normal = Normal::default();
        for _ in 0..16 {
            normal.next(&mut rng);
        }
        for value in &mut output {
            *value += normal.next(&mut rng);
        }
    }
    for value in &mut output {
        *value /= (TERMS as f32).sqrt();
    }
    output
}

pub fn mixed_noise_with_stride(controls: [f32; 3], stride: u32) -> [f32; WIDTH] {
    let hue = basis_noise_with_stride(controls[2], 0xc001babe, stride);
    let gender = basis_noise_with_stride(controls[0], 0xdeadbeef, stride);
    let tone = basis_noise_with_stride(controls[1], 0x0badbabe, stride);
    std::array::from_fn(|i| {
        let combined = (hue[i] as f64 + (0.3 * gender[i] as f64 + 0.3 * tone[i] as f64)) as f32;
        (combined as f64 / 1.086278049120021) as f32
    })
}

pub fn mixed_noise(controls: [f32; 3]) -> [f32; WIDTH] {
    mixed_noise_with_stride(controls, UI_TERM_STRIDE)
}
