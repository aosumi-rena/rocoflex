use crate::conditional_gate::GateError;

pub trait GenericFirstTap {
    fn overwrite_and_postprocess(&mut self,input:&[f32],output:&mut[f32])->Result<(),GateError>;
}

pub trait SourceAccumulatingMatrix {
    fn input_width(&self)->usize;
    fn output_width(&self)->usize;

    fn accumulate(&mut self,input:&[f32],destination:&mut[f32])->Result<(),GateError>;
}
