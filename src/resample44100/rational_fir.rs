use super::rational_147_80::{pair_dot28, StreamState};

const CAPACITY: usize = 256;

#[derive(Clone, Copy)]
pub enum RationalKernel {
    False28,
    True22,
}

impl RationalKernel {
    fn taps(self) -> usize {
        match self {
            Self::False28 => 28,
            Self::True22 => 22,
        }
    }
}

pub struct RationalFir {
    coefficients: Vec<f64>,
    numerator: usize,
    phases: usize,
    kernel: RationalKernel,
    history: Vec<f64>,
    initialized: Vec<bool>,
    state: StreamState,
    initial_phase: usize,
    initial_discard: usize,
    pub unrepresented_latency_fraction: f64,
}

impl RationalFir {
    pub fn source_147(
        coefficients: Vec<f64>,
        phases: usize,
        kernel: RationalKernel,
        input_latency: f64,
    ) -> Result<Self, &'static str> {
        let supported = match kernel {
            RationalKernel::False28 => matches!(phases, 40 | 80 | 160),
            RationalKernel::True22 => phases == 40,
        };
        if !supported {
            return Err("explicit source-selected rational147 phase domain required");
        }
        Self::from_source_table(coefficients, 147, phases, kernel, input_latency)
    }

    pub fn from_source_table(
        coefficients: Vec<f64>,
        numerator: usize,
        phases: usize,
        kernel: RationalKernel,
        input_latency: f64,
    ) -> Result<Self, &'static str> {
        let taps = kernel.taps();
        if numerator == 0 || numerator > i32::MAX as usize || phases == 0 || phases >= 1501
            || numerator as f64 / phases as f64 > (taps / 2) as f64
            || coefficients.len() != (phases + 2) * taps
            || coefficients.iter().any(|x| !x.is_finite())
        {
            return Err("explicit finite source-selected coeff1/stencil2 table required");
        }
        if !input_latency.is_finite() || !(0.0..i32::MAX as f64).contains(&input_latency) {
            return Err("finite nonnegative representable source input latency required");
        }
        let initial_discard = input_latency.trunc() as usize;
        let fraction = input_latency - initial_discard as f64;
        let initial_phase = (fraction * phases as f64).trunc() as usize;
        let unrepresented_latency_fraction = fraction - initial_phase as f64 / phases as f64;
        let mut value = Self {
            coefficients,
            numerator,
            phases,
            kernel,
            history: vec![0.0; CAPACITY + taps - 1],
            initialized: vec![false; CAPACITY + taps - 1],
            state: StreamState {
                available: 0, write_cursor: 0, read_cursor: 0,
                phase: 0, startup_discard: 0,
            },
            initial_phase,
            initial_discard,
            unrepresented_latency_fraction,
        };
        value.reset();
        Ok(value)
    }

    pub fn reset(&mut self) {
        let fill_limit = 257 - self.kernel.taps() / 2;
        self.initialized.fill(false);
        self.history[fill_limit..CAPACITY].fill(0.0);
        self.initialized[fill_limit..CAPACITY].fill(true);
        self.state = StreamState {
            available: 0, write_cursor: 0, read_cursor: fill_limit,
            phase: self.initial_phase, startup_discard: self.initial_discard,
        };
    }

    pub fn process_into(
        &mut self,
        mut input: &[f64],
        output: &mut Vec<f64>,
    ) -> Result<usize, &'static str> {
        if input.len() > i32::MAX as usize || input.iter().any(|x| !x.is_finite()) {
            return Err("finite representable source input span required");
        }
        let begin = output.len();
        let taps = self.kernel.taps();
        let half = taps / 2;
        let fill_limit = 257 - half;
        if self.state.startup_discard != 0 {
            if self.state.startup_discard >= input.len() {
                self.state.startup_discard -= input.len();
                return Ok(0);
            }
            input = &input[self.state.startup_discard..];
            self.state.startup_discard = 0;
        }
        while !input.is_empty() {
            let write = self.state.write_cursor;
            let count = input.len().min(CAPACITY - write)
                .min(fill_limit - self.state.available);
            if count == 0 {
                return Err("source rational streaming progress invariant failed");
            }
            self.history[write..write + count].copy_from_slice(&input[..count]);
            self.initialized[write..write + count].fill(true);
            if write < taps - 1 {
                let overlap = count.min(taps - 1 - write);
                self.history[CAPACITY + write..CAPACITY + write + overlap]
                    .copy_from_slice(&input[..overlap]);
                self.initialized[CAPACITY + write..CAPACITY + write + overlap].fill(true);
            }
            self.state.write_cursor = (write + count) & 255;
            self.state.available += count;
            while self.state.available > half {
                let start = self.state.read_cursor;
                if !self.initialized[start..start + taps].iter().all(|x| *x) {
                    return Err("source rational consumed an uninitialized history word");
                }
                let row = self.state.phase * taps;
                let source = &self.history[start..start + taps];
                let coefficients = &self.coefficients[row..row + taps];
                output.push(match self.kernel {
                    RationalKernel::False28 => pair_dot28(source, coefficients),
                    RationalKernel::True22 => pair_dot22(source, coefficients),
                });
                let total = self.state.phase + self.numerator;
                let consumed = total / self.phases;
                self.state.phase = total % self.phases;
                self.state.available -= consumed;
                self.state.read_cursor = (start + consumed) & 255;
            }
            input = &input[count..];
        }
        Ok(output.len() - begin)
    }
}

fn pair_dot22(source: &[f64], coefficients: &[f64]) -> f64 {
    let mut lanes = [0.0; 2];
    for pair in 0..11 {
        for lane in 0..2 {
            let index = pair * 2 + lane;
            let product = source[index] * coefficients[index];
            lanes[lane] = product + lanes[lane];
        }
    }
    lanes[1] + lanes[0]
}
