use super::{CheckedOutFrame, FramesPool, Parts, ReturnComplete, ReturnKind, return_graph as graph};

pub struct ReturnPool<'a, F> {
    pub bitmap: &'a mut [u64; 16],
    pub available: &'a mut [Option<Vec<f32>>],
    pub fallback: &'a mut Option<F>,
}

pub trait RequiredReturn<F> {
    type Error;

    fn checked_out_frame(&self) -> &CheckedOutFrame;
    fn admit_return(&mut self, pool: &Parts<F>) -> Result<ReturnKind, Self::Error>;

    fn pooled_callback_and_publish_current_raw(&mut self, pool: ReturnPool<'_, F>) -> Result<(), Self::Error>;

    fn admit_current_and_install_saved_slot(&mut self, pool: ReturnPool<'_, F>) -> Result<(), Self::Error>;
    fn delete_unpooled_vector(&mut self, pool: ReturnPool<'_, F>) -> Result<(), Self::Error>;
    fn free_unpooled_pointer_cell(&mut self, pool: ReturnPool<'_, F>) -> Result<(), Self::Error>;
}
pub enum ReturnError<E> {
    PriorFailure, PoolIdentity, Admission(E),
    CheckoutKind { expected: ReturnKind, admitted: ReturnKind },
    Control(graph::ReturnError<E>),
}
struct Return<'a, F, S> { pool: &'a mut Parts<F>, services: &'a mut S }
fn return_pool<F>(parts: &mut Parts<F>) -> ReturnPool<'_, F> {
    ReturnPool { bitmap: &mut parts.bitmap, available: &mut parts.available,
        fallback: &mut parts.fallback }
}
impl<F, S: RequiredReturn<F>> graph::ReturnServices for Return<'_, F, S> {
    type Error = S::Error;
    fn pooled_callback_and_publish_current_raw(&mut self) -> Result<(), Self::Error> {
        self.services.pooled_callback_and_publish_current_raw(return_pool(self.pool))
    }
    fn admit_current_and_install_saved_slot(&mut self) -> Result<(), Self::Error> {
        self.services.admit_current_and_install_saved_slot(return_pool(self.pool))
    }
    fn bitmap_words(&self) -> &[u64; 16] { &self.pool.bitmap }
    fn cached_owner_present(&self, index: usize) -> bool { self.pool.available[index].is_some() }
    fn delete_unpooled_vector(&mut self) -> Result<(), Self::Error> {
        self.services.delete_unpooled_vector(return_pool(self.pool))
    }
    fn free_unpooled_pointer_cell(&mut self) -> Result<(), Self::Error> {
        self.services.free_unpooled_pointer_cell(return_pool(self.pool))
    }
    fn finish_return(&mut self) { self.pool.halted = false; }
}
impl<F> FramesPool<F> {

    pub fn return_with<S: RequiredReturn<F>>(&mut self, services: &mut S)
        -> Result<ReturnComplete, ReturnError<S::Error>> {
        if self.parts.halted { return Err(ReturnError::PriorFailure); }
        self.parts.halted = true;

        let frame = services.checked_out_frame();
        if !frame.origin().matches(&self.parts.identity) {
            return Err(ReturnError::PoolIdentity);
        }
        let expected = frame.value_return_kind();
        let kind = services.admit_return(&self.parts).map_err(ReturnError::Admission)?;
        if kind != expected { return Err(ReturnError::CheckoutKind { expected, admitted: kind }); }
        let mut work = graph::ReturnWork::after_admission(kind);
        let result = {
            let mut bound = Return { pool: &mut self.parts, services };
            loop {
                match graph::advance_return(&mut work, &mut bound, usize::MAX) {
                    Ok(graph::ReturnAdvance::Paused) => {},
                    result => break result,
                }
            }
        };
        self.return_work = Some(work);
        match result {
            Ok(graph::ReturnAdvance::Complete(complete)) => Ok(complete),
            Ok(graph::ReturnAdvance::Paused) => unreachable!("same return work retained through internal pauses"),
            Err(error) => Err(ReturnError::Control(error)),
        }
    }
}
