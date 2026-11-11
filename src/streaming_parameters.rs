use std::collections::{TryReserveError, VecDeque};
use crate::{backend_factory::SourceFloatFactory,
    matrix_value_traits::{GenericFirstTap, SourceAccumulatingMatrix},
    projected_conv_binding::{ConvBindingError, MatrixBinding, ProjectedConvBinding,
        ProjectedConvConfig, SharedMatrixProfile},
    streaming_module::{self, ValueBindingFailure},
    streaming_projection::{self, Output, Phase, Row, StreamingProjectedConv},
    streaming_value_services::{self as values, RequiredPacked, RequiredRows, ValuePool, ValueServices}};

pub enum Child<M> {
    Matrix { value: M, compatible_profile: Option<SharedMatrixProfile> },
    Bias(Vec<f32>),
}
pub struct Workspace {
    pub history: VecDeque<Option<Row>>, pub flags224: u8, pub output: Output,
    pub pending_row: Option<Row>, pub phase: Phase, pub halted: bool,
}
pub struct BindingInputs {
    pub vtable_rva: u32, pub configuration: Vec<u8>, pub factory: SourceFloatFactory,
}
pub enum InitialState {
    Initialized { workspace: Workspace, input_pool: ValuePool, output_pool: ValuePool },

    Constructor { flags224: u8 },
}
impl InitialState {
    fn flags224(&self) -> u8 {
        match self { Self::Initialized { workspace, .. } => workspace.flags224,
            Self::Constructor { flags224 } => *flags224 }
    }
}
pub struct Parts<M, F, P> {
    pub inputs: BindingInputs, pub children: Vec<Child<M>>, pub state: InitialState,
    pub rows: F, pub packed: Option<P>,
}
pub enum Cause {
    Configuration(ConvBindingError), DuplicateBias { child: usize },
    Allocation(TryReserveError), Binding(ConvBindingError), ConstructorFlags,
}
pub enum Failure<M: GenericFirstTap + SourceAccumulatingMatrix, F: RequiredRows, P: RequiredPacked> {
    Preparation { cause: Cause, parts: Parts<M, F, P> },
    Controller { refusal: streaming_projection::RefusedState, services: values::Parts<M, F, P>,
        inputs: BindingInputs },
    Module { failure: ValueBindingFailure<M, F, P>, inputs: BindingInputs },
    Initialization { module: streaming_module::Module<ValueServices<M, F, P>>,
        inputs: BindingInputs, error: crate::conditional_gate::GateError },
}
pub struct Bound<M: GenericFirstTap + SourceAccumulatingMatrix, F: RequiredRows, P: RequiredPacked> {
    pub module: streaming_module::Module<ValueServices<M, F, P>>,
    pub inputs: BindingInputs,
}

pub fn bind<M: GenericFirstTap + SourceAccumulatingMatrix, F: RequiredRows, P: RequiredPacked>(
    parts: Parts<M, F, P>) -> Result<Bound<M, F, P>, Failure<M, F, P>> {
    let config = match ProjectedConvConfig::parse(&parts.inputs.configuration) {
        Ok(config) => config,
        Err(error) => return Err(Failure::Preparation { cause: Cause::Configuration(error), parts }),
    };
    if matches!(&parts.state, InitialState::Constructor { flags224 } if *flags224 & 3 != 0) {
        return Err(Failure::Preparation { cause: Cause::ConstructorFlags, parts });
    }
    let mut count = 0;
    let mut bias = false;
    for (child, value) in parts.children.iter().enumerate() {
        match value {
            Child::Matrix { .. } => count += 1,
            Child::Bias(_) => {
                if bias { return Err(Failure::Preparation { cause: Cause::DuplicateBias { child }, parts }); }
                bias = true;
            }
        }
    }
    let mut metadata = Vec::new();
    if let Err(error) = metadata.try_reserve_exact(count) {
        return Err(Failure::Preparation { cause: Cause::Allocation(error), parts });
    }
    for child in &parts.children {
        if let Child::Matrix { value, compatible_profile } = child {
            metadata.push(MatrixBinding { input_width: value.input_width(), output_width: value.output_width(),
                compatible_profile: *compatible_profile });
        }
    }

    let binding = match ProjectedConvBinding::bind(parts.inputs.vtable_rva, config, &metadata,
        parts.inputs.factory, parts.state.flags224() & 2 != 0) {
        Ok(binding) => binding,
        Err(error) => return Err(Failure::Preparation { cause: Cause::Binding(error), parts }),
    };
    let mut matrices = Vec::new();
    if let Err(error) = matrices.try_reserve_exact(count) {
        return Err(Failure::Preparation { cause: Cause::Allocation(error), parts });
    }
    let Parts { inputs, children, state, rows, packed } = parts;
    let mut actual_bias = None;
    for child in children {
        match child {
            Child::Matrix { value, .. } => matrices.push(value),
            Child::Bias(value) => actual_bias = Some(value),
        }
    }
    let (workspace, input_pool, output_pool, before_reset) = match state {
        InitialState::Initialized { workspace, input_pool, output_pool } =>
            (workspace, Some(input_pool), Some(output_pool), false),
        InitialState::Constructor { flags224 } => (Workspace { history: VecDeque::new(), flags224,
            output: Output { rows: Vec::new(), status: 0 }, pending_row: None,
            phase: Phase::Ready, halted: false }, None, None, true),
    };
    let services = values::Parts { input_pool, output_pool, matrices, bias: actual_bias, rows, packed };
    let Workspace { history, flags224, output, pending_row, phase, halted } = workspace;
    let state = streaming_projection::State { binding, history, flags224, has_bias: services.bias.is_some(),
        output, pending_row, phase, halted };
    let controller = match StreamingProjectedConv::from_explicit_state(state) {
        Ok(controller) => controller,
        Err(refusal) => return Err(Failure::Controller { refusal, services, inputs }),
    };
    let admitted = if before_reset {
        match ValueServices::from_before_reset_parts(&controller.state().binding, controller.state().has_bias, services) {
            Ok(services) => streaming_module::Module::from_explicit_services(controller, services)
                .map_err(ValueBindingFailure::Module),
            Err(refusal) => Err(ValueBindingFailure::Services { controller, refusal }),
        }
    } else { streaming_module::Module::from_streaming_value_parts(controller, services) };
    let mut module = match admitted {
        Ok(module) => module,
        Err(failure) => return Err(Failure::Module { failure, inputs }),
    };
    if before_reset {
        if let Err(error) = crate::conditional_gate::Projection::reset(&mut module) {
            return Err(Failure::Initialization { module, inputs, error });
        }
    }
    Ok(Bound { module, inputs })
}
