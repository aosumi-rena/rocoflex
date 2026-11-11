use crate::conditional_gate::{GateError, VectorKernels};
use crate::row_arithmetic::{add_condition, split_product};

#[derive(Debug, Clone, Copy)]
pub struct ArithmeticMode { pub fused_multiply_add: bool }

impl ArithmeticMode {
    fn multiply_add(self, a: f32, b: f32, c: f32) -> f32 {
        if self.fused_multiply_add { a.mul_add(b,c) } else { (a*b)+c }
    }

    fn polynomial(self, x: f32, first: u32, second: u32, third: u32) -> f32 {
        let abs=f32::from_bits(x.to_bits()&0x7fff_ffff);
        let square=x*x;
        let p=self.multiply_add(f32::from_bits(third),square,f32::from_bits(second));
        let p=self.multiply_add(p,abs,f32::from_bits(first));
        self.multiply_add(p,abs,1.0)
    }

    pub fn tanh(self, x: f32) -> f32 {
        let p=self.polynomial(x,0x3f7bfbbd,0x3f198f1d,0x3e060807);
        let reciprocal=rcpps(p);

        let selected=if !(x<0.0) {p} else {reciprocal};
        let square=selected*selected;
        (square-1.0)/(square+1.0)
    }

    pub fn sigmoid(self, x: f32) -> f32 {
        let p=self.polynomial(x,0x3efbfc65,0x3e194af5,0x3c060568);
        let reciprocal=rcpps(p);

        let selected=if x<0.0 {p} else {reciprocal};
        rcpps(selected*selected+1.0)
    }
}

#[cfg(target_arch="x86_64")]
pub fn rcpps(value: f32) -> f32 {
    use std::arch::x86_64::{_mm_cvtss_f32,_mm_rcp_ps,_mm_set1_ps};

    unsafe { _mm_cvtss_f32(_mm_rcp_ps(_mm_set1_ps(value))) }
}

#[cfg(not(target_arch="x86_64"))]
compile_error!("Source FloatVector RCPPS backend requires an explicit x86_64 target");

pub struct SourceGateKernels { pub arithmetic: ArithmeticMode }
impl VectorKernels for SourceGateKernels {
    fn slot88(&mut self, condition: &[f32], gate: &mut [f32]) -> Result<(), GateError> {
        add_condition(condition,gate)
    }
    fn split_activate_combine(&mut self, gate: &[f32], half_width: usize)
        -> Result<Vec<f32>,GateError>
    {
        split_product(gate,half_width,|x|self.arithmetic.tanh(x),|x|self.arithmetic.sigmoid(x))
    }
}
