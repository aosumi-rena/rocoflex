const COEFFICIENT_BITS: [[u32; 5]; 3] = [
    [0x3f7e1192, 0xbffe1192, 0x3f7e1192, 0xbffe1091, 0x3f7c2524],
    [0x3f7e950d, 0xbffe950d, 0x3f7e950d, 0xbffe940b, 0x3f7d2c1d],
    [0x3f7f7a0c, 0xbfff7a0c, 0x3f7f7a0c, 0xbfff790a, 0x3f7ef61c],
];

const CLEANUP_LOWER: f32 = f32::from_bits(0xb22bcc77);
const CLEANUP_UPPER: f32 = f32::from_bits(0x322bcc77);

pub(crate) struct SourceInputFilters {
    states: [[f32; 2]; 3],
}

impl SourceInputFilters {
    pub(crate) fn new_reference_48k() -> Self {
        Self { states: [[0.0; 2]; 3] }
    }

    pub(crate) fn process(&mut self, samples: &mut [f32; 240]) -> crate::Result<()> {
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err("input filters require finite samples".into());
        }

        let mut scratch = *samples;
        let mut states = self.states;
        for (entry, words) in COEFFICIENT_BITS.iter().enumerate() {
            let [b0, b1, b2, a1, a2] = words.map(f32::from_bits);
            let [mut s0, mut s1] = states[entry];

            for pair in scratch.chunks_exact_mut(2) {
                for sample in pair {
                    let x = *sample;
                    let feed0 = x * b0;
                    let y = feed0 + s0;
                    let feed1 = x * b1;
                    let feedback1 = a1 * y;
                    let delta1 = feed1 - feedback1;
                    let next0 = delta1 + s1;
                    let feed2 = x * b2;
                    let feedback2 = a2 * y;
                    let next1 = feed2 - feedback2;
                    if !y.is_finite() || !next0.is_finite() || !next1.is_finite() {
                        return Err(format!("input filter {entry} produced a nonfinite value"));
                    }
                    *sample = y;
                    s0 = next0;
                    s1 = next1;
                }
            }

            states[entry] = [cleanup(s0), cleanup(s1)];
        }

        *samples = scratch;
        self.states = states;
        Ok(())
    }
}

fn cleanup(state: f32) -> f32 {
    if state < CLEANUP_LOWER || state > CLEANUP_UPPER {
        state
    } else {
        0.0
    }
}
