use crate::{conditional_gate::Batch, gaussian_noise::PartnerCache,
    ltv_coefficient_math::ScalarUpper, paired_condition as condition, paired_features as features,
    paired_input as input, paired_interpolation as interpolation, paired_post as post};
#[path = "paired_post_join_restore.rs"]
mod restore;
pub use restore::Restored;
pub(crate) use restore::restore_retired_parts;

pub struct EngineParts<A, B> {
    pub module56: A, pub module40: B, pub groups: usize, pub enabled: bool,
    pub ended: u8, pub cursor_index512: i32, pub stage272: Batch, pub final304: Batch,
}
pub struct Engine<A, B> { parts: EngineParts<A, B> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineDomain { Groups, Ended, CursorIndex }
pub struct EngineFailure<A, B> { pub cause: EngineDomain, pub parts: EngineParts<A, B> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdoptError { Binding }
impl<A, B> Engine<A, B> {
    pub fn from_explicit_parts(parts: EngineParts<A, B>) -> Result<Self, EngineFailure<A, B>> {
        let cause = if parts.groups == 0 || parts.groups & 1 != 0 || parts.groups > i32::MAX as usize / 2 {
            Some(EngineDomain::Groups)
        } else if parts.ended > 1 { Some(EngineDomain::Ended) }
        else if parts.enabled && parts.cursor_index512 < 0 { Some(EngineDomain::CursorIndex) } else { None };
        if let Some(cause) = cause { return Err(EngineFailure { cause, parts }); }
        Ok(Self { parts })
    }
    pub fn parts(&self) -> &EngineParts<A, B> { &self.parts }
    pub fn into_parts(self) -> EngineParts<A, B> { self.parts }

    pub(crate) fn adopt_advanced_prefix_ended517(&mut self, ended: u8) -> Result<(), AdoptError> {
        if self.parts.ended != 0 || ended > 1 { return Err(AdoptError::Binding); }
        self.parts.ended = ended; Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Groups, Ended }
pub enum StartFailure<A, B, U: ScalarUpper, W: condition::Worker, S: interpolation::Storage,
    P: features::Frames, G: features::Gaussian, M: post::Module, R: input::Rows, H> {
    Admission { cause: Domain, completed: condition::interpolation_append::ConditionCompleted<W, S, U, P, G, M, R, H>,
        engine: Engine<A, B> },
    Binding { post: post::BindingFailure<A, B, U>, input240: Batch, held: History<W, S, P, G, M, R, H> },
}

pub struct History<W, S, P: features::Frames, G: features::Gaussian, M: post::Module, R: input::Rows, H> {
    feature_engine: features::QueueSuspended<P, G>, condition_engine: condition::SharedSuspended<W>,
    interpolation_engine: interpolation::MathSuspended<S>, input: input::Completed<M, R, H>,
    feature_trace: features::Trace, final_cache: PartnerCache, completed_frames: usize,
    last_frame: Option<features::FrameComplete>, enqueued: features::QueueComplete,
    selected: condition::WorkCount, condition_trace: condition::Trace, condition: condition::ConditionComplete,
    interpolation_trace: interpolation::Trace, interpolation: interpolation::InterpolationComplete,
    append_trace: condition::interpolation_append::AppendTrace,
    appended: condition::interpolation_append::AppendComplete,
}
impl<W, S, P: features::Frames, G: features::Gaussian, M: post::Module, R: input::Rows, H>
    History<W, S, P, G, M, R, H> {
    pub fn input(&self) -> &input::Completed<M, R, H> { &self.input }
    pub fn frame_provider(&self) -> &P { self.feature_engine.frames() }

    pub fn frame_provider_mut(&mut self) -> &mut P { self.feature_engine.frames_mut() }
    pub fn gaussian(&self) -> &G { self.feature_engine.gaussian() }
    pub fn worker(&self) -> &W { self.condition_engine.worker() }
    pub fn interpolation_storage(&self) -> &S { self.interpolation_engine.storage() }
    pub fn feature_trace(&self) -> &features::Trace { &self.feature_trace }
    pub fn condition_trace(&self) -> &condition::Trace { &self.condition_trace }
    pub fn interpolation_trace(&self) -> &interpolation::Trace { &self.interpolation_trace }
    pub fn append_trace(&self) -> &condition::interpolation_append::AppendTrace { &self.append_trace }
    pub fn final_cache(&self) -> PartnerCache { self.final_cache }

    pub(crate) fn split_input_held<T, X>(self, split: impl FnOnce(H) -> (T, X))
        -> (History<W, S, P, G, M, R, T>, X) {
        let Self { feature_engine, condition_engine, interpolation_engine, input,
            feature_trace, final_cache, completed_frames, last_frame, enqueued,
            selected, condition_trace, condition, interpolation_trace, interpolation,
            append_trace, appended } = self;
        let (input, extra) = input.split_held(split);
        (History { feature_engine, condition_engine, interpolation_engine, input,
            feature_trace, final_cache, completed_frames, last_frame, enqueued,
            selected, condition_trace, condition, interpolation_trace, interpolation,
            append_trace, appended }, extra)
    }
}
impl<W: condition::Worker, S: interpolation::Storage, U: ScalarUpper, P: features::Frames,
    G: features::Gaussian, M: post::Module, R: input::Rows, H>
    condition::interpolation_append::ConditionCompleted<W, S, U, P, G, M, R, H> {

    pub fn into_post_with_engine<A: post::Module, B: post::Module>(self, engine: Engine<A, B>)
        -> Result<post::Prepared<A, B, U, History<W, S, P, G, M, R, H>>,
            StartFailure<A, B, U, W, S, P, G, M, R, H>> {
        let condition = self.prepared();
        let cause = if engine.parts.groups != condition.upstream().prepared().state().parts().groups {
            Some(Domain::Groups)
        } else if engine.parts.ended != condition.state().parts().ended517 { Some(Domain::Ended) } else { None };
        if let Some(cause) = cause { return Err(StartFailure::Admission { cause, completed: self, engine }); }
        let (condition, earlier) = self.into_parts();
        let feature = condition.upstream.into_parts();
        let (feature_engine, noise_rows) = feature.state.suspend_queue();
        let (condition_engine, input240, cursor_count, cursor_words, cursor_halted) = condition.state.suspend_shared();
        let (interpolation_engine, math) = earlier.interpolation.suspend_math();
        let held = History { feature_engine, condition_engine, interpolation_engine, input: feature.input,
            feature_trace: feature.trace, final_cache: feature.final_cache, completed_frames: feature.completed_frames,
            last_frame: feature.last_frame, enqueued: feature.enqueued,
            selected: condition.selected, condition_trace: condition.trace, condition: condition.condition,
            interpolation_trace: earlier.interpolation_trace, interpolation: earlier.interpolation_complete,
            append_trace: earlier.append_trace, appended: earlier.appended };
        let EngineParts { module56, module40, groups, enabled, ended, cursor_index512,
            stage272, final304 } = engine.parts;
        let parts = post::Parts { module56, module40, math, groups, enabled, ended,
            cursor_index: cursor_index512, cursor_count, cursor_words, cursor_halted, noise_rows,
            stage272, final304: Some(final304) };
        match post::State::from_explicit_parts(parts) {
            Ok(state) => Ok(state.begin(input240, held)),
            Err(post) => Err(StartFailure::Binding { post, input240, held }),
        }
    }
}
