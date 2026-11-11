use super::{BranchSelected, Next, Retained, Services};

#[derive(Clone, Copy, Debug)]
pub struct History {
    pub actual_image_base: u64,
    pub vtable_word: u64,
    pub saved_r15: u64,
    pub vtable_store_returned: bool,
    pub member_store_returned: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { VtableStore, MemberStore, ArgumentRetirementRequired }
pub enum Fault<E> { Service(E), AlreadyStopped }
pub struct Rejected<S: Services> { pub selected: BranchSelected<S>, pub actual_image_base: u64 }
pub struct Prepared<S: Services> {
    retained: Retained<S>, history: History, phase: Phase,
    attempted: Option<Phase>, stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(DefaultWritten<S>), Failed(Stopped<S>) }
pub struct DefaultWritten<S: Services> { returned: RequiredRetirement<S> }
pub struct RequiredRetirement<S: Services> { pub retained: Retained<S>, pub history: History }
pub struct Parts<S: Services> {
    retained: Retained<S>, pub history: History, pub phase: Phase,
    pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn retained(&self) -> &Retained<S> { &self.retained }
}
impl<S: Services> DefaultWritten<S> {
    pub fn into_required_retirement(self) -> RequiredRetirement<S> { self.returned }
}
impl<S: Services> Prepared<S> {
    pub fn from_selected_default(selected: BranchSelected<S>, actual_image_base: u64)
        -> Result<Self, Rejected<S>> {
        if selected.next() != Next::DefaultCallback2b360f
            || selected.retained().captures.selected_second_inline.is_none() {
            return Err(Rejected { selected, actual_image_base });
        }
        let saved_r15 = selected.retained().captures.selected_second_inline.unwrap();

        let vtable_word = actual_image_base.wrapping_add(0x627ba8);
        Ok(Self { retained: selected.into_required_continuation(),
            history: History { actual_image_base, vtable_word, saved_r15,
                vtable_store_returned: false, member_store_returned: false },
            phase: Phase::VtableStore, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn history(&self) -> &History { &self.history }
    pub fn into_parts(self) -> Parts<S> {
        Parts { retained: self.retained, history: self.history, phase: self.phase,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::ArgumentRetirementRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            let p = &mut self.retained.parent;
            let result = match self.phase {
                Phase::VtableStore => p.services.write_parameter_word(&mut p.parameter, 0x1a8, self.history.vtable_word),
                Phase::MemberStore => p.services.write_parameter_word(&mut p.parameter, 0x1e0, self.history.saved_r15),
                Phase::ArgumentRetirementRequired => unreachable!(),
            };
            if let Err(error) = result {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::Service(error) });
            }
            match self.phase {
                Phase::VtableStore => { self.history.vtable_store_returned = true; self.phase = Phase::MemberStore; }
                Phase::MemberStore => { self.history.member_store_returned = true; self.phase = Phase::ArgumentRetirementRequired; }
                Phase::ArgumentRetirementRequired => unreachable!(),
            }
            self.attempted = None;
        }
        if self.phase != Phase::ArgumentRetirementRequired { return Poll::Pending(self); }
        Poll::Ready(DefaultWritten { returned: RequiredRetirement { retained: self.retained, history: self.history } })
    }
}
