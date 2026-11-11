use super::{edge_table::path_source::scan,path::{self,comparison,curve_source as curve,
    point_source as point},rectangle::bounds};
pub use comparison::OperandOrder;
#[path = "raster_old_path_mask.rs"] pub mod old_path_mask;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Request {
    Bounds(bounds::Request),Scan(scan::Request),Point(point::Request),Curve(curve::Request),
    Comparison(comparison::Request),Packed(comparison::PackedRequest),Maximum(curve::MaximumRequest),
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum WholeOperation {
    CeilF32,FloorF32,RoundedF32ToI32,CountF64ToI32,SlopeF64ToI32,
    QuantizeF32ToI64,QuantizeF64ToI64,
    CountSqrtCrt,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Primitive {
    Addss,Subss,Mulss,Divss,Minss,Maxss,Addsd,Mulsd,Sqrtsd,
    Comiss,Ucomiss,Ucomisd,Cmpeqss,Cmpless,CmpeqpsXmm,Cvtss2sd,Cvtsi2sdSigned32,Cvtsi2sdSigned64,
    Cvttss2si32,Cvttsd2si32,Cvttss2si64,Cvttsd2si64,
    Whole {operation:WholeOperation,identity:&'static str},
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Projection {
    F32,F64,Integer32,Integer64,PackedEqual,RawFlags,
    OrderedFlags(comparison::Relation),EqualMaskLowBit,

    OrderedMaskLowBit(comparison::Relation),
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Admitted {

    pub scope:&'static str,
    pub pc:u32,
    pub primitive:Primitive,
    pub operands:OperandOrder,
    pub projection:Projection,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum F32LoweredOperation {
    Addss {left:u32,right:u32},Subss {left:u32,right:u32},
    Mulss {left:u32,right:u32},Divss {left:u32,right:u32},
    Ucomiss {left:u32,right:u32},
    Cmpless {left:u32,right:u32},
    Cmpeqss {left:u32,right:u32},
}

#[derive(Clone,Copy,Debug)]
pub struct F32Lowering {
    request:Request,instruction:Admitted,operation:F32LoweredOperation,
}
impl F32Lowering {
    pub fn request(&self)->Request {self.request}
    pub fn instruction(&self)->Admitted {self.instruction}
    pub fn operation(&self)->F32LoweredOperation {self.operation}
}
#[derive(Clone,Copy,Debug)]
pub enum F32LoweringError {
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Unsupported {request:Request,instruction:Admitted},
}

pub fn lower_f32_subset(request:Request,instruction:Admitted)->Result<F32Lowering,F32LoweringError> {
    validate(request,instruction).map_err(|reason|F32LoweringError::InvalidAdmission {request,instruction,reason})?;
    let Some(pair)=f32_operands(request) else {
        return Err(F32LoweringError::Unsupported {request,instruction});
    };
    let [left,right]=ordered_pair(pair,instruction.operands);
    let operation=match instruction.primitive {
        Primitive::Addss=>F32LoweredOperation::Addss {left,right},
        Primitive::Subss=>F32LoweredOperation::Subss {left,right},
        Primitive::Mulss=>F32LoweredOperation::Mulss {left,right},
        Primitive::Divss=>F32LoweredOperation::Divss {left,right},
        Primitive::Ucomiss=>F32LoweredOperation::Ucomiss {left,right},

        Primitive::Cmpless=>F32LoweredOperation::Cmpless {left,right},

        Primitive::Cmpeqss=>F32LoweredOperation::Cmpeqss {left,right},
        _=>return Err(F32LoweringError::Unsupported {request,instruction}),
    };
    Ok(F32Lowering {request,instruction,operation})
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum F64LoweredOperation {
    Addsd {left:u64,right:u64},Mulsd {left:u64,right:u64},
    Cvtss2sd {source:u32},Cvtsi2sdSigned32 {value:i32},
}
#[derive(Clone,Copy,Debug)]
pub struct F64Lowering {
    request:Request,instruction:Admitted,operation:F64LoweredOperation,
}
impl F64Lowering {
    pub fn request(&self)->Request {self.request}
    pub fn instruction(&self)->Admitted {self.instruction}
    pub fn operation(&self)->F64LoweredOperation {self.operation}
}
#[derive(Clone,Copy,Debug)]
pub enum F64LoweringError {
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Unsupported {request:Request,instruction:Admitted},
}

pub fn lower_f64_subset(request:Request,instruction:Admitted)->Result<F64Lowering,F64LoweringError> {
    validate(request,instruction).map_err(|reason|F64LoweringError::InvalidAdmission {request,instruction,reason})?;
    let Some(operands)=f64_operands(request) else {
        return Err(F64LoweringError::Unsupported {request,instruction});
    };
    let operands=match operands {
        F64Operands::Binary(pair)=>F64Operands::Binary(ordered_pair(pair,instruction.operands)),
        unary=>unary,
    };
    let operation=match (instruction.primitive,operands) {
        (Primitive::Addsd,F64Operands::Binary([left,right]))=>F64LoweredOperation::Addsd {left,right},
        (Primitive::Mulsd,F64Operands::Binary([left,right]))=>F64LoweredOperation::Mulsd {left,right},
        (Primitive::Cvtss2sd,F64Operands::F32(source))=>F64LoweredOperation::Cvtss2sd {source},
        (Primitive::Cvtsi2sdSigned32,F64Operands::Signed32(value))=>F64LoweredOperation::Cvtsi2sdSigned32 {value},
        _=>return Err(F64LoweringError::Unsupported {request,instruction}),
    };
    Ok(F64Lowering {request,instruction,operation})
}

fn ordered_pair<T:Copy>(pair:[T;2],order:OperandOrder)->[T;2] {
    match order {
        OperandOrder::RequestLeftRight=>pair,
        OperandOrder::RequestRightLeft=>[pair[1],pair[0]],
    }
}
enum F64Operands {Binary([u64;2]),F32(u32),Signed32(i32)}
fn f64_operands(request:Request)->Option<F64Operands> {
    match request {
        Request::Scan(scan::Request {operation:scan::Operation::F64(
            scan::F64Operation::Add {destination,source}|scan::F64Operation::Mul {destination,source}),..})=>
            Some(F64Operands::Binary([destination,source])),
        Request::Scan(scan::Request {operation:scan::Operation::F32ToF64(source),..})=>Some(F64Operands::F32(source)),
        Request::Scan(scan::Request {operation:scan::Operation::IntegerToF64(scan::IntegerSource::Signed32(value)),..})=>
            Some(F64Operands::Signed32(value)),
        _=>None,
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ConversionLoweredOperation {
    Cvtsi2sdSigned64 {value:i64},Cvttsd2si32 {source:u64},
    Cvttss2si64 {source:u32},Cvttsd2si64 {source:u64},
}
#[derive(Clone,Copy,Debug)]
pub struct ConversionLowering {
    request:Request,instruction:Admitted,operation:ConversionLoweredOperation,
}
impl ConversionLowering {
    pub fn request(&self)->Request {self.request}
    pub fn instruction(&self)->Admitted {self.instruction}
    pub fn operation(&self)->ConversionLoweredOperation {self.operation}
}
#[derive(Clone,Copy,Debug)]
pub enum ConversionLoweringError {
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Unsupported {request:Request,instruction:Admitted},
}

pub fn lower_conversion_subset(request:Request,instruction:Admitted)
    ->Result<ConversionLowering,ConversionLoweringError>
{
    validate(request,instruction).map_err(|reason|ConversionLoweringError::InvalidAdmission {request,instruction,reason})?;
    let Some(operand)=conversion_operand(request) else {
        return Err(ConversionLoweringError::Unsupported {request,instruction});
    };
    let operation=match (instruction.primitive,operand) {
        (Primitive::Cvtsi2sdSigned64,ConversionOperand::Signed64(value))=>ConversionLoweredOperation::Cvtsi2sdSigned64 {value},
        (Primitive::Cvttsd2si32,ConversionOperand::F64(source))=>ConversionLoweredOperation::Cvttsd2si32 {source},
        (Primitive::Cvttss2si64,ConversionOperand::F32(source))=>ConversionLoweredOperation::Cvttss2si64 {source},
        (Primitive::Cvttsd2si64,ConversionOperand::F64(source))=>ConversionLoweredOperation::Cvttsd2si64 {source},
        _=>return Err(ConversionLoweringError::Unsupported {request,instruction}),
    };
    Ok(ConversionLowering {request,instruction,operation})
}
enum ConversionOperand {Signed64(i64),F32(u32),F64(u64)}
fn conversion_operand(request:Request)->Option<ConversionOperand> {
    match request {
        Request::Scan(scan::Request {operation:scan::Operation::IntegerToF64(scan::IntegerSource::Signed64(value)),..})=>
            Some(ConversionOperand::Signed64(value)),
        Request::Scan(scan::Request {operation:scan::Operation::Integer32(
            scan::Integer32Operation::CountResult {source}|scan::Integer32Operation::Slope {source}),..})=>
            Some(ConversionOperand::F64(source)),
        Request::Scan(scan::Request {operation:scan::Operation::Integer64(scan::Integer64Operation::FromF32 {source}),..})=>
            Some(ConversionOperand::F32(source)),
        Request::Scan(scan::Request {operation:scan::Operation::Integer64(scan::Integer64Operation::FromF64 {source}),..})=>
            Some(ConversionOperand::F64(source)),
        _=>None,
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum GuardFastLoweredOperation {
    Ucomisd {left:u64,right:u64},Sqrtsd {source:u64},
}
#[derive(Clone,Copy,Debug)]
pub struct GuardFastLowering {
    request:Request,instruction:Admitted,operation:GuardFastLoweredOperation,
}
impl GuardFastLowering {
    pub fn request(&self)->Request {self.request}
    pub fn instruction(&self)->Admitted {self.instruction}
    pub fn operation(&self)->GuardFastLoweredOperation {self.operation}
}
#[derive(Clone,Copy,Debug)]
pub enum GuardFastLoweringError {
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Unsupported {request:Request,instruction:Admitted},
}

pub fn lower_guard_fast_subset(request:Request,instruction:Admitted)
    ->Result<GuardFastLowering,GuardFastLoweringError>
{
    validate(request,instruction).map_err(|reason|GuardFastLoweringError::InvalidAdmission {request,instruction,reason})?;
    let Some(operand)=guard_fast_operand(request) else {
        return Err(GuardFastLoweringError::Unsupported {request,instruction});
    };
    let operand=match operand {
        GuardFastOperand::Guard(pair)=>GuardFastOperand::Guard(ordered_pair(pair,instruction.operands)),
        fast=>fast,
    };
    let operation=match (instruction.primitive,operand) {
        (Primitive::Ucomisd,GuardFastOperand::Guard([left,right]))=>GuardFastLoweredOperation::Ucomisd {left,right},
        (Primitive::Sqrtsd,GuardFastOperand::Fast(source))=>GuardFastLoweredOperation::Sqrtsd {source},
        _=>return Err(GuardFastLoweringError::Unsupported {request,instruction}),
    };
    Ok(GuardFastLowering {request,instruction,operation})
}
enum GuardFastOperand {Guard([u64;2]),Fast(u64)}
fn guard_fast_operand(request:Request)->Option<GuardFastOperand> {
    match request {
        Request::Scan(scan::Request {site:scan::Site::CountSqrtGuard,
            operation:scan::Operation::F64Comparison(scan::F64Comparison {left,right})})=>

            Some(GuardFastOperand::Guard([left,right])),
        Request::Scan(scan::Request {site:scan::Site::CountSqrt,
            operation:scan::Operation::F64(scan::F64Operation::Sqrt {source})})=>
            Some(GuardFastOperand::Fast(source)),
        _=>None,
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PackedMaxLoweredOperation {
    CmpeqpsXmm {left:[u32;4],right:[u32;4]},Maxss {left:u32,right:u32},
}
#[derive(Clone,Copy,Debug)]
pub struct PackedMaxLowering {
    request:Request,instruction:Admitted,operation:PackedMaxLoweredOperation,
}
impl PackedMaxLowering {
    pub fn request(&self)->Request {self.request}
    pub fn instruction(&self)->Admitted {self.instruction}
    pub fn operation(&self)->PackedMaxLoweredOperation {self.operation}
}
#[derive(Clone,Copy,Debug)]
pub enum PackedMaxLoweringError {
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Unsupported {request:Request,instruction:Admitted},
}

pub fn lower_packed_max_subset(request:Request,instruction:Admitted)
    ->Result<PackedMaxLowering,PackedMaxLoweringError>
{
    validate(request,instruction).map_err(|reason|PackedMaxLoweringError::InvalidAdmission {request,instruction,reason})?;
    let Some(operand)=packed_max_operand(request) else {
        return Err(PackedMaxLoweringError::Unsupported {request,instruction});
    };
    let operand=match operand {
        PackedMaxOperand::Packed(pair)=>PackedMaxOperand::Packed(ordered_pair(pair,instruction.operands)),
        PackedMaxOperand::Maximum(pair)=>PackedMaxOperand::Maximum(ordered_pair(pair,instruction.operands)),
    };
    let operation=match (instruction.primitive,operand) {
        (Primitive::CmpeqpsXmm,PackedMaxOperand::Packed([left,right]))=>PackedMaxLoweredOperation::CmpeqpsXmm {left,right},
        (Primitive::Maxss,PackedMaxOperand::Maximum([left,right]))=>PackedMaxLoweredOperation::Maxss {left,right},
        _=>return Err(PackedMaxLoweringError::Unsupported {request,instruction}),
    };
    Ok(PackedMaxLowering {request,instruction,operation})
}
enum PackedMaxOperand {Packed([[u32;4];2]),Maximum([u32;2])}
fn packed_max_operand(request:Request)->Option<PackedMaxOperand> {
    match request {
        Request::Packed(comparison::PackedRequest {left,right,..})=>Some(PackedMaxOperand::Packed([left,right])),
        Request::Maximum(curve::MaximumRequest {left,right,..})=>Some(PackedMaxOperand::Maximum([left,right])),
        _=>None,
    }
}

fn f32_arithmetic_operands(operation:path::ellipse_source::Operation)->[u32;2] {
    use path::ellipse_source::Operation::*;
    match operation {Add {destination,source}|Sub {destination,source}|Mul {destination,source}=>[destination,source]}
}
fn f32_operands(request:Request)->Option<[u32;2]> {
    match request {
        Request::Point(r)=>Some(f32_arithmetic_operands(r.operation)),
        Request::Curve(r)=>Some(f32_arithmetic_operands(r.operation)),
        Request::Bounds(bounds::Request {operation:bounds::Operation::Arithmetic(op),..})=>
            Some(f32_arithmetic_operands(op)),
        Request::Bounds(bounds::Request {operation:bounds::Operation::Predicate(bounds::Predicate::OrderedGreater {left,right}),..})=>
            Some([left,right]),
        Request::Comparison(r)=>Some([r.left,r.right]),
        Request::Scan(scan::Request {operation:scan::Operation::F32(scan::F32Operation::Arithmetic(op)),..})=>
            Some(f32_arithmetic_operands(op)),
        Request::Scan(scan::Request {operation:scan::Operation::F32(scan::F32Operation::Divide {destination,source}),..})=>
            Some([destination,source]),
        Request::Scan(scan::Request {operation:scan::Operation::Predicate(scan::Predicate::OrderedLessEqual {left,right}),..})=>
            Some([left,right]),
        _=>None,
    }
}

pub trait Admission {
    type Error;

    fn resolve(&mut self,request:Request)->Result<Admitted,Self::Error>;
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Reply {
    Bits32(u32),Bits64(u64),Integer32(i32),Integer64(i64),
    Flags {carry:bool,parity:bool,zero:bool},Mask32(u32),MasksXmm([u32;4]),
}

pub trait NumericOwner {
    type Error;

    fn execute(&mut self,request:Request,instruction:Admitted)->Result<Reply,Self::Error>;
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Attempt {
    pub request:Request,pub instruction:Option<Admitted>,pub reply:Option<Reply>,pub published:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AdmissionMismatch {EmptyScope,EmptyWholeIdentity,UnaryOperandOrder,PrimitiveOrProjection}
#[derive(Debug)]
pub enum Error<A,N> {
    AlreadyStopped {request:Request,previous:Option<Attempt>},
    Required {request:Request,error:A},
    InvalidAdmission {request:Request,instruction:Admitted,reason:AdmissionMismatch},
    Numeric {request:Request,instruction:Admitted,error:N},
    WrongReply {request:Request,instruction:Admitted,reply:Reply},
    ProjectionInvariant {attempt:Option<Attempt>},
}

pub struct Binding<'a,N:NumericOwner,A:Admission> {
    owner:&'a mut N,admission:&'a mut A,last:Option<Attempt>,stopped:bool,
}

pub struct MaskCompleted<'p,'a,N:NumericOwner,A:Admission> {
    graph:super::pixels::path_mask::Completed<'p>,binding:Binding<'a,N,A>,
}
impl<'p,'a,N:NumericOwner,A:Admission> MaskCompleted<'p,'a,N,A> {
    pub fn graph(&self)->&super::pixels::path_mask::Completed<'p> {&self.graph}
    pub fn binding(&self)->&Binding<'a,N,A> {&self.binding}

    pub fn into_parts(self)->(super::pixels::path_mask::Completed<'p>,Binding<'a,N,A>) {
        (self.graph,self.binding)
    }
}
#[must_use="retain SAME partial graph/table/walker AND borrowed numeric/admission owners"]
pub struct MaskFailure<'p,'a,N:NumericOwner,A:Admission> {

    graph:super::pixels::path_mask::Failure<'p,Error<A::Error,N::Error>>,
    binding:Binding<'a,N,A>,
}
impl<'p,'a,N:NumericOwner,A:Admission> MaskFailure<'p,'a,N,A> {
    pub fn graph(&self)->&super::pixels::path_mask::Failure<'p,Error<A::Error,N::Error>> {&self.graph}
    pub fn binding(&self)->&Binding<'a,N,A> {&self.binding}

    pub fn freeze_terminal(self)->TerminalMaskFailure<Error<A::Error,N::Error>> {
        let Self {graph,binding}=self;
        let graph=graph.freeze_terminal();
        let Binding {owner,admission,last,stopped}=binding;
        let _=(owner,admission);
        TerminalMaskFailure {graph,binding:TerminalBinding {last,stopped}}
    }
}
pub struct TerminalBinding {last:Option<Attempt>,stopped:bool}
impl TerminalBinding {
    pub fn last_attempt(&self)->Option<&Attempt> {self.last.as_ref()}
    pub fn stopped(&self)->bool {self.stopped}
}
#[must_use="retain actual Path/numeric/admission owners with terminal graph"]
pub struct TerminalMaskFailure<E> {
    graph:super::pixels::path_mask::TerminalFailure<E>,binding:TerminalBinding,
}
impl<E> TerminalMaskFailure<E> {
    pub fn graph(&self)->&super::pixels::path_mask::TerminalFailure<E> {&self.graph}
    pub fn binding(&self)->&TerminalBinding {&self.binding}
}

#[derive(Clone,Copy)]
enum Value {F32(u32),F64(u64),Integer32(i32),Integer64(i64),Predicate(bool),Packed([u32;4]),Flags(scan::ComparisonFlags)}
#[derive(Clone,Copy)]
enum Spec {
    Add32,Sub32,Mul32,Div32,Min32,Max32,Add64,Mul64,Sqrt64,
    Compare(comparison::Relation),Packed,Wide,Wide32,Wide64,
    RoundCeil,RoundFloor,Integer32F32,Count32,Slope32,Integer64F32,Integer64F64,
    Flags64,CrtSqrt64,
}

fn arithmetic(operation:path::ellipse_source::Operation)->Spec {
    use path::ellipse_source::Operation::*;
    match operation {Add {..}=>Spec::Add32,Sub {..}=>Spec::Sub32,Mul {..}=>Spec::Mul32}
}

fn specification(request:Request)->Spec {
    match request {
        Request::Point(r)=>arithmetic(r.operation),Request::Curve(r)=>arithmetic(r.operation),
        Request::Comparison(r)=>Spec::Compare(r.relation),Request::Packed(_)=>Spec::Packed,
        Request::Maximum(_)=>Spec::Max32,
        Request::Bounds(r)=>match r.operation {
            bounds::Operation::Arithmetic(op)=>arithmetic(op),
            bounds::Operation::Selection(bounds::Selection::Min {..})=>Spec::Min32,
            bounds::Operation::Selection(bounds::Selection::Max {..})=>Spec::Max32,
            bounds::Operation::Predicate(_)=>Spec::Compare(comparison::Relation::OrderedGreater),
            bounds::Operation::Round(bounds::Rounding::Ceil {..})=>Spec::RoundCeil,
            bounds::Operation::Round(bounds::Rounding::Floor {..})=>Spec::RoundFloor,
            bounds::Operation::Integer {..}=>Spec::Integer32F32,
        },
        Request::Scan(r)=>match r.operation {
            scan::Operation::F32(scan::F32Operation::Arithmetic(op))=>arithmetic(op),
            scan::Operation::F32(scan::F32Operation::Divide {..})=>Spec::Div32,
            scan::Operation::F64(scan::F64Operation::Add {..})=>Spec::Add64,
            scan::Operation::F64(scan::F64Operation::Mul {..})=>Spec::Mul64,
            scan::Operation::F64(scan::F64Operation::Sqrt {..})=>Spec::Sqrt64,
            scan::Operation::Predicate(_)=>Spec::Compare(comparison::Relation::OrderedLessEqual),
            scan::Operation::F32ToF64(_)=>Spec::Wide,
            scan::Operation::IntegerToF64(scan::IntegerSource::Signed32(_))=>Spec::Wide32,
            scan::Operation::IntegerToF64(scan::IntegerSource::Signed64(_))=>Spec::Wide64,
            scan::Operation::Integer32(scan::Integer32Operation::CountResult {..})=>Spec::Count32,
            scan::Operation::Integer32(scan::Integer32Operation::Slope {..})=>Spec::Slope32,
            scan::Operation::Integer64(scan::Integer64Operation::FromF32 {..})=>Spec::Integer64F32,
            scan::Operation::Integer64(scan::Integer64Operation::FromF64 {..})=>Spec::Integer64F64,
            scan::Operation::F64Comparison(_)=>Spec::Flags64,
            scan::Operation::CountSqrtCrt(_)=>Spec::CrtSqrt64,
        },
    }
}

fn validate(request:Request,instruction:Admitted)->Result<(),AdmissionMismatch> {
    use Primitive::*;
    if instruction.scope.is_empty() {return Err(AdmissionMismatch::EmptyScope);}
    if let Whole {identity,..}=instruction.primitive {
        if identity.is_empty() {return Err(AdmissionMismatch::EmptyWholeIdentity);}
    }
    if instruction.operands==OperandOrder::RequestRightLeft {
        match specification(request) {
            Spec::Sqrt64|Spec::Wide|Spec::Wide32|Spec::Wide64|Spec::RoundCeil|Spec::RoundFloor
            |Spec::Integer32F32|Spec::Count32|Spec::Slope32|Spec::Integer64F32|Spec::Integer64F64|Spec::CrtSqrt64=>
                return Err(AdmissionMismatch::UnaryOperandOrder),
            _=>{}
        }
    }

    let valid=match (specification(request),instruction.primitive,instruction.projection) {
        (Spec::Add32,Addss,Projection::F32)|(Spec::Sub32,Subss,Projection::F32)
        |(Spec::Mul32,Mulss,Projection::F32)|(Spec::Div32,Divss,Projection::F32)
        |(Spec::Min32,Minss,Projection::F32)|(Spec::Max32,Maxss,Projection::F32)
        |(Spec::Add64,Addsd,Projection::F64)|(Spec::Mul64,Mulsd,Projection::F64)
        |(Spec::Sqrt64,Sqrtsd,Projection::F64)|(Spec::Packed,CmpeqpsXmm,Projection::PackedEqual)
        |(Spec::Wide,Cvtss2sd,Projection::F64)|(Spec::Wide32,Cvtsi2sdSigned32,Projection::F64)
        |(Spec::Wide64,Cvtsi2sdSigned64,Projection::F64)
        |(Spec::Integer32F32,Cvttss2si32,Projection::Integer32)
        |(Spec::Count32,Cvttsd2si32,Projection::Integer32)
        |(Spec::Slope32,Cvttsd2si32,Projection::Integer32)
        |(Spec::Integer64F32,Cvttss2si64,Projection::Integer64)
        |(Spec::Integer64F64,Cvttsd2si64,Projection::Integer64)=>true,
        (Spec::Compare(relation),Comiss|Ucomiss,Projection::OrderedFlags(projected))=>relation==projected,
        (Spec::Compare(comparison::Relation::OrderedEqual),Cmpeqss,Projection::EqualMaskLowBit)=>true,
        (Spec::Flags64,Ucomisd,Projection::RawFlags)=>
            instruction.operands==OperandOrder::RequestLeftRight
            &&matches!(request,Request::Scan(scan::Request {site:scan::Site::CountSqrtGuard,
                operation:scan::Operation::F64Comparison(scan::F64Comparison {right:0,..})})),
        (Spec::Compare(comparison::Relation::OrderedLessEqual),Cmpless,
            Projection::OrderedMaskLowBit(comparison::Relation::OrderedLessEqual))=>
            instruction.operands==OperandOrder::RequestLeftRight,
        (Spec::RoundCeil,Whole {operation:WholeOperation::CeilF32,..},Projection::F32)
        |(Spec::RoundFloor,Whole {operation:WholeOperation::FloorF32,..},Projection::F32)
        |(Spec::Integer32F32,Whole {operation:WholeOperation::RoundedF32ToI32,..},Projection::Integer32)
        |(Spec::Count32,Whole {operation:WholeOperation::CountF64ToI32,..},Projection::Integer32)
        |(Spec::Slope32,Whole {operation:WholeOperation::SlopeF64ToI32,..},Projection::Integer32)
        |(Spec::Integer64F32,Whole {operation:WholeOperation::QuantizeF32ToI64,..},Projection::Integer64)
        |(Spec::Integer64F64,Whole {operation:WholeOperation::QuantizeF64ToI64,..},Projection::Integer64)
        |(Spec::CrtSqrt64,Whole {operation:WholeOperation::CountSqrtCrt,..},Projection::F64)=>true,
        _=>false,
    };
    if valid {Ok(())}else {Err(AdmissionMismatch::PrimitiveOrProjection)}
}

fn project(instruction:Admitted,reply:Reply)->Option<Value> {
    match (instruction.projection,reply) {
        (Projection::F32,Reply::Bits32(v))=>Some(Value::F32(v)),
        (Projection::F64,Reply::Bits64(v))=>Some(Value::F64(v)),
        (Projection::Integer32,Reply::Integer32(v))=>Some(Value::Integer32(v)),
        (Projection::Integer64,Reply::Integer64(v))=>Some(Value::Integer64(v)),
        (Projection::RawFlags,Reply::Flags {carry,parity,zero})=>
            Some(Value::Flags(scan::ComparisonFlags {carry,parity,zero})),
        (Projection::OrderedFlags(relation),Reply::Flags {carry,parity,zero})=>
            Some(Value::Predicate(comparison::ordered_relation_from_flags(relation,
                instruction.operands,carry,parity,zero))),
        (Projection::EqualMaskLowBit|Projection::OrderedMaskLowBit(_),Reply::Mask32(mask)) if mask==0||mask==u32::MAX=>
            Some(Value::Predicate(mask&1!=0)),
        (Projection::PackedEqual,Reply::MasksXmm(masks)) if masks.iter().all(|v|*v==0||*v==u32::MAX)=>
            Some(Value::Packed(masks)),
        _=>None,
    }
}

impl<'a,N:NumericOwner,A:Admission> Binding<'a,N,A> {
    pub fn borrow(owner:&'a mut N,admission:&'a mut A)->Self {
        Self {owner,admission,last:None,stopped:false}
    }
    pub fn owner(&self)->&N {&*self.owner}
    pub fn admission(&self)->&A {&*self.admission}
    pub fn last_attempt(&self)->Option<&Attempt> {self.last.as_ref()}
    pub fn stopped(&self)->bool {self.stopped}

    pub fn path_mask_source_retaining<'p>(mut self,clip:[i32;4],path:&'p path::Path,affine_words:[u32;6])
        ->Result<MaskCompleted<'p,'a,N,A>,MaskFailure<'p,'a,N,A>>
    {
        match super::pixels::path_mask_source_with_services(clip,path,affine_words,&mut self) {
            Ok(graph)=>Ok(MaskCompleted {graph,binding:self}),
            Err(graph)=>{
                self.stopped=true;
                Err(MaskFailure {graph,binding:self})
            },
        }
    }
    fn perform(&mut self,request:Request)->Result<Value,Error<A::Error,N::Error>> {
        if self.stopped {return Err(Error::AlreadyStopped {request,previous:self.last});}
        self.last=Some(Attempt {request,instruction:None,reply:None,published:false});
        let instruction=match self.admission.resolve(request) {
            Ok(value)=>value,
            Err(error)=>{self.stopped=true;return Err(Error::Required {request,error});},
        };
        self.last=Some(Attempt {request,instruction:Some(instruction),reply:None,published:false});
        if let Err(reason)=validate(request,instruction) {
            self.stopped=true;return Err(Error::InvalidAdmission {request,instruction,reason});
        }
        let reply=match self.owner.execute(request,instruction) {
            Ok(reply)=>reply,
            Err(error)=>{self.stopped=true;return Err(Error::Numeric {request,instruction,error});},
        };

        self.last=Some(Attempt {request,instruction:Some(instruction),reply:Some(reply),published:false});
        let value=match project(instruction,reply) {
            Some(value)=>value,
            None=>{self.stopped=true;return Err(Error::WrongReply {request,instruction,reply});},
        };
        self.last=Some(Attempt {request,instruction:Some(instruction),reply:Some(reply),published:true});
        Ok(value)
    }
    fn invariant(&mut self)->Error<A::Error,N::Error> {
        if let Some(attempt)=self.last.as_mut() {attempt.published=false;}
        self.stopped=true;Error::ProjectionInvariant {attempt:self.last}
    }
    fn f32_value(&mut self,request:Request)->Result<u32,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::F32(v)=>Ok(v),_=>Err(self.invariant())}
    }
    fn f64_value(&mut self,request:Request)->Result<u64,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::F64(v)=>Ok(v),_=>Err(self.invariant())}
    }
    fn predicate_value(&mut self,request:Request)->Result<bool,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::Predicate(v)=>Ok(v),_=>Err(self.invariant())}
    }
    fn flags_value(&mut self,request:Request)->Result<scan::ComparisonFlags,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::Flags(v)=>Ok(v),_=>Err(self.invariant())}
    }
    fn integer32_value(&mut self,request:Request)->Result<i32,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::Integer32(v)=>Ok(v),_=>Err(self.invariant())}
    }
    fn integer64_value(&mut self,request:Request)->Result<i64,Error<A::Error,N::Error>> {
        match self.perform(request)? {Value::Integer64(v)=>Ok(v),_=>Err(self.invariant())}
    }
}

impl<N:NumericOwner,A:Admission> point::Fp for Binding<'_,N,A> {
    type Error=Error<A::Error,N::Error>;
    fn point_operation(&mut self,request:point::Request)->Result<u32,Self::Error> {
        self.f32_value(Request::Point(request))
    }
}
impl<N:NumericOwner,A:Admission> curve::Fp for Binding<'_,N,A> {
    type Error=Error<A::Error,N::Error>;
    fn curve_operation(&mut self,request:curve::Request)->Result<u32,Self::Error> {
        self.f32_value(Request::Curve(request))
    }
    fn curve_comparison(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        self.predicate_value(Request::Comparison(request))
    }
    fn near_maximum(&mut self,request:curve::MaximumRequest)->Result<u32,Self::Error> {
        self.f32_value(Request::Maximum(request))
    }
}
impl<N:NumericOwner,A:Admission> comparison::Services for Binding<'_,N,A> {
    type Error=Error<A::Error,N::Error>;
    fn compare(&mut self,request:comparison::Request)->Result<bool,Self::Error> {
        self.predicate_value(Request::Comparison(request))
    }
}
impl<N:NumericOwner,A:Admission> comparison::PackedServices for Binding<'_,N,A> {
    fn compare_equal_xmm(&mut self,request:comparison::PackedRequest)->Result<[u32;4],Self::Error> {
        match self.perform(Request::Packed(request))? {Value::Packed(v)=>Ok(v),_=>Err(self.invariant())}
    }
}
impl<N:NumericOwner,A:Admission> bounds::BoundsServices for Binding<'_,N,A> {
    type Error=Error<A::Error,N::Error>;
    fn bounds_f32(&mut self,site:bounds::Site,operation:bounds::F32Operation)->Result<u32,Self::Error> {
        self.f32_value(Request::Bounds(bounds::Request {site,operation:bounds::Operation::Arithmetic(operation)}))
    }
    fn bounds_select(&mut self,site:bounds::Site,operation:bounds::Selection)->Result<u32,Self::Error> {
        self.f32_value(Request::Bounds(bounds::Request {site,operation:bounds::Operation::Selection(operation)}))
    }
    fn bounds_compare(&mut self,site:bounds::Site,operation:bounds::Predicate)->Result<bool,Self::Error> {
        self.predicate_value(Request::Bounds(bounds::Request {site,operation:bounds::Operation::Predicate(operation)}))
    }
    fn bounds_round(&mut self,site:bounds::Site,operation:bounds::Rounding)->Result<u32,Self::Error> {
        self.f32_value(Request::Bounds(bounds::Request {site,operation:bounds::Operation::Round(operation)}))
    }
    fn bounds_integer(&mut self,site:bounds::Site,source:u32)->Result<i32,Self::Error> {
        self.integer32_value(Request::Bounds(bounds::Request {site,operation:bounds::Operation::Integer {source}}))
    }
}
impl<N:NumericOwner,A:Admission> scan::ScanServices for Binding<'_,N,A> {
    type Error=Error<A::Error,N::Error>;
    fn f32(&mut self,site:scan::Site,operation:scan::F32Operation)->Result<u32,Self::Error> {
        self.f32_value(Request::Scan(scan::Request {site,operation:scan::Operation::F32(operation)}))
    }
    fn f64(&mut self,site:scan::Site,operation:scan::F64Operation)->Result<u64,Self::Error> {
        self.f64_value(Request::Scan(scan::Request {site,operation:scan::Operation::F64(operation)}))
    }
    fn f64_comparison(&mut self,site:scan::Site,operation:scan::F64Comparison)->Result<scan::ComparisonFlags,Self::Error> {
        self.flags_value(Request::Scan(scan::Request {site,operation:scan::Operation::F64Comparison(operation)}))
    }
    fn count_sqrt_crt(&mut self,site:scan::Site,call:scan::CountSqrtCrtCall)->Result<u64,Self::Error> {
        self.f64_value(Request::Scan(scan::Request {site,operation:scan::Operation::CountSqrtCrt(call)}))
    }
    fn predicate(&mut self,site:scan::Site,operation:scan::Predicate)->Result<bool,Self::Error> {
        self.predicate_value(Request::Scan(scan::Request {site,operation:scan::Operation::Predicate(operation)}))
    }
    fn widen(&mut self,site:scan::Site,source:u32)->Result<u64,Self::Error> {
        self.f64_value(Request::Scan(scan::Request {site,operation:scan::Operation::F32ToF64(source)}))
    }
    fn integer_wide(&mut self,site:scan::Site,source:scan::IntegerSource)->Result<u64,Self::Error> {
        self.f64_value(Request::Scan(scan::Request {site,operation:scan::Operation::IntegerToF64(source)}))
    }
    fn integer32(&mut self,site:scan::Site,operation:scan::Integer32Operation)->Result<i32,Self::Error> {
        self.integer32_value(Request::Scan(scan::Request {site,operation:scan::Operation::Integer32(operation)}))
    }
    fn integer64(&mut self,site:scan::Site,operation:scan::Integer64Operation)->Result<i64,Self::Error> {
        self.integer64_value(Request::Scan(scan::Request {site,operation:scan::Operation::Integer64(operation)}))
    }
}
