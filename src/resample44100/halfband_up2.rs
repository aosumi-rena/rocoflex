#[derive(Clone, Copy)]
pub enum ProfileVariant {
    False,
    True,
}

pub(super) fn ordinary_coefficients(stage: u32, variant: ProfileVariant) -> &'static [u64] {
    match (variant, stage) {
        (ProfileVariant::True, 0) => &[
            0x3fe3e4688f744ff5, 0xbfc5e5fcbb0f5082, 0x3fb1d08b1dbe1798,
            0xbf9bfeeb2e6caf63, 0x3f83097dc4210f58, 0xbf64eb88e8d3a56d,
            0x3f413a1559b4a20b, 0xbf12d3501066a08a, 0x3ed4781daf1d2ae8,
        ],
        (ProfileVariant::True, 1) => &[
            0x3fe39258b4748c2e, 0xbfc2df4823a18f7e, 0x3fa7375989e16805,
            0xbf86f3e3fa2f7b2a, 0x3f5e0787645fb981, 0xbf23077f487eeae0,
        ],
        (ProfileVariant::True, 2) => &[
            0x3fe3639bc04416c1, 0xbfc14ce859f3e71a, 0x3fa1ee40f7e97338,
            0xbf79e8e49bbfda36, 0x3f42303fc760c3ea,
        ],
        (ProfileVariant::True, 3) => &[
            0x3fe324bfd995dd6d, 0xbfbeaacc9c975935, 0x3f9898085cfd847f,
            0xbf642688eaeb1af7,
        ],
        (ProfileVariant::True, 4) => &[
            0x3fe2c041cae5da9b, 0xbfb90315b8e42a06, 0x3f88083b0ddf70d3,
        ],
        (ProfileVariant::True, 5) => &[
            0x3fe2c01072e03783, 0xbfb900c565e46520, 0x3f88020e77161b33,
        ],
        (ProfileVariant::True, _) => &[
            0x3fe2c0041b1f8dd5, 0xbfb9003145b09268, 0x3f88008365a12143,
        ],
        (ProfileVariant::False, 0) => &[
            0x3fe419ebbad1de75, 0xbfc8135bbbf40c16, 0x3fb7466ffb54920b,
            0xbfa7f1a5e136d079, 0x3f97d8af0bcadeea, 0xbf8609aaf0d07358,
            0x3f725f4653750f7c, 0xbf5aeeec12350919, 0x3f40db35c5691d82,
            0xbf21511e984a2a76, 0x3efb76adcb55c38a, 0xbece0883700e0a2d,
            0x3e911b5e464d4ff8,
        ],
        (ProfileVariant::False, 1) => &[
            0x3fe3b71f277520fb, 0xbfc42ed2a3cfc01e, 0x3fac34b74bd8a7dc,
            0xbf914d1ef7f0f249, 0x3f702522772d259e, 0xbf43dbcce9389dcd,
            0x3f07ecb447d97cf9,
        ],
        (ProfileVariant::False, 2) => &[
            0x3fe3668c528eb267, 0xbfc16496d9281e75, 0x3fa232dcf91d7af3,
            0xbf7aad1031ea032a, 0x3f4316d240e53954,
        ],
        (ProfileVariant::False, 3) => &[
            0x3fe325af800c13b2, 0xbfbeb84f95f1ed7d, 0x3f98b632bb26901c,
            0xbf6457232850ea7f,
        ],
        (ProfileVariant::False, 4) => &[
            0x3fe3246bec9997d7, 0xbfbea612bb9d04ae, 0x3f988d81bd0ac9b2,
            0xbf6415a30e4ed3c0,
        ],
        (ProfileVariant::False, 5) => &[
            0x3fe2c02502669c9a, 0xbfb901bc2deebd6d, 0x3f8804a0d5d8290c,
        ],
        (ProfileVariant::False, _) => &[
            0x3fe2c00940ae4b6e, 0xbfb9006f093d8337, 0x3f8801281e5963c8,
        ],
    }
}

