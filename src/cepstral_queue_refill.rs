use super::State;
use crate::conditional_gate::Batch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Queue { At288, At328 }
impl Queue { fn index(self) -> usize { match self { Self::At288 => 0, Self::At328 => 1 } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    queue: Queue, loaded_count: u64, request_word: u32, adjustment_word: u32,
    requested_word: u32, selected_word: u64, loop_count: i64,
}
impl Selection {

    pub fn queue288(queue_count: u64, selected_deque_length: u32, adjustment: u32) -> Self {
        let requested_word = selected_deque_length.wrapping_sub(adjustment);
        let selected_word = if (requested_word as i32) < (queue_count as u32 as i32) {
            u64::from(requested_word)
        } else { queue_count };
        Self { queue: Queue::At288, loaded_count: queue_count, request_word: selected_deque_length,
            adjustment_word: adjustment, requested_word, selected_word,
            loop_count: i64::from(selected_word as u32 as i32) }
    }

    pub fn queue328(queue_count_word: u32, limit_word: u32) -> Self {
        let selected_word = if (limit_word as i32) < (queue_count_word as i32) {
            limit_word
        } else { queue_count_word };
        Self { queue: Queue::At328, loaded_count: u64::from(queue_count_word),
            request_word: limit_word, adjustment_word: 0, requested_word: limit_word,
            selected_word: u64::from(selected_word), loop_count: i64::from(selected_word as i32) }
    }
    pub fn queue(self) -> Queue { self.queue }
    pub fn loaded_count(self) -> u64 { self.loaded_count }
    pub fn request_word(self) -> u32 { self.request_word }
    pub fn adjustment_word(self) -> u32 { self.adjustment_word }
    pub fn requested_word(self) -> u32 { self.requested_word }
    pub fn saved_word(self) -> u64 { self.selected_word }
    pub fn signed_loop_count(self) -> i64 { self.loop_count }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { CountChanged, NegativeCount, PrefixUnavailable }
pub struct StartFailure { pub cause: Domain, pub queues: State, pub output: Batch,
    pub selection: Selection, pub raw_status: u8 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Clear, Reserve, Move, Erase, Complete, Failed }
pub enum Cause { Allocation(std::collections::TryReserveError), Control }
pub struct Failure { pub phase: Phase, pub cause: Cause }
pub enum Poll<'a> { Pending(Phase), Complete, Failed(&'a Failure) }
pub struct Trace {
    pub selection: Selection, pub raw_status: u8, pub moved: usize,
    pub erased: Option<usize>, pub retired_rows: Option<Vec<Vec<f32>>>,
    pub old_status: u8,
}
pub struct Run { queues: State, output: Batch, count: usize, trace: Trace,
    phase: Phase, failure: Option<Failure> }
pub struct Completed { queues: State, output: Batch, trace: Trace }

pub struct WithMixer<P> { mixer: super::MixerCompleted<P>, append_trace: super::Trace, run: Run }
pub struct WithMixerStartFailure<P> {
    pub mixer: super::MixerCompleted<P>, pub append_trace: super::Trace, pub refill: StartFailure,
}
pub struct WithMixerCompleted<P> {
    mixer: super::MixerCompleted<P>, append_trace: super::Trace, refill: Completed,
}
impl<P> super::Completed<P> {
    pub fn begin_refill(self, selection: Selection, output: Batch, raw_status: u8)
        -> Result<WithMixer<P>, WithMixerStartFailure<P>>
    {
        match self.queues.begin_refill(selection, output, raw_status) {
            Ok(run) => Ok(WithMixer { mixer: self.mixer, append_trace: self.trace, run }),
            Err(refill) => Err(WithMixerStartFailure { mixer: self.mixer, append_trace: self.trace, refill }),
        }
    }
}
impl<P> WithMixer<P> {
    pub fn mixer(&self) -> &super::MixerCompleted<P> { &self.mixer }
    pub fn append_trace(&self) -> &super::Trace { &self.append_trace }
    pub fn refill(&self) -> &Run { &self.run }
    pub fn poll(&mut self, budget: usize) -> Poll<'_> { self.run.poll(budget) }
    pub fn into_completed(self) -> Result<WithMixerCompleted<P>, Self> {
        match self.run.into_completed() {
            Ok(refill) => Ok(WithMixerCompleted { mixer: self.mixer, append_trace: self.append_trace, refill }),
            Err(run) => Err(Self { mixer: self.mixer, append_trace: self.append_trace, run }),
        }
    }
}
impl<P> WithMixerCompleted<P> {
    pub fn mixer(&self) -> &super::MixerCompleted<P> { &self.mixer }
    pub fn refill(&self) -> &Completed { &self.refill }
    pub fn into_parts(self) -> (super::MixerCompleted<P>, super::Trace, Completed) {
        (self.mixer, self.append_trace, self.refill)
    }
}

