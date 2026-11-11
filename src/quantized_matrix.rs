use crate::{Result, timbre::{f32_at, u32_at, quantized_i16_nearest_even, rounded_high_i16_product}};

mod tile64;
pub(crate) use tile64::Layout as DenseLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverwriteFold { WrapThenClamp, SaturateEach }
impl OverwriteFold {
    pub(crate) fn saturate_each(self) -> bool { self == Self::SaturateEach }
}

pub enum BatchCause {
    Extent,
    Environment(crate::matrix_f32::Error),
    Matrix(String),
}
pub struct BatchError { pub row: Option<usize>, pub cause: BatchCause }

pub struct QuantizedWorkspace {
    quant: Vec<i32>,
    acc: Vec<i32>,
}
impl QuantizedWorkspace {
    pub fn new(max_input: usize, max_output: usize) -> Result<Self> {
        if !(1..=65_536).contains(&max_input) || !(1..=65_536).contains(&max_output) {
            return Err("Invalid quantized workspace dimensions".into());
        }
        let mut quant = Vec::new();
        quant.try_reserve_exact(max_input).map_err(|error| error.to_string())?;
        quant.resize(max_input, 0);
        let mut acc = Vec::new();
        acc.try_reserve_exact(max_output).map_err(|error| error.to_string())?;
        acc.resize(max_output, 0);
        Ok(Self { quant, acc })
    }

    pub(crate) fn active_buffers(&mut self, input: usize, output: usize)
        -> Result<(&mut [i32], &mut [i32])>
    {
        if self.quant.len() < input || self.acc.len() < output {
            return Err("Quantized workspace is smaller than the matrix dimensions".into());
        }
        Ok((&mut self.quant[..input], &mut self.acc[..output]))
    }
}

#[derive(Debug)]
pub struct QuantizedMatrix {
    pub(crate) in_dim: usize, pub(crate) out_dim: usize,
    pub(crate) weights: Vec<i16>, pub(crate) scales: Vec<f32>, pub(crate) shift: u32,
    pub(crate) tile64: Option<tile64::Packed64>,
}
impl QuantizedMatrix {
    pub fn from_payload(matrix: &[u8], shift: u32) -> Result<Self> {
        parse_matrix_and_bias(matrix, None, shift).map(|(matrix, _)| matrix)
    }

    pub(crate) fn prepare_dense_layout(&mut self, layout: DenseLayout) -> Result<()> {
        self.prepare_dense_layout_using(layout, quantizer_avx2_available(),
            |weights, count| weights.try_reserve_exact(count))
    }
    fn prepare_dense_layout_using(&mut self, layout: DenseLayout, avx2: bool,
        reserve: impl FnOnce(&mut Vec<i16>, usize)
            -> std::result::Result<(), std::collections::TryReserveError>) -> Result<()>
    {

        let candidate = if layout != DenseLayout::RowMajor {
            if self.shift > 30 || self.scales.len() != self.out_dim
                || self.scales.iter().any(|x| !x.is_finite()) {
                return Err("Invalid parameters for quantized tile64 preparation".into());
            }
            tile64::Packed64::prepare_with(self.in_dim, self.out_dim, &self.weights,
                layout, avx2, reserve).map_err(|error| error.to_string())?
        } else { None };
        self.tile64 = candidate;
        Ok(())
    }
    pub fn input_width(&self) -> usize { self.in_dim }
    pub fn output_width(&self) -> usize { self.out_dim }
    pub(crate) fn values(&self) -> MatrixValues<'_> {
        MatrixValues { in_dim: self.in_dim, out_dim: self.out_dim, weights: &self.weights,
            scales: &self.scales, shift: self.shift,
            tile64: self.tile64.as_ref().filter(|p| p.matches(self.in_dim, self.out_dim)) }
    }
    pub fn forward_unbiased_saturation(&self, src: &[f32], saturate_each: bool) -> Result<Vec<f32>> {
        self.values().forward_unbiased_saturation(src, saturate_each)
    }
    pub fn forward_add_wide(&self, src: &[f32], existing: &mut [f32]) -> Result<()> {
        self.values().forward_add_wide(src, existing)
    }

    pub fn forward_add_saturating_f32(&self, src: &[f32], existing: &mut [f32]) -> Result<()> {
        self.values().forward_add_saturating_f32(src, existing)
    }

    pub fn forward_unbiased_saturation_into(&self, src: &[f32], dst: &mut [f32],
        saturate_each: bool, work: &mut QuantizedWorkspace) -> Result<()>
    {
        self.values().forward_unbiased_saturation_into(src, dst, saturate_each, work)
    }

    pub fn forward_add_saturating_f32_with_workspace(&self, src: &[f32],
        existing: &mut [f32], work: &mut QuantizedWorkspace) -> Result<()>
    {
        self.values().forward_add_saturating_f32_with_workspace(src, existing, work)
    }

    pub fn execute_read_backings(&self, input: &dyn crate::logical_read_rows::ReadRows,
        destinations: &mut [crate::vector_f32::Backing], overwrite_fold: OverwriteFold)
        -> std::result::Result<(), BatchError>
    {
        crate::matrix_batch::execute(input.row_count(), self.in_dim, self.out_dim,
            |row| input.read_row(row), destinations, BatchCause::Extent,
            |source, output| {
                let output = output.get_mut(..self.out_dim).ok_or(BatchCause::Extent)?;
                crate::matrix_f32::check_environment().map_err(BatchCause::Environment)?;
                let values = self.forward_unbiased_saturation(source, overwrite_fold.saturate_each())
                    .map_err(BatchCause::Matrix)?;
                for (word, value) in output.iter_mut().zip(values) { *word = value.to_bits(); }
                Ok(())
            }).map_err(|(row, cause)| BatchError { row, cause })
    }
}

