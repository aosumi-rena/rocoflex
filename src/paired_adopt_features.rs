use super::*;

pub struct FeaturesStartFailure<M: Module, R: input::Rows, H, W, A, B, O,
    P: features::Frames, G: features::Gaussian> {
    pub start: features::StartFailure<P, G, M, R, H>, pub adopted: AdoptedOwners<W, A, B, O>,
}
pub struct FeaturesPrepared<M: Module, R: input::Rows, H, W, A, B, O,
    P: features::Frames, G: features::Gaussian> {
    child: features::Prepared<P, G, M, R, H>, adopted: AdoptedOwners<W, A, B, O>,
}
pub struct FeaturesCompleted<M: Module, R: input::Rows, H, W, A, B, O,
    P: features::Frames, G: features::Gaussian> {
    child: features::Completed<P, G, M, R, H>, adopted: AdoptedOwners<W, A, B, O>,
}
pub struct FeaturesCompletedParts<M: Module, R: input::Rows, H, W, A, B, O,
    P: features::Frames, G: features::Gaussian> {
    pub features: features::Completed<P, G, M, R, H>, pub adopted: AdoptedOwners<W, A, B, O>,
}
impl<M: Module, R: input::Rows, H, W, A, B, O> Completed<M, R, H, W, A, B, O> {

    pub fn into_features<P: features::Frames, G: features::Gaussian>(self, state: features::State<P, G>)
        -> Result<FeaturesPrepared<M, R, H, W, A, B, O, P, G>, FeaturesStartFailure<M, R, H, W, A, B, O, P, G>> {
        let parts = self.into_parts();
        match state.begin(parts.input) {
            Ok(child) => Ok(FeaturesPrepared { child, adopted: parts.adopted }),
            Err(start) => Err(FeaturesStartFailure { start, adopted: parts.adopted }),
        }
    }
}
impl<M: Module, R: input::Rows, H, W, A, B, O, P: features::Frames, G: features::Gaussian>
    FeaturesPrepared<M, R, H, W, A, B, O, P, G> {
    pub fn features(&self) -> &features::Prepared<P, G, M, R, H> { &self.child }
    pub fn adopted(&self) -> &AdoptedOwners<W, A, B, O> { &self.adopted }
    pub fn poll(&mut self, budget: usize) -> features::Poll<'_, P::Error, G::Error> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<FeaturesCompleted<M, R, H, W, A, B, O, P, G>, Self> {
        match self.child.into_completed() {
            Ok(child) => Ok(FeaturesCompleted { child, adopted: self.adopted }),
            Err(child) => Err(Self { child, adopted: self.adopted }),
        }
    }
}
impl<M: Module, R: input::Rows, H, W, A, B, O, P: features::Frames, G: features::Gaussian>
    FeaturesCompleted<M, R, H, W, A, B, O, P, G> {
    pub fn features(&self) -> &features::Prepared<P, G, M, R, H> { self.child.prepared() }
    pub fn adopted(&self) -> &AdoptedOwners<W, A, B, O> { &self.adopted }
    pub fn into_parts(self) -> FeaturesCompletedParts<M, R, H, W, A, B, O, P, G> {
        FeaturesCompletedParts { features: self.child, adopted: self.adopted }
    }
}
