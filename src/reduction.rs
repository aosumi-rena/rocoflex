use crate::timbre::{f32_at, runtime_rcpps, runtime_silu, u32_at};
use crate::{Result, model::ModelNode as Node};

#[derive(Debug)]
pub struct FloatLinear {
    pub in_dim: usize,
    pub out_dim: usize,
    weights: Vec<f32>,
    bias: Vec<f32>,
}

impl FloatLinear {

    pub fn bias(&self) -> &[f32] { &self.bias }

    pub fn matrix_from_explicit_plan(&self, plan: crate::matrix_policy::Plan,
        backing: crate::matrix_policy::CoefficientBacking)
        -> std::result::Result<crate::matrix_policy::BoundMatrix, crate::matrix_policy::BindingFailure>
    {
        if plan.request() != crate::matrix_policy::Request::FloatParameters {
            return Err(crate::matrix_policy::BindingFailure {
                error: crate::matrix_policy::BindingError::RequestKind, plan, backing,
            });
        }
        if plan.dimensions() != (self.out_dim, self.in_dim) {
            return Err(crate::matrix_policy::BindingFailure {
                error: crate::matrix_policy::BindingError::LogicalDimensions, plan, backing,
            });
        }
        let logical = self.weights.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        plan.bind_row_major(&logical, backing)
    }

    pub fn half_matrix_with_backing(&self, policy: crate::matrix_half::CachePolicy,
        all_initial_words: Vec<u16>)
        -> std::result::Result<crate::matrix_half::Matrix, crate::matrix_half::BindingFailure>
    {
        let logical = self.weights.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        crate::matrix_half::Matrix::from_row_major_bits(self.out_dim, self.in_dim,
            &logical, policy, all_initial_words)
    }

    pub fn generic_f32_matrix_with_backing(&self, policy: crate::matrix_f32::CachePolicy,
        all_initial_words: Vec<u32>)
        -> std::result::Result<crate::matrix_f32::Matrix, crate::matrix_f32::BindingFailure>
    {
        let logical = self.weights.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        crate::matrix_f32::Matrix::from_row_major_bits(self.out_dim, self.in_dim,
            &logical, policy, all_initial_words)
    }

    pub fn from_payloads(matrix: &[u8], bias: &[u8]) -> Result<Self> {
        let out_dim = u32_at(matrix, 0)? as usize;
        let in_dim = u32_at(matrix, 4)? as usize;
        if out_dim == 0 || in_dim == 0 || out_dim > 65_536 || in_dim > 65_536 {
            return Err("invalid float matrix dimensions".into());
        }
        let count = out_dim.checked_mul(in_dim).ok_or("float matrix overflow")?;
        if matrix.len() != 8 + count * 4
            || bias.len() != 4 + out_dim * 4
            || u32_at(bias, 0)? as usize != out_dim
        {
            return Err("float matrix/bias payload size mismatch".into());
        }
        let weights = matrix[8..]
            .chunks_exact(4)
            .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
            .collect::<Vec<_>>();
        let biases = (0..out_dim)
            .map(|j| f32_at(bias, 4 + j * 4))
            .collect::<Result<Vec<_>>>()?;
        if weights.iter().chain(&biases).any(|v| !v.is_finite()) {
            return Err("nonfinite float parameter".into());
        }
        Ok(Self {
            in_dim,
            out_dim,
            weights,
            bias: biases,
        })
    }

    pub fn forward_stages(&self, input: &[f32]) -> Result<(Vec<f32>, Vec<f32>)> {
        if input.len() != self.in_dim || input.iter().any(|v| !v.is_finite()) {
            return Err("float source dimension/nonfinite error".into());
        }
        let mut matrix = vec![0.0_f32; self.out_dim];
        for (j, out) in matrix.iter_mut().enumerate() {
            let mut acc = 0.0_f32;
            for (i, &x) in input.iter().enumerate() {
                acc = x.mul_add(self.weights[j * self.in_dim + i], acc);
            }
            *out = acc;
        }
        let biased = matrix
            .iter()
            .zip(&self.bias)
            .map(|(&a, &b)| a + b)
            .collect();
        Ok((matrix, biased))
    }

    pub fn forward(&self, input: &[f32]) -> Result<Vec<f32>> {
        Ok(self.forward_stages(input)?.1)
    }
}

pub fn runtime_tanh(value: f32) -> f32 {
    let a = value.abs();
    let a2 = value * value;
    let p = a2.mul_add(0.13088999688625336_f32, 0.599839985370636_f32);
    let p = a.mul_add(p, 0.98430997133255_f32);
    let p = a.mul_add(p, 1.0_f32);
    let s = if value < 0.0 { runtime_rcpps(p) } else { p };
    let s2 = s * s;
    (s2 - 1.0_f32) / (s2 + 1.0_f32)
}

