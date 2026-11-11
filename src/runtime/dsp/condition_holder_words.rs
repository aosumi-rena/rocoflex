#[path = "producer_rng.rs"]
#[allow(dead_code)]
mod producer_rng;

pub struct State { counter: u32, count: usize, backing: Box<[u32]> }
pub struct BindingFailure { pub counter: u32, pub count: usize, pub backing: Vec<u32> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { NeedsPhaseReallocation }
impl State {
    pub fn from_explicit_state(counter: u32, count: usize, backing: Vec<u32>)
        -> Result<Self, BindingFailure>
    {
        if count > backing.len() { return Err(BindingFailure { counter, count, backing }); }
        Ok(Self { counter, count, backing: backing.into_boxed_slice() })
    }

    pub(super) fn from_initial_5c590_zeroed() -> Self {
        Self { counter: 0, count: 0, backing: Vec::new().into_boxed_slice() }
    }
    pub fn counter(&self) -> u32 { self.counter }
    pub fn count(&self) -> usize { self.count }
    pub fn full_backing(&self) -> &[u32] { &self.backing }
    pub fn logical_words(&self) -> &[u32] { &self.backing[..self.count] }
    pub fn prepare(&mut self, count: usize) -> Result<(), Error> {
        if count > self.backing.len() { return Err(Error::NeedsPhaseReallocation); }

        if count > self.count { self.backing[self.count..count].fill(0); }
        self.count = count;
        for (index, word) in self.backing[..count].iter_mut().enumerate() {
            *word = self.counter.wrapping_add(index as u32);
        }
        Ok(())
    }

    pub fn condition_returned(&mut self) {
        self.counter = self.counter.wrapping_add(self.count as u32);
    }

    pub fn first_conditional_boundary(&mut self) {
        if self.count != 0 {
            let remainder = self.counter % 0x7fff_ffff;
            let seed = if remainder == 0 { 1 } else { remainder };
            let mut random = producer_rng::SourceUniform32::from_state_word(seed);
            for word in &mut self.backing[..self.count] { *word = random.next_state(); }
        }
        self.count = 0;
    }

    pub(super) fn reallocate_phase_from_explicit_allocation(&mut self,
        new_count: usize, allocated: Vec<u32>)
        -> Result<(crate::phase_reallocation::PhaseReallocationPlan, Box<[u32]>),
            (crate::phase_reallocation::PhaseAllocationError, Vec<u32>)>
    {
        let plan = match crate::phase_reallocation::PhaseReallocationPlan::bind(
            self.count, self.backing.len(), new_count)
        {
            Ok(plan) => plan,
            Err(cause) => return Err((cause, allocated)),
        };
        let backing = plan.apply_retaining(&self.backing, allocated)?;
        let old = std::mem::replace(&mut self.backing, backing);
        self.count = plan.new_count();
        Ok((plan, old))
    }
}
