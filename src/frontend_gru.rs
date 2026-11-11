use crate::{
    Result,
    gru_parameters::QuantizedGruParameters,
    model::ModelTree,
    modl3_values::{forward_row, RowError},
    quantized_gru::{activate_frontend, new_work},
};

pub struct FrontendGru {
    parameters: QuantizedGruParameters,
}

impl FrontendGru {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let n = root
            .children
            .first()
            .and_then(|n| n.children.get(2))
            .ok_or("frontend GRU missing")?;
        let parameters = QuantizedGruParameters::from_model_node(n, [10, 10, 10, 9, 9, 9])?;
        if parameters.input_width() != 256 || parameters.hidden_width() != 384 {
            return Err("GRU parameter dimensions mismatch".into());
        }
        Ok(Self { parameters })
    }

    pub fn forward256(&self, src: &[f32], state: &mut [f32]) -> Result<Vec<f32>> {
        if src.len() % 256 != 0
            || state.len() != 384
            || src.iter().chain(state.iter()).any(|x| !x.is_finite())
        {
            return Err("GRU requires finite rows x256 and384 state".into());
        }
        let mut out = Vec::new();
        let p = &self.parameters.layers;
        let biases = std::array::from_fn(|index| p[index].bias.as_slice());
        let mut work = new_work(384);
        for row in src.chunks_exact(256) {
            crate::matrix_f32::check_environment()
                .map_err(|_| "GRU row requires RNE/masked/gradual controls".to_owned())?;

            forward_row(row, state, biases, &self.parameters.reset_bias,
                &self.parameters.update_bias, &mut work,
                &mut |index, input, destination| p[index].forward_add_wide(input, destination),
                &mut |kind, values| -> Result<()> { activate_frontend(kind, values); Ok(()) })
                .map_err(|error| match error {
                    RowError::Matrix { reason, .. } | RowError::Activation { reason, .. } => reason,
                    RowError::Width { .. } => "GRU recurrent workspace dimensions differ".to_owned(),
                    RowError::Control { .. } => "GRU recurrent row is stopped".to_owned(),
                })?;
            out.extend_from_slice(state);
        }
        Ok(out)
    }
}
