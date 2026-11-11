pub use crate::conditional_gate::Batch;
use crate::{condition_history::ConditionHistory, conditional_ncwn::{ConditionalNcwn, ConditionedTransform},
    conditional_gate::{GateError, Projection}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimensions { pub input: usize, pub output: usize, pub condition: usize }
pub trait Stage {
    type Error;
    fn dimensions(&self) -> Dimensions;
    fn forward(&mut self, input: &Batch, condition: &Batch) -> Result<Batch, Self::Error>;
}
impl<G: ConditionedTransform, S: Projection, R: Projection, H: Projection> Stage for ConditionalNcwn<G, S, R, H> {
    type Error = GateError;
    fn dimensions(&self) -> Dimensions {
        let config = self.config();
        Dimensions { input: config.input_width, output: config.output_width, condition: config.condition_width }
    }
    fn forward(&mut self, input: &Batch, condition: &Batch) -> Result<Batch, GateError> {
        ConditionalNcwn::forward(self, input, condition)
    }
}

pub struct State<A, B> {
    first: A, second: B, pending: ConditionHistory<Vec<f32>>, dimensions: [Dimensions; 2],
}
pub struct BindingFailure<A, B> {
    pub reason: &'static str, pub first: A, pub second: B, pub pending: ConditionHistory<Vec<f32>>,
}
pub struct StartFailure<A, B> {
    pub reason: &'static str, pub state: State<A, B>, pub input: Batch, pub holder: Batch,
    pub incoming_status: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Ready, FirstInFlight, ConditionAppend, ConditionAppendInFlight, SecondReady, SecondInFlight, Complete, Failed }
pub enum Error<A, B> {
    Environment(crate::matrix_f32::Error), First(A), Second(B),
    DimensionsChanged { stage: usize, expected: Dimensions, actual: Dimensions },
    ResultDomain { stage: usize }, MissingConditions { required: usize, available: usize },
    Interrupted { phase: Phase },
}
pub struct Failure<A, B> { pub phase: Phase, pub error: Error<A, B> }
pub enum Poll<'a, A, B> { Pending(Phase), Complete, Failed(&'a Failure<A, B>) }
pub struct Run<A: Stage, B: Stage> {
    state: State<A, B>, input: Batch, holder: Batch, incoming_status: u8,
    first_output: Option<Batch>, rebuilt: Option<Batch>, output: Option<Batch>, phase: Phase,
    failure: Option<Failure<A::Error, B::Error>>,
}
pub struct Completed<A, B> {
    state: State<A, B>, input: Batch, holder_after_move: Batch, incoming_status: u8,
    first_output: Batch, rebuilt: Batch, output: Batch,
}
pub struct Trace {
    pub input: Batch, pub holder_after_move: Batch, pub incoming_status: u8,
    pub first_output: Batch, pub rebuilt_conditions: Batch, pub output: Batch,
}

fn valid_rows(rows: &[Vec<f32>], width: usize) -> bool {
    rows.iter().all(|row| row.len() == width && row.iter().all(|value| value.is_finite()))
}
fn valid_dimensions(d: Dimensions) -> bool {
    [d.input, d.output, d.condition].iter().all(|&width| width > 0 && width <= i32::MAX as usize)
}
impl<A: Stage, B: Stage> State<A, B> {

    pub fn from_explicit_state(first: A, second: B, pending: ConditionHistory<Vec<f32>>)
        -> Result<Self, BindingFailure<A, B>>
    {
        let dimensions = [first.dimensions(), second.dimensions()];
        let reason = if !dimensions.iter().all(|&d| valid_dimensions(d))
            || dimensions[0].output != dimensions[1].input
            || dimensions[0].condition != dimensions[1].condition {
            Some("conditioned stage dimensions differ")
        } else if pending.pending().any(|row| row.len() != dimensions[0].condition
            || row.iter().any(|value| !value.is_finite())) {
            Some("interstage condition history has invalid rows")
        } else { None };
        if let Some(reason) = reason { return Err(BindingFailure { reason, first, second, pending }); }
        Ok(Self { first, second, pending, dimensions })
    }
    pub fn first(&self) -> &A { &self.first }
    pub fn second(&self) -> &B { &self.second }
    pub fn pending_conditions(&self) -> &ConditionHistory<Vec<f32>> { &self.pending }
    pub fn dimensions(&self) -> [Dimensions; 2] { self.dimensions }

