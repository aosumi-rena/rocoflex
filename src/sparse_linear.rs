use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    timbre::{f32_at, quantized_i16_nearest_even, u32_at},
};

pub struct SparseLinear {
    pub in_dim: usize,
    pub out_dim: usize,
    pub bias: Vec<f32>,
    shift: u32,
    scales: Vec<f32>,
    columns: Vec<usize>,
    rowptr: Vec<usize>,
    weights: Vec<i16>,
}

const PRIM5: &[u64] = &[
    0x6bc221d6c85175d2,
    0x18d43fa2703955c4,
    0xc8998f352221e0fb,
    0x4dec2118e5aeeb60,
    0xb9d95948b0cbf654,
    0x99d85faf2504d90b,
    0xaabd01a51bf7c05e,
    0x188609fae3aa3ce6,
    0x624733cb4720f774,
    0x8e50c76ab90b1783,
    0x215573387f980a11,
    0xbfe9967ae4624f25,
];

impl SparseLinear {
    pub(crate) fn from_nodes(matrix: &Node<'_>, bias: &Node<'_>) -> Result<Self> {
        matrix.require(0x7fca40ff, b"prim5", PRIM5)?;
        bias.require(
            0x7fca40ff,
            b"prim1",
            crate::model_tags::decoded_hashes("prim1"),
        )?;
        if !matrix.children.is_empty() || !bias.children.is_empty() {
            return Err("sparse parameters must be leaves".into());
        }
        Self::from_payloads(matrix.data, bias.data)
    }

    pub fn from_payloads(matrix: &[u8], bias: &[u8]) -> Result<Self> {
        let shift = u32_at(matrix, 4)?;
        if u32_at(matrix, 0)? != 16 || shift > 30 || u32_at(matrix, 8)? != 0 {
            return Err("unsupported sparse quantization header".into());
        }
        let out_dim = u32_at(matrix, 12)? as usize;
        if out_dim == 0 || out_dim > 65536 || out_dim % 16 != 0 {
            return Err("invalid sparse output dimensions".into());
        }
        let sub = 16 + out_dim * 4;
        let in_dim = u32_at(matrix, sub + 4)? as usize;
        if u32_at(matrix, sub)? as usize != out_dim
            || in_dim == 0
            || in_dim > 32768
            || u32_at(matrix, sub + 8)? != 16
            || u32_at(matrix, sub + 12)? != 1
        {
            return Err("unsupported sparse block geometry".into());
        }
        let blocks = u32_at(matrix, sub + 16)? as usize;
        if blocks > (out_dim / 16) * in_dim {
            return Err("sparse block count exceeds matrix".into());
        }
        let co = sub + 20;
        let ro = co + blocks * 2;
        let data = ro + (out_dim / 16 + 1) * 4;
        if matrix.len() != data + blocks * 32
            || bias.len() != 4 + out_dim * 4
            || u32_at(bias, 0)? as usize != out_dim
        {
            return Err("sparse payload extent/bias mismatch".into());
        }
        let columns = matrix[co..ro]
            .chunks_exact(2)
            .map(|v| u16::from_le_bytes(v.try_into().unwrap()) as usize)
            .collect::<Vec<_>>();
        if columns.iter().any(|&c| c >= in_dim) {
            return Err("sparse column out of bounds".into());
        }
        let rowptr = (0..=out_dim / 16)
            .map(|i| u32_at(matrix, ro + i * 4).map(|v| v as usize))
            .collect::<Result<Vec<_>>>()?;
        if rowptr[0] != 0
            || *rowptr.last().unwrap() != blocks
            || rowptr.windows(2).any(|v| v[0] > v[1] || v[1] > blocks)
        {
            return Err("invalid sparse row pointers".into());
        }
        for span in rowptr.windows(2) {
            if columns[span[0]..span[1]].windows(2).any(|v| v[0] >= v[1]) {
                return Err("sparse columns must be strictly ordered".into());
            }
        }
        let scales = (0..out_dim)
            .map(|i| f32_at(matrix, 16 + i * 4).map(|v| v / (1_u32 << shift) as f32))
            .collect::<Result<Vec<_>>>()?;
        let biases = (0..out_dim)
            .map(|i| f32_at(bias, 4 + i * 4))
            .collect::<Result<Vec<_>>>()?;
        if scales.iter().chain(&biases).any(|v| !v.is_finite()) {
            return Err("sparse parameters must be finite".into());
        }
        let weights = matrix[data..]
            .chunks_exact(2)
            .map(|v| i16::from_le_bytes(v.try_into().unwrap()))
            .collect();
        Ok(Self {
            in_dim,
            out_dim,
            bias: biases,
            shift,
            scales,
            columns,
            rowptr,
            weights,
        })
    }

