use crate::Result;

pub(crate) fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset.checked_add(4).ok_or("offset overflow")?)
        .ok_or("truncated u32")?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

pub(crate) fn f32_at(data: &[u8], offset: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(data, offset)?))
}

pub(crate) fn rounded_high_i16_product(product: i32) -> i32 {
    (((product + 0x4000) >> 15) as i16) as i32
}

pub(crate) fn quantized_i16_nearest_even(scaled: f32) -> i32 {
    let rounded = f64::from(scaled).round_ties_even();
    let converted = if rounded.is_finite() && (-2_147_483_648.0..2_147_483_648.0).contains(&rounded)
    {
        rounded as i32
    } else {
        i32::MIN
    };
    converted.clamp(i16::MIN as i32, i16::MAX as i32)
}

#[derive(Debug)]
pub struct QuantizedLinear {
    pub in_dim: usize,
    pub out_dim: usize,
    weights: Vec<i16>,
    scales: Vec<f32>,
    pub(crate) bias: Vec<f32>,
    shift: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantizedParameterOwnerError { Dimensions, Shift, Extent, Nonfinite }

impl QuantizedLinear {

    pub(crate) fn into_parameter_owners(self)
        -> std::result::Result<(crate::quantized_matrix::QuantizedMatrix, Vec<f32>),
            (QuantizedParameterOwnerError, Self)>
    {
        let error = if self.in_dim == 0 || self.out_dim == 0
            || self.in_dim > 65_536 || self.out_dim > 65_536 {
            Some(QuantizedParameterOwnerError::Dimensions)
        } else if self.shift > 30 {
            Some(QuantizedParameterOwnerError::Shift)
        } else if self.in_dim.checked_mul(self.out_dim) != Some(self.weights.len())
            || self.scales.len() != self.out_dim || self.bias.len() != self.out_dim {
            Some(QuantizedParameterOwnerError::Extent)
        } else if self.scales.iter().chain(&self.bias).any(|value| !value.is_finite()) {
            Some(QuantizedParameterOwnerError::Nonfinite)
        } else { None };
        if let Some(error) = error { return Err((error, self)); }
        let Self { in_dim, out_dim, weights, scales, bias, shift } = self;
        Ok((crate::quantized_matrix::QuantizedMatrix { in_dim, out_dim, weights, scales, shift, tile64: None }, bias))
    }

    pub fn from_payloads(matrix: &[u8], bias: &[u8], shift: u32) -> Result<Self> {
        let (matrix, bias) = crate::quantized_matrix::parse_matrix_and_bias(matrix, Some(bias), shift)?;
        let crate::quantized_matrix::QuantizedMatrix { in_dim, out_dim, weights, scales, shift, tile64: _ } = matrix;
        Ok(Self { in_dim, out_dim, weights, scales, shift,
            bias: bias.expect("supplied bias validated by shared parser") })
    }

    fn matrix_values(&self) -> crate::quantized_matrix::MatrixValues<'_> {
        crate::quantized_matrix::MatrixValues { in_dim: self.in_dim, out_dim: self.out_dim,
            weights: &self.weights, scales: &self.scales, shift: self.shift, tile64: None }
    }

    pub fn forward(&self, src: &[f32]) -> Result<Vec<f32>> {
        self.forward_saturation(src, false)
    }

    pub fn forward_unbiased(&self, src: &[f32]) -> Result<Vec<f32>> {
        self.forward_unbiased_saturation(src, false)
    }

    pub fn forward_unbiased_saturation(
        &self,
        src: &[f32],
        saturate_each: bool,
    ) -> Result<Vec<f32>> {
        self.matrix_values().forward_unbiased_saturation(src, saturate_each)
    }

    pub fn forward_saturation(&self, src: &[f32], saturate_each: bool) -> Result<Vec<f32>> {
        let acc = self.scratch(src, saturate_each)?;
        Ok(acc
            .iter()
            .enumerate()
            .map(|(j, &a)| {
                let scaled = a as f32 * self.scales[j];
                scaled + self.bias[j]
            })
            .collect())
    }

    fn scratch(&self, src: &[f32], saturate_each: bool) -> Result<Vec<i32>> {
        self.matrix_values().scratch(src, saturate_each)
    }

    pub fn forward_add_wide(&self, src: &[f32], existing: &mut [f32]) -> Result<()> {
        self.matrix_values().forward_add_wide(src, existing)
    }
}

pub fn runtime_rcpps(value: f32) -> f32 {
    let bits = value.to_bits();
    let exp = ((bits >> 23) & 255) as i32;
    if exp == 0 || exp == 255 {
        return 1.0 / value;
    }
    let index = ((bits & 0x7fffff) >> 12) as f32;
    let base = if index == 2047.0 {
        4097.0
    } else {
        (8192.0_f32 / (1.0_f32 + (index + 0.5_f32) / 2048.0_f32)).round_ties_even()
    } / 8192.0_f32;
    let out = (f64::from(base) * 2.0_f64.powi(127 - exp)) as f32;
    f32::from_bits(out.to_bits() | (bits & 0x80000000))
}

