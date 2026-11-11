use super::*;

impl<A: post::Module, B: post::Module, U: ScalarUpper, H, O>
    Carried<post::Prepared<A, B, U, H>, PackingOwner<O>> {
    pub fn poll(&mut self, budget: usize) -> post::Poll<'_, A::Error, B::Error> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<Carried<post::Completed<A, B, U, H>, PackingOwner<O>>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<A: post::Module, B: post::Module, U: ScalarUpper, H, O: packing::CurrentRows>
    Carried<post::Completed<A, B, U, H>, PackingOwner<O>> {
    pub fn into_packing(self) -> Result<Carried<packed::Prepared<A, B, U, H, O>, Adopted>,
        Refused<packed::StartFailure<A, B, U, H, O>, Adopted>> {
        match self.child.into_packing(self.held.packing) {
            Ok(child) => Ok(Carried { child, held: self.held.adopted }),
            Err(start) => Err(Refused { start, held: self.held.adopted }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, O: packing::CurrentRows>
    Carried<packed::Prepared<A, B, U, H, O>, Adopted> {
    pub fn poll(&mut self, budget: usize) -> packing::Poll<'_, O::Error> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<Carried<packed::Packed<A, B, U, H, O>, Adopted>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<A, B, U: ScalarUpper, H, O: packing::CurrentRows>
    Carried<packed::Packed<A, B, U, H, O>, Adopted> {
    pub fn into_retirement<E: retirement::Erasure<Context = H>>(self, erasure: E)
        -> Result<Carried<retirement::Prepared<A, B, U, H, O, E>, Adopted>,
            Refused<retirement::StartFailure<A, B, U, H, O, E>, Adopted>> {
        match self.child.into_retirement(erasure) {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(start) => Err(Refused { start, held: self.held }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, O, E: retirement::Erasure>
    Carried<retirement::Prepared<A, B, U, H, O, E>, Adopted> {
    pub fn poll(&mut self, budget: usize) -> retirement::Poll<'_, E::Error>
        where E: retirement::Erasure<Context = H> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<Carried<retirement::Completed<A, B, U, H, O, E>, Adopted>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<A, B, U: ScalarUpper, W: condition::Worker, S: interpolation::Storage, P: features::Frames,
    G: features::Gaussian, M: post::Module, R: input::Rows, H, O, E: retirement::Erasure>
    Carried<retirement::Completed<A, B, U, join::History<W, S, P, G, M, R, H>, O, E>, Adopted> {

    pub fn into_joined_state(self)
        -> Result<Carried<join::Restored<A, B, U, W, S, P, G, M, R, H, O, E>, Adopted>, Self> {
        self.finish(|child| child.into_joined_state())
    }
}
impl<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O, E>
    Carried<join::Restored<A, B, U, W, S, P, G, M, R, H, O, E>, Adopted> {
    pub fn into_parts(self) -> (join::Restored<A, B, U, W, S, P, G, M, R, H, O, E>, Adopted) {
        (self.child, self.held)
    }
}
