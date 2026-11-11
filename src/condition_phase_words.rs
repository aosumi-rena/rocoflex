use std::collections::TryReserveError;
use crate::{condition::{self, Dct, PhaseServices}, condition_gru, condition_parameters,
    quantized_gru::{self, Activations, QuantizedGru}};
#[path = "runtime/dsp/condition_holder_words.rs"]
mod words;

pub struct State { words: words::State }
pub struct BindingFailure { pub counter: u32, pub logical_count: usize, pub backing: Vec<u32> }

pub struct PhaseGrowthRetirement {
    pub plan: crate::phase_reallocation::PhaseReallocationPlan,
    pub full_backing: Box<[u32]>,
}
pub enum PhaseGrowthError {
    Plan { cause: crate::phase_reallocation::PhaseAllocationError,
        requested_count: usize, spare_word: u32 },
    Allocation { cause: TryReserveError,
        plan: crate::phase_reallocation::PhaseReallocationPlan, spare_word: u32 },
    Application { cause: crate::phase_reallocation::PhaseAllocationError,
        plan: crate::phase_reallocation::PhaseReallocationPlan,
        spare_word: u32, allocation: Vec<u32> },
}
impl State {
    pub fn from_explicit_holder_state(counter: u32, logical_count: usize, backing: Vec<u32>)
        -> Result<Self, BindingFailure>
    {
        words::State::from_explicit_state(counter, logical_count, backing)
            .map(|words| Self { words })
            .map_err(|failure| BindingFailure { counter: failure.counter,
                logical_count: failure.count, backing: failure.backing })
    }
    pub fn counter(&self) -> u32 { self.words.counter() }
    pub fn logical_count(&self) -> usize { self.words.count() }
    pub fn full_backing(&self) -> &[u32] { self.words.full_backing() }

    pub fn from_initial_5c590_zeroed() -> Self {
        Self { words: words::State::from_initial_5c590_zeroed() }
    }

    pub fn grow_phase_capacity(mut self, new_count: usize, spare_word: u32)
        -> Result<(Self, PhaseGrowthRetirement), (PhaseGrowthError, Self)>
    {
        let plan = match crate::phase_reallocation::PhaseReallocationPlan::bind(
            self.words.count(), self.words.full_backing().len(), new_count)
        {
            Ok(plan) => plan,
            Err(cause) => return Err((PhaseGrowthError::Plan {
                cause, requested_count: new_count, spare_word }, self)),
        };
        let allocated = match plan.allocate_with_explicit_spare_word(spare_word) {
            Ok(allocated) => allocated,
            Err(cause) => return Err((PhaseGrowthError::Allocation { cause, plan, spare_word }, self)),
        };
        match self.words.reallocate_phase_from_explicit_allocation(new_count, allocated) {
            Ok((plan, full_backing)) => Ok((self, PhaseGrowthRetirement { plan, full_backing })),
            Err((cause, allocation)) => Err((PhaseGrowthError::Application {
                cause, plan, spare_word, allocation }, self)),
        }
    }
}

pub enum StartFailure<D, P, A> {
    RequiredPhaseCapacity { state: State, runtime: condition_parameters::Bound<D, P, A>,
        input: Vec<Vec<f32>>, status: u8 },
    Capacity { cause: TryReserveError, state: State, runtime: condition_parameters::Bound<D, P, A>,
        input: Vec<Vec<f32>>, status: u8 },
    Condition { state: State, failure: condition::StartFailure<D, P>, recurrent: QuantizedGru<A> },
}
pub struct ConditionRun<D: Dct, P: PhaseServices, A> {
    state: State, condition: condition::Run<D, P>, recurrent: QuantizedGru<A>,
}
pub enum RecurrentStartFailure<D: Dct, P: PhaseServices, A> {
    Incomplete(ConditionRun<D, P, A>),
    Rejected { state: State, failure: condition_gru::StartFailure<D, P, A> },
}
pub struct RecurrentRun<D, P, A: Activations> {
    state: State, recurrent: condition_gru::Run<D, P, A>,
}
pub struct Completed<D, P, A> {
    pub state: State,
    pub condition: condition_gru::Completed<D, P, A>,
}

impl State {

    pub fn begin<D: Dct, P: PhaseServices, A: Activations>(mut self,
        runtime: condition_parameters::Bound<D, P, A>, input: Vec<Vec<f32>>, status: u8)
        -> Result<ConditionRun<D, P, A>, StartFailure<D, P, A>>
    {
        let count = input.len();
        if count > self.words.full_backing().len() {
            return Err(StartFailure::RequiredPhaseCapacity { state: self, runtime, input, status });
        }
        let mut phase_words = Vec::new();
        if let Err(cause) = phase_words.try_reserve_exact(count) {
            return Err(StartFailure::Capacity { cause, state: self, runtime, input, status });
        }
        if self.words.prepare(count).is_err() {
            return Err(StartFailure::RequiredPhaseCapacity { state: self, runtime, input, status });
        }
        phase_words.extend_from_slice(self.words.logical_words());
        let condition_parameters::Bound { condition, recurrent } = runtime;
        match condition.begin(input, phase_words, status) {
            Ok(condition) => Ok(ConditionRun { state: self, condition, recurrent }),
            Err(failure) => Err(StartFailure::Condition { state: self, failure, recurrent }),
        }
    }
}

impl<D: Dct, P: PhaseServices, A: Activations> ConditionRun<D, P, A> {
    pub fn state(&self) -> &State { &self.state }
    pub fn condition(&self) -> &condition::Run<D, P> { &self.condition }
    pub fn recurrent(&self) -> &QuantizedGru<A> { &self.recurrent }
    pub fn poll(&mut self, row_budget: usize) -> condition::Poll<'_, D::Error, P::Error> {
        self.condition.poll(row_budget)
    }

    pub fn into_recurrent(self) -> Result<RecurrentRun<D, P, A>, RecurrentStartFailure<D, P, A>> {
        let Self { state, condition, recurrent } = self;
        let completed = match condition.into_completed() {
            Ok(completed) => completed,
            Err(condition) => return Err(RecurrentStartFailure::Incomplete(
                Self { state, condition, recurrent })),
        };
        match completed.into_recurrent(recurrent) {
            Ok(recurrent) => Ok(RecurrentRun { state, recurrent }),
            Err(failure) => Err(RecurrentStartFailure::Rejected { state, failure }),
        }
    }
}

impl<D, P, A: Activations> RecurrentRun<D, P, A> {
    pub fn state(&self) -> &State { &self.state }
    pub fn recurrent(&self) -> &condition_gru::Run<D, P, A> { &self.recurrent }
    pub fn poll(&mut self, row_budget: usize) -> quantized_gru::Poll<'_, A::Error> {
        self.recurrent.poll(row_budget)
    }

    pub fn into_completed(self) -> Result<Completed<D, P, A>, Self> {
        let Self { mut state, recurrent } = self;
        let condition = match recurrent.into_completed() {
            Ok(completed) => completed,
            Err(recurrent) => return Err(Self { state, recurrent }),
        };
        state.words.condition_returned();
        state.words.first_conditional_boundary();
        Ok(Completed { state, condition })
    }
}