    pub fn begin(self, input: Batch, holder: Batch, incoming_status: u8)
        -> Result<Run<A, B>, StartFailure<A, B>>
    {
        let reason = if !valid_rows(&input.rows, self.dimensions[0].input)
            || !valid_rows(&holder.rows, self.dimensions[0].condition) {
            Some("conditioned stages require finite complete input/holder rows")
        } else if input.rows.len() > i32::MAX as usize || holder.rows.len() > i32::MAX as usize {
            Some("conditioned stage row count exceeds the value domain")
        } else { None };
        if let Some(reason) = reason { return Err(StartFailure { reason, state: self, input, holder, incoming_status }); }
        Ok(Run { state: self, input, holder, incoming_status, first_output: None,
            rebuilt: None, output: None, phase: Phase::Ready, failure: None })
    }
}
impl<A: Stage, B: Stage> Run<A, B> {
    pub fn state(&self) -> &State<A, B> { &self.state }
    pub fn input(&self) -> &Batch { &self.input }
    pub fn holder(&self) -> &Batch { &self.holder }
    pub fn first_output(&self) -> Option<&Batch> { self.first_output.as_ref() }
    pub fn rebuilt_conditions(&self) -> Option<&Batch> { self.rebuilt.as_ref() }
    pub fn output(&self) -> Option<&Batch> { self.output.as_ref() }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn failure(&self) -> Option<&Failure<A::Error, B::Error>> { self.failure.as_ref() }
    fn fail(&mut self, error: Error<A::Error, B::Error>) {
        self.failure = Some(Failure { phase: self.phase, error }); self.phase = Phase::Failed;
    }
    fn dimensions_match(&mut self, stage: usize, actual: Dimensions) -> bool {
        let expected = self.state.dimensions[stage];
        if actual == expected { true } else {
            self.fail(Error::DimensionsChanged { stage, expected, actual }); false
        }
    }

    pub fn poll(&mut self, effect_budget: usize) -> Poll<'_, A::Error, B::Error> {
        if self.failure.is_none() {
            for _ in 0..effect_budget {
                match self.phase {
                    Phase::Ready => {
                        if !self.dimensions_match(0, self.state.first.dimensions()) { break; }
                        if let Err(error) = crate::matrix_f32::check_environment() {
                            self.fail(Error::Environment(error)); break;
                        }
                        self.phase = Phase::FirstInFlight;
                        match self.state.first.forward(&self.input, &self.holder) {
                            Ok(batch) => self.first_output = Some(batch),
                            Err(error) => { self.fail(Error::First(error)); break; }
                        }
                        if !self.dimensions_match(0, self.state.first.dimensions()) { break; }
                        let first = self.first_output.as_ref().expect("first return retained");
                        if first.rows.len() > i32::MAX as usize || !valid_rows(&first.rows, self.state.dimensions[0].output) {
                            self.fail(Error::ResultDomain { stage: 0 }); break;
                        }
                        self.phase = Phase::ConditionAppend;
                    }
                    Phase::ConditionAppend => {
                        let required = self.first_output.as_ref().expect("first return retained").rows.len();
                        self.phase = Phase::ConditionAppendInFlight;
                        let moved = std::mem::take(&mut self.holder.rows);
                        match self.state.pending.append_and_take(moved, required) {
                            Ok(rows) => self.rebuilt = Some(Batch { rows, status: self.incoming_status }),
                            Err(error) => {
                                self.fail(Error::MissingConditions { required: error.required, available: error.available }); break;
                            }
                        }
                        self.phase = Phase::SecondReady;
                    }
                    Phase::SecondReady => {
                        if !self.dimensions_match(1, self.state.second.dimensions()) { break; }
                        if let Err(error) = crate::matrix_f32::check_environment() {
                            self.fail(Error::Environment(error)); break;
                        }
                        self.phase = Phase::SecondInFlight;
                        let first = self.first_output.as_ref().expect("first return retained");
                        let condition = self.rebuilt.as_ref().expect("refilled conditions retained");
                        match self.state.second.forward(first, condition) {
                            Ok(batch) => self.output = Some(batch),
                            Err(error) => { self.fail(Error::Second(error)); break; }
                        }
                        if !self.dimensions_match(1, self.state.second.dimensions()) { break; }
                        let output = self.output.as_ref().expect("second return retained");
                        if output.rows.len() > i32::MAX as usize || !valid_rows(&output.rows, self.state.dimensions[1].output) {
                            self.fail(Error::ResultDomain { stage: 1 }); break;
                        }
                        self.phase = Phase::Complete;
                    }
                    Phase::FirstInFlight | Phase::ConditionAppendInFlight | Phase::SecondInFlight => {
                        self.fail(Error::Interrupted { phase: self.phase }); break;
                    }
                    Phase::Complete | Phase::Failed => break,
                }
            }
        }
        if let Some(failure) = &self.failure { Poll::Failed(failure) }
        else if self.phase == Phase::Complete { Poll::Complete }
        else { Poll::Pending(self.phase) }
    }
    pub fn into_completed(self) -> Result<Completed<A, B>, Self> {
        if self.failure.is_some() || self.phase != Phase::Complete { return Err(self); }
        Ok(Completed { state: self.state, input: self.input, holder_after_move: self.holder,
            incoming_status: self.incoming_status,
            first_output: self.first_output.expect("completed first call"),
            rebuilt: self.rebuilt.expect("completed refill"), output: self.output.expect("completed second call") })
    }
}
impl<A, B> Completed<A, B> {
    pub fn state(&self) -> &State<A, B> { &self.state }
    pub fn output(&self) -> &Batch { &self.output }
    pub fn first_output(&self) -> &Batch { &self.first_output }
    pub fn rebuilt_conditions(&self) -> &Batch { &self.rebuilt }

    pub fn into_parts(self) -> (State<A, B>, Trace) {
        (self.state, Trace { input: self.input, holder_after_move: self.holder_after_move,
            incoming_status: self.incoming_status, first_output: self.first_output,
            rebuilt_conditions: self.rebuilt, output: self.output })
    }
}
