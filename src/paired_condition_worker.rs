use crate::{conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    conditional_ncwn::{BorrowedConditionedTransform, ConditionalNcwn, ConditionedTransform},
    paired_condition::{Borrowed176, Worker}, module_output::Publication};

pub enum Failure {
    Network(GateError), RetirementHistory(std::collections::TryReserveError),
}

pub struct NcwnWorker<G, S, R, H> {
    module: ConditionalNcwn<G, S, R, H>, failure: Option<Failure>, completed_calls: usize,
    publication: Publication,
}
impl<G, S, R, H> NcwnWorker<G, S, R, H> {
    pub fn from_explicit_module(module: ConditionalNcwn<G, S, R, H>) -> Self {
        Self { module, failure: None, completed_calls: 0, publication: Publication::new() }
    }
    pub fn module(&self) -> &ConditionalNcwn<G, S, R, H> { &self.module }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
    pub fn completed_calls(&self) -> usize { self.completed_calls }

    pub fn retired_output(&self) -> &[Batch] { self.publication.retired() }
    pub fn pending_retirement(&self) -> Option<&Batch> { self.publication.pending() }

    pub fn into_ready_module(self) -> Result<(ConditionalNcwn<G, S, R, H>, Vec<Batch>, usize), Self> {
        if self.failure.is_some() || self.publication.pending().is_some() { return Err(self); }
        let (retired, _) = self.publication.into_parts();
        Ok((self.module, retired, self.completed_calls))
    }
}
impl<G: ConditionedTransform + BorrowedConditionedTransform, S: Projection,
    R: Projection + BorrowedProjection, H: Projection> Worker for NcwnWorker<G, S, R, H> {
    type Error = GateError;
    fn forward176(&mut self, input: Borrowed176<'_>, conditions208: &mut Batch,
        retained240: &mut Batch) -> Result<(), Self::Error> {
        if self.failure.is_some() { return Err(GateError::ResetRequired); }
        let result = self.module.forward_borrowed(&input, conditions208);
        let output = match result {
            Ok(output) => output,
            Err(error) => { self.failure = Some(Failure::Network(error.clone())); return Err(error); }
        };

        if let Err(error) = self.publication.publish(retained240, output) {
            self.failure = Some(Failure::RetirementHistory(error));
            return Err(GateError::Dependency("Ncwn output retirement history stopped"));
        }
        self.completed_calls += 1; Ok(())
    }
}
impl<G: ConditionedTransform, S: Projection, R: Projection, H: Projection> NcwnWorker<G, S, R, H> {

    pub fn reset_for_constructor(&mut self) -> Result<(), GateError> {
        if self.failure.is_some() || self.publication.pending().is_some() {
            return Err(GateError::ResetRequired);
        }
        match self.module.reset() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(Failure::Network(error.clone()));
                Err(error)
            }
        }
    }
}
