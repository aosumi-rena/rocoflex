use crate::{Result, model::ModelNode, model_tags::decoded_hashes,
    timbre::QuantizedLinear};

pub struct QuantizedGruParameters {
    pub(crate) layers: [QuantizedLinear; 6],
    pub(crate) reset_bias: Vec<f32>,
    pub(crate) update_bias: Vec<f32>,
    input_width: usize,
    hidden_width: usize,
    shifts: [u32; 6],
}

impl QuantizedGruParameters {

    pub fn from_model_node(node: &ModelNode<'_>, shifts: [u32; 6]) -> Result<Self> {
        node.require(0x7fca41ff, b"modl3", decoded_hashes("modl3"))?;
        if !node.data.is_empty() || node.children.len() != 12 {
            return Err("GRU requires twelve alternating parameter leaves".into());
        }
        let mut layers = Vec::with_capacity(6);
        for (index, shift) in shifts.iter().copied().enumerate() {
            let matrix = &node.children[2 * index];
            let bias = &node.children[2 * index + 1];
            matrix.require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
            bias.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !matrix.children.is_empty() || !bias.children.is_empty() {
                return Err("GRU parameters must be leaves".into());
            }
            layers.push(QuantizedLinear::from_payloads(matrix.data, bias.data, shift)?);
        }
        let layers: [QuantizedLinear; 6] = layers.try_into()
            .map_err(|_| "GRU parameter count".to_owned())?;
        let input_width = layers[0].in_dim;
        let hidden_width = layers[0].out_dim;
        if layers.iter().enumerate().any(|(index, matrix)| {
            matrix.out_dim != hidden_width || matrix.in_dim !=
                if index < 3 { input_width } else { hidden_width }
        }) {
            return Err("GRU input/recurrent matrix dimensions differ".into());
        }

        crate::matrix_f32::check_environment()
            .map_err(|_| "GRU bias preparation requires RNE/masked/gradual controls".to_owned())?;
        let reset_bias: Vec<f32> = layers[0].bias.iter().zip(&layers[3].bias)
            .map(|(left, right)| *left + *right).collect();
        let update_bias: Vec<f32> = layers[1].bias.iter().zip(&layers[4].bias)
            .map(|(left, right)| *left + *right).collect();
        if reset_bias.iter().chain(&update_bias).any(|value| !value.is_finite()) {
            return Err("GRU bias sums exceed the finite backend".into());
        }
        Ok(Self { layers, reset_bias, update_bias, input_width, hidden_width, shifts })
    }

    pub fn input_width(&self) -> usize { self.input_width }
    pub fn hidden_width(&self) -> usize { self.hidden_width }
    pub fn shifts(&self) -> [u32; 6] { self.shifts }
    pub fn matrices(&self) -> &[QuantizedLinear; 6] { &self.layers }
    pub fn reset_bias(&self) -> &[f32] { &self.reset_bias }
    pub fn update_bias(&self) -> &[f32] { &self.update_bias }
}
