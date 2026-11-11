const TAPS: usize = 28;
const PHASES: usize = 80;
const HALF: usize = 14;
const CAPACITY: usize = 256;
const FILL_LIMIT: usize = 243;

pub struct RationalTable28 {
    coefficients: Vec<f64>,
}

impl RationalTable28 {
    pub fn from_coeff1_stencil2(coefficients: Vec<f64>) -> Result<Self, &'static str> {
        if coefficients.len() != (PHASES + 2) * TAPS
            || coefficients.iter().any(|x| !x.is_finite())
        {
            return Err("finite 82x28 source coeff1/stencil2 table required");
        }
        Ok(Self { coefficients })
    }

    fn row(&self, phase: usize) -> &[f64] {
        &self.coefficients[phase * TAPS..(phase + 1) * TAPS]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamState {
    pub available: usize,
    pub write_cursor: usize,
    pub read_cursor: usize,
    pub phase: usize,
    pub startup_discard: usize,
}

pub struct Rational147To80 {
    table: RationalTable28,
    history: [f64; CAPACITY + TAPS - 1],
    initialized: [bool; CAPACITY + TAPS - 1],
    state: StreamState,
    initial_phase: usize,
    initial_discard: usize,
    pub unrepresented_latency_fraction: f64,
}

impl Rational147To80 {
    pub fn new(table: RationalTable28, input_latency: f64) -> Result<Self, &'static str> {

        if !input_latency.is_finite() || !(0.0..i32::MAX as f64).contains(&input_latency) {
            return Err("finite nonnegative representable source input latency required");
        }
        let initial_discard = input_latency.trunc() as usize;
        let fraction = input_latency - initial_discard as f64;
        let initial_phase = (fraction * PHASES as f64).trunc() as usize;
        let unrepresented_latency_fraction = fraction - initial_phase as f64 / PHASES as f64;
        let mut value = Self {
            table,

            history: [0.0; CAPACITY + TAPS - 1],
            initialized: [false; CAPACITY + TAPS - 1],
            state: StreamState { available: 0, write_cursor: 0, read_cursor: 0,
                phase: 0, startup_discard: 0 },
            initial_phase,
            initial_discard,
            unrepresented_latency_fraction,
        };
        value.reset();
        Ok(value)
    }

    pub fn state(&self) -> StreamState {
        self.state
    }

    pub fn reset(&mut self) {
        self.initialized.fill(false);

        self.history[FILL_LIMIT..CAPACITY].fill(0.0);
        self.initialized[FILL_LIMIT..CAPACITY].fill(true);
        self.state = StreamState {
            available: 0, write_cursor: 0, read_cursor: FILL_LIMIT,
            phase: self.initial_phase, startup_discard: self.initial_discard,
        };
    }

    pub fn process_into(&mut self, mut input: &[f64], output: &mut Vec<f64>) -> Result<usize, &'static str> {

        if input.len() > i32::MAX as usize || input.iter().any(|x| !x.is_finite()) {
            return Err("finite representable source input span required");
        }
        let begin = output.len();
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
                .min(FILL_LIMIT - self.state.available);
            assert!(count > 0, "source streaming progress invariant");
            self.history[write..write + count].copy_from_slice(&input[..count]);
            self.initialized[write..write + count].fill(true);
            if write < TAPS - 1 {
                let overlap = count.min(TAPS - 1 - write);
                self.history[CAPACITY + write..CAPACITY + write + overlap]
                    .copy_from_slice(&input[..overlap]);
                self.initialized[CAPACITY + write..CAPACITY + write + overlap].fill(true);
            }
            self.state.write_cursor = (write + count) & 255;
            self.state.available += count;
            self.emit_available(output);
            input = &input[count..];
        }
        Ok(output.len() - begin)
    }

    fn emit_available(&mut self, output: &mut Vec<f64>) {
        while self.state.available > HALF {
            let start = self.state.read_cursor;
            assert!(self.initialized[start..start + TAPS].iter().all(|x| *x),
                "source consumed an uninitialized history word");
            output.push(pair_dot28(&self.history[start..start + TAPS],
                self.table.row(self.state.phase)));
            let total = self.state.phase + 147;
            let consumed = total / PHASES;
            self.state.phase = total % PHASES;
            self.state.available -= consumed;
            self.state.read_cursor = (start + consumed) & 255;
        }
    }
}

pub(super) fn pair_dot28(source: &[f64], coefficients: &[f64]) -> f64 {
    let mut lanes = [0.0; 2];
    for pair in 0..14 {
        for lane in 0..2 {
            let index = pair * 2 + lane;
            let product = source[index] * coefficients[index];
            lanes[lane] = product + lanes[lane];
        }
    }
    lanes[1] + lanes[0]
}
