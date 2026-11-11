use std::collections::VecDeque;
use crate::{conditional_gate::Batch, ltv_filter, ltv_stream,
    model2_operator_binding::{Ltvfs1Config13, OperatorBindingError, SourceLtvfs1Binding},
    userdata::{self, Role}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity { pub decoded_tag: u64, pub outer_class: u32, pub inner_class: u32 }

pub struct Outer<M, F> {
    pub frame: i32,
    pub ended_status: u8,
    pub signal_fifo: VecDeque<Option<Vec<f32>>>,
    pub coefficient_output: Batch,
    pub pair_signal: Batch,
    pub math: M,
    pub fft: F,
}
pub struct Parts<M, F, K> { pub identity: Identity, pub outer: Outer<M, F>, pub kernel: K }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError { UnsupportedRole(Role), MissingConfiguration(Role) }

pub struct Bound<M, F, K> {
    pub role: Role,
    pub configuration: Ltvfs1Config13,
    pub state: ltv_filter::State<M, F, K>,
}
pub enum Failure<M, F, K> {
    Selection { cause: SelectionError, parts: Parts<M, F, K> },
    Identity { cause: OperatorBindingError, role: Role, configuration: Ltvfs1Config13,
        parts: Parts<M, F, K> },
    Filter { role: Role, configuration: Ltvfs1Config13, identity: Identity,
        failure: ltv_filter::BindingFailure<M, F, K> },
}
pub enum StreamFailure<M, F, O: ltv_stream::Outputs> {
    Selection { cause: SelectionError, parts: Parts<M, F, ltv_stream::Parts<O>> },
    Identity { cause: OperatorBindingError, role: Role, configuration: Ltvfs1Config13,
        parts: Parts<M, F, ltv_stream::Parts<O>> },
    Kernel { role: Role, configuration: Ltvfs1Config13, identity: Identity,
        outer: Outer<M, F>, failure: ltv_stream::BindingFailure<O> },
    Filter(Failure<M, F, ltv_stream::Kernel<O>>),
}

fn configuration(primary: &userdata::Primary, role: Role) -> Result<Ltvfs1Config13, SelectionError> {
    if !matches!(role, Role::Filter136 | Role::Filter144 | Role::ExtraFilter) {
        return Err(SelectionError::UnsupportedRole(role));
    }
    primary.filter(role).ok_or(SelectionError::MissingConfiguration(role))
}
fn bind_identity(identity: Identity, configuration: Ltvfs1Config13)
    -> Result<SourceLtvfs1Binding, OperatorBindingError>
{
    SourceLtvfs1Binding::bind(identity.decoded_tag, identity.outer_class, identity.inner_class, configuration)
}
fn finish<M, F, K: ltv_filter::PairedKernel>(role: Role, configuration: Ltvfs1Config13,
    binding: SourceLtvfs1Binding, parts: Parts<M, F, K>) -> Result<Bound<M, F, K>, Failure<M, F, K>>
{
    let Parts { identity, outer: Outer { frame, ended_status, signal_fifo,
        coefficient_output, pair_signal, math, fft }, kernel } = parts;
    match ltv_filter::State::from_explicit_parts(ltv_filter::Parts {
        binding, frame, ended_status, signal_fifo, coefficient_output, pair_signal, math, fft, kernel,
    }) {
        Ok(state) => Ok(Bound { role, configuration, state }),
        Err(failure) => Err(Failure::Filter { role, configuration, identity, failure }),
    }
}

pub fn bind_initialized_userdata<M, F, K: ltv_filter::PairedKernel>(userdata: &userdata::Decoded<'_>,
    role: Role, parts: Parts<M, F, K>) -> Result<Bound<M, F, K>, Failure<M, F, K>>
{
    let configuration = match configuration(userdata.primary(), role) {
        Ok(configuration) => configuration,
        Err(cause) => return Err(Failure::Selection { cause, parts }),
    };
    let binding = match bind_identity(parts.identity, configuration) {
        Ok(binding) => binding,
        Err(cause) => return Err(Failure::Identity { cause, role, configuration, parts }),
    };
    finish(role, configuration, binding, parts)
}

pub fn bind_stream_userdata<M, F, O: ltv_stream::Outputs>(userdata: &userdata::Decoded<'_>,
    role: Role, parts: Parts<M, F, ltv_stream::Parts<O>>)
    -> Result<Bound<M, F, ltv_stream::Kernel<O>>, StreamFailure<M, F, O>>
{
    let configuration = match configuration(userdata.primary(), role) {
        Ok(configuration) => configuration,
        Err(cause) => return Err(StreamFailure::Selection { cause, parts }),
    };
    let binding = match bind_identity(parts.identity, configuration) {
        Ok(binding) => binding,
        Err(cause) => return Err(StreamFailure::Identity { cause, role, configuration, parts }),
    };
    bind_admitted_stream(role, binding, parts)
}

pub(crate) fn bind_admitted_stream<M, F, O: ltv_stream::Outputs>(role: Role,
    binding: SourceLtvfs1Binding, parts: Parts<M, F, ltv_stream::Parts<O>>)
    -> Result<Bound<M, F, ltv_stream::Kernel<O>>, StreamFailure<M, F, O>>
{
    let configuration = binding.config();
    let Parts { identity, outer, kernel } = parts;
    let kernel = match ltv_stream::Kernel::from_explicit_parts(kernel) {
        Ok(kernel) => kernel,
        Err(failure) => return Err(StreamFailure::Kernel { role, configuration, identity, outer, failure }),
    };
    finish(role, configuration, binding, Parts { identity, outer, kernel }).map_err(StreamFailure::Filter)
}
