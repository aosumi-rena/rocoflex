use super::{Admitted, Admission, OperandOrder, Primitive, Projection, Request};
use super::super::path::{self, comparison, curve_source as curve, point_source as point};
use path::old212200_source::{Instruction, Opcode};
use path::ellipse_source::Operation;
use super::super::edge_table::path_source::scan;
#[path = "raster_old_scan.rs"] mod scan_sites;

pub const SCOPE: &str = "OLD-VST-02132422/path-mask-value";

#[derive(Clone, Copy, Debug)]
pub enum Area { DeviceBounds, Scan, OtherComparison, CurveArithmetic }
#[derive(Debug)]
pub enum Required {
    Unmapped { request: Request, area: Area },
    RequestMismatch { request: Request, pc: Option<u32> },
    Constructor(path::old_constructor_source::Error),
    Iterator(path::old212200_source::Error),
    UnexpectedInstruction { request: Request, instruction: Instruction },

    CountSquareRootCrtCall { request: Request, call_pc: u32, target_rva: u32, return_pc: u32 },
}

pub struct OldPathMaskSites { _explicit_scope: () }
impl OldPathMaskSites {
    pub fn declared_old_value_scope() -> Self { Self { _explicit_scope: () } }
}
impl Admission for OldPathMaskSites {
    type Error = Required;
    fn resolve(&mut self, request: Request) -> Result<Admitted, Required> { resolve_known(request) }
}

pub fn resolve_known(request: Request) -> Result<Admitted, Required> {
    match request {
        Request::Point(value) => point_instruction(value),
        Request::Curve(value) => curve_instruction(value),
        Request::Comparison(value) => {
            if let Some(instruction) = path::old_constructor_source::comparison_instruction(value)
                .map_err(Required::Constructor)? {
                return existing_instruction(request, instruction);
            }
            if let Some(instruction) = path::old212200_source::comparison_instruction(value)
                .map_err(Required::Iterator)? {
                return existing_instruction(request, instruction);
            }
            Err(Required::Unmapped { request, area: Area::OtherComparison })
        }
        Request::Packed(value) => {
            let instruction = path::old_constructor_source::packed_instruction(value)
                .map_err(Required::Constructor)?;
            existing_instruction(request, instruction)
        }
        Request::Maximum(value) => {
            let instruction = path::old212200_source::maximum_instruction(value)
                .map_err(Required::Iterator)?;
            existing_instruction(request, instruction)
        }

        Request::Bounds(_) => Err(Required::Unmapped { request, area: Area::DeviceBounds }),
        Request::Scan(value) => scan_sites::instruction(value),
    }
}

fn existing_instruction(request: Request, instruction: Instruction) -> Result<Admitted, Required> {
    let (primitive, projection) = match (request, instruction.opcode) {
        (Request::Comparison(value), Opcode::Ucomiss) =>
            (Primitive::Ucomiss, Projection::OrderedFlags(value.relation)),
        (Request::Comparison(value), Opcode::Cmpeqss)
            if value.relation == comparison::Relation::OrderedEqual =>
            (Primitive::Cmpeqss, Projection::EqualMaskLowBit),
        (Request::Packed(_), Opcode::CmpeqpsXmm) => (Primitive::CmpeqpsXmm, Projection::PackedEqual),
        (Request::Maximum(_), Opcode::Maxss) => (Primitive::Maxss, Projection::F32),
        _ => return Err(Required::UnexpectedInstruction { request, instruction }),
    };
    Ok(Admitted { scope: SCOPE, pc: instruction.pc, primitive, projection,
        operands: if instruction.reversed { OperandOrder::RequestRightLeft }
            else { OperandOrder::RequestLeftRight } })
}

fn arithmetic(request: Request, operation: Operation, pc: u32, primitive: Primitive)
    -> Result<Admitted, Required> {
    let matches = matches!((operation, primitive),
        (Operation::Add { .. }, Primitive::Addss) | (Operation::Sub { .. }, Primitive::Subss)
        | (Operation::Mul { .. }, Primitive::Mulss));
    if !matches { return Err(Required::RequestMismatch { request, pc: Some(pc) }); }
    Ok(Admitted { scope: SCOPE, pc, primitive, operands: OperandOrder::RequestLeftRight,
        projection: Projection::F32 })
}

fn point_instruction(value: point::Request) -> Result<Admitted, Required> {
    use point::{Role as R, Value as V};

    let pcs = match value.role {
        R::Ordinary => [0x2126f8,0x2126fe,0x212703,0x212708,0x2126e6,0x2126dc,0x2126ea,0x2126ee],
        R::QuadraticControl => [0x2124cd,0x2124d2,0x2124d6,0x2124e0,0x2124ad,0x2124a6,0x2124b1,0x2124ba],
        R::QuadraticEnd => [0x2124fb,0x212500,0x212505,0x21250a,0x2124ee,0x2124e9,0x2124f3,0x2124f7],
        R::CubicFirst => [0x212647,0x21264c,0x212650,0x21265a,0x212627,0x212620,0x21262b,0x212634],
        R::CubicSecond => [0x21267f,0x212684,0x212689,0x21268e,0x212670,0x212667,0x212675,0x21267a],
        R::CubicEnd => [0x2126a5,0x2126aa,0x2126af,0x2126b4,0x212698,0x212693,0x21269d,0x2126a1],
    };
    let primitive = match value.value {
        V::FirstYProduct | V::SecondYProduct | V::FirstXProduct | V::SecondXProduct => Primitive::Mulss,
        V::YSum | V::Y | V::XSum | V::X => Primitive::Addss,
    };
    arithmetic(Request::Point(value), value.operation, pcs[value.value as usize], primitive)
}

