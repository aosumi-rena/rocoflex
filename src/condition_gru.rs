use crate::{condition::{self, Dct, PhaseServices}, quantized_gru::{self, Activations, QuantizedGru}};

pub struct StartFailure<D, P, A> {
    pub reason: &'static str, pub condition: condition::Completed<D, P>, pub model: QuantizedGru<A>,
}
pub struct Run<D, P, A: Activations> {
    condition: condition::Condition<D, P>, input: Vec<Vec<f32>>, phase_words: Vec<u32>,
    work: condition::Work, gru: quantized_gru::Run<A>,
}
pub struct Completed<D, P, A> {
    pub condition: condition::Condition<D, P>, pub input: Vec<Vec<f32>>, pub phase_words: Vec<u32>,
    pub work: condition::Work, pub recurrent: quantized_gru::Completed<A>,
}

impl<D: Dct, P: PhaseServices> condition::Completed<D, P> {
    pub fn into_recurrent<A: Activations>(self, model: QuantizedGru<A>)
        -> Result<Run<D, P, A>, StartFailure<D, P, A>>
    {
        if model.parameters().input_width() != self.condition.output_width() {
            return Err(StartFailure { reason: "conditioning/recurrent widths differ", condition: self, model });
        }
        match model.begin(self.rows, self.status) {
            Ok(gru) => Ok(Run { condition: self.condition, input: self.input,
                phase_words: self.phase_words, work: self.work, gru }),
            Err(failed) => Err(StartFailure { reason: failed.reason, model: failed.model,
                condition: condition::Completed { condition: self.condition, input: self.input,
                    phase_words: self.phase_words, status: failed.status, rows: failed.input, work: self.work } }),
        }
    }
}
impl<D, P, A: Activations> Run<D, P, A> {
    pub fn condition(&self) -> &condition::Condition<D, P> { &self.condition }
    pub fn input(&self) -> &[Vec<f32>] { &self.input }
    pub fn phase_words(&self) -> &[u32] { &self.phase_words }
    pub fn condition_work(&self) -> &condition::Work { &self.work }
    pub fn recurrent(&self) -> &quantized_gru::Run<A> { &self.gru }
    pub fn poll(&mut self, row_budget: usize) -> quantized_gru::Poll<'_, A::Error> { self.gru.poll(row_budget) }
    pub fn into_completed(self) -> Result<Completed<D, P, A>, Self> {
        match self.gru.into_completed() {
            Ok(recurrent) => Ok(Completed { condition: self.condition, input: self.input,
                phase_words: self.phase_words, work: self.work, recurrent }),
            Err(gru) => Err(Self { condition: self.condition, input: self.input,
                phase_words: self.phase_words, work: self.work, gru }),
        }
    }
}
