use crate::{
    dct_parameters::{self, Decoded, MatrixStorage},
    matrix_policy::{self, CoefficientFill, CompilerCacheState, FactoryState, Plan, Request},
    userdata,
    value_row_allocation::{self as allocation, BackingPolicy},
};

#[derive(Clone, Copy)]
pub struct MatrixPolicy {
    pub factory: FactoryState,
    pub cache: CompilerCacheState,
    pub fill: CoefficientFill,
}

#[derive(Clone, Copy)]
pub struct Policy {

    pub matrices: [MatrixPolicy; 2],
    pub output: BackingPolicy,
}

pub struct Parts<'a> { pub parameters: Decoded<'a>, pub policy: Policy }

pub enum Failure<'a> {
    Plan { matrix: usize, cause: matrix_policy::Error, parts: Parts<'a> },
    FirstMatrixAllocation {
        cause: matrix_policy::AllocationError,
        parts: Parts<'a>,
        plans: [Plan; 2],
    },
    SecondMatrixAllocation {
        cause: matrix_policy::AllocationError,
        parts: Parts<'a>,
        first: MatrixStorage,
        second_plan: Plan,
    },
    OutputAllocation {
        cause: allocation::Error,
        parts: Parts<'a>,
        matrices: [MatrixStorage; 2],
    },
    Binding { failure: dct_parameters::Failure<'a>, policy: Policy },
}

pub fn bind<'a>(parts: Parts<'a>) -> Result<dct_parameters::Bound<'a>, Failure<'a>> {
    let make_plan = |index: usize| {
        let (rows, columns) = parts.parameters.payloads()[index].dimensions();
        let policy = parts.policy.matrices[index];
        Plan::from_explicit_state(policy.factory, policy.cache,
            Request::DctCoefficients, rows, columns)
    };
    let first_plan = match make_plan(0) {
        Ok(plan) => plan,
        Err(cause) => return Err(Failure::Plan { matrix: 0, cause, parts }),
    };
    let second_plan = match make_plan(1) {
        Ok(plan) => plan,
        Err(cause) => return Err(Failure::Plan { matrix: 1, cause, parts }),
    };
    let first = match first_plan.allocate_backing(parts.policy.matrices[0].fill) {
        Ok(backing) => MatrixStorage { plan: first_plan, backing },
        Err(cause) => return Err(Failure::FirstMatrixAllocation {
            cause, parts, plans: [first_plan, second_plan] }),
    };
    let second = match second_plan.allocate_backing(parts.policy.matrices[1].fill) {
        Ok(backing) => MatrixStorage { plan: second_plan, backing },
        Err(cause) => return Err(Failure::SecondMatrixAllocation {
            cause, parts, first, second_plan }),
    };
    let matrices = [first, second];
    let output_width = parts.parameters.payloads()[0].dimensions().0;
    let output = match allocation::allocate_backing(output_width, parts.policy.output) {
        Ok(output) => output,
        Err(cause) => return Err(Failure::OutputAllocation { cause, parts, matrices }),
    };
    let Parts { parameters, policy } = parts;

    dct_parameters::bind_initialized(dct_parameters::Parts { parameters, matrices, output })
        .map_err(|failure| Failure::Binding { failure, policy })
}

pub enum UserdataFailure<'a> {
    Decode { cause: userdata::Error, policy: Policy },
    Binding(Failure<'a>),
}

pub fn bind_userdata<'a>(userdata: &userdata::Decoded<'a>, policy: Policy)
    -> Result<dct_parameters::Bound<'a>, UserdataFailure<'a>>
{
    let parameters = userdata.decode_dct_parameters()
        .map_err(|cause| UserdataFailure::Decode { cause, policy })?;
    bind(Parts { parameters, policy }).map_err(UserdataFailure::Binding)
}