fn curve_instruction(value: curve::Request) -> Result<Admitted, Required> {
    use comparison::CurveKind as K;
    use curve::ArithmeticSite as A;
    let request = Request::Curve(value);
    let unknown = || Required::Unmapped { request, area: Area::CurveArithmetic };
    let (pc, primitive) = match value.site {
        A::MidpointAdd { point, axis } | A::MidpointHalf { point, axis } => {
            let half = matches!(value.site, A::MidpointHalf { .. });
            if axis >= 2 { return Err(unknown()); }
            let (add, mul): (&[[u32;2]], &[[u32;2]]) = match value.kind {
                K::Quadratic => (&[[0x212789,0x21279f],[0x2127b4,0x2127c2],[0x2127d0,0x2127de]],
                    &[[0x212795,0x2127a4],[0x2127b9,0x2127c7],[0x2127d5,0x2127e3]]),
                K::Cubic => (&[[0x212924,0x21293a],[0x21294f,0x21295d],[0x21296d,0x212985],
                    [0x212993,0x2129a1],[0x2129c0,0x2129e1],[0x21313a,0x213117]],
                    &[[0x212930,0x21293f],[0x212954,0x212962],[0x212972,0x21298a],
                    [0x212998,0x2129a6],[0x2129c5,0x2129e6],[0x21313f,0x213124]]),
                K::Line => return Err(unknown()),
            };
            let table = if half { mul } else { add };
            let row = table.get(point as usize).ok_or_else(unknown)?;
            if half && !matches!(value.operation, Operation::Mul { source: 0x3f000000, .. }) {
                return Err(Required::RequestMismatch { request, pc: Some(row[axis as usize]) });
            }
            (row[axis as usize], if half { Primitive::Mulss } else { Primitive::Addss })
        }
        A::Delta { half, axis } | A::Square { half, axis } => {
            let square = matches!(value.site, A::Square { .. });
            if axis >= 2 { return Err(unknown()); }
            let (delta, squares): (&[[u32;2]], &[[u32;2]]) = match value.kind {
                K::Quadratic => (&[[0x212c69,0x212c71]], &[[0x212c7a,0x212c76]]),
                K::Cubic => (&[[0x2129af,0x2129b7],[0x2129fc,0x2129f3]],
                    &[[0x2129cd,0x2129c9],[0x212a06,0x212a02]]),
                K::Line => return Err(unknown()),
            };
            let row = (if square { squares } else { delta }).get(half as usize).ok_or_else(unknown)?;
            if square && !matches!(value.operation, Operation::Mul { destination, source } if destination == source) {
                return Err(Required::RequestMismatch { request, pc: Some(row[axis as usize]) });
            }
            (row[axis as usize], if square { Primitive::Mulss } else { Primitive::Subss })
        }
        A::Distance { half } => {
            let pc = match (value.kind, half) {
                (K::Quadratic, 0) => 0x212c7e,
                (K::Cubic, 0) => 0x2129d1, (K::Cubic, 1) => 0x212a0a,
                _ => return Err(unknown()),
            };
            (pc, Primitive::Addss)
        }
        A::NearSubtract { kind, pair } | A::NearScale { kind, pair } => {
            if kind != value.kind { return Err(Required::RequestMismatch { request, pc: None }); }
            let scale = matches!(value.site, A::NearScale { .. });
            let (subtract, multiply): (&[u32], &[u32]) = match kind {
                K::Line => (&[0x212577,0x2123f6,0x21343a,0x21347c],
                    &[0x2125a4,0x212423,0x213463,0x2134a7]),
                K::Quadratic => (&[0x212864,0x212ada,0x212be1,0x212d5c],
                    &[0x212898,0x212b0c,0x212c17,0x212d92]),
                K::Cubic => (&[0x212a8c,0x212dd6,0x212e8e,0x212ed3,0x212f8a,0x212fd1,0x2131b1,0x2131fa],
                    &[0x212ac0,0x212e08,0x212ec0,0x212f05,0x212fbb,0x213000,0x2131e3,0x21322a]),
            };
            let pc = *(if scale { multiply } else { subtract }).get(pair as usize).ok_or_else(unknown)?;
            if scale && !matches!(value.operation, Operation::Mul { source: 0x34000000, .. }) {
                return Err(Required::RequestMismatch { request, pc: Some(pc) });
            }
            (pc, if scale { Primitive::Mulss } else { Primitive::Subss })
        }
    };
    arithmetic(request, value.operation, pc, primitive)
}
