use super::halfband_up2::{ordinary_coefficients, ProfileVariant};

pub struct HalfbandDown2 {
    coefficients: &'static [u64],
    storage: Vec<f64>,
    capacity: usize,
    pending: usize,
    write: usize,
    read: usize,
    initial_discard: usize,
    discard: usize,
    fractional_latency: f64,
}

impl HalfbandDown2 {
    pub fn ordinary(
        stage_index: u32,
        variant: ProfileVariant,
        input_latency: f64,
    ) -> Result<Self, &'static str> {
        if stage_index > i32::MAX as u32 {
            return Err("nonnegative signed stage-index scope required");
        }
        let scaled_latency = input_latency * 0.5;
        if !input_latency.is_finite() || input_latency < 0.0
            || !scaled_latency.is_finite() || scaled_latency > i32::MAX as f64
        {
            return Err("finite nonnegative representable source latency required");
        }
        let initial_discard = scaled_latency.trunc() as usize;
        let fractional_latency = scaled_latency - initial_discard as f64;
        let coefficients = ordinary_coefficients(stage_index, variant);
        let length = coefficients.len();
        let capacity = 1025 - 2 * length;
        let mut result = Self {
            coefficients,
            storage: vec![0.0; 1024 + 4 * length - 2],
            capacity,
            pending: 0,
            write: 0,
            read: capacity,
            initial_discard,
            discard: initial_discard,
            fractional_latency,
        };
        result.reset();
        Ok(result)
    }

    pub fn fractional_latency(&self) -> f64 {
        self.fractional_latency
    }

    pub fn reset(&mut self) {
        self.pending = 0;
        self.write = 0;
        self.read = self.capacity;
        self.discard = self.initial_discard;
        self.storage[self.capacity..1024].fill(0.0);
    }

    fn sample(&self, center: usize) -> f64 {
        let mut accumulator = self.storage[center];
        for (index, coefficient) in self.coefficients.iter().enumerate() {
            let offset = index * 2 + 1;
            let sum = self.storage[center + offset] + self.storage[center - offset];
            let product = sum * f64::from_bits(*coefficient);
            accumulator = product + accumulator;
        }
        accumulator
    }

    pub fn process(&mut self, input: &[f64]) -> Result<Vec<f64>, &'static str> {
        if input.len() > i32::MAX as usize || input.iter().any(|x| !x.is_finite()) {
            return Err("bounded finite halfband source input required");
        }
        let length = self.coefficients.len();
        let retained = 2 * length - 1;
        let overlap = 2 * retained;
        let mut output = Vec::new();
        let mut consumed = 0usize;
        while consumed < input.len() {
            let count = (input.len() - consumed)
                .min(1024 - self.write)
                .min(self.capacity - self.pending);
            if count == 0 {
                return Err("source halfband ring progress invariant failed");
            }
            self.storage[self.write..self.write + count]
                .copy_from_slice(&input[consumed..consumed + count]);
            if self.write < overlap {
                let duplicate_count = count.min(overlap - self.write);
                self.storage.copy_within(
                    self.write..self.write + duplicate_count, self.write + 1024,
                );
            }
            self.write = (self.write + count) & 1023;
            self.pending += count;
            consumed += count;
            if self.pending > retained {
                let produced = (self.pending - retained + 1) / 2;
                for offset in 0..produced {
                    let read = (self.read + offset * 2) & 1023;
                    output.push(self.sample(retained + read));
                }
                self.read = (self.read + produced * 2) & 1023;
                self.pending -= produced * 2;
            }
        }
        let skipped = self.discard.min(output.len());
        self.discard -= skipped;
        if skipped != 0 {
            output.drain(..skipped);
        }
        Ok(output)
    }
}
