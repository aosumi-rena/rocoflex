use crate::{conditional_gate::Batch, paired_scalar_values as graph};
pub use graph::{PackingComplete, ScalarPackProgress, ScalarPackStage};

pub trait CurrentRows {
    type Error;
    fn acquire_current(&mut self) -> Result<Vec<f32>, Self::Error>;
}
pub struct Parts<P> {
    pub frame: usize, pub filled: usize, pub ended: u8,
    pub current: Option<Vec<f32>>, pub output: Batch, pub rows: P,
}
pub struct State<P> { parts: Parts<P>, halted: bool }

pub(crate) struct OutputSuspended<P> {
    frame: usize, filled: usize, ended: u8, current: Option<Vec<f32>>, rows: P, halted: bool,
}
impl<P> OutputSuspended<P> {
    pub(crate) fn restore(self, output: Batch) -> State<P> {
        State { parts: Parts { frame: self.frame, filled: self.filled, ended: self.ended,
            current: self.current, output, rows: self.rows }, halted: self.halted }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Frame, Filled, Ended, Current, OutputWidth, InputCount, InputRow }
pub struct BindingFailure<P> { pub cause: Domain, pub parts: Parts<P> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdoptError { PriorFailure, State }
pub struct StartFailure<P> { pub cause: Domain, pub state: State<P>, pub input: Batch }
impl<P> State<P> {
    pub fn from_explicit_parts(parts: Parts<P>) -> Result<Self, BindingFailure<P>> {
        let cause = if parts.frame == 0 || parts.frame > i32::MAX as usize { Some(Domain::Frame) }
            else if parts.filled >= parts.frame { Some(Domain::Filled) }
            else if parts.ended > 1 { Some(Domain::Ended) }
            else if parts.current.as_ref().is_none_or(|row| row.len() != parts.frame) { Some(Domain::Current) }
            else if parts.output.rows.iter().any(|row| row.len() != parts.frame) { Some(Domain::OutputWidth) }
            else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn is_halted(&self) -> bool { self.halted }

    pub fn into_parts(self) -> Parts<P> { self.parts }
    pub(crate) fn adopt_advanced_prefix_ended517(&mut self, ended: u8) -> Result<(), AdoptError> {
        if self.halted { return Err(AdoptError::PriorFailure); }
        if self.parts.ended != 0 || ended > 1 { return Err(AdoptError::State); }
        self.parts.ended = ended; Ok(())
    }
    pub(crate) fn suspend_output(self) -> (OutputSuspended<P>, Batch) {
        let Parts { frame, filled, ended, current, output, rows } = self.parts;
        (OutputSuspended { frame, filled, ended, current, rows, halted: self.halted }, output)
    }
    pub fn begin(self, input: Batch) -> Result<Prepared<P>, StartFailure<P>> where P: CurrentRows {
        let cause = if input.rows.len() > i32::MAX as usize { Some(Domain::InputCount) }
            else if input.rows.iter().any(Vec::is_empty) { Some(Domain::InputRow) }
            else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, state: self, input }); }
        let work = graph::ScalarPackWork::after_explicit_admission(input.rows.len(), self.parts.ended);
        let mut state = self;
        state.halted = true;
        let old_status = state.parts.output.status;
        Ok(Prepared { owner: Owner { state, input, trace: Trace { old_status,
            retired_old: Vec::new(), pending_retirement: None, pending_append: None,
            appended: 0, acquired: 0 } }, work, failure: None, complete: None })
    }
}
pub struct Trace {
    pub old_status: u8,

