use super::{PitchBase, PitchBaseStages};
use crate::{bank::streaming::ConvolutionCursor, timbre::runtime_silu};

pub struct PitchBaseStream<'a> {
    base: &'a PitchBase,
    first: ConvolutionCursor,
    second: ConvolutionCursor,
    accepted: usize,
    row_limit: usize,
    ended: bool,
    halted: bool,
}
pub(crate) struct PitchBaseState {
    first:ConvolutionCursor,second:ConvolutionCursor,accepted:usize,
    row_limit:usize,ended:bool,halted:bool,
}
impl PitchBaseState {
    pub(crate) fn count_cursor_state(&self)->(bool,bool) {
        let (a,af)=self.first.count_cursor_state();let (b,bf)=self.second.count_cursor_state();
        (a || b || self.accepted!=0 || self.ended,af || bf || self.halted)
    }
}
pub struct PitchBaseBatch<'a> {

    pub first_rows: [usize; 2],
    pub stages: PitchBaseStages,
    pub final_input: bool,
    pub state: PitchBaseStream<'a>,
}
pub struct PitchBaseStreamFailure<'a> {
    pub reason: String,
    pub stages: PitchBaseStages,
    pub advanced: PitchBaseStream<'a>,
}

impl<'a> PitchBaseStream<'a> {
    pub(crate) fn into_state(self)->PitchBaseState {
        let Self {first,second,accepted,row_limit,ended,halted,..}=self;
        PitchBaseState {first,second,accepted,row_limit,ended,halted}
    }
    pub(crate) fn from_state(base:&'a PitchBase,state:PitchBaseState)->Self {
        let PitchBaseState {first,second,accepted,row_limit,ended,halted}=state;
        Self {base,first,second,accepted,row_limit,ended,halted}
    }
    pub(super) fn new(base: &'a PitchBase, row_limit: usize) -> crate::Result<Self> {
        if row_limit == 0 || row_limit > i32::MAX as usize {
            return Err("Pitch base stream requires a positive i32 row limit".into());
        }
        Ok(Self { base, first: ConvolutionCursor::new(&base.first, row_limit)?,
            second: ConvolutionCursor::new(&base.second, row_limit)?,
            accepted: 0, row_limit, ended: false, halted: false })
    }

    pub fn accepted_rows(&self) -> usize { self.accepted }
    pub fn emitted_rows(&self) -> usize { self.second.emitted_rows() }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, reason: impl Into<String>, stages: PitchBaseStages)
        -> PitchBaseStreamFailure<'a>
    {
        PitchBaseStreamFailure { reason: reason.into(), stages, advanced: self }
    }

    pub fn push(mut self, input384: &[f32], final_input: bool)
        -> Result<PitchBaseBatch<'a>, PitchBaseStreamFailure<'a>>
    {
        let mut stages = PitchBaseStages { conv0: Vec::new(), activated0: Vec::new(),
            conv1: Vec::new(), base256: Vec::new() };
        if self.ended || self.halted {
            return Err(self.failure("Pitch base stream is already ended or halted", stages));
        }
        if input384.len() % 384 != 0 || input384.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Pitch base stream requires finite complete 384-lane rows", stages));
        }
        let Some(accepted) = self.accepted.checked_add(input384.len() / 384)
            .filter(|&n| n <= self.row_limit) else {
            return Err(self.failure("Pitch base stream row limit exceeded", stages));
        };
        if final_input && accepted == 0 {
            return Err(self.failure("Pitch base stream requires nonempty final input", stages));
        }
        let first_rows = [self.first.emitted_rows(), self.second.emitted_rows()];
        self.accepted = accepted;
        self.ended = final_input;
        if let Err(reason) = self.first.append(&self.base.first, input384, final_input, &mut stages.conv0) {
            self.halted = true;
            return Err(self.failure(reason, stages));
        }
        stages.activated0 = stages.conv0.iter().map(|&v| runtime_silu(v)).collect();
        if let Err(reason) = self.second.append(&self.base.second, &stages.activated0,
            final_input, &mut stages.conv1)
        {
            self.halted = true;
            return Err(self.failure(reason, stages));
        }
        stages.base256 = stages.conv1.iter().map(|&v| runtime_silu(v)).collect();
        if final_input && (self.first.emitted_rows() != accepted || self.second.emitted_rows() != accepted) {
            self.halted = true;
            return Err(self.failure("Pitch base final histories have unequal row counts", stages));
        }
        Ok(PitchBaseBatch { first_rows, stages, final_input, state: self })
    }
}
