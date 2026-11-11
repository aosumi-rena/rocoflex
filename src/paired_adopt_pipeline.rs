use super::{Adopted, AdoptedOwners, FeaturesCompleted};
use crate::{ltv_coefficient_math::ScalarUpper, paired_condition as condition,
    paired_features as features, paired_input as input, paired_interpolation as interpolation,
    paired_post as post, paired_post_join as join, paired_scalar_packing as packing};
use condition::interpolation_append as append;
use post::{packing as packed, retirement};
#[path = "paired_call.rs"]
pub mod call;
#[path = "root_cepstral_handoff.rs"]
pub mod root_cepstral_handoff;

pub struct Carried<C, T> { child: C, held: T }
impl<C, T> Carried<C, T> {
    pub fn child(&self) -> &C { &self.child }
    pub fn held(&self) -> &T { &self.held }
    fn finish<N>(self, complete: impl FnOnce(C) -> Result<N, C>) -> Result<Carried<N, T>, Self> {
        match complete(self.child) {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(child) => Err(Self { child, held: self.held }),
        }
    }
}

pub struct Refused<E, T> { pub start: E, pub held: T }
pub struct SuffixOwners<A, B, O> { engine: join::Engine<A, B>, packing: packing::State<O>, adopted: Adopted }
impl<A, B, O> SuffixOwners<A, B, O> {
    pub fn engine(&self) -> &join::Engine<A, B> { &self.engine }
    pub fn packing(&self) -> &packing::State<O> { &self.packing }
    pub fn adoption(&self) -> &Adopted { &self.adopted }
}
pub struct PackingOwner<O> { packing: packing::State<O>, adopted: Adopted }
impl<O> PackingOwner<O> {
    pub fn packing(&self) -> &packing::State<O> { &self.packing }
    pub fn adoption(&self) -> &Adopted { &self.adopted }
}
#[path = "paired_adopt_interpolation.rs"]
mod interpolation_step;
#[path = "paired_adopt_condition.rs"]
mod condition_step;
#[path = "paired_adopt_post.rs"]
mod post_step;
