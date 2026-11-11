use super::State;
use crate::conditional_gate::Batch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Move, CleanDonors, ResetEnd, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { InputWidth, Count }
pub struct StartFailure<H> { pub cause: Domain, pub signal: State, pub source: Batch, pub held: H }
pub enum Error { Allocation(std::collections::TryReserveError), DonorChanged { ordinal: usize } }
pub struct Trace {
    pub original_source_count: usize, pub source_status: u8, pub fifo_count_before: usize,
    pub moved: usize, pub cleaned: usize, pub reset_end: bool,
}
pub struct Prepared<H> {
    signal: State, source: Batch, held: H, trace: Trace, stage: Stage,
    failure: Option<Error>,
}
pub enum Poll<'a> { Pending(Stage), Complete, Failed(&'a Error) }
pub struct Completed<H> { prepared: Prepared<H> }
pub struct CompletedParts<H> { pub signal: State, pub source: Batch, pub held: H, pub trace: Trace }
pub struct History<H> { pub held: H, pub source: Batch, pub trace: Trace }

impl State {

    pub fn append_explicit_returned_values<H>(self, source: Batch, held: H)
        -> Result<Prepared<H>, StartFailure<H>>
    {
        let cause = if source.rows.iter().any(|row| row.len() != self.parts.width) { Some(Domain::InputWidth) }
            else if self.parts.fifo.len().checked_add(source.rows.len())
                .is_none_or(|count| count > i32::MAX as usize) { Some(Domain::Count) }
            else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, signal: self, source, held }); }
        let trace = Trace { original_source_count: source.rows.len(), source_status: source.status,
            fifo_count_before: self.parts.fifo.len(), moved: 0, cleaned: 0, reset_end: false };
        Ok(Prepared { signal: self, source, held, trace, stage: Stage::Move, failure: None })
    }
}
impl<H> Prepared<H> {
    pub fn signal(&self) -> &State { &self.signal }
    pub fn source(&self) -> &Batch { &self.source }
    pub fn held(&self) -> &H { &self.held }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn stage(&self) -> Stage { self.stage }
    pub fn failure(&self) -> Option<&Error> { self.failure.as_ref() }

    pub fn poll(&mut self, mut budget: usize) -> Poll<'_> {
        while self.failure.is_none() && self.stage != Stage::Complete {
            match self.stage {
                Stage::Move if self.trace.moved == self.trace.original_source_count => {
                    self.stage = Stage::CleanDonors; continue;
                }
                Stage::CleanDonors if self.trace.cleaned == self.trace.original_source_count => {
                    self.stage = Stage::ResetEnd; continue;
                }
                _ => {},
            }
            if budget == 0 { break; }
            let result = match self.stage {
                Stage::Move => self.move_next(),
                Stage::CleanDonors => {
                    let ordinal = self.trace.cleaned;
                    if !self.source.rows[ordinal].is_empty() { Err(Error::DonorChanged { ordinal }) }
                    else { self.trace.cleaned += 1; Ok(()) }
                }
                Stage::ResetEnd => {

                    self.source.rows.clear();
                    self.trace.reset_end = true;
                    self.stage = Stage::Complete;
                    Ok(())
                }
                Stage::Complete => unreachable!("private append stage"),
            };
            budget -= 1;
            if let Err(error) = result { self.failure = Some(error); }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.stage == Stage::Complete { Poll::Complete }
        else { Poll::Pending(self.stage) }
    }
    fn move_next(&mut self) -> Result<(), Error> {

        self.signal.parts.fifo.try_reserve(1).map_err(Error::Allocation)?;
        let ordinal = self.trace.moved;
        let row = std::mem::take(&mut self.source.rows[ordinal]);

        self.signal.parts.fifo.push_back(Some(row));
        self.trace.moved += 1;
        Ok(())
    }
    pub fn into_completed(self) -> Result<Completed<H>, Self> {
        if self.failure.is_some() || self.stage != Stage::Complete { Err(self) }
        else { Ok(Completed { prepared: self }) }
    }
}
impl<H> Completed<H> {
    pub fn prepared(&self) -> &Prepared<H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<H> {
        let owner = self.prepared;
        CompletedParts { signal: owner.signal, source: owner.source, held: owner.held, trace: owner.trace }
    }

    pub fn into_refill<C>(self, coefficients: crate::cepstral::queue::Completed<C>, request: super::Request)
        -> Result<super::Prepared<C, History<H>>, super::StartFailure<C, History<H>>>
    {
        let parts = self.into_parts();
        let history = History { held: parts.held, source: parts.source, trace: parts.trace };
        super::Prepared::from_completed_queues(parts.signal, coefficients, history, request)
    }
}