impl State {

    pub fn begin_refill(self, selection: Selection, output: Batch, raw_status: u8)
        -> Result<Run, StartFailure>
    {
        let available = self.parts.queued[selection.queue.index()].len();
        let cause = if selection.loaded_count != available as u64 { Some(Domain::CountChanged) }
            else if selection.loop_count < 0 { Some(Domain::NegativeCount) }
            else if selection.loop_count as u64 > available as u64 { Some(Domain::PrefixUnavailable) }
            else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, queues: self, output, selection, raw_status }); }
        let trace = Trace { selection, raw_status, moved: 0, erased: None,
            retired_rows: None, old_status: output.status };
        Ok(Run { queues: self, output, count: selection.loop_count as usize,
            trace, phase: Phase::Clear, failure: None })
    }
}
impl Run {
    pub fn queues(&self) -> &State { &self.queues }
    pub fn output(&self) -> &Batch { &self.output }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }

    pub fn poll(&mut self, budget: usize) -> Poll<'_> {
        if self.failure.is_none() {
            for _ in 0..budget {
                if matches!(self.phase, Phase::Complete | Phase::Failed) { break; }
                if let Err(cause) = self.advance() {
                    self.failure = Some(Failure { phase: self.phase, cause });
                    self.phase = Phase::Failed;
                    break;
                }
            }
        }
        if let Some(failure) = &self.failure { Poll::Failed(failure) }
        else if self.phase == Phase::Complete { Poll::Complete }
        else { Poll::Pending(self.phase) }
    }
    fn advance(&mut self) -> Result<(), Cause> {
        match self.phase {
            Phase::Clear => {
                self.trace.retired_rows = Some(std::mem::take(&mut self.output.rows));
                self.output.status = self.trace.raw_status;
                self.phase = Phase::Reserve;
            }
            Phase::Reserve => {
                self.output.rows.try_reserve(self.count).map_err(Cause::Allocation)?;
                self.phase = Phase::Move;
            }
            Phase::Move if self.trace.moved == self.count => { self.phase = Phase::Erase; }
            Phase::Move => {
                let queue = &mut self.queues.parts.queued[self.trace.selection.queue.index()];
                let row = queue.get_mut(self.trace.moved).ok_or(Cause::Control)?;
                if row.len() != self.queues.parts.width { return Err(Cause::Control); }
                self.output.rows.push(std::mem::take(row));
                self.trace.moved += 1;
            }
            Phase::Erase => {
                let queue = &mut self.queues.parts.queued[self.trace.selection.queue.index()];
                if queue.len() < self.count || queue.iter().take(self.count).any(|row| !row.is_empty()) {
                    return Err(Cause::Control);
                }
                queue.drain(..self.count);
                self.trace.erased = Some(self.count);
                self.phase = Phase::Complete;
            }
            Phase::Complete | Phase::Failed => {}
        }
        Ok(())
    }
    pub fn into_completed(self) -> Result<Completed, Self> {
        if self.failure.is_some() || self.phase != Phase::Complete { return Err(self); }
        Ok(Completed { queues: self.queues, output: self.output, trace: self.trace })
    }
}
impl Completed {
    pub fn queues(&self) -> &State { &self.queues }
    pub fn output(&self) -> &Batch { &self.output }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn into_parts(self) -> (State, Batch, Trace) { (self.queues, self.output, self.trace) }
}
