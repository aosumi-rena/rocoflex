use super::{BankProducer, streaming::ConvolutionCursor};

pub struct BankStream<'a> {
    producer: &'a BankProducer,
    convolutions: Vec<ConvolutionCursor>,
    row_limit: usize,
    accepted: usize,
    emitted: usize,
    ended: bool,
    halted: bool,
}

pub struct BankBatch<'a> {
    pub first_row: usize,
    pub work384: Vec<f32>,
    pub bank8: Vec<f32>,
    pub bank384: Vec<f32>,
    pub final_input: bool,
    pub state: BankStream<'a>,
}

pub struct BankStreamFailure<'a> {
    pub reason: String,
    pub block: Option<usize>,
    pub intermediate: Vec<f32>,
    pub work384: Vec<f32>,
    pub bank8: Vec<f32>,
    pub bank384: Vec<f32>,
    pub advanced: BankStream<'a>,
}

impl<'a> BankStream<'a> {
    pub(super) fn new(producer: &'a BankProducer, row_limit: usize) -> crate::Result<Self> {
        if producer.blocks.len() != 5 || row_limit == 0
            || row_limit > i32::MAX as usize || row_limit.checked_mul(384).is_none()
        {
            return Err("Bank stream requires five blocks and a positive i32 row limit".into());
        }
        let convolutions = producer.blocks.iter()
            .map(|b| ConvolutionCursor::new(&b.conv, row_limit))
            .collect::<crate::Result<Vec<_>>>()?;
        Ok(Self { producer, convolutions, row_limit, accepted: 0,
            emitted: 0, ended: false, halted: false })
    }

    pub fn accepted_rows(&self) -> usize { self.accepted }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, reason: impl Into<String>, block: Option<usize>,
        intermediate: Vec<f32>, work384: Vec<f32>, bank8: Vec<f32>, bank384: Vec<f32>)
        -> BankStreamFailure<'a>
    {
        BankStreamFailure { reason: reason.into(), block, intermediate,
            work384, bank8, bank384, advanced: self }
    }

    pub fn push(mut self, input60: &[f32], final_input: bool)
        -> Result<BankBatch<'a>, BankStreamFailure<'a>>
    {
        if self.ended || self.halted {
            return Err(self.failure("Bank stream is already ended or halted", None,
                Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }
        if input60.len() % 60 != 0 || input60.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Bank stream requires finite complete 60-lane rows", None,
                Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }
        let Some(accepted) = self.accepted.checked_add(input60.len() / 60)
            .filter(|&n| n <= self.row_limit) else {
            return Err(self.failure("Bank stream row limit exceeded", None,
                Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        };
        if final_input && accepted == 0 {
            return Err(self.failure("Bank stream requires nonempty final input", None,
                Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }
        self.accepted = accepted;
        self.ended = final_input;
        let first_row = self.emitted;
        let mut source = input60.to_vec();
        for i in 0..self.producer.blocks.len() {
            let block = &self.producer.blocks[i];
            let mut output = Vec::new();
            if let Err(reason) = self.convolutions[i].append(
                &block.conv, &source, final_input, &mut output)
            {
                self.halted = true;
                return Err(self.failure(reason, Some(i), source, output, Vec::new(), Vec::new()));
            }
            for row in output.chunks_exact_mut(384) {
                for j in 0..384 {
                    let scaled = row[j] * block.scale[j];
                    row[j] = (scaled + block.bias[j]).max(0.0);
                }
            }
            source = output;
        }
        let mut bank8 = Vec::new();
        let mut bank384 = Vec::new();
        for row in source.chunks_exact(384) {
            let small = match self.producer.head8.forward_saturation(row, true) {
                Ok(row) => row,
                Err(reason) => {
                    self.halted = true;
                    return Err(self.failure(reason, None, Vec::new(), source, bank8, bank384));
                }
            };
            let wide = match self.producer.head384.forward_saturation(row, true) {
                Ok(row) => row,
                Err(reason) => {
                    bank8.extend(small);
                    self.halted = true;
                    return Err(self.failure(reason, None, Vec::new(), source, bank8, bank384));
                }
            };
            bank8.extend(small);
            bank384.extend(wide);
        }
        self.emitted += source.len() / 384;
        Ok(BankBatch { first_row, work384: source, bank8, bank384, final_input, state: self })
    }

    pub fn push_vec90(self, input72: &[f32], final_input: bool)
        -> Result<BankBatch<'a>, BankStreamFailure<'a>>
    {
        if input72.len() % 72 != 0 || input72.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Bank stream requires finite complete 72-lane rows", None,
                Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        }
        let input = input72.chunks_exact(72)
            .flat_map(|row| row[..60].iter().copied()).collect::<Vec<_>>();
        self.push(&input, final_input)
    }
}
