use crate::{paired_adopt_values as graph, paired_condition as condition, paired_features as features,
    paired_input as input, paired_post::Module, paired_post_join as post, paired_scalar_packing as packing};
pub use graph::{Adopted, AdoptProgress, AdoptStage, AdoptError as ControlError};
#[path = "paired_adopt_features.rs"]
mod feature_step;
pub use feature_step::{FeaturesPrepared, FeaturesCompleted, FeaturesStartFailure, FeaturesCompletedParts};
#[path = "paired_adopt_pipeline.rs"]
pub mod pipeline;

pub enum ServiceError { Worker(condition::AdoptError), Post(post::AdoptError), Packing(packing::AdoptError) }
struct Owner<M: Module, R: input::Rows, H, W, A, B, O> {
    input: input::Completed<M, R, H>, condition: condition::State<W>,
    engine: post::Engine<A, B>, packing: packing::State<O>,
}
pub struct Prepared<M: Module, R: input::Rows, H, W, A, B, O> {
    owner: Owner<M, R, H, W, A, B, O>, work: graph::AdoptWork,
    failure: Option<ControlError<ServiceError>>, complete: Option<Adopted>,
}
pub enum Poll<'a> { Pending(AdoptProgress), Complete, Failed(&'a ControlError<ServiceError>) }
pub struct Completed<M: Module, R: input::Rows, H, W, A, B, O> { prepared: Prepared<M, R, H, W, A, B, O> }
pub struct CompletedParts<M: Module, R: input::Rows, H, W, A, B, O> {
    pub input: input::Completed<M, R, H>, pub adopted: AdoptedOwners<W, A, B, O>,
}

pub struct AdoptedOwners<W, A, B, O> {
    condition: condition::State<W>, engine: post::Engine<A, B>, packing: packing::State<O>, adopted: Adopted,
}
impl<W, A, B, O> AdoptedOwners<W, A, B, O> {
    pub fn condition(&self) -> &condition::State<W> { &self.condition }
    pub fn engine(&self) -> &post::Engine<A, B> { &self.engine }
    pub fn packing(&self) -> &packing::State<O> { &self.packing }
    pub fn adoption(&self) -> &Adopted { &self.adopted }
    pub fn into_parts(self) -> (condition::State<W>, post::Engine<A, B>, packing::State<O>, Adopted) {
        (self.condition, self.engine, self.packing, self.adopted)
    }
}
impl<M: Module, R: input::Rows, H, W, A, B, O> Prepared<M, R, H, W, A, B, O> {

    pub fn after_scalar(input: input::Completed<M, R, H>, condition: condition::State<W>,
        engine: post::Engine<A, B>, packing: packing::State<O>) -> Self {
        let ended = input.prepared().state().parts().ended517;
        Self { owner: Owner { input, condition, engine, packing },
            work: graph::AdoptWork::after_scalar_prefix(ended), failure: None, complete: None }
    }
    pub fn input(&self) -> &input::Completed<M, R, H> { &self.owner.input }
    pub fn condition(&self) -> &condition::State<W> { &self.owner.condition }
    pub fn engine(&self) -> &post::Engine<A, B> { &self.owner.engine }
    pub fn packing(&self) -> &packing::State<O> { &self.owner.packing }
    pub fn progress(&self) -> AdoptProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&ControlError<ServiceError>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::AdoptAdvance::Paused) => {},
                Ok(graph::AdoptAdvance::Complete(marker)) => self.complete = Some(marker),
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete } else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<M, R, H, W, A, B, O>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<M: Module, R: input::Rows, H, W, A, B, O> Completed<M, R, H, W, A, B, O> {
    pub fn prepared(&self) -> &Prepared<M, R, H, W, A, B, O> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<M, R, H, W, A, B, O> {
        let owner = self.prepared.owner;
        CompletedParts { input: owner.input, adopted: AdoptedOwners { condition: owner.condition,
            engine: owner.engine, packing: owner.packing, adopted: self.prepared.complete.expect("private adoption completion") } }
    }
}
impl<M: Module, R: input::Rows, H, W, A, B, O> graph::AdoptServices for Owner<M, R, H, W, A, B, O> {
    type Error = ServiceError;
    fn adopt_worker_ended517(&mut self, ended: u8) -> Result<(), Self::Error> {
        self.condition.adopt_advanced_prefix_ended517(ended).map_err(ServiceError::Worker)
    }
    fn adopt_post_ended517(&mut self, ended: u8) -> Result<(), Self::Error> {
        self.engine.adopt_advanced_prefix_ended517(ended).map_err(ServiceError::Post)
    }
    fn adopt_packing_ended517(&mut self, ended: u8) -> Result<(), Self::Error> {
        self.packing.adopt_advanced_prefix_ended517(ended).map_err(ServiceError::Packing)
    }
}
