use crate::{paired_features::{Frames, FeatureRow}, paired_vector_checkout_values as graph};
pub use graph::{CheckoutComplete, CheckoutProgress, CheckoutStage, CheckoutError};
use crate::paired_vector_return_values as return_graph;
pub use return_graph::{ReturnComplete, ReturnKind, ReturnProgress, ReturnStage};
#[path = "paired_frames_return.rs"]
mod returns;
pub use returns::{RequiredReturn, ReturnError, ReturnPool};
#[path = "paired_frames_identity.rs"]
mod identity;
pub use identity::{CheckoutOrigin, PoolIdentity};

pub trait Fallback {
    type Error;
    fn allocate_pointer_cell8(&mut self) -> Result<(), Self::Error>;
    fn create_with_required_factory(&mut self) -> Result<(), Self::Error>;
    fn admit_created_and_finish(&mut self) -> Result<Vec<f32>, Self::Error>;
}

pub enum AbsentFallback {}
impl Fallback for AbsentFallback {
    type Error = std::convert::Infallible;
    fn allocate_pointer_cell8(&mut self) -> Result<(), Self::Error> { match *self {} }
    fn create_with_required_factory(&mut self) -> Result<(), Self::Error> { match *self {} }
    fn admit_created_and_finish(&mut self) -> Result<Vec<f32>, Self::Error> { match *self {} }
}
pub struct Parts<F> {
    pub identity: PoolIdentity,
    pub bitmap: [u64; 16], pub available: Vec<Option<Vec<f32>>>, pub fallback: Option<F>, pub halted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { SlotCount, BitmapOwner { index: usize } }
pub struct BindingFailure<F> { pub cause: Domain, pub parts: Parts<F> }
pub enum ServiceError<E> { MissingFallback, Fallback(E), MissingCached { index: usize } }
pub struct FramesPool<F> {
    parts: Parts<F>, work: Option<graph::CheckoutWork>, completed_output: Option<Vec<f32>>,
    return_work: Option<return_graph::ReturnWork>,
}

pub struct CheckedOutFrame { row: Vec<f32>, complete: CheckoutComplete, origin: CheckoutOrigin }
impl CheckedOutFrame {
    pub fn samples(&self) -> &[f32] { &self.row }
    pub fn samples_mut(&mut self) -> &mut [f32] { &mut self.row }
    pub fn completion(&self) -> &CheckoutComplete { &self.complete }
    pub fn origin(&self) -> &CheckoutOrigin { &self.origin }

    pub fn value_return_kind(&self) -> ReturnKind {
        if self.complete.choice().is_some() { ReturnKind::Pooled } else { ReturnKind::Unpooled }
    }
    pub fn into_parts(self) -> (Vec<f32>, CheckoutComplete, CheckoutOrigin) {
        (self.row, self.complete, self.origin)
    }
}
impl<F> FramesPool<F> {
    pub fn from_explicit_parts(parts: Parts<F>) -> Result<Self, BindingFailure<F>> {
        let cause = if parts.available.len() != 1024 { Some(Domain::SlotCount) } else {
            (0..1024).find(|&i| (parts.bitmap[i / 64] & (1u64 << (i & 63)) != 0) != parts.available[i].is_some())
                .map(|index| Domain::BitmapOwner { index })
        };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }

        Ok(Self { parts, work: None, completed_output: None, return_work: None })
    }
    pub fn parts(&self) -> &Parts<F> { &self.parts }
    pub fn is_halted(&self) -> bool { self.parts.halted }
    pub fn progress(&self) -> Option<CheckoutProgress> { self.work.as_ref().map(graph::CheckoutWork::progress) }
    pub fn return_progress(&self) -> Option<ReturnProgress> { self.return_work.as_ref().map(return_graph::ReturnWork::progress) }
    pub fn pending_output(&self) -> Option<&[f32]> { self.completed_output.as_deref() }
    pub fn into_ready_parts(self) -> Result<Parts<F>, Self> {
        if self.parts.halted || self.completed_output.is_some() { Err(self) } else { Ok(self.parts) }
    }
}
struct Checkout<'a, F> { parts: &'a mut Parts<F>, output: &'a mut Option<Vec<f32>> }
impl<F: Fallback> graph::CheckoutServices for Checkout<'_, F> {
    type Error = ServiceError<F::Error>;
    fn bitmap_words(&self) -> &[u64; 16] { &self.parts.bitmap }
    fn take_cached_and_finish(&mut self, choice: graph::CachedChoice) -> Result<(), Self::Error> {

        self.parts.bitmap[choice.word()] &= !choice.mask();
        let row = self.parts.available[choice.index()].take()
            .ok_or(ServiceError::MissingCached { index: choice.index() })?;
        *self.output = Some(row); self.parts.halted = false; Ok(())
    }
    fn allocate_pointer_cell8(&mut self) -> Result<(), Self::Error> {
        self.parts.fallback.as_mut().ok_or(ServiceError::MissingFallback)?
            .allocate_pointer_cell8().map_err(ServiceError::Fallback)
    }
    fn create_with_required_factory(&mut self) -> Result<(), Self::Error> {
        self.parts.fallback.as_mut().ok_or(ServiceError::MissingFallback)?
            .create_with_required_factory().map_err(ServiceError::Fallback)
    }
    fn admit_created_and_finish(&mut self) -> Result<(), Self::Error> {
        let row = self.parts.fallback.as_mut().ok_or(ServiceError::MissingFallback)?
            .admit_created_and_finish().map_err(ServiceError::Fallback)?;
        *self.output = Some(row); self.parts.halted = false; Ok(())
    }
}
impl<F: Fallback> FramesPool<F> {

    pub fn acquire_frame(&mut self) -> Result<CheckedOutFrame, CheckoutError<ServiceError<F::Error>>> {
        if self.parts.halted { return Err(CheckoutError::PriorFailure); }
        self.parts.halted = true;
        let mut work = graph::CheckoutWork::after_admission();
        let outcome = {
            let mut services = Checkout { parts: &mut self.parts, output: &mut self.completed_output };
            loop {
                match graph::advance_checkout(&mut work, &mut services, usize::MAX) {
                    Ok(graph::CheckoutAdvance::Paused) => {},
                    result => break result,
                }
            }
        };
        self.work = Some(work);
        match outcome {
            Ok(graph::CheckoutAdvance::Complete(marker)) => {
                Ok(CheckedOutFrame { row: self.completed_output.take()
                    .expect("completed checkout owns actual row"), complete: marker,
                    origin: CheckoutOrigin::from_owner(&self.parts.identity) })
            }
            Ok(graph::CheckoutAdvance::Paused) => unreachable!("same work retained through internal pauses"),

            Err(error) => Err(error),
        }
    }
}
impl<F: Fallback> Frames for FramesPool<F> {
    type Error = CheckoutError<ServiceError<F::Error>>;
    fn acquire(&mut self) -> Result<FeatureRow, Self::Error> {
        self.acquire_frame().map(FeatureRow::from_checkout)
    }
}