    fn validate_source(&self, src: &[f32]) -> Result<()> {
        if src.len() != self.in_dim || src.iter().any(|v| !v.is_finite()) {
            return Err("invalid sparse source".into());
        }
        Ok(())
    }

    fn quantized_values<'a>(&'a self, src: &'a [f32]) -> impl Iterator<Item = i32> + 'a {
        let factor = (1_u32 << self.shift) as f32;
        src.iter().map(move |v| quantized_i16_nearest_even(v * factor))
    }

    pub fn scratch(&self, src: &[f32]) -> Result<Vec<i32>> {
        self.validate_source(src)?;
        let quant = self.quantized_values(src).collect::<Vec<_>>();
        let mut acc = vec![0_i32; self.out_dim];
        self.accumulate(&quant, &mut acc);
        Ok(acc)
    }

    fn scratch_into<'w>(&self, src: &[f32],
        work: &'w mut crate::quantized_matrix::QuantizedWorkspace) -> Result<&'w [i32]>
    {
        self.validate_source(src)?;
        let (quant, acc) = work.active_buffers(self.in_dim, self.out_dim)?;
        for (target, value) in quant.iter_mut().zip(self.quantized_values(src)) {
            *target = value;
        }
        acc.fill(0);
        self.accumulate(quant, acc);
        Ok(acc)
    }

