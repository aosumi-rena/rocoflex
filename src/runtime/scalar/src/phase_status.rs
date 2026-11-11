use super::finite_power::ExplicitExceptionState;

pub(super) struct PhaseStatusAccumulator {
    pub(super) state: ExplicitExceptionState,
    pub(super) raised: u32,
}
impl PhaseStatusAccumulator {
    pub(super) fn new(state: ExplicitExceptionState) -> Self { Self { state, raised: 0 } }
    pub(super) fn raise(&mut self, flags: u32) {
        self.state = self.state.with_source_status(flags);
        self.raised |= flags;
    }
}
