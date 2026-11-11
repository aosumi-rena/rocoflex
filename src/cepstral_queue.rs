use std::collections::VecDeque;
use super::Completed as MixerCompleted;
#[path = "cepstral_queue_refill.rs"]
pub mod refill;

pub struct Parts {
    pub width: usize,

    pub queued: [VecDeque<Vec<f32>>; 2],
}
pub struct State { parts: Parts }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Width, Row { which: usize, index: usize }, Count { which: usize },
}
pub struct BindingFailure { pub cause: Domain, pub parts: Parts }
pub struct StartFailure<P> {
    pub cause: Domain, pub queues: State, pub mixer: MixerCompleted<P>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Reserve(usize), Append(usize), Clear(usize), Complete, Failed }
pub struct Failure {
    pub phase: Phase,
    pub cause: std::collections::TryReserveError,
}
pub enum Poll<'a> { Pending(Phase), Complete, Failed(&'a Failure) }
pub struct Trace {
    pub old_counts: [usize; 2], pub packet_counts: [usize; 2],
    pub packet_status: [u8; 2], pub appended: [usize; 2], pub cleared: [bool; 2],
}
pub struct Run<P> {
    queues: State, mixer: MixerCompleted<P>, trace: Trace,
    phase: Phase, failure: Option<Failure>,
}
pub struct Completed<P> { queues: State, mixer: MixerCompleted<P>, trace: Trace }

impl State {

    pub fn from_explicit_parts(parts: Parts) -> Result<Self, BindingFailure> {
        let mut cause = if parts.width == 0 || parts.width > i32::MAX as usize {
            Some(Domain::Width)
        } else { None };
        for (which, queue) in parts.queued.iter().enumerate() {
            if cause.is_some() { break; }
            if queue.len() > i32::MAX as usize { cause = Some(Domain::Count { which }); }
            else if let Some(index) = queue.iter().position(|row| row.len() != parts.width) {
                cause = Some(Domain::Row { which, index });
            }
        }
        if let Some(cause) = cause { Err(BindingFailure { cause, parts }) }
        else { Ok(Self { parts }) }
    }
    pub fn parts(&self) -> &Parts { &self.parts }
    pub fn into_explicit_parts(self) -> Parts { self.parts }

    pub fn begin<P>(self, mixer: MixerCompleted<P>) -> Result<Run<P>, StartFailure<P>> {
        let cause = if self.parts.width != mixer.state.parts.output_width {
            Some(Domain::Width)
        } else {
            (0..2).find_map(|which| self.parts.queued[which].len()
                .checked_add(mixer.packets[which].rows.len())
                .is_none_or(|count| count > i32::MAX as usize)
                .then_some(Domain::Count { which }))
        };
        if let Some(cause) = cause { return Err(StartFailure { cause, queues: self, mixer }); }
        let trace = Trace {
            old_counts: std::array::from_fn(|which| self.parts.queued[which].len()),
            packet_counts: std::array::from_fn(|which| mixer.packets[which].rows.len()),
            packet_status: std::array::from_fn(|which| mixer.packets[which].status),
            appended: [0; 2], cleared: [false; 2],
        };
        Ok(Run { queues: self, mixer, trace, phase: Phase::Reserve(0), failure: None })
    }
}
impl<P> Run<P> {
    pub fn queues(&self) -> &State { &self.queues }
    pub fn mixer(&self) -> &MixerCompleted<P> { &self.mixer }
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
    fn advance(&mut self) -> Result<(), std::collections::TryReserveError> {
        match self.phase {
            Phase::Reserve(which) => {
                self.queues.parts.queued[which].try_reserve(self.trace.packet_counts[which])?;
                self.phase = if which == 0 { Phase::Reserve(1) } else { Phase::Append(0) };
            }
            Phase::Append(which) => {
                let rows = &mut self.mixer.packets[which].rows;
                self.queues.parts.queued[which].extend(rows.drain(..));
                self.trace.appended[which] = self.trace.packet_counts[which];
                self.phase = Phase::Clear(which);
            }
            Phase::Clear(which) => {

                self.mixer.packets[which].status = 0;
                self.mixer.state.parts.outputs[which].status = 0;
                self.trace.cleared[which] = true;
                self.phase = if which == 0 { Phase::Append(1) } else { Phase::Complete };
            }
            Phase::Complete | Phase::Failed => {}
        }
        Ok(())
    }
    pub fn into_completed(self) -> Result<Completed<P>, Self> {
        if self.failure.is_some() || self.phase != Phase::Complete { return Err(self); }
        Ok(Completed { queues: self.queues, mixer: self.mixer, trace: self.trace })
    }
}
impl<P> Completed<P> {

    pub(crate) fn from_returned_filter(queues: State, mixer: MixerCompleted<P>, trace: Trace) -> Self {
        Self { queues, mixer, trace }
    }
    pub fn queues(&self) -> &State { &self.queues }
    pub fn mixer_after_move(&self) -> &MixerCompleted<P> { &self.mixer }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn into_parts(self) -> (State, MixerCompleted<P>, Trace) {
        (self.queues, self.mixer, self.trace)
    }
}
