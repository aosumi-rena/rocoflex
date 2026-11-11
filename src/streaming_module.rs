use crate::{conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    logical_read_rows::ReadRows, streaming_projection::{self, Services, StreamingProjectedConv}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { Forward, Reset }
pub struct Failure<E> {
    pub operation: Operation, pub completed_calls: usize, pub error: streaming_projection::Error<E>,
}
pub struct BindingFailure<S> { pub module: StreamingProjectedConv, pub services: S }
pub enum ValueBindingFailure<M, F, P> {
    Services { controller: StreamingProjectedConv,
        refusal: crate::streaming_value_services::Refusal<M, F, P> },
    Module(BindingFailure<crate::streaming_value_services::ValueServices<M, F, P>>),
}
pub struct Module<S: Services> {
    module: StreamingProjectedConv, services: S, failures: Vec<Failure<S::Error>>,
    latched: bool, reset_failed: bool, inflight: Option<Operation>, completed_calls: usize,
}

impl<M: crate::matrix_value_traits::GenericFirstTap + crate::matrix_value_traits::SourceAccumulatingMatrix,
    F: crate::streaming_value_services::RequiredRows, P: crate::streaming_value_services::RequiredPacked>
    Module<crate::streaming_value_services::ValueServices<M, F, P>>
{

    pub fn from_streaming_value_parts(controller: StreamingProjectedConv,
        parts: crate::streaming_value_services::Parts<M, F, P>)
        -> Result<Self, ValueBindingFailure<M, F, P>>
    {
        let services = match crate::streaming_value_services::ValueServices::from_explicit_parts(
            &controller.state().binding, controller.state().has_bias, parts) {
            Ok(services) => services,
            Err(refusal) => return Err(ValueBindingFailure::Services { controller, refusal }),
        };
        Self::from_explicit_services(controller, services).map_err(ValueBindingFailure::Module)
    }
}

impl<S: Services> Module<S> {
    pub fn from_explicit_services(module: StreamingProjectedConv, services: S)
        -> Result<Self, BindingFailure<S>>
    {
        if module.state().halted || module.state().pending_row.is_some() {
            return Err(BindingFailure { module, services });
        }
        Ok(Self { module, services, failures: Vec::new(), latched: false,
            reset_failed: false, inflight: None, completed_calls: 0 })
    }
    pub fn source(&self) -> &StreamingProjectedConv { &self.module }
    pub fn services(&self) -> &S { &self.services }
    pub fn failures(&self) -> &[Failure<S::Error>] { &self.failures }
    pub fn inflight(&self) -> Option<Operation> { self.inflight }
    pub fn completed_calls(&self) -> usize { self.completed_calls }
    pub fn input_width(&self) -> usize { self.module.state().binding.input_width() }
    pub fn output_width(&self) -> usize { self.module.state().binding.output_width() }

    pub fn into_ready_parts(self) -> Result<(StreamingProjectedConv, S, Vec<Failure<S::Error>>, usize), Self> {
        if self.latched || self.inflight.is_some() || self.module.state().halted { return Err(self); }
        Ok((self.module, self.services, self.failures, self.completed_calls))
    }
    fn forward_rows(&mut self, input: &dyn ReadRows) -> Result<Batch, GateError> {
        if self.latched || self.inflight.is_some() { return Err(GateError::ResetRequired); }
        self.inflight = Some(Operation::Forward);
        let result = self.module.forward_borrowed(input, &mut self.services);
        self.inflight = None;
        match result {
            Ok(output) => { self.completed_calls += 1; Ok(output) }
            Err(error) => {
                self.latched = true;
                self.failures.push(Failure { operation: Operation::Forward, completed_calls: self.completed_calls, error });
                Err(GateError::Dependency("source streaming projection stopped"))
            }
        }
    }
}
impl<S: Services> Projection for Module<S> {
    fn reset(&mut self) -> Result<(), GateError> {

        if self.reset_failed || self.inflight.is_some() { return Err(GateError::ResetRequired); }
        self.inflight = Some(Operation::Reset);
        let result = self.module.reset(&mut self.services);
        self.inflight = None;
        match result {
            Ok(()) => { self.latched = false; Ok(()) }
            Err(error) => {
                self.latched = true; self.reset_failed = true;
                self.failures.push(Failure { operation: Operation::Reset, completed_calls: self.completed_calls, error });
                Err(GateError::Dependency("source streaming projection reset stopped"))
            }
        }
    }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> {
        self.forward_rows(input)
    }
}
impl<S: Services> BorrowedProjection for Module<S> {
    fn forward_borrowed(&mut self, input: &dyn ReadRows) -> Result<Batch, GateError> { self.forward_rows(input) }
}