pub struct HalfbandUp2 {
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

impl HalfbandUp2 {
    pub fn ordinary(
        stage_index: u32,
        variant: ProfileVariant,
        input_latency: f64,
    ) -> Result<Self, &'static str> {
        if stage_index > i32::MAX as u32 {
            return Err("nonnegative signed stage-index scope required");
        }
        let scaled_latency = input_latency * 2.0;
        if !input_latency.is_finite() || input_latency < 0.0
            || !scaled_latency.is_finite() || scaled_latency > i32::MAX as f64
        {
            return Err("finite nonnegative representable source latency required");
        }
        let initial_discard = scaled_latency.trunc() as usize;
        let fractional_latency = scaled_latency - initial_discard as f64;
        let coefficients = ordinary_coefficients(stage_index, variant);
        let length = coefficients.len();
        let capacity = 513 - length;
        let mut result = Self {
            coefficients,
            storage: vec![0.0; 512 + 2 * length - 1],
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
        self.storage[self.capacity..512].fill(0.0);
    }

    fn pair_product(&self, center: usize, pair: usize) -> [f64; 2] {
        let low = pair * 2;
        let high = low + 1;
        let low_sum = self.storage[center - low] + self.storage[center + low + 1];
        let high_sum = self.storage[center - high] + self.storage[center + high + 1];
        [
            low_sum * f64::from_bits(self.coefficients[low]),
            high_sum * f64::from_bits(self.coefficients[high]),
        ]
    }

    fn odd_sample(&self, center: usize) -> f64 {
        let pair_count = self.coefficients.len() / 2;
        let mut even = self.pair_product(center, 0);
        let lanes = if pair_count == 1 {
            even
        } else {
            let mut odd = self.pair_product(center, 1);
            for pair in 2..pair_count {
                let product = self.pair_product(center, pair);
                let accumulator = if pair & 1 == 0 { &mut even } else { &mut odd };
                accumulator[0] = product[0] + accumulator[0];
                accumulator[1] = product[1] + accumulator[1];
            }
            if (pair_count - 1) & 1 == 0 {
                [even[0] + odd[0], even[1] + odd[1]]
            } else {
                [odd[0] + even[0], odd[1] + even[1]]
            }
        };
        let sum = lanes[1] + lanes[0];
        if self.coefficients.len() & 1 == 0 {
            sum
        } else {
            let last = self.coefficients.len() - 1;
            let tail = self.storage[center + last + 1] + self.storage[center - last];
            let product = tail * f64::from_bits(self.coefficients[last]);
            product + sum
        }
    }

    pub fn process(&mut self, input: &[f64]) -> Result<Vec<f64>, &'static str> {
        if input.len() > i32::MAX as usize / 2 || input.iter().any(|x| !x.is_finite()) {
            return Err("bounded finite halfband source input required");
        }
        let length = self.coefficients.len();
        let overlap = 2 * length - 1;
        let mut output = Vec::with_capacity(input.len() * 2);
        let mut consumed = 0usize;
        while consumed < input.len() {
            let count = (input.len() - consumed)
                .min(512 - self.write)
                .min(self.capacity - self.pending);
            if count == 0 {
                return Err("source halfband ring progress invariant failed");
            }
            self.storage[self.write..self.write + count]
                .copy_from_slice(&input[consumed..consumed + count]);
            if self.write < overlap {
                let duplicate_count = count.min(overlap - self.write);
                self.storage.copy_within(
                    self.write..self.write + duplicate_count, self.write + 512,
                );
            }
            self.write = (self.write + count) & 511;
            self.pending += count;
            consumed += count;
            if self.pending > length {
                let produced = self.pending - length;
                for offset in 0..produced {
                    let read = (self.read + offset) & 511;
                    let center = length - 1 + read;
                    output.push(self.storage[center]);
                    output.push(self.odd_sample(center));
                }
                self.read = (self.read + produced) & 511;
                self.pending -= produced;
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
