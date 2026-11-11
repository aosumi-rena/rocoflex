use super::*;
use crate::{paired_scalar_packing as packing, paired_post::retirement};

pub struct Restored<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O, E> {
    pub engine: Engine<A, B>, pub features: features::State<P, G>, pub condition: condition::State<W>,
    pub interpolation: interpolation::State<S, U>, pub packing: packing::State<O>, pub erasure: E,
    pub input: input::Completed<M, R, H>, pub feature_trace: features::Trace,
    pub final_cache: PartnerCache, pub completed_frames: usize, pub last_frame: Option<features::FrameComplete>,
    pub enqueued: features::QueueComplete, pub selected: condition::WorkCount,
    pub condition_trace: condition::Trace, pub condition_complete: condition::ConditionComplete,
    pub interpolation_trace: interpolation::Trace, pub interpolation_complete: interpolation::InterpolationComplete,
    pub append_trace: condition::interpolation_append::AppendTrace,
    pub appended: condition::interpolation_append::AppendComplete,
    pub post_trace: post::Trace, pub post_complete: post::PostComplete,
    pub packing_trace: packing::Trace, pub packing_complete: packing::PackingComplete,
    pub retirement: retirement::RetirementComplete,
}
impl<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O, E: retirement::Erasure>
    retirement::Completed<A, B, U, History<W, S, P, G, M, R, H>, O, E> {

    pub fn into_joined_state(self) -> Result<Restored<A, B, U, W, S, P, G, M, R, H, O, E>, Self> {
        if self.prepared().continuation().post.parts().final304.is_none() { return Err(self); }
        match restore_retired_parts(self.into_parts()) {
            Ok(restored) => Ok(restored),
            Err(_) => unreachable!("same completed final304 checked before pure move"),
        }
    }
}

pub(crate) fn restore_retired_parts<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O, E>(retired: retirement::CompletedParts<A, B, U, History<W, S, P, G, M, R, H>, O, E>)
    -> Result<Restored<A, B, U, W, S, P, G, M, R, H, O, E>,
        retirement::CompletedParts<A, B, U, History<W, S, P, G, M, R, H>, O, E>> {
        if retired.continuation.post.parts().final304.is_none() { return Err(retired); }
        let post::packing::Continuation { post: post_state, packing, input240, held,
            post_trace, packing_trace, post_completion, packing_completion } = retired.continuation;
        let post::Parts { module56, module40, math, groups, enabled, ended, cursor_index,
            cursor_count, cursor_words, cursor_halted, noise_rows, stage272, final304 } = post_state.into_parts();
        let engine = Engine { parts: EngineParts { module56, module40, groups, enabled, ended,
            cursor_index512: cursor_index, stage272, final304: final304.expect("completed packing restored actual304") } };
        let features = held.feature_engine.restore(noise_rows);
        let condition = held.condition_engine.restore(input240, cursor_count, cursor_words, cursor_halted);
        let interpolation = held.interpolation_engine.restore(math);
        Ok(Restored { engine, features, condition, interpolation, packing, erasure: retired.erasure,
            input: held.input, feature_trace: held.feature_trace, final_cache: held.final_cache,
            completed_frames: held.completed_frames, last_frame: held.last_frame, enqueued: held.enqueued,
            selected: held.selected, condition_trace: held.condition_trace, condition_complete: held.condition,
            interpolation_trace: held.interpolation_trace, interpolation_complete: held.interpolation,
            append_trace: held.append_trace, appended: held.appended,
            post_trace, post_complete: post_completion, packing_trace, packing_complete: packing_completion,
            retirement: retired.retirement })
}
