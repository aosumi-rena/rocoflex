use crate::{
    activation::ArithmeticMode,
    condition_dct,
    condition_parameters::{self, Decoded},
    condition_phase::{self, SourceWordTrig},
    dct_parameters,
    dct_value_binding,
    quantized_gru::GenericVectorActivations,
    source_scalar::SourceF64ValueMath,
    userdata,
};

pub struct Initialization {
    pub phase_math: SourceF64ValueMath,
    pub source_trig_word: u32,
    pub recurrent_arithmetic: ArithmeticMode,
    pub hidden: Vec<f32>,
}

pub struct InitialGenericFloatClear1 {
    pub phase_math: SourceF64ValueMath,
    pub source_trig_word: u32,
    pub recurrent_arithmetic: ArithmeticMode,
}

pub struct Parts<'a> {
    pub parameters: Decoded,
    pub dct_parameters: dct_parameters::Decoded<'a>,
    pub dct_policy: dct_value_binding::Policy,
    pub initialization: Initialization,
}

pub enum Failure<'a> {
    Phase {
        failure: condition_phase::BindingFailure<SourceWordTrig>,
        parameters: Decoded,
        dct_parameters: dct_parameters::Decoded<'a>,
        dct_policy: dct_value_binding::Policy,
        recurrent_arithmetic: ArithmeticMode,
        hidden: Vec<f32>,
    },
    Dct {
        failure: dct_value_binding::Failure<'a>,
        parameters: Decoded,
        phase: condition_phase::Provider<SourceWordTrig>,
        recurrent_arithmetic: ArithmeticMode,
        hidden: Vec<f32>,
    },
    Runtime {
        failure: condition_parameters::Failure<condition_dct::Provider,
            condition_phase::Provider<SourceWordTrig>, GenericVectorActivations>,
        dct_parameters: dct_parameters::Decoded<'a>,
    },
}

pub struct Bound<'a> {

    pub dct_parameters: dct_parameters::Decoded<'a>,
    pub runtime: condition_parameters::Bound<condition_dct::Provider,
        condition_phase::Provider<SourceWordTrig>, GenericVectorActivations>,
}

pub fn bind<'a>(parts: Parts<'a>) -> Result<Bound<'a>, Failure<'a>> {
    let Parts { parameters, dct_parameters, dct_policy,
        initialization: Initialization { phase_math, source_trig_word,
            recurrent_arithmetic, hidden } } = parts;
    let phase = match condition_phase::Provider::from_explicit_math(phase_math,
        SourceWordTrig::from_explicit_source_word(source_trig_word))
    {
        Ok(phase) => phase,
        Err(failure) => return Err(Failure::Phase { failure, parameters,
            dct_parameters, dct_policy, recurrent_arithmetic, hidden }),
    };
    let dct_parameters::Bound { parameters: dct_parameters, provider: dct } =
        match dct_value_binding::bind(dct_value_binding::Parts {
            parameters: dct_parameters, policy: dct_policy })
        {
            Ok(bound) => bound,
            Err(failure) => return Err(Failure::Dct { failure, parameters, phase,
                recurrent_arithmetic, hidden }),
        };
    let activations = GenericVectorActivations::from_explicit_arithmetic(recurrent_arithmetic);
    match condition_parameters::bind_initialized(condition_parameters::Parts {
        parameters, dct, phase, hidden, activations })
    {
        Ok(runtime) => Ok(Bound { dct_parameters, runtime }),
        Err(failure) => Err(Failure::Runtime { failure, dct_parameters }),
    }
}

pub enum UserdataFailure<'a> {
    ConditionDecode {
        cause: userdata::Error,
        shifts: [u32; 6],
        dct_policy: dct_value_binding::Policy,
        initialization: Initialization,
    },
    DctDecode {
        cause: userdata::Error,
        parameters: Decoded,
        dct_policy: dct_value_binding::Policy,
        initialization: Initialization,
    },
    Binding(Failure<'a>),
}

pub fn bind_userdata<'a>(userdata: &userdata::Decoded<'a>, shifts: [u32; 6],
    dct_policy: dct_value_binding::Policy, initialization: Initialization)
    -> Result<Bound<'a>, UserdataFailure<'a>>
{
    let parameters = match userdata.decode_condition_parameters(shifts) {
        Ok(parameters) => parameters,
        Err(cause) => return Err(UserdataFailure::ConditionDecode {
            cause, shifts, dct_policy, initialization }),
    };
    let dct_parameters = match userdata.decode_dct_parameters() {
        Ok(parameters) => parameters,
        Err(cause) => return Err(UserdataFailure::DctDecode {
            cause, parameters, dct_policy, initialization }),
    };
    bind(Parts { parameters, dct_parameters, dct_policy, initialization })
        .map_err(UserdataFailure::Binding)
}

pub enum InitialUserdataFailure<'a> {
    ConditionDecode {
        cause: userdata::Error,
        shifts: [u32; 6],
        dct_policy: dct_value_binding::Policy,
        initialization: InitialGenericFloatClear1,
    },
    DctDecode {
        cause: userdata::Error,
        parameters: Decoded,
        dct_policy: dct_value_binding::Policy,
        initialization: InitialGenericFloatClear1,
    },
    HiddenAllocation {
        cause: crate::value_row_allocation::LogicalError,
        parameters: Decoded,
        dct_parameters: dct_parameters::Decoded<'a>,
        dct_policy: dct_value_binding::Policy,
        initialization: InitialGenericFloatClear1,
    },
    Binding(Failure<'a>),
}

pub fn bind_initial_generic_float_clear1_userdata<'a>(userdata: &userdata::Decoded<'a>,
    shifts: [u32; 6], dct_policy: dct_value_binding::Policy,
    initialization: InitialGenericFloatClear1)
    -> Result<Bound<'a>, InitialUserdataFailure<'a>>
{
    let parameters = match userdata.decode_condition_parameters(shifts) {
        Ok(parameters) => parameters,
        Err(cause) => return Err(InitialUserdataFailure::ConditionDecode {
            cause, shifts, dct_policy, initialization }),
    };
    let dct_parameters = match userdata.decode_dct_parameters() {
        Ok(parameters) => parameters,
        Err(cause) => return Err(InitialUserdataFailure::DctDecode {
            cause, parameters, dct_policy, initialization }),
    };
    let hidden = match crate::value_row_allocation::allocate_logical_row(
        parameters.recurrent().hidden_width(), 0)
    {
        Ok(hidden) => hidden,
        Err(cause) => return Err(InitialUserdataFailure::HiddenAllocation {
            cause, parameters, dct_parameters, dct_policy, initialization }),
    };
    let InitialGenericFloatClear1 { phase_math, source_trig_word, recurrent_arithmetic } = initialization;
    bind(Parts { parameters, dct_parameters, dct_policy,
        initialization: Initialization { phase_math, source_trig_word, recurrent_arithmetic, hidden } })
        .map_err(InitialUserdataFailure::Binding)
}
