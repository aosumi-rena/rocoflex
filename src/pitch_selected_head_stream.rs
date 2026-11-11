use super::{SelectedPitchHead, SelectedPitchRows};
use crate::pitch_score::{PitchBaseStages, PitchBaseStream, PitchBaseStreamFailure,
    PitchProfileStages, PitchScoreStages};

pub struct SelectedPitchWork {
    pub profile: Option<PitchProfileStages>,
    pub score: Option<PitchScoreStages>,
    pub voicing: Vec<f32>,
    pub auxiliary: Vec<f32>,
}

impl SelectedPitchWork {
    pub(super) fn empty() -> Self {
        Self { profile: None, score: None, voicing: Vec::new(), auxiliary: Vec::new() }
    }
}

pub struct SelectedPitchStream<'a> {
    head: &'a SelectedPitchHead,
    base: Option<PitchBaseStream<'a>>,
    previous: i32,
    emitted: usize,
    ended: bool,
    halted: bool,
}
pub(crate) struct SelectedPitchState {
    base:Option<crate::pitch_score::PitchBaseState>,previous:i32,
    emitted:usize,ended:bool,halted:bool,
}
impl SelectedPitchState {

    pub(crate) fn count_cursor_state(&self)->(bool,bool,bool) {
        let (advanced,failed)=self.base.as_ref().map(|b|b.count_cursor_state()).unwrap_or((false,false));
        (advanced || self.emitted!=0 || self.ended,failed || self.halted,self.base.is_none())
    }
}
pub struct SelectedPitchBatch<'a> {
    pub first_row: usize,
    pub base_first_rows: [usize; 2],
    pub base: PitchBaseStages,
    pub rows: Option<SelectedPitchRows>,
    pub final_input: bool,
    pub state: SelectedPitchStream<'a>,
}
pub struct SelectedPitchStreamFailure<'a> {
    pub reason: String,

    pub pending_previous_position: Option<i32>,
    pub pending_work: Option<SelectedPitchWork>,
    pub completed_base: Option<PitchBaseStages>,
    pub base_failure: Option<PitchBaseStreamFailure<'a>>,
    pub advanced: SelectedPitchStream<'a>,
}

impl<'a> SelectedPitchStream<'a> {
    pub(crate) fn into_state(self)->SelectedPitchState {
        let Self {base,previous,emitted,ended,halted,..}=self;
        SelectedPitchState {base:base.map(PitchBaseStream::into_state),previous,emitted,ended,halted}
    }
    pub(crate) fn from_state(head:&'a SelectedPitchHead,state:SelectedPitchState)->Self {
        let SelectedPitchState {base,previous,emitted,ended,halted}=state;
        Self {head,base:base.map(|state|PitchBaseStream::from_state(&head.base,state)),
            previous,emitted,ended,halted}
    }
    pub(super) fn new(head: &'a SelectedPitchHead, previous: i32, row_limit: usize)
        -> crate::Result<Self>
    {
        if !(-1..128).contains(&previous) {
            return Err("Pitch head stream requires an explicit previous position in -1..128".into());
        }
        let base = head.base.stream_with_limit(row_limit)?;
        Ok(Self { head, base: Some(base), previous, emitted: 0, ended: false, halted: false })
    }

    pub fn previous_position(&self) -> i32 { self.previous }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, reason: impl Into<String>, completed_base: Option<PitchBaseStages>,
        base_failure: Option<PitchBaseStreamFailure<'a>>) -> SelectedPitchStreamFailure<'a>
    {
        SelectedPitchStreamFailure { reason: reason.into(), pending_previous_position: None,
            pending_work: None, completed_base, base_failure, advanced: self }
    }

    pub fn push(mut self, input384: &[f32], final_input: bool)
        -> Result<SelectedPitchBatch<'a>, SelectedPitchStreamFailure<'a>>
    {
        if self.ended || self.halted || self.base.is_none() {
            return Err(self.failure("Selected pitch stream is already ended or halted", None, None));
        }
        let base = self.base.take().expect("admitted pitch base owner");
        let batch = match base.push(input384, final_input) {
            Ok(batch) => batch,
            Err(failure) => {
                let reason = failure.reason.clone();
                self.halted = true;
                return Err(self.failure(reason, None, Some(failure)));
            }
        };
        self.base = Some(batch.state);
        if batch.first_rows[1] != self.emitted {
            self.halted = true;
            return Err(self.failure("Selected pitch base row range is discontinuous", Some(batch.stages), None));
        }
        let rows = if batch.stages.base256.is_empty() { None } else {
            let mut previous = self.previous;
            let mut work = SelectedPitchWork::empty();
            match self.head.forward_base256_into_work(&batch.stages.base256, &mut previous, &mut work) {
                Ok(rows) => { self.previous = previous; Some(rows) }
                Err(reason) => {
                    self.halted = true;
                    let mut failure = self.failure(reason, Some(batch.stages), None);
                    failure.pending_previous_position = Some(previous);
                    failure.pending_work = Some(work);
                    return Err(failure);
                }
            }
        };
        let first_row = self.emitted;
        self.emitted += batch.stages.base256.len() / 256;
        self.ended = final_input;
        Ok(SelectedPitchBatch { first_row, base_first_rows: batch.first_rows, base: batch.stages,
            rows, final_input, state: self })
    }
}