#[derive(Debug)]
pub struct TimbreReduction {
    layers: [FloatLinear; 3],
}

pub struct ReductionStages {
    pub matrix: [Vec<f32>; 3],
    pub biased: [Vec<f32>; 3],
    pub activated: [Vec<f32>; 3],
}

impl TimbreReduction {
    pub fn from_linears(layers: [FloatLinear; 3]) -> Result<Self> {
        let dims = [(128, 384), (384, 384), (384, 4)];
        if layers
            .iter()
            .zip(dims)
            .any(|(l, d)| (l.in_dim, l.out_dim) != d)
        {
            return Err("expected target4 dimensions128->384->384->4".into());
        }
        Ok(Self { layers })
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = crate::model::ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported reduction model version".into());
        }
        let root = tree.root;
        let wrapper = root.children.get(8).ok_or("target4 wrapper missing")?;
        if wrapper.kind != 0x7fca41ff
            || wrapper.tag != 0x945e3e7c92f8bff2
            || wrapper.data != [128, 0, 4, 0]
            || wrapper.children.len() != 1
        {
            return Err("unsupported target4 wrapper".into());
        }
        let seq = &wrapper.children[0];
        seq.require(0x7fca41ff, b"modm0", &[0xfcd304106476a6d8])?;
        if !seq.data.is_empty() || seq.children.len() != 6 {
            return Err("target4 sequence topology mismatch".into());
        }
        let linear_hashes = [0xddc4e8788e55c2ba, 0xba21e4adc0201e69, 0x244ffe3d30966367];
        let matrix_hashes = [0xbfe9957ae4624a3a, 0x188612fae3aa2499, 0xb9d96248b0cbf9d7];
        let vector_hashes = [0x18d43ba270395308, 0x4dec2518e5aee25c, 0x8e50c36ab90b1cd7];
        fn linear(node: &Node<'_>, lh: u64, mh: u64, bh: u64) -> Result<FloatLinear> {
            node.require(0x7fca41ff, b"modl0", &[lh])?;
            if !node.data.is_empty() || node.children.len() != 2 {
                return Err("float linear topology mismatch".into());
            }
            let m = &node.children[0];
            let b = &node.children[1];
            m.require(0x7fca40ff, b"prim0", &[mh])?;
            b.require(0x7fca40ff, b"prim1", &[bh])?;
            if !m.children.is_empty() || !b.children.is_empty() {
                return Err("float parameters must be leaves".into());
            }
            FloatLinear::from_payloads(m.data, b.data)
        }
        let mut layers = Vec::new();
        for i in 0..3 {
            layers.push(linear(
                &seq.children[2 * i],
                linear_hashes[i],
                matrix_hashes[i],
                vector_hashes[i],
            )?);
        }
        for (idx, name, hash) in [
            (1, b"moda7".as_slice(), 0xd658382397d01530),
            (3, b"moda7".as_slice(), 0xddd7ef788e4757f6),
            (5, b"moda1".as_slice(), 0xc5954f15b4a53b51),
        ] {
            let act = &seq.children[idx];
            act.require(0x7fca41ff, name, &[hash])?;
            if !act.data.is_empty() || !act.children.is_empty() {
                return Err("activation must be empty leaf".into());
            }
        }
        Self::from_linears(layers.try_into().map_err(|_| "three layers required")?)
    }

    pub fn forward_stages(&self, input: &[f32]) -> Result<ReductionStages> {
        let mut stages = ReductionStages {
            matrix: std::array::from_fn(|_| Vec::new()),
            biased: std::array::from_fn(|_| Vec::new()),
            activated: std::array::from_fn(|_| Vec::new()),
        };
        for i in 0..3 {
            let x = if i == 0 {
                input
            } else {
                &stages.activated[i - 1]
            };
            let (matrix, biased) = self.layers[i].forward_stages(x)?;
            let activated = biased
                .iter()
                .map(|&v| {
                    if i < 2 {
                        runtime_silu(v)
                    } else {
                        runtime_tanh(v)
                    }
                })
                .collect();
            stages.matrix[i] = matrix;
            stages.biased[i] = biased;
            stages.activated[i] = activated;
        }
        Ok(stages)
    }

    pub fn forward(&self, input: &[f32]) -> Result<Vec<f32>> {
        Ok(self.forward_stages(input)?.activated[2].clone())
    }
}