pub fn runtime_sigmoid(value: f32) -> f32 {
    let abs = value.abs();
    let abs2 = value * value;
    let p = abs2.mul_add(0.008179999887943268_f32, 0.14970000088214874_f32);
    let p = abs.mul_add(p, 0.49215999245643616_f32);
    let p = abs.mul_add(p, 1.0_f32);
    let selected = if value < 0.0 { p } else { runtime_rcpps(p) };
    runtime_rcpps(1.0_f32 + selected * selected)
}

pub fn runtime_silu(value: f32) -> f32 {
    value * runtime_sigmoid(value)
}

#[derive(Debug)]
pub struct TimbreHead {
    first: QuantizedLinear,
    last: QuantizedLinear,
}

pub struct TimbreStages {
    pub hidden0: Vec<f32>,
    pub hidden1: Vec<f32>,
    pub out8: Vec<f32>,
}

impl TimbreHead {
    pub(crate) fn into_linear_owners(self) -> (QuantizedLinear, QuantizedLinear) {
        (self.first, self.last)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = crate::model::ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported timbre model version".into());
        }
        let branch = tree
            .root
            .children
            .get(3)
            .and_then(|node| node.children.first())
            .ok_or("Timbre projection is missing")?;
        branch.require(
            0x7fca41ff,
            b"modm0",
            crate::model_tags::decoded_hashes("modm0"),
        )?;
        if branch.kind != 0x7fca41ff || !branch.data.is_empty() || branch.children.len() != 3 {
            return Err("Unsupported timbre projection layout".into());
        }
        let activation = &branch.children[1];
        if activation.kind != 0x7fca41ff
            || activation.tag != 0x854ffc91db215b7a
            || !activation.data.is_empty()
            || !activation.children.is_empty()
        {
            return Err("Unsupported timbre activation".into());
        }
        fn linear(node: &crate::model::ModelNode<'_>) -> Result<QuantizedLinear> {
            node.require(
                0x7fca41ff,
                b"modl0",
                crate::model_tags::decoded_hashes("modl0"),
            )?;
            if node.kind != 0x7fca41ff || !node.data.is_empty() || node.children.len() != 2 {
                return Err("Unsupported linear layout".into());
            }
            let matrix = &node.children[0];
            let bias = &node.children[1];
            matrix.require(
                0x7fca40ff,
                b"prim4",
                crate::model_tags::decoded_hashes("prim4"),
            )?;
            bias.require(
                0x7fca40ff,
                b"prim1",
                crate::model_tags::decoded_hashes("prim1"),
            )?;
            if matrix.kind != 0x7fca40ff
                || bias.kind != 0x7fca40ff
                || !matrix.children.is_empty()
                || !bias.children.is_empty()
            {
                return Err("Unsupported linear parameter layout".into());
            }
            QuantizedLinear::from_payloads(matrix.data, bias.data, 13)
        }
        Self::from_linears(linear(&branch.children[0])?, linear(&branch.children[2])?)
    }

    pub fn from_linears(first: QuantizedLinear, last: QuantizedLinear) -> Result<Self> {
        if (first.in_dim, first.out_dim, last.in_dim, last.out_dim) != (11, 128, 128, 8) {
            return Err("expected head dimensions 11->128->8".into());
        }
        Ok(Self { first, last })
    }

    pub fn forward(&self, entry11: &[f32]) -> Result<TimbreStages> {
        let hidden0 = self.first.forward_saturation(entry11, true)?;
        let hidden1 = hidden0.iter().map(|&v| runtime_silu(v)).collect::<Vec<_>>();
        let out8 = self.last.forward_saturation(&hidden1, true)?;
        Ok(TimbreStages {
            hidden0,
            hidden1,
            out8,
        })
    }

    pub fn forward_vec90(&self, rows72: &[f32]) -> Result<TimbreStages> {
        if rows72.is_empty() || rows72.len() % 72 != 0 || rows72.iter().any(|v| !v.is_finite()) {
            return Err("expected nonempty finite rows x72".into());
        }
        let mut maxima = [f32::NEG_INFINITY; 72];
        for row in rows72.chunks_exact(72) {
            for (a, &v) in maxima.iter_mut().zip(row) {
                if v > *a {
                    *a = v;
                }
            }
        }
        let prefix = maxima[..60]
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let entry = maxima[60..71]
            .iter()
            .map(|v| v - prefix)
            .collect::<Vec<_>>();
        self.forward(&entry)
    }
}