pub(crate) fn parse_matrix_and_bias(matrix: &[u8], bias: Option<&[u8]>, shift: u32)
    -> Result<(QuantizedMatrix, Option<Vec<f32>>)> {
    if shift > 30 { return Err("quantization shift exceeds limit".into()); }
    if u32_at(matrix, 0)? != 16 || u32_at(matrix, 4)? != shift || u32_at(matrix, 8)? != 0 {
        return Err("unsupported quantized matrix metadata".into());
    }
    let out_dim = u32_at(matrix, 12)? as usize;
    if out_dim == 0 || out_dim > 65_536 { return Err("invalid out dimension".into()); }
    let dims = 16 + out_dim * 4;
    if u32_at(matrix, dims)? as usize != out_dim { return Err("matrix scale/dimension mismatch".into()); }
    let in_dim = u32_at(matrix, dims + 4)? as usize;
    if in_dim == 0 || in_dim > 65_536 { return Err("invalid in dimension".into()); }
    let weight_count = in_dim.checked_mul(out_dim).ok_or("matrix overflow")?;
    if matrix.len() != dims + 8 + weight_count * 2 { return Err("matrix payload size mismatch".into()); }
    if let Some(bias) = bias {
        if u32_at(bias, 0)? as usize != out_dim || bias.len() != 4 + out_dim * 4 {
            return Err("bias dimension mismatch".into());
        }
    }
    let mut scales = Vec::with_capacity(out_dim);
    let mut biases = bias.map(|_| Vec::with_capacity(out_dim));
    let qscale = (1_u32 << shift) as f32;
    for j in 0..out_dim {
        let scale = f32_at(matrix, 16 + j * 4)? / qscale;
        let b = bias.map(|bias| f32_at(bias, 4 + j * 4)).transpose()?;
        if !scale.is_finite() || b.is_some_and(|b| !b.is_finite()) {
            return Err("nonfinite linear parameter".into());
        }
        scales.push(scale);
        if let (Some(biases), Some(b)) = (biases.as_mut(), b) { biases.push(b); }
    }
    let weights = matrix[dims + 8..].chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
    Ok((QuantizedMatrix { in_dim, out_dim, weights, scales, shift, tile64: None }, biases))
}

