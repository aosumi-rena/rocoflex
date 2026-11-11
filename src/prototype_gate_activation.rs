use crate::{Result,activation::ArithmeticMode};
use std::hint::black_box;

fn dimensions(main:&[f32],condition:&[f32],output:&[f32])->Result<()> {
    if output.len().checked_mul(2)!=Some(main.len()) || condition.len()!=main.len() {
        return Err("NCWN activation dimensions differ".into());
    }
    Ok(())
}
#[inline]
fn scalar_lane(mode:ArithmeticMode,main:&[f32],condition:&[f32],i:usize,half:usize)->f32 {
    mode.tanh(main[i]+condition[i]) * mode.sigmoid(main[half+i]+condition[half+i])
}
fn scalar_body(mode:ArithmeticMode,main:&[f32],condition:&[f32],output:&mut[f32]) {
    let half=output.len();
    for (i,value) in output.iter_mut().enumerate() {*value=scalar_lane(mode,main,condition,i,half);}
}
#[inline(never)]
pub fn scalar_into(mode:ArithmeticMode,main:&[f32],condition:&[f32],output:&mut[f32])->Result<()> {
    dimensions(main,condition,output)?;
    scalar_body(mode,main,condition,output);
    Ok(())
}
pub fn candidate_available(mode:ArithmeticMode)->bool {
    #[cfg(target_arch="x86_64")]
    {std::is_x86_feature_detected!("avx")
        && (!mode.fused_multiply_add || std::is_x86_feature_detected!("fma"))}
    #[cfg(not(target_arch="x86_64"))]
    {let _=mode;false}
}
pub fn apply(mode:ArithmeticMode,main:&[f32],condition:&[f32],output:&mut[f32])->Result<()> {
    dimensions(main,condition,output)?;
    #[cfg(target_arch="x86_64")]
    if candidate_available(mode) {

        let exceptional=unsafe {
            if mode.fused_multiply_add {simd::fused(main,condition,output)}
            else {simd::separate(main,condition,output)}
        };
        if exceptional {

            type ScalarRow=fn(ArithmeticMode,&[f32],&[f32],&mut[f32])->Result<()>;
            let reference=black_box(scalar_into as ScalarRow);
            return reference(mode,main,condition,output);
        }
        return Ok(());
    }
    scalar_into(mode,main,condition,output)
}

