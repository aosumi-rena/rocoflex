use std::collections::VecDeque;
use crate::{gaussian_noise, paired_feature_values as graph, paired_input, paired_post::Module};
pub use crate::paired_feature_row::{FeatureRow, FeatureRowOwner};
pub use graph::{FrameComplete, FrameProgress, QueueComplete, QueueProgress};
#[path = "paired_features_services.rs"]
mod services;

pub trait Frames {
    type Error;
    fn acquire(&mut self) -> Result<FeatureRow, Self::Error>;
}

pub trait Gaussian {
    type Error;
    fn standard(&mut self, cache: &mut gaussian_noise::PartnerCache) -> Result<u32, Self::Error>;
}

pub struct StandardGaussian {
    profile: gaussian_noise::ScalarProfile, attempt_limit: usize,
    uniform_word: u32, consumed: usize, failure: Option<gaussian_noise::FailureReason>,
}
impl StandardGaussian {
    pub fn from_explicit_parts(profile: gaussian_noise::ScalarProfile, attempt_limit: usize,
        uniform_word: u32, consumed: usize) -> Self {
        Self { profile, attempt_limit, uniform_word, consumed, failure: None }
    }
    pub fn uniform_word(&self) -> u32 { self.uniform_word }
    pub fn consumed_uniforms(&self) -> usize { self.consumed }
    pub fn failure(&self) -> Option<&gaussian_noise::FailureReason> { self.failure.as_ref() }
}
impl Gaussian for StandardGaussian {
    type Error = gaussian_noise::FailureReason;
    fn standard(&mut self, cache: &mut gaussian_noise::PartnerCache) -> Result<u32, Self::Error> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let result = gaussian_noise::draw_standard_values(self.profile, self.attempt_limit,
            &mut self.uniform_word, cache, &mut self.consumed);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }
}
pub struct Parts<P, G> {
    pub groups: usize, pub frames: P, pub gaussian: G, pub queue400: VecDeque<FeatureRow>,
}
pub struct State<P, G> { parts: Parts<P, G>, halted: bool }
pub(crate) struct QueueSuspended<P, G> { groups: usize, frames: P, gaussian: G, halted: bool }
impl<P, G> QueueSuspended<P, G> {
    pub(crate) fn frames(&self) -> &P { &self.frames }
    pub(crate) fn frames_mut(&mut self) -> &mut P { &mut self.frames }
    pub(crate) fn gaussian(&self) -> &G { &self.gaussian }
    pub(crate) fn restore(self, queue400: VecDeque<FeatureRow>) -> State<P, G> {
        State { parts: Parts { groups: self.groups, frames: self.frames, gaussian: self.gaussian, queue400 }, halted: self.halted }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Groups, QueueWidth, InputGroups, InputCount }
pub struct BindingFailure<P, G> { pub cause: Domain, pub parts: Parts<P, G> }
pub struct StartFailure<P, G, M: Module, R: paired_input::Rows, H> {
    pub cause: Domain, pub state: State<P, G>, pub input: paired_input::Completed<M, R, H>,
}
impl<P, G> State<P, G> {
    pub fn from_explicit_parts(parts: Parts<P, G>) -> Result<Self, BindingFailure<P, G>> {
        let cause = if parts.groups == 0 || parts.groups & 1 != 0 || parts.groups > i32::MAX as usize / 2 {
            Some(Domain::Groups)
        } else if parts.queue400.iter().any(|row| row.len() != 2 * parts.groups) {
            Some(Domain::QueueWidth)
        } else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<P, G> { &self.parts }
    pub fn is_halted(&self) -> bool { self.halted }

    pub fn into_parts(self) -> Parts<P, G> { self.parts }
    pub(crate) fn suspend_queue(self) -> (QueueSuspended<P, G>, VecDeque<FeatureRow>) {
        let Parts { groups, frames, gaussian, queue400 } = self.parts;
        (QueueSuspended { groups, frames, gaussian, halted: self.halted }, queue400)
    }
    pub fn begin<M: Module, R: paired_input::Rows, H>(mut self, input: paired_input::Completed<M, R, H>)
        -> Result<Prepared<P, G, M, R, H>, StartFailure<P, G, M, R, H>> where P: Frames, G: Gaussian {
        let source = input.prepared().state().parts();
        let count = source.feature144.rows.len();
        let cause = if source.groups != self.parts.groups { Some(Domain::InputGroups) }
            else if count > i32::MAX as usize { Some(Domain::InputCount) } else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, state: self, input }); }
        self.halted = true;
        Ok(Prepared { owner: Owner { state: self, input,

            cache: gaussian_noise::PartnerCache::fresh_for_paired_call(),
            noise_halted: false, completed_frames: 0, cached_row: None,
            frame_work: None, frame_complete: None, pending_frame: None,
            trace: Trace { row: None, input_width: None, acquired: 0,
                copied: 0, gaussian_stored: 0, last_bits: None, appended: 0 } },
            work: graph::QueueWork::after_explicit_admission(count), failure: None, complete: None })
    }
}
pub struct Trace {
    pub row: Option<usize>, pub input_width: Option<usize>, pub acquired: usize,
    pub copied: usize, pub gaussian_stored: usize, pub last_bits: Option<u32>, pub appended: usize,
}
pub enum FrameServiceError<P, G> {
    Acquire(P), Gaussian(G), InputWidth { expected: usize, actual: usize },
    TargetWidth { expected: usize, actual: usize }, CounterOverflow, Control(&'static str),
}
pub enum ServiceError<P, G> {
    Frame(graph::FrameError<FrameServiceError<P, G>>),
    Allocation(std::collections::TryReserveError), Control(&'static str),
}
struct Owner<P, G, M: Module, R: paired_input::Rows, H> {
    state: State<P, G>, input: paired_input::Completed<M, R, H>, cache: gaussian_noise::PartnerCache,
    noise_halted: bool, completed_frames: usize, cached_row: Option<usize>,
    frame_work: Option<graph::FrameWork>, frame_complete: Option<FrameComplete>,
    pending_frame: Option<FeatureRow>, trace: Trace,
}
pub struct Prepared<P: Frames, G: Gaussian, M: Module, R: paired_input::Rows, H> {
    owner: Owner<P, G, M, R, H>, work: graph::QueueWork,
    failure: Option<graph::QueueError<ServiceError<P::Error, G::Error>>>, complete: Option<QueueComplete>,
}
pub enum Poll<'a, P, G> {
    Pending(QueueProgress), Complete, Failed(&'a graph::QueueError<ServiceError<P, G>>),
}
pub struct Completed<P: Frames, G: Gaussian, M: Module, R: paired_input::Rows, H> {
    prepared: Prepared<P, G, M, R, H>,
}
pub struct CompletedParts<P, G, M: Module, R: paired_input::Rows, H> {
    pub state: State<P, G>, pub input: paired_input::Completed<M, R, H>, pub trace: Trace,

