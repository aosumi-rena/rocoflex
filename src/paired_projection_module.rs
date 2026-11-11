use crate::{conditional_gate::{Batch, GateError, Projection}, module_output::Publication, paired_post::Module};
pub enum Failure { Projection(GateError), RetirementHistory(std::collections::TryReserveError), CounterOverflow }
pub struct ProjectionModule<P> {
    projection: P, publication: Publication, completed_calls: usize, failure: Option<Failure>,
}
impl<P> ProjectionModule<P> {

    pub fn from_explicit_projection(projection: P) -> Self {
        Self { projection, publication: Publication::new(), completed_calls: 0, failure: None }
    }
    pub fn projection(&self) -> &P { &self.projection }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
    pub fn completed_calls(&self) -> usize { self.completed_calls }
    pub fn retired_output(&self) -> &[Batch] { self.publication.retired() }
    pub fn pending_retirement(&self) -> Option<&Batch> { self.publication.pending() }

    pub fn into_ready_projection(self) -> Result<(P, Vec<Batch>, usize), Self> {
        if self.failure.is_some() || self.publication.pending().is_some() { return Err(self); }
        let (retired, _) = self.publication.into_parts();
        Ok((self.projection, retired, self.completed_calls))
    }
}
impl<S: crate::streaming_projection::Services> ProjectionModule<crate::streaming_module::Module<S>> {

    pub fn from_streaming_parts(module: crate::streaming_projection::StreamingProjectedConv, services: S)
        -> Result<Self, crate::streaming_module::BindingFailure<S>> {
        crate::streaming_module::Module::from_explicit_services(module, services)
            .map(Self::from_explicit_projection)
    }
}
impl<P: Projection> Module for ProjectionModule<P> {
    type Error = GateError;
    fn forward(&mut self, input: &mut Batch, output: &mut Batch) -> Result<(), Self::Error> {
        if self.failure.is_some() { return Err(GateError::ResetRequired); }
        let Some(next_calls) = self.completed_calls.checked_add(1) else {
            self.failure = Some(Failure::CounterOverflow);
            return Err(GateError::Dependency("projection adapter counter overflow"));
        };

        let completed = match self.projection.forward(input) {
            Ok(completed) => completed,
            Err(error) => {
                self.failure = Some(Failure::Projection(error.clone()));
                return Err(error);
            }
        };
        if let Err(error) = self.publication.publish(output, completed) {
            self.failure = Some(Failure::RetirementHistory(error));
            return Err(GateError::Dependency("projection output retirement history stopped"));
        }
        self.completed_calls = next_calls; Ok(())
    }
}
impl<P: Projection> ProjectionModule<P> {

    pub fn reset_for_constructor(&mut self) -> Result<(), GateError> {
        if self.failure.is_some() || self.publication.pending().is_some() {
            return Err(GateError::ResetRequired);
        }
        match self.projection.reset() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(Failure::Projection(error.clone()));
                Err(error)
            }
        }
    }
}
