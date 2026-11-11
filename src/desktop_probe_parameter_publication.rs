use super::super::source_parameter_math::{self as parameter,range};
use crate::source_scalar::{ExplicitExceptionState,ScalarF32Architecture,
    SourceF64ValueError,SourceF64ValueMath};

struct Normalization {
    input:u64, math:SourceF64ValueMath,
    scaled:Option<u64>, normalized:Option<u64>, narrowed:Option<u32>,
}
impl Normalization {
    fn run(&mut self)->Result<u32,SourceF64ValueError> {

        let scaled=self.math.multiply_bits(self.input,0x3fd0_0000_0000_0000)?;
        self.scaled=Some(scaled);
        let normalized=self.math.add_bits(scaled,0x3fe0_0000_0000_0000)?;
        self.normalized=Some(normalized);
        let narrowed=self.math.narrow_f64_bits(normalized)?;
        self.narrowed=Some(narrowed);Ok(narrowed)
    }
}

struct ObservedRange;
enum NoFunction {}
enum Required { Numeric, CustomFunction, NamedCall, NativeSetter }
struct Values {math:parameter::Math}
impl range::Services for Values {
    type Parameter=();
    type Range=ObservedRange;
    type Function=NoFunction;
    type Error=Required;
    fn range_virtual_90(&mut self,_parameter:&mut ())->Result<ObservedRange,Required> {

        Ok(ObservedRange)
    }
    fn read_field(&mut self,_range:&mut ObservedRange,field:range::Field)->Result<u32,Required> {
        Ok(match field {
            range::Field::Start=>0xc000_0000,range::Field::End=>0x4000_0000,
            range::Field::Interval=>0,range::Field::Skew=>0x3f80_0000,
        })
    }
    fn symmetric_skew(&mut self,_range:&mut ObservedRange)->Result<bool,Required> {Ok(false)}
    fn function(&mut self,_range:&mut ObservedRange,_field:range::FunctionField)
        ->Result<Option<NoFunction>,Required> {Ok(None)}
    fn custom(&mut self,_pc:u32,_function:&mut NoFunction,_arguments:&mut [u32;3])
        ->Result<u32,Required> {Err(Required::CustomFunction)}
    fn binary(&mut self,pc:u32,op:range::Binary,left:u32,right:u32)->Result<u32,Required> {
        self.math.binary(pc,op,left,right).map_err(|_|Required::Numeric)
    }
    fn compare(&mut self,pc:u32,op:range::CompareOpcode,left:u32,right:u32)
        ->Result<range::Comparison,Required> {
        self.math.compare(pc,op,left,right).map_err(|_|Required::Numeric)
    }
    fn call(&mut self,_pc:u32,_call:range::Call,_value:u32)->Result<u32,Required> {

        Err(Required::NamedCall)
    }
    fn exchange_public_160(&mut self,_parameter:&mut (),_value:u32)->Result<u32,Required> {
        Err(Required::NativeSetter)
    }
    fn read_public_160(&mut self,_parameter:&mut ())->Result<u32,Required> {
        Err(Required::NativeSetter)
    }
    fn tail_virtual_98(&mut self,_parameter:&mut (),_value:u32)->Result<(),Required> {
        Err(Required::NativeSetter)
    }
}

enum FailureKind {
    Nonfinite {input:u64,entry:ExplicitExceptionState},
    Binding {input:u64,entry:ExplicitExceptionState,reason:SourceF64ValueError},
    Normalization {owner:Normalization,reason:SourceF64ValueError},
    RangeBinding {owner:Normalization,reason:parameter::BindFailure},
    Conversion {owner:Normalization,stopped:range::Stopped<Values>},
}

pub(super) struct Failed {kind:FailureKind}
pub(super) struct Completed {
    normalization:Normalization,
    converted:range::Converted<Values>,
}
impl Completed {
    pub fn bits(&self)->u32 {self.converted.value()}
    pub fn state(&self)->ExplicitExceptionState {self.converted.owner().services().math.state()}
}

pub(super) fn calculate(input:u64,entry:ExplicitExceptionState,architecture:ScalarF32Architecture)
    ->Result<Completed,Failed> {
    if input&0x7ff0_0000_0000_0000==0x7ff0_0000_0000_0000 {
        return Err(Failed {kind:FailureKind::Nonfinite {input,entry}});
    }
    let math=match SourceF64ValueMath::bind(architecture,entry) {
        Ok(math)=>math,
        Err(reason)=>return Err(Failed {kind:FailureKind::Binding {input,entry,reason}}),
    };
    let mut normalization=Normalization {input,math,scaled:None,normalized:None,narrowed:None};
    let narrowed=match normalization.run() {
        Ok(bits)=>bits,
        Err(reason)=>return Err(Failed {kind:FailureKind::Normalization {owner:normalization,reason}}),
    };
    let math=match parameter::Math::bind(architecture,normalization.math.exception_state()) {
        Ok(math)=>math,
        Err(reason)=>return Err(Failed {kind:FailureKind::RangeBinding {owner:normalization,reason}}),
    };
    match range::Prepared::at_2b2da0(Values {math},(),narrowed).run() {
        Ok(converted)=>Ok(Completed {normalization,converted}),
        Err(stopped)=>Err(Failed {kind:FailureKind::Conversion {owner:normalization,stopped}}),
    }
}
