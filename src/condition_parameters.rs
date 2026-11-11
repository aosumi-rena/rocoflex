use crate::{
    condition::{self, Condition, Config, Dct, PhaseServices},
    gru_parameters::QuantizedGruParameters,
    model::ModelNode,
    quantized_gru::{self, Activations, QuantizedGru},
};

pub struct Decoded {
    config: Config,
    frequencies: Vec<f32>,
    recurrent: QuantizedGruParameters,
}
impl Decoded {
    pub fn configuration(&self) -> Config { self.config }
    pub fn frequencies(&self) -> &[f32] { &self.frequencies }
    pub fn recurrent(&self) -> &QuantizedGruParameters { &self.recurrent }
}

pub fn decode_selected(configuration: &[u8], frequencies: &ModelNode<'_>,
    recurrent: &ModelNode<'_>, shifts: [u32; 6]) -> crate::Result<Decoded>
{
    let config = Config::parse(configuration).ok_or("invalid conditioning configuration")?;
    let width = usize::try_from(crate::timbre::u32_at(frequencies.data, 0)?)
        .map_err(|_| "conditioning frequency count exceeds this target")?;
    let frequencies = crate::bank::vector(frequencies, width)?;
    let recurrent = QuantizedGruParameters::from_model_node(recurrent, shifts)?;
    Ok(Decoded { config, frequencies, recurrent })
}

pub struct Parts<D, P, A> {
    pub parameters: Decoded,
    pub dct: D,
    pub phase: P,
    pub hidden: Vec<f32>,
    pub activations: A,
}
pub enum Failure<D, P, A> {
    Condition {
        failure: condition::BindingFailure<D, P>,
        recurrent: QuantizedGruParameters,
        hidden: Vec<f32>,
        activations: A,
    },
    RecurrentWidth {
        condition: Condition<D, P>,
        recurrent: QuantizedGruParameters,
        hidden: Vec<f32>,
        activations: A,
    },
    Recurrent {
        condition: Condition<D, P>,
        failure: quantized_gru::BindingFailure<A>,
    },
}
pub struct Bound<D, P, A> {
    pub condition: Condition<D, P>,
    pub recurrent: QuantizedGru<A>,
}

pub fn bind_initialized<D: Dct, P: PhaseServices, A: Activations>(parts: Parts<D, P, A>)
    -> Result<Bound<D, P, A>, Failure<D, P, A>>
{
    let Parts { parameters: Decoded { config, frequencies, recurrent },
        dct, phase, hidden, activations } = parts;
    let condition = match Condition::from_explicit_services(config, frequencies, dct, phase) {
        Ok(condition) => condition,
        Err(failure) => return Err(Failure::Condition { failure, recurrent, hidden, activations }),
    };
    if condition.output_width() != recurrent.input_width() {
        return Err(Failure::RecurrentWidth { condition, recurrent, hidden, activations });
    }
    match QuantizedGru::from_explicit_state(recurrent, hidden, activations) {
        Ok(recurrent) => Ok(Bound { condition, recurrent }),
        Err(failure) => Err(Failure::Recurrent { condition, failure }),
    }
}
