use super::{scan, Admitted, OperandOrder, Primitive, Projection, Request, Required, SCOPE};
use super::comparison::Relation;
use super::Operation as Arithmetic;
use scan::{Endpoint, Site, Operation, F32Operation, F64Operation, IntegerSource,
    Integer32Operation, Integer64Operation, Predicate};

fn endpoint(value: Endpoint, from: u32, to: u32) -> u32 {
    match value { Endpoint::From => from, Endpoint::To => to }
}

pub(super) fn instruction(value: scan::Request) -> Result<Admitted, Required> {
    use Primitive as P;
    use Projection as R;
    let request = Request::Scan(value);
    let (pc, primitive, projection) = match (value.site, value.operation) {
        (Site::CountWide, Operation::IntegerToF64(IntegerSource::Signed32(count))) if count >= 0 =>
            (0x2119d5, P::Cvtsi2sdSigned32, R::F64),
        (Site::CountSqrtGuard, Operation::F64Comparison(scan::F64Comparison {right:0,..})) =>
            (0x2119df, P::Ucomisd, R::RawFlags),

        (Site::CountSqrt, Operation::F64(F64Operation::Sqrt { .. })) =>
            (0x2119e5, P::Sqrtsd, R::F64),
        (Site::CountSqrtCrt, Operation::CountSqrtCrt(_)) => {
            return Err(Required::CountSquareRootCrtCall { request,
                call_pc:0x2119eb,target_rva:0x5875c8,return_pc:0x2119f0 });
        }
        (Site::CountInteger, Operation::Integer32(Integer32Operation::CountResult { .. })) =>
            (0x2119f0, P::Cvttsd2si32, R::Integer32),

        (Site::YHalf(end), Operation::Predicate(Predicate::OrderedLessEqual { left: 0, .. })) =>
            (endpoint(end, 0x211cb1, 0x211cdf), P::Cmpless,
                R::OrderedMaskLowBit(Relation::OrderedLessEqual)),
        (Site::YScale(end), Operation::F32(F32Operation::Arithmetic(Arithmetic::Mul { source: 0x43800000, .. }))) =>
            (endpoint(end, 0x211cc5, 0x211cf3), P::Mulss, R::F32),
        (Site::YBias(end), Operation::F32(F32Operation::Arithmetic(Arithmetic::Add { source, .. })))
            if source == 0x3f000000 || source == 0xbf000000 =>
            (endpoint(end, 0x211cca, 0x211cf8), P::Addss, R::F32),
        (Site::YInteger(end), Operation::Integer64(Integer64Operation::FromF32 { .. })) =>
            (endpoint(end, 0x211cce, 0x211cfc), P::Cvttss2si64, R::Integer64),

        (Site::DeltaX, Operation::F32(F32Operation::Arithmetic(Arithmetic::Sub { .. }))) =>
            (0x211d62, P::Subss, R::F32),
        (Site::StartXScale, Operation::F32(F32Operation::Arithmetic(Arithmetic::Mul { source: 0x43800000, .. }))) =>
            (0x211d66, P::Mulss, R::F32),
        (Site::DeltaY, Operation::F32(F32Operation::Arithmetic(Arithmetic::Sub { .. }))) =>
            (0x211d6b, P::Subss, R::F32),
        (Site::Slope, Operation::F32(F32Operation::Divide { .. })) =>
            (0x211d6f, P::Divss, R::F32),
        (Site::SlopeWide, Operation::F32ToF64(_)) => (0x211d73, P::Cvtss2sd, R::F64),

        (Site::SlopeInteger, Operation::Integer32(Integer32Operation::Slope { .. })) =>
            (0x211d80, P::Cvttsd2si32, R::Integer32),

        (Site::StartXWide, Operation::F32ToF64(_)) => (0x211d91, P::Cvtss2sd, R::F64),

        (Site::MidpointWide, Operation::IntegerToF64(IntegerSource::Signed64(_))) =>
            (0x211e2b, P::Cvtsi2sdSigned64, R::F64),
        (Site::SampleMul, Operation::F64(F64Operation::Mul { .. })) => (0x211e30, P::Mulsd, R::F64),
        (Site::SampleAdd, Operation::F64(F64Operation::Add { .. })) => (0x211e35, P::Addsd, R::F64),
        (Site::SampleInteger, Operation::Integer64(Integer64Operation::FromF64 { .. })) =>
            (0x211e3a, P::Cvttsd2si64, R::Integer64),
        _ => return Err(Required::RequestMismatch { request, pc: None }),
    };
    Ok(Admitted { scope: SCOPE, pc, primitive, projection, operands: OperandOrder::RequestLeftRight })
}
