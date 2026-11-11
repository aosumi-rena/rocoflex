use crate::{
    matrix_policy::{self, BindingError, CoefficientBacking, CoefficientFill, Plan},
    model::ModelNode,
    model_tags::decoded_hashes,
    modl0_projection::{self, Modl0Projection},
    modl0_value_services::{self as values, Calls, LifecyclePolicy, ValueServices},
    reduction::FloatLinear,
    value_row_allocation::{self as allocation, BackingPolicy},
    vector_f32::Backing,
};

pub struct Decoded { linear: FloatLinear }
impl Decoded {
    pub fn input_width(&self) -> usize { self.linear.in_dim }
    pub fn output_width(&self) -> usize { self.linear.out_dim }
    pub fn parameters(&self) -> &FloatLinear { &self.linear }
}

pub fn decode_float_node(node: &ModelNode<'_>) -> crate::Result<Decoded> {
    node.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
    if !node.data.is_empty() || node.children.len() != 2 {
        return Err("float Modl0 requires exactly matrix and additive parameter children".into());
    }
    let matrix = &node.children[0];
    let additive = &node.children[1];
    matrix.require(0x7fca40ff, b"prim0", decoded_hashes("prim0"))?;
    additive.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
    if !matrix.children.is_empty() || !additive.children.is_empty() {
        return Err("float Modl0 parameters must be leaves".into());
    }
    FloatLinear::from_payloads(matrix.data, additive.data).map(|linear| Decoded { linear })
}

pub struct Workspace {
    pub matrix_plan: Plan,
    pub matrix_backing: CoefficientBacking,

    pub additive_post: Backing,
    pub calls: Calls,
    pub lifecycle: LifecyclePolicy,
    pub needs_reset: bool,
}
pub struct Parts {
    pub parameters: Decoded,
    pub workspace: Workspace,
}
pub enum Cause {
    AdditiveWidth { expected: usize, actual: usize },
    Matrix(BindingError),
}
pub struct Failure { pub cause: Cause, pub parts: Parts }

pub struct AllocationWorkspace {
    pub matrix_plan: Plan,
    pub coefficient_fill: CoefficientFill,
    pub additive: BackingPolicy,
    pub calls: Calls,
    pub lifecycle: LifecyclePolicy,
    pub needs_reset: bool,
}
pub struct AllocationParts { pub parameters: Decoded, pub workspace: AllocationWorkspace }
pub enum AllocationCause {
    Matrix(BindingError), Coefficients(matrix_policy::AllocationError), Additive(allocation::Error),
}
pub enum AllocationFailure {
    Preparation { cause: AllocationCause, parts: AllocationParts,
        matrix_backing: Option<CoefficientBacking> },
    Binding(Failure),
}

pub fn bind_allocated(parts: AllocationParts) -> Result<Bound, AllocationFailure> {
    if parts.workspace.matrix_plan.request() != matrix_policy::Request::FloatParameters {
        return Err(AllocationFailure::Preparation {
            cause: AllocationCause::Matrix(BindingError::RequestKind), parts, matrix_backing: None });
    }
    let dimensions = (parts.parameters.output_width(), parts.parameters.input_width());
    if parts.workspace.matrix_plan.dimensions() != dimensions {
        return Err(AllocationFailure::Preparation {
            cause: AllocationCause::Matrix(BindingError::LogicalDimensions), parts, matrix_backing: None });
    }
    let matrix_backing = match parts.workspace.matrix_plan.allocate_backing(parts.workspace.coefficient_fill) {
        Ok(backing) => backing,
        Err(cause) => return Err(AllocationFailure::Preparation {
            cause: AllocationCause::Coefficients(cause), parts, matrix_backing: None }),
    };
    let additive_post = match allocation::allocate_backing(dimensions.0, parts.workspace.additive) {
        Ok(backing) => backing,
        Err(cause) => return Err(AllocationFailure::Preparation {
            cause: AllocationCause::Additive(cause), parts, matrix_backing: Some(matrix_backing) }),
    };
    let AllocationParts { parameters, workspace: AllocationWorkspace {
        matrix_plan, coefficient_fill: _, additive: _, calls, lifecycle, needs_reset } } = parts;
    bind_initialized(Parts { parameters, workspace: Workspace {
        matrix_plan, matrix_backing, additive_post, calls, lifecycle, needs_reset,
    } }).map_err(AllocationFailure::Binding)
}

pub struct Bound {
    pub parameters: Decoded,
    pub projection: Modl0Projection<ValueServices>,
}

pub fn bind_initialized(parts: Parts) -> Result<Bound, Failure> {
    let width = parts.parameters.output_width();
    if parts.workspace.additive_post.logical_width() != width {
        return Err(Failure {
            cause: Cause::AdditiveWidth { expected: width, actual: parts.workspace.additive_post.logical_width() },
            parts,
        });
    }
    let Parts { parameters, workspace: Workspace { matrix_plan, matrix_backing,
        mut additive_post, calls, lifecycle, needs_reset } } = parts;

    let matrix = match parameters.linear.matrix_from_explicit_plan(matrix_plan, matrix_backing) {
        Ok(matrix) => matrix,
        Err(failure) => return Err(Failure {
            cause: Cause::Matrix(failure.error),
            parts: Parts { parameters, workspace: Workspace {
                matrix_plan: failure.plan, matrix_backing: failure.backing,
                additive_post, calls, lifecycle, needs_reset } },
        }),
    };

    for (word, value) in additive_post.full_bits_mut()[..width].iter_mut()
        .zip(parameters.linear.bias())
    {
        *word = value.to_bits();
    }
    let services = ValueServices::from_explicit_parts(values::Parts {
        matrix: crate::modl0_matrix_value::Matrix::Float(matrix), calls,
        additive_post: Some(additive_post), lifecycle,
    });
    Ok(Bound { parameters, projection: Modl0Projection::from_explicit_parts(
        modl0_projection::Parts { services, needs_reset }) })
}