    pub final_cache: gaussian_noise::PartnerCache, pub completed_frames: usize,
    pub last_frame: Option<FrameComplete>, pub enqueued: QueueComplete,
}
impl<P: Frames, G: Gaussian, M: Module, R: paired_input::Rows, H> Prepared<P, G, M, R, H> {
    pub fn state(&self) -> &State<P, G> { &self.owner.state }
    pub fn input(&self) -> &paired_input::Completed<M, R, H> { &self.owner.input }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn cache(&self) -> gaussian_noise::PartnerCache { self.owner.cache }
    pub fn pending_frame(&self) -> Option<&[f32]> { self.owner.pending_frame.as_deref() }
    pub fn pending_frame_owner(&self) -> Option<&FeatureRow> { self.owner.pending_frame.as_ref() }
    pub fn completed_frames(&self) -> usize { self.owner.completed_frames }
    pub fn noise_halted(&self) -> bool { self.owner.noise_halted }
    pub fn progress(&self) -> QueueProgress { self.work.progress() }
    pub fn frame_progress(&self) -> Option<FrameProgress> { self.owner.frame_work.as_ref().map(graph::FrameWork::progress) }
    pub fn failure(&self) -> Option<&graph::QueueError<ServiceError<P::Error, G::Error>>> { self.failure.as_ref() }

    pub fn poll(&mut self, budget: usize) -> Poll<'_, P::Error, G::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance_queue(&mut self.work, &mut self.owner, budget) {
                Ok(graph::QueueAdvance::Paused) => {},
                Ok(graph::QueueAdvance::Complete(marker)) => {
                    self.owner.state.halted = false; self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete } else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<P, G, M, R, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<P: Frames, G: Gaussian, M: Module, R: paired_input::Rows, H> Completed<P, G, M, R, H> {
    pub fn prepared(&self) -> &Prepared<P, G, M, R, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<P, G, M, R, H> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, input: owner.input, trace: owner.trace,
            final_cache: owner.cache, completed_frames: owner.completed_frames,
            last_frame: owner.frame_complete, enqueued: self.prepared.complete.expect("private queue completion") }
    }
}
