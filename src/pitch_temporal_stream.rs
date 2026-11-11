use super::{PitchTemporal, PitchTemporalState, PitchTemporalStages};

pub struct PitchTemporalStream<'a> {
    temporal: &'a PitchTemporal,
    hidden: PitchTemporalState,
    row_limit: usize,
    emitted: usize,
    ended: bool,
    halted: bool,
}

pub struct PitchTemporalStartFailure {
    pub reason: String,
    pub state: PitchTemporalState,
}

pub struct PitchTemporalBatch<'a> {
    pub first_row: usize,
    pub stages: Option<PitchTemporalStages>,
    pub final_input: bool,
    pub state: PitchTemporalStream<'a>,
}

pub struct PitchTemporalStreamFailure<'a> {
    pub reason: String,

    pub pending_hidden: Option<PitchTemporalState>,

    pub pending_stages: Option<PitchTemporalStages>,
    pub state: PitchTemporalStream<'a>,
}

impl<'a> PitchTemporalStream<'a> {
    pub(super) fn new(temporal: &'a PitchTemporal, state: PitchTemporalState, row_limit: usize)
        -> Result<Self, PitchTemporalStartFailure>
    {
        if row_limit == 0 || row_limit > i32::MAX as usize
            || row_limit.checked_mul(1024).is_none()
            || state.first.len() != 1024 || state.second.len() != 1024
            || state.first.iter().chain(&state.second).any(|v| !v.is_finite())
        {
            return Err(PitchTemporalStartFailure {
                reason: "Pitch temporal stream requires finite explicit hidden vectors and a positive i32 row limit".into(),
                state,
            });
        }
        Ok(Self { temporal, hidden: state, row_limit, emitted: 0, ended: false, halted: false })
    }

    pub fn hidden_state(&self) -> &PitchTemporalState { &self.hidden }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn into_hidden_state(self) -> PitchTemporalState { self.hidden }

    fn failure(self, reason: impl Into<String>) -> PitchTemporalStreamFailure<'a> {
        PitchTemporalStreamFailure { reason: reason.into(), pending_hidden: None,
            pending_stages: None, state: self }
    }

    pub fn push(mut self, input384: &[f32], final_input: bool)
        -> Result<PitchTemporalBatch<'a>, PitchTemporalStreamFailure<'a>>
    {
        if self.ended || self.halted {
            return Err(self.failure("Pitch temporal stream is already ended or halted"));
        }
        if input384.len() % 384 != 0 || input384.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Pitch temporal stream requires finite complete 384-lane rows"));
        }
        let rows = input384.len() / 384;
        let Some(emitted) = self.emitted.checked_add(rows).filter(|&n| n <= self.row_limit) else {
            return Err(self.failure("Pitch temporal stream row limit exceeded"));
        };
        let first_row = self.emitted;
        let stages = if rows == 0 { None } else {
            let mut next = PitchTemporalState {
                first: self.hidden.first.clone(), second: self.hidden.second.clone(),
            };
            let mut stages = PitchTemporalStages {
                first_gru: Vec::new(), first_main: Vec::new(), first_bypass: Vec::new(),
                first_sum: Vec::new(), second_gru: Vec::new(), second_main: Vec::new(),
                second_bypass: Vec::new(), output384: Vec::new(),
            };
            if let Err(reason) = self.advance_stages(input384, &mut next, &mut stages) {
                self.halted = true;
                let mut failure = self.failure(reason);
                failure.pending_hidden = Some(next);
                failure.pending_stages = Some(stages);
                return Err(failure);
            }
            if next.first.iter().chain(&next.second).any(|v| !v.is_finite()) {
                self.halted = true;
                let mut failure = self.failure("Pitch recurrent state left the finite domain");
                failure.pending_hidden = Some(next);
                failure.pending_stages = Some(stages);
                return Err(failure);
            }
            self.hidden = next;
            Some(stages)
        };
        self.emitted = emitted;
        self.ended = final_input;
        Ok(PitchTemporalBatch { first_row, stages, final_input, state: self })
    }

    fn advance_stages(&self, input: &[f32], next: &mut PitchTemporalState,
        stages: &mut PitchTemporalStages) -> crate::Result<()>
    {
        for row in input.chunks_exact(384) {
            stages.first_gru.extend(self.temporal.gru[0].forward(row, &mut next.first)?);
            require_finite(&next.first)?;
        }
        for row in stages.first_gru.chunks_exact(1024) {
            stages.first_main.extend(self.temporal.main_from_gru(0, row)?);
        }
        require_finite(&stages.first_main)?;
        for row in input.chunks_exact(384) {
            stages.first_bypass.extend(self.temporal.bypass[0].forward(row)?);
        }
        require_finite(&stages.first_bypass)?;
        if stages.first_main.len() != stages.first_bypass.len() {
            return Err("Pitch first residual row mismatch".into());
        }
        stages.first_sum.extend(stages.first_bypass.iter().zip(&stages.first_main).map(|(b,m)| b+m));
        require_finite(&stages.first_sum)?;
        for row in stages.first_sum.chunks_exact(1024) {
            stages.second_gru.extend(self.temporal.gru[1].forward(row, &mut next.second)?);
            require_finite(&next.second)?;
        }
        for row in stages.second_gru.chunks_exact(1024) {
            stages.second_main.extend(self.temporal.main_from_gru(1, row)?);
        }
        require_finite(&stages.second_main)?;
        for row in stages.first_sum.chunks_exact(1024) {
            stages.second_bypass.extend(self.temporal.bypass[1].forward(row)?);
        }
        require_finite(&stages.second_bypass)?;
        if stages.second_main.len() != stages.second_bypass.len() {
            return Err("Pitch second residual row mismatch".into());
        }
        stages.output384.extend(stages.second_bypass.iter().zip(&stages.second_main).map(|(b,m)| b+m));
        require_finite(&stages.output384)
    }
}

fn require_finite(values: &[f32]) -> crate::Result<()> {
    if values.iter().any(|v| !v.is_finite()) {
        Err("Pitch temporal intermediate left the finite domain".into())
    } else { Ok(()) }
}
