use crate::{conditional_gate::Batch,
    ltv_coefficient_math::{Provider, ScalarUpper},
    paired_adopt::pipeline::call::Continuation,
    paired_features::StandardGaussian,
    paired_frames_pool::{self as frames, AbsentFallback, FramesPool},
    paired_value_rows::{CurrentRows, CurrentSource, InputRows, InterpolationStorage},
    streaming_rows::ValueRows, vocoder_userdata_binding as userdata,
    vocoder_value_binding as paired};

pub struct RuntimeInputs<U: ScalarUpper> {
    pub disabled_cursor512: i32,
    pub math: Provider<U>, pub gaussian: StandardGaussian,
    pub scratch: paired::Scratch, pub packing_output: Batch,
    pub continuation: Continuation,
}
pub struct Inputs<U: ScalarUpper> {
    pub input_factory: ValueRows,
    pub interpolation_factory: ValueRows,
    pub current: CurrentSource,

    pub frames: frames::Parts<AbsentFallback>,
    pub runtime: RuntimeInputs<U>,
}

pub enum Failure<U: ScalarUpper> {
    InputFactory { inputs: Inputs<U> },
    InterpolationFactory {
        input_rows: InputRows, interpolation_factory: ValueRows,
        current: CurrentSource, frames: frames::Parts<AbsentFallback>, runtime: RuntimeInputs<U>,
    },
    Frames {
        input_rows: InputRows, current_rows: CurrentRows,
        interpolation_storage: InterpolationStorage,
        failure: frames::BindingFailure<AbsentFallback>, runtime: RuntimeInputs<U>,
    },
}

pub fn bind_rated_runtime<U: ScalarUpper>(inputs: Inputs<U>)
    -> Result<userdata::RatedRuntime<InputRows, CurrentRows, InterpolationStorage,
        U, FramesPool<AbsentFallback>>, Failure<U>>
{
    let Inputs { input_factory, interpolation_factory, current, frames, runtime } = inputs;
    let input_rows = match InputRows::from_explicit_factory(input_factory) {
        Ok(rows) => rows,
        Err(input_factory) => return Err(Failure::InputFactory { inputs: Inputs {
            input_factory, interpolation_factory, current, frames, runtime,
        } }),
    };
    let interpolation_storage = match InterpolationStorage::from_explicit_factory(interpolation_factory) {
        Ok(storage) => storage,
        Err(interpolation_factory) => return Err(Failure::InterpolationFactory {
            input_rows, interpolation_factory, current, frames, runtime,
        }),
    };
    let current_rows = CurrentRows::from_explicit_source(current);
    let frames = match FramesPool::from_explicit_parts(frames) {
        Ok(pool) => pool,
        Err(failure) => return Err(Failure::Frames {
            input_rows, current_rows, interpolation_storage, failure, runtime,
        }),
    };
    let RuntimeInputs { disabled_cursor512, math, gaussian, scratch, packing_output, continuation } = runtime;
    Ok(userdata::RatedRuntime { disabled_cursor512,
        providers: paired::Providers { input_rows, current_rows, interpolation_storage, math, frames, gaussian },
        scratch, packing_output, continuation,
    })
}
