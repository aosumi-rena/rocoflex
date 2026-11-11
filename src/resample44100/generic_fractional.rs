use super::fractional_stencil::QuadraticCoefficients;

const CAPACITY: usize = 256;

pub struct GenericFractional {
    table: QuadraticCoefficients,
    source_rate: f64,
    target_rate: f64,
    history: Vec<f64>,
    initialized: Vec<bool>,
    available: usize,
    write: usize,
    read: usize,
    initial_discard: usize,
    discard: usize,
    initial_fraction: f64,
    fraction: f64,
    output_counter: i32,
    input_integer: i32,
    epoch_base: f64,
}

impl GenericFractional {
    pub fn from_source_table(
        table: QuadraticCoefficients,
        source_rate: f64,
        target_rate: f64,
        input_latency: f64,
    ) -> Result<Self, &'static str> {
        let taps = table.taps;
        if !matches!(taps, 22 | 28) || table.phase_count == 0
            || table.phase_count > i32::MAX as usize || table.row_stride != taps * 3
            || table.coefficients.len() != (table.phase_count + 1) * table.row_stride
            || table.coefficients.iter().any(|x| !x.is_finite())
        {
            return Err("bounded source-selected quadratic22/28 table required");
        }
        if !source_rate.is_finite() || !target_rate.is_finite()
            || source_rate <= 0.0 || target_rate <= 0.0
            || source_rate / target_rate > (taps / 2) as f64
        {
            return Err("bounded finite positive source/target rate domain required");
        }
        if !input_latency.is_finite() || !(0.0..i32::MAX as f64).contains(&input_latency) {
            return Err("finite nonnegative representable source input latency required");
        }
        let initial_discard = input_latency.trunc() as usize;
        let initial_fraction = input_latency - initial_discard as f64;
        let mut value = Self {
            table, source_rate, target_rate,
            history: vec![0.0; CAPACITY + taps - 1],
            initialized: vec![false; CAPACITY + taps - 1],
            available: 0, write: 0, read: 0,
            initial_discard, discard: initial_discard,
            initial_fraction, fraction: initial_fraction,
            output_counter: 0, input_integer: 0, epoch_base: 0.0,
        };
        value.reset();
        Ok(value)
    }

    pub fn reset(&mut self) {
        let fill_limit = 257 - self.table.taps / 2;
        self.initialized.fill(false);
        self.history[fill_limit..CAPACITY].fill(0.0);
        self.initialized[fill_limit..CAPACITY].fill(true);
        self.available = 0;
        self.write = 0;
        self.read = fill_limit;
        self.discard = self.initial_discard;
        self.fraction = self.initial_fraction;
        self.output_counter = 0;
        self.input_integer = 0;
        self.rebase();
    }

    fn rebase(&mut self) {
        let mut base = self.fraction;
        base *= self.target_rate;
        base /= self.source_rate;
        self.epoch_base = base;
    }

    fn sample(&self) -> Result<f64, &'static str> {
        if !(0.0..1.0).contains(&self.fraction) {
            return Err("source fractional phase outside supported domain");
        }
        let position = self.fraction * self.table.phase_count as f64;
        let phase = position.trunc() as usize;
        let fraction = position - phase as f64;
        let square = fraction * fraction;
        let row = self.table.row(phase)?;
        let pairs = self.table.taps / 2;
        let bulk_pairs = pairs & !3;
        let mut lanes = [0.0; 2];
        for pair in 0..pairs {
            for lane in 0..2 {
                let base = pair * 6 + lane;
                let linear = row[base + 2] * fraction;
                let quadratic = row[base + 4] * square;
                let coefficient = (quadratic + linear) + row[base];
                let source = self.history[self.read + pair * 2 + lane];
                let product = if pair < bulk_pairs && pair & 3 == 0 {
                    source * coefficient
                } else {
                    coefficient * source
                };
                lanes[lane] = if pair < bulk_pairs {
                    product + lanes[lane]
                } else {
                    lanes[lane] + product
                };
            }
        }
        Ok(lanes[1] + lanes[0])
    }

    fn emit_available(&mut self, output: &mut Vec<f64>) -> Result<(), &'static str> {
        let taps = self.table.taps;
        while self.available > taps / 2 {
            if !self.initialized[self.read..self.read + taps].iter().all(|x| *x) {
                return Err("source generic fractional consumed uninitialized history");
            }
            output.push(self.sample()?);
            let counter = self.output_counter.checked_add(1)
                .ok_or("source generic output counter overflow")?;
            let mut position = f64::from(counter);
            position += self.epoch_base;
            position *= self.source_rate;
            position /= self.target_rate;
            if !position.is_finite() || position < 0.0 || position >= i32::MAX as f64 {
                return Err("source generic position outside signed integer domain");
            }
            let integer = position.trunc() as i32;
            let consumed = integer.checked_sub(self.input_integer)
                .ok_or("source generic integer displacement overflow")?;
            if consumed < 0 || consumed as usize > self.available {
                return Err("source generic displacement outside available history");
            }
            self.output_counter = counter;
            self.input_integer = integer;
            self.fraction = position - f64::from(integer);
            self.available -= consumed as usize;
            self.read = (self.read + consumed as usize) & 255;
        }
        Ok(())
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
        if self.discard != 0 {
            if self.discard >= input.len() {
                self.discard -= input.len();
                return Ok(0);
            }
            input = &input[self.discard..];
            self.discard = 0;
        }
        let taps = self.table.taps;
        let fill_limit = 257 - taps / 2;
        while !input.is_empty() {
            let write = self.write;
            let count = input.len().min(CAPACITY - write).min(fill_limit - self.available);
            if count == 0 {
                return Err("source generic fractional progress invariant failed");
            }
            self.history[write..write + count].copy_from_slice(&input[..count]);
            self.initialized[write..write + count].fill(true);
            if write < taps - 1 {
                let overlap = count.min(taps - 1 - write);
                self.history[CAPACITY + write..CAPACITY + write + overlap]
                    .copy_from_slice(&input[..overlap]);
                self.initialized[CAPACITY + write..CAPACITY + write + overlap].fill(true);
            }
            self.write = (write + count) & 255;
            self.available += count;
            self.emit_available(output)?;
            input = &input[count..];
        }
        if self.output_counter >= 1001 {
            self.output_counter = 0;
            self.input_integer = 0;
            self.rebase();
        }
        Ok(output.len() - begin)
    }
}