#[cfg(target_arch="x86_64")]
mod simd {
    use super::{ArithmeticMode,scalar_lane};
    use std::arch::x86_64::*;
    #[inline]
    #[target_feature(enable="avx")]
    unsafe fn reciprocal(value:__m256)->__m256 {

        let low=_mm_rcp_ps(_mm256_castps256_ps128(value));
        let high=_mm_rcp_ps(_mm256_extractf128_ps::<1>(value));
        _mm256_insertf128_ps::<1>(_mm256_castps128_ps256(low),high)
    }
    macro_rules! separate_madd {($a:expr,$b:expr,$c:expr)=>{_mm256_add_ps(_mm256_mul_ps($a,$b),$c)}}
    macro_rules! fused_madd {($a:expr,$b:expr,$c:expr)=>{_mm256_fmadd_ps($a,$b,$c)}}
    macro_rules! body {
        ($main:ident,$condition:ident,$output:ident,$madd:ident,$fused:literal)=>{{

            unsafe {
                let half=$output.len();let full=half/8*8;
                let one=_mm256_set1_ps(1.);let zero=_mm256_setzero_ps();
                let sign_mask=_mm256_set1_ps(f32::from_bits(0x7fff_ffff));
                let exponent_mask=_mm256_set1_ps(f32::from_bits(0x7f80_0000));
                let mode=ArithmeticMode {fused_multiply_add:$fused};
                let mut exceptional_mask=0u32;
                let polynomial=|x:__m256,first:u32,second:u32,third:u32| {
                    let absolute=_mm256_and_ps(x,sign_mask);
                    let square=_mm256_mul_ps(x,x);
                    let p=$madd!(_mm256_set1_ps(f32::from_bits(third)),square,
                        _mm256_set1_ps(f32::from_bits(second)));
                    let p=$madd!(p,absolute,_mm256_set1_ps(f32::from_bits(first)));
                    $madd!(p,absolute,one)
                };
                for i in (0..full).step_by(8) {
                    let x=_mm256_add_ps(_mm256_loadu_ps($main.as_ptr().add(i)),
                        _mm256_loadu_ps($condition.as_ptr().add(i)));
                    let y=_mm256_add_ps(_mm256_loadu_ps($main.as_ptr().add(half+i)),
                        _mm256_loadu_ps($condition.as_ptr().add(half+i)));
                    let p=polynomial(x,0x3f7bfbbd,0x3f198f1d,0x3e060807);
                    let selected=_mm256_blendv_ps(p,reciprocal(p),_mm256_cmp_ps::<_CMP_LT_OQ>(x,zero));
                    let square=_mm256_mul_ps(selected,selected);
                    let left=_mm256_div_ps(_mm256_sub_ps(square,one),_mm256_add_ps(square,one));
                    let p=polynomial(y,0x3efbfc65,0x3e194af5,0x3c060568);
                    let selected=_mm256_blendv_ps(reciprocal(p),p,_mm256_cmp_ps::<_CMP_LT_OQ>(y,zero));
                    let right=reciprocal(_mm256_add_ps(_mm256_mul_ps(selected,selected),one));
                    let result=_mm256_mul_ps(left,right);
                    _mm256_storeu_ps($output.as_mut_ptr().add(i),result);

                    let exponents=_mm256_and_ps(result,exponent_mask);
                    exceptional_mask|=_mm256_movemask_ps(
                        _mm256_cmp_ps::<_CMP_EQ_OQ>(exponents,exponent_mask)) as u32;
                }
                for i in full..half {
                    let value=scalar_lane(mode,$main,$condition,i,half);
                    $output[i]=value;
                    exceptional_mask|=u32::from(value.to_bits()&0x7f80_0000==0x7f80_0000);
                }
                exceptional_mask!=0
            }
        }};
    }
    #[target_feature(enable="avx")]
    pub(super) unsafe fn separate(main:&[f32],condition:&[f32],output:&mut[f32])->bool {
        body!(main,condition,output,separate_madd,false)
    }
    #[target_feature(enable="avx,fma")]
    pub(super) unsafe fn fused(main:&[f32],condition:&[f32],output:&mut[f32])->bool {
        body!(main,condition,output,fused_madd,true)
    }
}

const ROW_LIMIT:usize=4096;
const HALF_LIMIT:usize=256;
#[repr(C)]
struct Record {half:u16,mode:u8,reserved:u8,offset:u32,ordinal:u64}
const _:()=assert!(std::mem::size_of::<Record>()==16);
pub struct GateCheck {
    records:Vec<Record>,arena:Vec<f32>,used:usize,seen:u64,
    failure:Option<&'static str>,
}
impl GateCheck {
    fn record(&mut self,mode:ArithmeticMode,main:&[f32],condition:&[f32],output:&[f32]) {
        let ordinal=self.seen;self.seen=self.seen.saturating_add(1);
        if self.failure.is_some() {return;}
        let half=output.len();
        if !(1..=HALF_LIMIT).contains(&half) || main.len()!=2*half || condition.len()!=2*half {
            self.failure=Some("Captured gate shape exceeds admission");return;
        }
        if main.iter().chain(condition).chain(output).any(|x|!x.is_finite()) {
            self.failure=Some("Nonfinite actual gate input or scalar publication");return;
        }
        let end=self.used+4*half;
        if self.records.len()==ROW_LIMIT || end>self.arena.len() {
            self.failure=Some("Gate collector truncated at fixed row/arena budget");return;
        }
        self.arena[self.used..self.used+2*half].copy_from_slice(main);
        self.arena[self.used+2*half..end].copy_from_slice(condition);
        self.records.push(Record {half:half as u16,mode:u8::from(mode.fused_multiply_add),
            reserved:0,offset:self.used as u32,ordinal});
        self.used=end;
    }

}

pub(super) fn apply_checked(mode:ArithmeticMode,main:&[f32],condition:&[f32],output:&mut[f32],
    check:Option<&mut GateCheck>)->Result<()> {
    if let Some(check)=check {
        scalar_into(mode,main,condition,output)?;
        check.record(mode,main,condition,output);
        Ok(())
    } else {apply(mode,main,condition,output)}
}
