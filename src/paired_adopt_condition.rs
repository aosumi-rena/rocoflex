use super::*;

impl<W, S: interpolation::Storage, U: ScalarUpper, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, A, B, O>
    Carried<append::Prepared<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>> {
    pub fn poll(&mut self, budget: usize) -> append::AppendPoll<'_> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<append::Completed<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<W: condition::Worker, S: interpolation::Storage, U: ScalarUpper, P: features::Frames,
    G: features::Gaussian, M: post::Module, R: input::Rows, H, A, B, O>
    Carried<append::Completed<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>> {
    pub fn into_condition(self)
        -> Result<Carried<append::ConditionPrepared<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>,
            Refused<append::ConditionStartFailure<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>> {
        match self.child.into_condition() {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(start) => Err(Refused { start, held: self.held }),
        }
    }
}
impl<W: condition::Worker, S: interpolation::Storage, U: ScalarUpper, P: features::Frames,
    G: features::Gaussian, M: post::Module, R: input::Rows, H, A, B, O>
    Carried<append::ConditionPrepared<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>> {
    pub fn poll(&mut self, budget: usize) -> condition::Poll<'_, W::Error> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<append::ConditionCompleted<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<W: condition::Worker, S: interpolation::Storage, U: ScalarUpper, P: features::Frames,
    G: features::Gaussian, M: post::Module, R: input::Rows, H, A: post::Module, B: post::Module, O>
    Carried<append::ConditionCompleted<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>> {
    pub fn into_post(self)
        -> Result<Carried<post::Prepared<A, B, U, join::History<W, S, P, G, M, R, H>>, PackingOwner<O>>,
            Refused<join::StartFailure<A, B, U, W, S, P, G, M, R, H>, PackingOwner<O>>> {
        let SuffixOwners { engine, packing, adopted } = self.held;
        let held = PackingOwner { packing, adopted };
        match self.child.into_post_with_engine(engine) {
            Ok(child) => Ok(Carried { child, held }),
            Err(start) => Err(Refused { start, held }),
        }
    }
}