fn quantize_scalar(src: &[f32], factor: f32, quant: &mut [i32]) {
    for (destination, &value) in quant.iter_mut().zip(src) {
        *destination = quantized_i16_nearest_even(value * factor);
    }
}
fn quantizer_avx2_available() -> bool {
    #[cfg(target_arch = "x86_64")]
    { std::arch::is_x86_feature_detected!("avx2") }
    #[cfg(not(target_arch = "x86_64"))]
    { false }
}
fn quantize_dispatch(src: &[f32], factor: f32, quant: &mut [i32], use_avx2: bool) {
    #[cfg(target_arch = "x86_64")]
    if use_avx2 {

        unsafe { quantize_avx2(src, factor, quant); }
        return;
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = use_avx2;
    quantize_scalar(src, factor, quant);
}
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn quantize_avx2(src: &[f32], factor: f32, quant: &mut [i32]) {
    use std::arch::x86_64::*;

    unsafe {
        let full = src.len() / 8 * 8;
        let multiplier = _mm256_set1_ps(factor);
        let lower = _mm256_set1_ps(-2_147_483_648.0);
        let upper = _mm256_set1_ps(2_147_483_648.0);
        let indefinite = _mm256_set1_epi32(i32::MIN);
        let minimum = _mm256_set1_epi32(i16::MIN as i32);
        let maximum = _mm256_set1_epi32(i16::MAX as i32);
        for i in (0..full).step_by(8) {
            let scaled = _mm256_mul_ps(_mm256_loadu_ps(src.as_ptr().add(i)), multiplier);
            let rounded = _mm256_round_ps::<{_MM_FROUND_TO_NEAREST_INT | _MM_FROUND_NO_EXC}>(scaled);
            let valid = _mm256_and_ps(_mm256_cmp_ps::<_CMP_GE_OQ>(rounded, lower),
                _mm256_cmp_ps::<_CMP_LT_OQ>(rounded, upper));

            let safe = _mm256_blendv_ps(_mm256_setzero_ps(), rounded, valid);
            let converted = _mm256_cvttps_epi32(safe);
            let converted = _mm256_blendv_epi8(indefinite, converted, _mm256_castps_si256(valid));
            let packed_values = _mm256_max_epi32(minimum, _mm256_min_epi32(maximum, converted));
            _mm256_storeu_si256(quant.as_mut_ptr().add(i).cast(), packed_values);
        }
        quantize_scalar(&src[full..], factor, &mut quant[full..]);
    }
}

pub(crate) struct MatrixValues<'a> {
    pub in_dim: usize, pub out_dim: usize, pub weights: &'a [i16],
    pub scales: &'a [f32], pub shift: u32,
    pub tile64: Option<&'a tile64::Packed64>,
}
impl MatrixValues<'_> {

    fn validate_source(&self, src: &[f32]) -> Result<()> {
        if src.len() != self.in_dim || src.iter().any(|v| !v.is_finite()) {
            return Err("linear source dimension/nonfinite error".into());
        }
        Ok(())
    }
    pub fn scratch(&self, src: &[f32], saturate_each: bool) -> Result<Vec<i32>> {

        self.validate_source(src)?;
        let mut quant = vec![0_i32; self.in_dim];
        let mut acc = vec![0_i32; self.out_dim];
        self.fill_scratch(src, saturate_each, &mut quant, &mut acc);
        Ok(acc)
    }
    fn scratch_into<'w>(&self, src: &[f32], saturate_each: bool,
        work: &'w mut QuantizedWorkspace) -> Result<&'w [i32]>
    {
        self.validate_source(src)?;
        let (quant, acc) = work.active_buffers(self.in_dim, self.out_dim)?;
        self.fill_scratch(src, saturate_each, quant, acc);
        Ok(acc)
    }

    fn fill_scratch(&self, src: &[f32], saturate_each: bool,
        quant: &mut [i32], acc: &mut [i32])
    {
        let factor = (1_u32 << self.shift) as f32;
        let use_avx2 = quantizer_avx2_available();
        quantize_dispatch(src, factor, quant, use_avx2);
        acc.fill(0);
        #[cfg(target_arch = "x86_64")]
        {
            if use_avx2 {

                unsafe {
                    if saturate_each { self.accumulate_avx2::<true>(quant, acc); }
                    else { self.accumulate_avx2::<false>(quant, acc); }
                }
            } else if saturate_each { self.accumulate_sse2::<true>(quant, acc); }
            else { self.accumulate_sse2::<false>(quant, acc); }
        }
        #[cfg(not(target_arch = "x86_64"))]
        self.accumulate_scalar(quant, acc, saturate_each);
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn accumulate_scalar(&self, quant: &[i32], acc: &mut [i32], saturate_each: bool) {
        for (i, &q) in quant.iter().enumerate() {
            for (j, a) in acc.iter_mut().enumerate() {
                let p = q * i32::from(self.weights[i * self.out_dim + j]);
                *a = a.wrapping_add(rounded_high_i16_product(p));
                if saturate_each { *a = (*a).clamp(-32768, 32767); }
            }
        }
        for a in acc { *a = (*a).clamp(-32768, 32767); }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn accumulate_avx2<const SATURATE_EACH: bool>(&self, quant: &[i32], acc: &mut [i32]) {

        unsafe {self.accumulate_avx2_tiled::<SATURATE_EACH,false>(quant,acc);}
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn accumulate_avx2_tiled<const SATURATE_EACH: bool,const WIDE_128: bool>(
        &self, quant: &[i32], acc: &mut [i32]) {
        use std::arch::x86_64::*;
        let full = self.out_dim / 16 * 16;

        unsafe {
            let lower = _mm256_set1_epi32(-32768);
            let upper = _mm256_set1_epi32(32767);
            let tiled = if SATURATE_EACH { full / 64 * 64 } else { 0 };
            let wide_tiled = if SATURATE_EACH && WIDE_128 { full / 128 * 128 } else { 0 };
            for base in (0..wide_tiled).step_by(128) {

                let mut a0 = _mm256_setzero_si256();
                let mut a1 = _mm256_setzero_si256();
                let mut a2 = _mm256_setzero_si256();
                let mut a3 = _mm256_setzero_si256();
                let mut a4 = _mm256_setzero_si256();
                let mut a5 = _mm256_setzero_si256();
                let mut a6 = _mm256_setzero_si256();
                let mut a7 = _mm256_setzero_si256();
                for (i, &value) in quant.iter().enumerate() {
                    let start = i * self.out_dim + base;
                    let weights = &self.weights[start..start + 128];
                    let q = _mm256_set1_epi16(value as i16);
                    a0 = _mm256_adds_epi16(a0, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().cast()), q));
                    a1 = _mm256_adds_epi16(a1, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(16).cast()), q));
                    a2 = _mm256_adds_epi16(a2, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(32).cast()), q));
                    a3 = _mm256_adds_epi16(a3, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(48).cast()), q));
                    a4 = _mm256_adds_epi16(a4, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(64).cast()), q));
                    a5 = _mm256_adds_epi16(a5, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(80).cast()), q));
                    a6 = _mm256_adds_epi16(a6, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(96).cast()), q));
                    a7 = _mm256_adds_epi16(a7, _mm256_mulhrs_epi16(_mm256_loadu_si256(weights.as_ptr().add(112).cast()), q));
                }

                macro_rules! store16 {
                    ($saturated:expr,$start:expr)=>{{
                        let saturated=$saturated;
                        let low=_mm256_cvtepi16_epi32(_mm256_castsi256_si128(saturated));
                        let high=_mm256_cvtepi16_epi32(_mm256_extracti128_si256::<1>(saturated));
                        let destination=&mut acc[$start..$start+16];
                        _mm256_storeu_si256(destination.as_mut_ptr().cast(),low);
                        _mm256_storeu_si256(destination.as_mut_ptr().add(8).cast(),high);
                    }};
                }
                store16!(a0,base);store16!(a1,base+16);
                store16!(a2,base+32);store16!(a3,base+48);
                store16!(a4,base+64);store16!(a5,base+80);
                store16!(a6,base+96);store16!(a7,base+112);
            }
            for base in (wide_tiled..tiled).step_by(64) {

                let mut a0 = _mm256_setzero_si256();
                let mut a1 = _mm256_setzero_si256();
                let mut a2 = _mm256_setzero_si256();
                let mut a3 = _mm256_setzero_si256();
                for (i, &value) in quant.iter().enumerate() {

                    let weights = if !WIDE_128 {
                        if let Some(packed) = self.tile64 {
                            let start = ((base / 64) * self.in_dim + i) * 64;
                            &packed.weights[start..start + 64]
                        } else {
                            let start = i * self.out_dim + base;
                            &self.weights[start..start + 64]
                        }
                    } else {
                        let start = i * self.out_dim + base;
                        &self.weights[start..start + 64]
                    };
                    let q = _mm256_set1_epi16(value as i16);
                    let w0 = _mm256_loadu_si256(weights.as_ptr().cast());
                    let w1 = _mm256_loadu_si256(weights.as_ptr().add(16).cast());
                    let w2 = _mm256_loadu_si256(weights.as_ptr().add(32).cast());
                    let w3 = _mm256_loadu_si256(weights.as_ptr().add(48).cast());
                    a0 = _mm256_adds_epi16(a0, _mm256_mulhrs_epi16(w0, q));
                    a1 = _mm256_adds_epi16(a1, _mm256_mulhrs_epi16(w1, q));
                    a2 = _mm256_adds_epi16(a2, _mm256_mulhrs_epi16(w2, q));
                    a3 = _mm256_adds_epi16(a3, _mm256_mulhrs_epi16(w3, q));
                }
                let output = &mut acc[base..base + 64];
                for (saturated, destination) in [a0, a1, a2, a3].into_iter()
                    .zip(output.chunks_exact_mut(16))
                {
                    let low = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(saturated));
                    let high = _mm256_cvtepi16_epi32(_mm256_extracti128_si256::<1>(saturated));
                    _mm256_storeu_si256(destination.as_mut_ptr().cast(), low);
                    _mm256_storeu_si256(destination.as_mut_ptr().add(8).cast(), high);
                }
            }
            for base in (tiled..full).step_by(16) {
                let mut low = _mm256_setzero_si256();
                let mut high = _mm256_setzero_si256();
                let mut saturated = _mm256_setzero_si256();
                for (i, &value) in quant.iter().enumerate() {
                    let start = i * self.out_dim + base;
                    let weights = &self.weights[start..start + 16];
                    let w = _mm256_loadu_si256(weights.as_ptr().cast());
                    let q = _mm256_set1_epi16(value as i16);

                    let product = _mm256_mulhrs_epi16(w, q);
                    if SATURATE_EACH {

                        saturated = _mm256_adds_epi16(saturated, product);
                    } else {
                        low = _mm256_add_epi32(low, _mm256_cvtepi16_epi32(_mm256_castsi256_si128(product)));
                        high = _mm256_add_epi32(high, _mm256_cvtepi16_epi32(_mm256_extracti128_si256::<1>(product)));
                    }
                }
                if SATURATE_EACH {
                    low = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(saturated));
                    high = _mm256_cvtepi16_epi32(_mm256_extracti128_si256::<1>(saturated));
                } else {
                    low = _mm256_max_epi32(lower, _mm256_min_epi32(upper, low));
                    high = _mm256_max_epi32(lower, _mm256_min_epi32(upper, high));
                }
                let output = &mut acc[base..base + 16];
                _mm256_storeu_si256(output.as_mut_ptr().cast(), low);
                _mm256_storeu_si256(output.as_mut_ptr().add(8).cast(), high);
            }
        }
        for (j, a) in acc.iter_mut().enumerate().skip(full) {
            for (i, &q) in quant.iter().enumerate() {
                let p = q * i32::from(self.weights[i * self.out_dim + j]);
                *a = a.wrapping_add(rounded_high_i16_product(p));
                if SATURATE_EACH { *a = (*a).clamp(-32768, 32767); }
            }
            *a = (*a).clamp(-32768, 32767);
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn accumulate_sse2<const SATURATE_EACH: bool>(&self, quant: &[i32], acc: &mut [i32]) {
        use std::arch::x86_64::*;
        let full = self.out_dim / 16 * 16;

        unsafe {
            let zero = _mm_setzero_si128();
            let rounding = _mm_set1_epi32(0x4000);
            for base in (0..full).step_by(16) {
                let mut sums = [zero; 4];
                for (i, &value) in quant.iter().enumerate() {
                    let q = _mm_set1_epi32(value & 0xffff);
                    let start = i * self.out_dim + base;
                    let weights = &self.weights[start..start + 16];
                    let w0 = _mm_loadu_si128(weights.as_ptr().cast());
                    let w1 = _mm_loadu_si128(weights.as_ptr().add(8).cast());
                    let products = [
                        _mm_madd_epi16(_mm_unpacklo_epi16(w0, zero), q),
                        _mm_madd_epi16(_mm_unpackhi_epi16(w0, zero), q),
                        _mm_madd_epi16(_mm_unpacklo_epi16(w1, zero), q),
                        _mm_madd_epi16(_mm_unpackhi_epi16(w1, zero), q),
                    ];
                    for (sum, product) in sums.iter_mut().zip(products) {
                        let rounded = _mm_srai_epi32::<15>(_mm_add_epi32(product, rounding));

                        let narrowed = _mm_srai_epi32::<16>(_mm_slli_epi32::<16>(rounded));
                        *sum = _mm_add_epi32(*sum, narrowed);
                        if SATURATE_EACH {
                            let packed = _mm_packs_epi32(*sum, *sum);
                            *sum = _mm_unpacklo_epi16(packed, _mm_srai_epi16::<15>(packed));
                        }
                    }
                }

                let output = &mut acc[base..base + 16];
                for (pair, destination) in sums.chunks_exact(2).zip(output.chunks_exact_mut(8)) {
                    let packed = _mm_packs_epi32(pair[0], pair[1]);
                    let sign = _mm_srai_epi16::<15>(packed);
                    _mm_storeu_si128(destination.as_mut_ptr().cast(), _mm_unpacklo_epi16(packed, sign));
                    _mm_storeu_si128(destination.as_mut_ptr().add(4).cast(), _mm_unpackhi_epi16(packed, sign));
                }
            }
        }
        for (j, a) in acc.iter_mut().enumerate().skip(full) {
            for (i, &q) in quant.iter().enumerate() {
                let p = q * i32::from(self.weights[i * self.out_dim + j]);
                *a = a.wrapping_add(rounded_high_i16_product(p));
                if SATURATE_EACH { *a = (*a).clamp(-32768, 32767); }
            }
            *a = (*a).clamp(-32768, 32767);
        }
    }
    pub fn forward_unbiased_saturation(&self, src: &[f32], saturate_each: bool) -> Result<Vec<f32>> {
        let acc = self.scratch(src, saturate_each)?;
        Ok(acc.iter().enumerate().map(|(j, &a)| a as f32 * self.scales[j]).collect())
    }
    fn forward_unbiased_saturation_into(&self, src: &[f32], dst: &mut [f32],
        saturate_each: bool, work: &mut QuantizedWorkspace) -> Result<()>
    {
        if dst.len() != self.out_dim { return Err("overwrite destination width mismatch".into()); }
        let scratch = self.scratch_into(src, saturate_each, work)?;
        for j in 0..self.out_dim {
            dst[j] = scratch[j] as f32 * self.scales[j];
        }
        Ok(())
    }
    fn forward_add_saturating_f32(&self, src: &[f32], existing: &mut [f32]) -> Result<()> {
        if existing.len() != self.out_dim { return Err("additive destination width mismatch".into()); }
        let scratch = self.scratch(src, true)?;
        self.add_scaled(&scratch, existing);
        Ok(())
    }
    fn forward_add_saturating_f32_with_workspace(&self, src: &[f32],
        existing: &mut [f32], work: &mut QuantizedWorkspace) -> Result<()>
    {
        if existing.len() != self.out_dim { return Err("additive destination width mismatch".into()); }
        let scratch = self.scratch_into(src, true, work)?;
        self.add_scaled(scratch, existing);
        Ok(())
    }
    fn add_scaled(&self, scratch: &[i32], existing: &mut [f32]) {
        for j in 0..self.out_dim {
            let scaled = scratch[j] as f32 * self.scales[j];
            existing[j] = scaled + existing[j];
        }
    }

    pub fn forward_add_wide(&self, src: &[f32], existing: &mut [f32]) -> Result<()> {
        if existing.len() != self.out_dim { return Err("additive destination width mismatch".into()); }
        let scratch = self.scratch(src, false)?;
        for j in 0..self.out_dim {
            existing[j] = (f64::from(existing[j]) + f64::from(scratch[j]) * f64::from(self.scales[j])) as f32;
        }
        Ok(())
    }
}

