use super::{DenseMatrix,MatrixStorage};
use crate::Result;

pub(super) struct PackedFloatConvolution {
    input:usize,output:usize,padded:usize,taps:usize,weights:Vec<f32>,
}
impl PackedFloatConvolution {
    pub(super) fn prepare(matrices:&[DenseMatrix])->Result<Self> {
        let first=matrices.first().ok_or("Packed Float convolution has no taps")?;
        let input=first.input_width;let output=first.output_width;
        let padded=output.checked_add(7).ok_or("Packed Float width overflow")? /8*8;
        let count=matrices.len().checked_mul(input).and_then(|n|n.checked_mul(padded))
            .ok_or("Packed Float coefficient extent overflow")?;
        let mut weights=Vec::new();weights.try_reserve_exact(count).map_err(|e|e.to_string())?;
        weights.resize(count,0.);
        for (tap,matrix) in matrices.iter().enumerate() {
            let MatrixStorage::Float(source)=&matrix.storage else {
                return Err("The selected packed Float convolution requires actual Float taps".into());
            };
            if matrix.input_width!=input || matrix.output_width!=output || source.len()!=input*output {
                return Err("Packed Float tap shape differs".into());
            }
            for i in 0..input {for o in 0..output {
                weights[(tap*input+i)*padded+o]=source[o*input+i];
            }}
        }
        Ok(Self {input,output,padded,taps:matrices.len(),weights})
    }
    pub(super) fn apply<'a>(&self,mut row:impl FnMut(usize)->&'a[f32],output:&mut[f32])->Result<()> {
        if output.len()!=self.output || (0..self.taps).any(|tap|row(tap).len()!=self.input) {
            return Err("Packed Float row dimensions differ".into());
        }
        #[cfg(target_arch="x86_64")]
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {

            unsafe {self.apply_avx2(&mut row,output);}
            return Ok(());
        }
        self.apply_scalar(&mut row,output);
        Ok(())
    }
    fn apply_scalar<'a>(&self,row:&mut impl FnMut(usize)->&'a[f32],output:&mut[f32]) {
        output.fill(0.);let mut first=0;
        while first<self.input {
            let remaining=self.input-first;let end=first+if remaining>=128 {64}else{remaining};
            for tap in 0..self.taps {
                let input=row(tap);
                for (o,value) in output.iter_mut().enumerate() {
                    for i in first..end {*value=self.weights[(tap*self.input+i)*self.padded+o].mul_add(input[i],*value);}
                }
            }
            first=end;
        }
    }
    #[cfg(target_arch="x86_64")]
    #[target_feature(enable="avx2,fma")]
    unsafe fn apply_avx2<'a>(&self,row:&mut impl FnMut(usize)->&'a[f32],output:&mut[f32]) {
        use std::arch::x86_64::*;
        output.fill(0.);let full=self.output/8*8;let tiled=full/32*32;let mut first=0;

        unsafe {
            while first<self.input {
                let remaining=self.input-first;let end=first+if remaining>=128 {64}else{remaining};
                for tap in 0..self.taps {
                    let input=row(tap);

                    for o in (0..tiled).step_by(32) {
                        let mut a=_mm256_loadu_ps(output.as_ptr().add(o));
                        let mut b=_mm256_loadu_ps(output.as_ptr().add(o+8));
                        let mut c=_mm256_loadu_ps(output.as_ptr().add(o+16));
                        let mut d=_mm256_loadu_ps(output.as_ptr().add(o+24));
                        for i in first..end {
                            let weights=self.weights.as_ptr().add((tap*self.input+i)*self.padded+o);
                            let value=_mm256_set1_ps(input[i]);
                            a=_mm256_fmadd_ps(_mm256_loadu_ps(weights),value,a);
                            b=_mm256_fmadd_ps(_mm256_loadu_ps(weights.add(8)),value,b);
                            c=_mm256_fmadd_ps(_mm256_loadu_ps(weights.add(16)),value,c);
                            d=_mm256_fmadd_ps(_mm256_loadu_ps(weights.add(24)),value,d);
                        }
                        _mm256_storeu_ps(output.as_mut_ptr().add(o),a);
                        _mm256_storeu_ps(output.as_mut_ptr().add(o+8),b);
                        _mm256_storeu_ps(output.as_mut_ptr().add(o+16),c);
                        _mm256_storeu_ps(output.as_mut_ptr().add(o+24),d);
                    }
                    for o in (tiled..full).step_by(8) {
                        let mut sum=_mm256_loadu_ps(output.as_ptr().add(o));
                        for i in first..end {
                            let weight=_mm256_loadu_ps(self.weights.as_ptr().add((tap*self.input+i)*self.padded+o));
                            sum=_mm256_fmadd_ps(weight,_mm256_set1_ps(input[i]),sum);
                        }
                        _mm256_storeu_ps(output.as_mut_ptr().add(o),sum);
                    }
                    for o in full..self.output {
                        for i in first..end {
                            output[o]=self.weights[(tap*self.input+i)*self.padded+o].mul_add(input[i],output[o]);
                        }
                    }
                }
                first=end;
            }
        }
    }
}
