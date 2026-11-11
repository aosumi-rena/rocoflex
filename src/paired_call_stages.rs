use super::*;

impl<M: post::Module, R: input::Rows, H, W, A, B, O, S, U: ScalarUpper, P, G>
    Carried<input::Prepared<M, R, History<H>>, ScalarOwners<W, A, B, O, S, U, P, G>> {
    pub fn poll(&mut self, budget: usize) -> input::Poll<'_, M::Error, R::Error> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<input::Completed<M, R, History<H>>, ScalarOwners<W, A, B, O, S, U, P, G>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<M: post::Module, R: input::Rows, H, W, A, B, O, S, U: ScalarUpper, P, G>
    Carried<input::Completed<M, R, History<H>>, ScalarOwners<W, A, B, O, S, U, P, G>> {
    pub fn into_adoption(self)
        -> Carried<adopt::Prepared<M, R, History<H>, W, A, B, O>, NextOwners<S, U, P, G>> {
        let ScalarOwners { condition, engine, packing, next } = self.held;
        Carried { child: adopt::Prepared::after_scalar(self.child, condition, engine, packing), held: next }
    }
}
impl<M: post::Module, R: input::Rows, H, W, A, B, O, S, U: ScalarUpper, P, G>
    Carried<adopt::Prepared<M, R, History<H>, W, A, B, O>, NextOwners<S, U, P, G>> {
    pub fn poll(&mut self, budget: usize) -> adopt::Poll<'_> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<adopt::Completed<M, R, History<H>, W, A, B, O>, NextOwners<S, U, P, G>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<M: post::Module, R: input::Rows, H, W, A, B, O, S, U: ScalarUpper, P: features::Frames, G: features::Gaussian>
    Carried<adopt::Completed<M, R, History<H>, W, A, B, O>, NextOwners<S, U, P, G>> {
    pub fn into_features(self)
        -> Result<Carried<adopt::FeaturesPrepared<M, R, History<H>, W, A, B, O, P, G>, interpolation::State<S, U>>,
            Refused<adopt::FeaturesStartFailure<M, R, History<H>, W, A, B, O, P, G>, interpolation::State<S, U>>> {
        match self.child.into_features(self.held.features) {
            Ok(child) => Ok(Carried { child, held: self.held.interpolation }),
            Err(start) => Err(Refused { start, held: self.held.interpolation }),
        }
    }
}
impl<M: post::Module, R: input::Rows, H, W, A, B, O, S, U: ScalarUpper, P: features::Frames, G: features::Gaussian>
    Carried<adopt::FeaturesPrepared<M, R, History<H>, W, A, B, O, P, G>, interpolation::State<S, U>> {
    pub fn poll(&mut self, budget: usize) -> features::Poll<'_, P::Error, G::Error> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<adopt::FeaturesCompleted<M, R, History<H>, W, A, B, O, P, G>, interpolation::State<S, U>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<M: post::Module, R: input::Rows, H, W, A, B, O, S: interpolation::Storage, U: ScalarUpper,
    P: features::Frames, G: features::Gaussian>
    Carried<adopt::FeaturesCompleted<M, R, History<H>, W, A, B, O, P, G>, interpolation::State<S, U>> {
    pub fn into_interpolation(self)
        -> Result<Carried<interpolation::Prepared<S, U, P, G, M, R, History<H>>, AdoptedOwners<W, A, B, O>>,
            Refused<interpolation::StartFailure<S, U, P, G, M, R, History<H>>, AdoptedOwners<W, A, B, O>>> {
        self.child.into_interpolation(self.held)
    }
}