    fn accumulate(&self, quant: &[i32], acc: &mut [i32]) {
        #[cfg(target_arch = "x86_64")]
        {
            if std::is_x86_feature_detected!("avx2") {

                unsafe { self.accumulate_avx2(quant, acc) };
            } else {
                self.accumulate_sse2(quant, acc);
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        self.accumulate_scalar(quant, acc);
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn accumulate_scalar(&self, quant: &[i32], acc: &mut [i32]) {
        for br in 0..self.out_dim / 16 {
            let start = self.rowptr[br];
            let mut partial = [[0_i16; 16]; 4];
            for k in start..self.rowptr[br + 1] {
                let q = quant[self.columns[k]];
                let part = &mut partial[(k - start) & 3];
                for lane in 0..16 {
                    let p = q * i32::from(self.weights[k * 16 + lane]);
                    let product = crate::timbre::rounded_high_i16_product(p) as i16;
                    part[lane] = part[lane].saturating_add(product);
                }
            }
            for lane in 0..16 {
                let left = partial[0][lane].saturating_add(partial[1][lane]);
                let right = partial[2][lane].saturating_add(partial[3][lane]);
                acc[br * 16 + lane] = i32::from(left.saturating_add(right));
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn accumulate_sse2(&self, quant: &[i32], acc: &mut [i32]) {
        use std::arch::x86_64::*;

        unsafe {
            let zero = _mm_setzero_si128();
            let rounding = _mm_set1_epi32(0x4000);
            for br in 0..self.out_dim / 16 {
                let start = self.rowptr[br];
                let mut low = [zero; 4];
                let mut high = [zero; 4];
                for k in start..self.rowptr[br + 1] {
                    let q = _mm_set1_epi32(quant[self.columns[k]] & 0xffff);
                    let weights = self.weights.as_ptr().add(k * 16);
                    let w0 = _mm_loadu_si128(weights.cast());
                    let w1 = _mm_loadu_si128(weights.add(8).cast());
                    let p0 = _mm_madd_epi16(_mm_unpacklo_epi16(w0, zero), q);
                    let p1 = _mm_madd_epi16(_mm_unpackhi_epi16(w0, zero), q);
                    let p2 = _mm_madd_epi16(_mm_unpacklo_epi16(w1, zero), q);
                    let p3 = _mm_madd_epi16(_mm_unpackhi_epi16(w1, zero), q);
                    let r0 = _mm_srai_epi32::<16>(_mm_slli_epi32::<16>(
                        _mm_srai_epi32::<15>(_mm_add_epi32(p0, rounding)),
                    ));
                    let r1 = _mm_srai_epi32::<16>(_mm_slli_epi32::<16>(
                        _mm_srai_epi32::<15>(_mm_add_epi32(p1, rounding)),
                    ));
                    let r2 = _mm_srai_epi32::<16>(_mm_slli_epi32::<16>(
                        _mm_srai_epi32::<15>(_mm_add_epi32(p2, rounding)),
                    ));
                    let r3 = _mm_srai_epi32::<16>(_mm_slli_epi32::<16>(
                        _mm_srai_epi32::<15>(_mm_add_epi32(p3, rounding)),
                    ));
                    let part = (k - start) & 3;
                    low[part] = _mm_adds_epi16(low[part], _mm_packs_epi32(r0, r1));
                    high[part] = _mm_adds_epi16(high[part], _mm_packs_epi32(r2, r3));
                }
                let low = _mm_adds_epi16(
                    _mm_adds_epi16(low[0], low[1]),
                    _mm_adds_epi16(low[2], low[3]),
                );
                let high = _mm_adds_epi16(
                    _mm_adds_epi16(high[0], high[1]),
                    _mm_adds_epi16(high[2], high[3]),
                );
                let low_sign = _mm_srai_epi16::<15>(low);
                let high_sign = _mm_srai_epi16::<15>(high);
                let output = acc.as_mut_ptr().add(br * 16);
                _mm_storeu_si128(output.cast(), _mm_unpacklo_epi16(low, low_sign));
                _mm_storeu_si128(output.add(4).cast(), _mm_unpackhi_epi16(low, low_sign));
                _mm_storeu_si128(output.add(8).cast(), _mm_unpacklo_epi16(high, high_sign));
                _mm_storeu_si128(output.add(12).cast(), _mm_unpackhi_epi16(high, high_sign));
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn accumulate_avx2(&self, quant: &[i32], acc: &mut [i32]) {
        use std::arch::x86_64::*;

        unsafe {
            for br in 0..self.out_dim / 16 {
                let start = self.rowptr[br];
                let end = self.rowptr[br + 1];
                let mut p0 = _mm256_setzero_si256();
                let mut p1 = _mm256_setzero_si256();
                let mut p2 = _mm256_setzero_si256();
                let mut p3 = _mm256_setzero_si256();
                let mut k = start;
                while end - k >= 4 {
                    let q0 = _mm256_set1_epi16(quant[self.columns[k]] as i16);
                    let q1 = _mm256_set1_epi16(quant[self.columns[k + 1]] as i16);
                    let q2 = _mm256_set1_epi16(quant[self.columns[k + 2]] as i16);
                    let q3 = _mm256_set1_epi16(quant[self.columns[k + 3]] as i16);
                    let w0 = _mm256_loadu_si256(self.weights.as_ptr().add(k * 16).cast());
                    let w1 = _mm256_loadu_si256(self.weights.as_ptr().add((k + 1) * 16).cast());
                    let w2 = _mm256_loadu_si256(self.weights.as_ptr().add((k + 2) * 16).cast());
                    let w3 = _mm256_loadu_si256(self.weights.as_ptr().add((k + 3) * 16).cast());
                    p0 = _mm256_adds_epi16(p0, _mm256_mulhrs_epi16(w0, q0));
                    p1 = _mm256_adds_epi16(p1, _mm256_mulhrs_epi16(w1, q1));
                    p2 = _mm256_adds_epi16(p2, _mm256_mulhrs_epi16(w2, q2));
                    p3 = _mm256_adds_epi16(p3, _mm256_mulhrs_epi16(w3, q3));
                    k += 4;
                }
                if k < end {
                    let q = _mm256_set1_epi16(quant[self.columns[k]] as i16);
                    let w = _mm256_loadu_si256(self.weights.as_ptr().add(k * 16).cast());
                    p0 = _mm256_adds_epi16(p0, _mm256_mulhrs_epi16(w, q));
                    k += 1;
                }
                if k < end {
                    let q = _mm256_set1_epi16(quant[self.columns[k]] as i16);
                    let w = _mm256_loadu_si256(self.weights.as_ptr().add(k * 16).cast());
                    p1 = _mm256_adds_epi16(p1, _mm256_mulhrs_epi16(w, q));
                    k += 1;
                }
                if k < end {
                    let q = _mm256_set1_epi16(quant[self.columns[k]] as i16);
                    let w = _mm256_loadu_si256(self.weights.as_ptr().add(k * 16).cast());
                    p2 = _mm256_adds_epi16(p2, _mm256_mulhrs_epi16(w, q));
                }
                let result = _mm256_adds_epi16(_mm256_adds_epi16(p0, p1), _mm256_adds_epi16(p2, p3));
                let low = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(result));
                let high = _mm256_cvtepi16_epi32(_mm256_extracti128_si256::<1>(result));
                let output = acc.as_mut_ptr().add(br * 16);
                _mm256_storeu_si256(output.cast(), low);
                _mm256_storeu_si256(output.add(8).cast(), high);
            }
        }
    }

    pub fn forward(&self, src: &[f32]) -> Result<Vec<f32>> {
        Ok(self
            .scratch(src)?
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let scaled = *a as f32 * self.scales[i];
                scaled + self.bias[i]
            })
            .collect())
    }

    pub fn forward_add(&self, src: &[f32], dest: &mut [f32]) -> Result<()> {
        if dest.len() != self.out_dim {
            return Err("sparse destination mismatch".into());
        }
        self.add_scratch(&self.scratch(src)?, dest);
        Ok(())
    }

    pub fn forward_add_with_workspace(&self, src: &[f32], dest: &mut [f32],
        work: &mut crate::quantized_matrix::QuantizedWorkspace) -> Result<()>
    {
        if dest.len() != self.out_dim {
            return Err("sparse destination mismatch".into());
        }
        self.add_scratch(self.scratch_into(src, work)?, dest);
        Ok(())
    }

    fn add_scratch(&self, acc: &[i32], dest: &mut [f32]) {
        for (i, a) in acc.iter().enumerate() {
            let scaled = *a as f32 * self.scales[i];
            dest[i] = scaled + dest[i];
        }
    }
}

pub fn pitch_bypass(model: &[u8], block: usize) -> Result<SparseLinear> {
    if block > 1 || u32_at(model, 0)? != 0x7fca00ff || u32_at(model, 4)? != 2 {
        return Err("pitch bypass requires qualified v2 model/block".into());
    }
    let tree = ModelTree::parse(model)?;
    let root = tree.root;
    let pitch = root.children.get(1).ok_or("missing pitch frontend")?;
    let seq = pitch
        .children
        .get(7)
        .ok_or("missing pitch temporal stack")?;
    seq.require(
        0x7fca41ff,
        b"modm0",
        crate::model_tags::decoded_hashes("modm0"),
    )?;
    if seq.children.len() != 2 || !seq.data.is_empty() {
        return Err("pitch stack layout".into());
    }
    let wrapper = &seq.children[block];
    let expected_tag = [1355573478173270741_u64, 2302632375832338006][block];
    let dims = [(384, 1024), (1024, 384)][block];
    wrapper.require(
        0x7fca41ff,
        b"",
        &[expected_tag ^ 0xf22ee5adeedbda94 ^ 0xb2f45c29815fb08d],
    )?;
    if wrapper.children.len() != 2
        || wrapper.data.len() != 8
        || u32_at(wrapper.data, 0)? != dims.0
        || u32_at(wrapper.data, 4)? != dims.1
    {
        return Err("pitch residual wrapper mismatch".into());
    }
    let linear = &wrapper.children[1];
    linear.require(
        0x7fca41ff,
        b"modl0",
        crate::model_tags::decoded_hashes("modl0"),
    )?;
    if linear.children.len() != 2 || !linear.data.is_empty() {
        return Err("pitch bypass layout".into());
    }
    let q = SparseLinear::from_nodes(&linear.children[0], &linear.children[1])?;
    if (q.in_dim, q.out_dim) != (dims.0 as usize, dims.1 as usize) {
        return Err("pitch bypass shape mismatch".into());
    }
    Ok(q)
}

