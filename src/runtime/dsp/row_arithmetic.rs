use crate::conditional_gate::{GateError, VectorKernels};

pub struct FloatVectorGateKernels {
    pub first_half_activation: fn(f32) -> f32,
    pub second_half_activation: fn(f32) -> f32,
}

impl VectorKernels for FloatVectorGateKernels {
    fn slot88(&mut self, condition: &[f32], gate: &mut [f32]) -> Result<(), GateError> {
        add_condition(condition, gate)
    }

    fn split_activate_combine(&mut self, gate: &[f32], half_width: usize)
        -> Result<Vec<f32>, GateError>
    {
        split_product(gate, half_width, self.first_half_activation, self.second_half_activation)
    }
}

pub fn add_condition(condition: &[f32], gate: &mut [f32]) -> Result<(), GateError> {
    if condition.len() != gate.len() {
        return Err(GateError::RowWidth { expected:gate.len(), actual:condition.len() });
    }
    for (destination, &source) in gate.iter_mut().zip(condition) { *destination += source; }
    Ok(())
}

pub fn split_product(gate: &[f32], half_width: usize,
                     first_activation: impl Fn(f32)->f32, second_activation: impl Fn(f32)->f32)
    -> Result<Vec<f32>, GateError>
{
    if gate.len() != 2 * half_width {
        return Err(GateError::RowWidth { expected:2*half_width, actual:gate.len() });
    }
    let (first, second) = gate.split_at(half_width);
    Ok(first.iter().zip(second).map(|(&left,&right)| {
        first_activation(left) * second_activation(right)
    }).collect())
}