    pub retired_old: Vec<Vec<f32>>, pub pending_retirement: Option<Vec<f32>>,
    pub pending_append: Option<Vec<f32>>, pub appended: usize, pub acquired: usize,
}
pub enum ServiceError<E> {
    Allocation(std::collections::TryReserveError), Acquire(E),
    InvalidAcquiredWidth { expected: usize, actual: usize }, Control(&'static str),
}
struct Owner<P> { state: State<P>, input: Batch, trace: Trace }
pub struct Prepared<P: CurrentRows> {
    owner: Owner<P>, work: graph::ScalarPackWork,
    failure: Option<graph::ScalarPackError<ServiceError<P::Error>>>, complete: Option<PackingComplete>,
}
pub enum Poll<'a, E> { Pending(ScalarPackProgress), Complete, Failed(&'a graph::ScalarPackError<ServiceError<E>>) }
pub struct Completed<P: CurrentRows> { prepared: Prepared<P> }
pub struct CompletedParts<P> {
    pub state: State<P>, pub input: Batch, pub trace: Trace, pub packing: PackingComplete,
}
impl<P: CurrentRows> Prepared<P> {
    pub fn state(&self) -> &State<P> { &self.owner.state }
    pub fn input(&self) -> &Batch { &self.owner.input }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> ScalarPackProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::ScalarPackError<ServiceError<P::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, P::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::ScalarPackAdvance::Paused) => {},
                Ok(graph::ScalarPackAdvance::Complete(marker)) => {
                    self.complete = Some(marker); self.owner.state.halted = false;
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete }
        else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<P>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) }
        else { Ok(Completed { prepared: self }) }
    }
}
impl<P: CurrentRows> Completed<P> {
    pub fn prepared(&self) -> &Prepared<P> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<P> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, input: owner.input, trace: owner.trace,
            packing: self.prepared.complete.expect("private scalar-packing completion") }
    }
}
impl<P: CurrentRows> graph::ScalarPackServices for Owner<P> {
    type Error = ServiceError<P::Error>;
    fn clear_output_and_set_status(&mut self, ended_status: u8) -> Result<(), Self::Error> {
        for row in &mut self.state.parts.output.rows {

            self.trace.pending_retirement = Some(std::mem::take(row));
            self.trace.retired_old.try_reserve(1).map_err(ServiceError::Allocation)?;
            self.trace.retired_old.push(self.trace.pending_retirement.take().expect("retained old output"));
        }
        self.state.parts.output.rows.clear();
        self.state.parts.output.status = 0;
        self.state.parts.output.status = ended_status;
        Ok(())
    }
    fn copy_lane0_and_test_full(&mut self, row: usize) -> Result<bool, Self::Error> {
        let bits = self.input.rows.get(row).and_then(|row| row.first())
            .ok_or(ServiceError::Control("admitted scalar input lane"))?.to_bits();
        let at = self.state.parts.filled;
        self.state.parts.filled = at + 1;
        let destination = self.state.parts.current.as_mut().and_then(|row| row.get_mut(at))
            .ok_or(ServiceError::Control("current scalar-packing slot"))?;
        *destination = f32::from_bits(bits);
        Ok(self.state.parts.filled == self.state.parts.frame)
    }
    fn append_completed_current(&mut self, _row: usize) -> Result<(), Self::Error> {
        self.trace.pending_append = Some(self.state.parts.current.take()
            .ok_or(ServiceError::Control("full scalar-packing current"))?);
        self.state.parts.output.rows.try_reserve(1).map_err(ServiceError::Allocation)?;
        self.state.parts.output.rows.push(self.trace.pending_append.take().expect("retained completed current"));
        self.trace.appended += 1;
        Ok(())
    }
    fn reset_filled_after_append(&mut self, _row: usize) -> Result<(), Self::Error> {
        self.state.parts.filled = 0;
        Ok(())
    }
    fn acquire_install_admit_current(&mut self, _row: usize) -> Result<(), Self::Error> {
        let next = self.state.parts.rows.acquire_current().map_err(ServiceError::Acquire)?;
        let actual = next.len();
        self.state.parts.current = Some(next);
        self.trace.acquired += 1;
        if actual != self.state.parts.frame {
            return Err(ServiceError::InvalidAcquiredWidth { expected: self.state.parts.frame, actual });
        }
        Ok(())
    }
}
