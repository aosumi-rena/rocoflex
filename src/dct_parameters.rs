use crate::{
    condition_dct,
    dct::{self, Dct},
    matrix_policy::{self, BoundMatrix, CoefficientBacking, ParameterPayload, PayloadError, Plan, Request},
    vector_f32::Backing,
};

pub struct Decoded<'a> { payloads: [ParameterPayload<'a>; 2] }
impl<'a> Decoded<'a> {
    pub fn payloads(&self) -> &[ParameterPayload<'a>; 2] { &self.payloads }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError { pub matrix: usize, pub error: PayloadError }

pub fn decode_selected<'a>(first: &'a [u8], second: &'a [u8]) -> Result<Decoded<'a>, DecodeError> {
    let first = ParameterPayload::from_bytes(first).map_err(|error| DecodeError { matrix: 0, error })?;
    let second = ParameterPayload::from_bytes(second).map_err(|error| DecodeError { matrix: 1, error })?;
    Ok(Decoded { payloads: [first, second] })
}

pub struct MatrixStorage { pub plan: Plan, pub backing: CoefficientBacking }
pub struct Parts<'a> {
    pub parameters: Decoded<'a>,
    pub matrices: [MatrixStorage; 2],
    pub output: Backing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Request { matrix: usize },
    Dimensions { matrix: usize },
    OutputWidth { expected: usize, actual: usize },
}
pub enum Failure<'a> {
    Admission { reason: Admission, parts: Parts<'a> },
    FirstMatrix {
        failure: matrix_policy::BindingFailure,
        parameters: Decoded<'a>,
        second: MatrixStorage,
        output: Backing,
    },
    SecondMatrix {
        first: BoundMatrix,
        failure: matrix_policy::BindingFailure,
        parameters: Decoded<'a>,
        output: Backing,
    },
    Dct { failure: dct::BindingFailure, parameters: Decoded<'a>, output: Backing },
    Provider { failure: condition_dct::BindingFailure, parameters: Decoded<'a> },
}
pub struct Bound<'a> {
    pub parameters: Decoded<'a>,
    pub provider: condition_dct::Provider,
}

pub fn bind_initialized<'a>(parts: Parts<'a>) -> Result<Bound<'a>, Failure<'a>> {
    for index in 0..2 {
        if parts.matrices[index].plan.request() != Request::DctCoefficients {
            return Err(Failure::Admission { reason: Admission::Request { matrix: index }, parts });
        }
        if parts.matrices[index].plan.dimensions() != parts.parameters.payloads[index].dimensions() {
            return Err(Failure::Admission { reason: Admission::Dimensions { matrix: index }, parts });
        }
    }
    let expected = parts.parameters.payloads[0].dimensions().0;
    let actual = parts.output.logical_width();
    if expected != actual {
        return Err(Failure::Admission { reason: Admission::OutputWidth { expected, actual }, parts });
    }
    let Parts { parameters, matrices: [first, second], output } = parts;
    let first = match parameters.payloads[0].bind(first.plan, first.backing) {
        Ok(first) => first,
        Err(failure) => return Err(Failure::FirstMatrix { failure, parameters, second, output }),
    };
    let second = match parameters.payloads[1].bind(second.plan, second.backing) {
        Ok(second) => second,
        Err(failure) => return Err(Failure::SecondMatrix { first, failure, parameters, output }),
    };
    let dct = match Dct::from_explicit_matrices([first, second]) {
        Ok(dct) => dct,
        Err(failure) => return Err(Failure::Dct { failure, parameters, output }),
    };
    match condition_dct::Provider::from_explicit_backing(dct, output) {
        Ok(provider) => Ok(Bound { parameters, provider }),
        Err(failure) => Err(Failure::Provider { failure, parameters }),
    }
}
