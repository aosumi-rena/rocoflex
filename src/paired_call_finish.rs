use super::*;

impl<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O, E>
    Carried<join::Restored<A, B, U, W, S, P, G, M, R, History<H>, O, E>, Adopted> {
    pub fn finish_call(self)
        -> (join::Restored<A, B, U, W, S, P, G, M, R, H, O, E>, Adopted, ValueCallComplete) {
        let join::Restored { engine, features, condition, interpolation, packing, erasure, input,
            feature_trace, final_cache, completed_frames, last_frame, enqueued, selected,
            condition_trace, condition_complete, interpolation_trace, interpolation_complete,
            append_trace, appended, post_trace, post_complete, packing_trace, packing_complete, retirement } = self.child;

        let (input, complete) = input.split_held(History::finish);
        (join::Restored { engine, features, condition, interpolation, packing, erasure, input,
            feature_trace, final_cache, completed_frames, last_frame, enqueued, selected,
            condition_trace, condition_complete, interpolation_trace, interpolation_complete,
            append_trace, appended, post_trace, post_complete, packing_trace, packing_complete, retirement },
            self.held, complete)
    }
}
