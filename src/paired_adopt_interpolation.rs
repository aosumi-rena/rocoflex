use super::*;

impl<M: post::Module, R: input::Rows, H, W, A, B, O, P: features::Frames, G: features::Gaussian>
    FeaturesCompleted<M, R, H, W, A, B, O, P, G> {
    pub fn into_interpolation<S: interpolation::Storage, U: ScalarUpper>(self, state: interpolation::State<S, U>)
        -> Result<Carried<interpolation::Prepared<S, U, P, G, M, R, H>, AdoptedOwners<W, A, B, O>>,
            Refused<interpolation::StartFailure<S, U, P, G, M, R, H>, AdoptedOwners<W, A, B, O>>> {
        let parts = self.into_parts();
        match state.begin(parts.features) {
            Ok(child) => Ok(Carried { child, held: parts.adopted }),
            Err(start) => Err(Refused { start, held: parts.adopted }),
        }
    }
}
impl<S: interpolation::Storage, U: ScalarUpper, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, W, A, B, O>
    Carried<interpolation::Prepared<S, U, P, G, M, R, H>, AdoptedOwners<W, A, B, O>> {
    pub fn poll(&mut self, budget: usize) -> interpolation::Poll<'_, S::Error> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<interpolation::Completed<S, U, P, G, M, R, H>, AdoptedOwners<W, A, B, O>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<S: interpolation::Storage, U: ScalarUpper, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, W, A, B, O>
    Carried<interpolation::Completed<S, U, P, G, M, R, H>, AdoptedOwners<W, A, B, O>> {
    pub fn into_append(self)
        -> Result<Carried<append::Prepared<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>,
            Refused<append::AppendStartFailure<W, S, U, P, G, M, R, H>, SuffixOwners<A, B, O>>> {
        let (condition, engine, packing, adopted) = self.held.into_parts();
        let held = SuffixOwners { engine, packing, adopted };
        match condition.append_interpolation(self.child) {
            Ok(child) => Ok(Carried { child, held }),
            Err(start) => Err(Refused { start, held }),
        }
    }
}
