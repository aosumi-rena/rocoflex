use crate::{
    model::ModelNode,
    model_tags::decoded_hashes,
    modl0_matrix_value::Matrix,
    modl0_projection::{self, Modl0Projection},
    modl0_value_services::{self as values, Calls, LifecyclePolicy, ValueServices},
    quantized_matrix::{self, OverwriteFold, QuantizedMatrix},
    value_row_allocation::{self as allocation, BackingPolicy},
    vector_f32::Backing,
};

pub struct Decoded { matrix: QuantizedMatrix, bias: Vec<f32> }
impl Decoded {
    pub fn matrix(&self) -> &QuantizedMatrix { &self.matrix }
    pub fn bias(&self) -> &[f32] { &self.bias }
    pub fn from_owned_linear(linear: crate::timbre::QuantizedLinear)
        -> Result<Self, (crate::timbre::QuantizedParameterOwnerError, crate::timbre::QuantizedLinear)>
    {
        linear.into_parameter_owners().map(|(matrix, bias)| Self { matrix, bias })
    }
}

pub fn decode_quantized_node(node: &ModelNode<'_>, shift: u32) -> crate::Result<Decoded> {
    node.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
    if !node.data.is_empty() || node.children.len() != 2 {
        return Err("quantized Modl0 requires matrix and additive parameter children".into());
    }
    let matrix = &node.children[0];
    let bias = &node.children[1];
    matrix.require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
    bias.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
    if !matrix.children.is_empty() || !bias.children.is_empty() {
        return Err("quantized Modl0 parameters must be leaves".into());
    }
    let (matrix, bias) = quantized_matrix::parse_matrix_and_bias(matrix.data, Some(bias.data), shift)?;
    Ok(Decoded { matrix, bias: bias.expect("supplied bias validated by shared parser") })
}

pub struct Workspace {
    pub additive_post: Backing,
    pub calls: Calls,
    pub overwrite_fold: OverwriteFold,
    pub lifecycle: LifecyclePolicy,
    pub needs_reset: bool,
}
pub struct Parts { pub parameters: Decoded, pub workspace: Workspace }
pub struct Failure { pub expected_width: usize, pub actual_width: usize, pub parts: Parts }
pub struct AllocationParts {
    pub parameters: Decoded,
    pub additive: BackingPolicy,
    pub calls: Calls,
    pub overwrite_fold: OverwriteFold,
    pub lifecycle: LifecyclePolicy,
    pub needs_reset: bool,
}
pub enum AllocationFailure {
    Preparation { cause: allocation::Error, parts: AllocationParts },
    Binding(Failure),
}

pub fn bind_allocated(parts: AllocationParts) -> Result<Bound, AllocationFailure> {
    let additive_post = match allocation::allocate_backing(parts.parameters.matrix.output_width(), parts.additive) {
        Ok(backing) => backing,
        Err(cause) => return Err(AllocationFailure::Preparation { cause, parts }),
    };
    let AllocationParts { parameters, additive: _, calls, overwrite_fold, lifecycle, needs_reset } = parts;
    bind_initialized(Parts { parameters, workspace: Workspace {
        additive_post, calls, overwrite_fold, lifecycle, needs_reset,
    } }).map_err(AllocationFailure::Binding)
}
pub struct Bound {

    pub logical_bias: Vec<f32>,
    pub projection: Modl0Projection<ValueServices>,
}

pub fn bind_initialized(parts: Parts) -> Result<Bound, Failure> {
    let width = parts.parameters.matrix.output_width();
    let actual_width = parts.workspace.additive_post.logical_width();
    if actual_width != width {
        return Err(Failure { expected_width: width, actual_width, parts });
    }
    let Parts { parameters: Decoded { matrix, bias }, workspace: Workspace {
        mut additive_post, calls, overwrite_fold, lifecycle, needs_reset,
    } } = parts;
    for (word, value) in additive_post.full_bits_mut()[..width].iter_mut().zip(&bias) {
        *word = value.to_bits();
    }
    let services = ValueServices::from_explicit_parts(values::Parts {
        matrix: Matrix::Quantized { matrix, overwrite_fold },
        additive_post: Some(additive_post), calls, lifecycle,
    });
    Ok(Bound { logical_bias: bias, projection: Modl0Projection::from_explicit_parts(
        modl0_projection::Parts { services, needs_reset }) })
}
