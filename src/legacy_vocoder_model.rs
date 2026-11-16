//! Legacy vocoder network.

use std::collections::VecDeque;

use crate::{
    model::{ModelNode, ModelTree},
    prototype_synthesis_networks::{
        AnalysisBank8, DenseMatrix, NcwnNetwork, NeuralConv, SynthesisBank8, SynthesisPost,
    },
    Result,
};

const CHANNELS: usize = 72;
const HIDDEN: usize = 256;
const OUTPUT_BINS: usize = 512;
const LOOKAHEAD: usize = 4;
const COMPONENT: u32 = 0x7fca40ff;
const MODULE: u32 = 0x7fca41ff;

pub(super) struct NeuralFrame {
    pub raw: [f32; CHANNELS],
    pub normalized: [f32; CHANNELS],
    pub rd: f32,
    pub spectra: [Vec<f32>; 3],
    pub modulation: [f32; 10],
}

pub(super) struct Prepared {
    pub features: Model,
    pub down: AnalysisBank8,
    pub up: SynthesisBank8,
    pub residual: NcwnNetwork,
    pub post: SynthesisPost,
    pub log_gain: f32,
    pub modulation_bias: f32,
    pub gate_noise: bool,
    pub ratios: [f32; 5],
}

fn word(data: &[u8], at: usize) -> Result<u32> {
    let bytes = data.get(at..at + 4).ok_or("Truncated legacy vocoder parameter")?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn number(data: &[u8], at: usize) -> Result<f32> {
    let value = f32::from_bits(word(data, at)?);
    if !value.is_finite() { return Err("Nonfinite legacy vocoder parameter".into()); }
    Ok(value)
}

fn tag(node: &ModelNode<'_>, kind: u32, name: &str) -> Result<()> {
    if node.version() != 1 || name.len() > 8 {
        return Err("Legacy vocoder requires an unwrapped DNNI v1 tree".into());
    }
    let mut bytes = [0_u8; 8];
    bytes[..name.len()].copy_from_slice(name.as_bytes());
    if node.kind != kind || node.tag != u64::from_le_bytes(bytes) {
        return Err(format!("Legacy vocoder expected {name}"));
    }
    Ok(())
}

fn children(node: &ModelNode<'_>, count: usize) -> Result<()> {
    if node.children.len() != count {
        return Err("Legacy vocoder child count differs from the supported topology".into());
    }
    Ok(())
}

fn empty_leaf(node: &ModelNode<'_>, kind: u32, name: &str) -> Result<()> {
    tag(node, kind, name)?;
    if !node.data.is_empty() || !node.children.is_empty() {
        return Err(format!("Legacy {name} must be an empty leaf"));
    }
    Ok(())
}

fn vector(node: &ModelNode<'_>, width: usize) -> Result<Vec<f32>> {
    tag(node, COMPONENT, "prim1")?;
    if !node.children.is_empty() || word(node.data, 0)? as usize != width
        || node.data.len() != 4 + width * 4
    {
        return Err("Legacy vocoder vector extent differs".into());
    }
    (0..width).map(|i| number(node.data, 4 + 4 * i)).collect()
}

struct Normalizer {
    lower: Vec<f32>,
    upper: Vec<f32>,
    output_lower: f32,
    output_upper: f32,
}

impl Normalizer {
    fn prepare(node: &ModelNode<'_>, width: usize) -> Result<Self> {
        tag(node, COMPONENT, "cmpu0")?;
        children(node, 3)?;
        if !node.data.is_empty() { return Err("Legacy normalizer payload differs".into()); }
        let lower = vector(&node.children[0], width)?;
        let upper = vector(&node.children[1], width)?;
        let target = vector(&node.children[2], 2)?;
        if target[0] >= target[1] || lower.iter().zip(&upper).any(|(a, b)| a >= b) {
            return Err("Legacy normalizer bounds are invalid".into());
        }
        Ok(Self { lower, upper, output_lower: target[0], output_upper: target[1] })
    }

    fn apply(&self, raw: &[f32; CHANNELS]) -> Result<[f32; CHANNELS]> {
        if raw.iter().any(|x| !x.is_finite()) || raw[1] <= 0.0 {
            return Err("Legacy vocoder requires finite physical72 rows with positive pitch in Hz".into());
        }
        let mut normalized = [0.0; CHANNELS];
        for i in 0..CHANNELS {
            let input = if i == 1 { raw[i].ln() } else { raw[i] };
            let denominator = (self.upper[i] - self.lower[i]).max(1.0e-5);
            let ratio = (input - self.lower[i]) / denominator;
            normalized[i] = (ratio * (self.output_upper - self.output_lower) + self.output_lower)
                .clamp(self.output_lower, self.output_upper);
        }
        if normalized.iter().any(|v| !v.is_finite()) {
            return Err("Legacy normalizer produced a nonfinite feature".into());
        }
        Ok(normalized)
    }
}

fn convolution(node: &ModelNode<'_>, input: usize, output: usize, taps: u32,
               padding: u32, dilation: u32) -> Result<NeuralConv> {
    tag(node, MODULE, "modl1")?;
    if node.data.len() != 20 || (0..5).map(|i| word(node.data, i * 4))
        .collect::<Result<Vec<_>>>()? != [taps, 1, padding, dilation, 1]
    {
        return Err("Legacy convolution parameters differ".into());
    }
    children(node, taps as usize + 1)?;
    for matrix in &node.children[..taps as usize] { tag(matrix, COMPONENT, "prim4")?; }
    tag(&node.children[taps as usize], COMPONENT, "prim1")?;
    let conv = NeuralConv::from_node(node)?;
    if conv.input_width != input || conv.output_width != output {
        return Err("Legacy convolution dimensions differ".into());
    }
    Ok(conv)
}

fn pointwise(conv: &NeuralConv, input: &[f32]) -> Result<Vec<f32>> {
    let output = conv.forward_with_padding(&[input.to_vec()], 0, 0)?;
    if output.len() != 1 { return Err("Legacy pointwise convolution returned the wrong row count".into()); }
    Ok(output.into_iter().next().unwrap())
}

struct Centered {
    conv: NeuralConv,
    previous: Vec<f32>,
    current: Option<(u64, Vec<f32>)>,
}

impl Centered {
    fn new(conv: NeuralConv) -> Self {
        Self { previous: vec![0.0; conv.input_width], conv, current: None }
    }

    fn push(&mut self, ordinal: u64, row: &[f32]) -> Result<Option<(u64, Vec<f32>)>> {
        if row.len() != self.conv.input_width || row.iter().any(|v| !v.is_finite()) {
            return Err("Invalid centered legacy convolution input".into());
        }
        let Some((center_ordinal, center)) = self.current.take() else {
            self.current = Some((ordinal, row.to_vec()));
            return Ok(None);
        };
        if center_ordinal.checked_add(1) != Some(ordinal) {
            return Err("Centered legacy convolution ordinal differs".into());
        }
        let window = [self.previous.clone(), center.clone(), row.to_vec()];
        let mut output = self.conv.forward_with_padding(&window, 0, 0)?;
        if output.len() != 1 { return Err("Centered legacy convolution returned the wrong row count".into()); }
        self.previous = center;
        self.current = Some((ordinal, row.to_vec()));
        Ok(Some((center_ordinal, output.pop().unwrap())))
    }
}

struct Shape {
    first: Centered,
    second: Centered,
    last: NeuralConv,
}

impl Shape {
    fn prepare(node: &ModelNode<'_>) -> Result<Self> {
        tag(node, MODULE, "modm0")?;
        children(node, 6)?;
        if !node.data.is_empty() { return Err("Legacy shape network payload differs".into()); }
        empty_leaf(&node.children[1], MODULE, "moda0")?;
        empty_leaf(&node.children[3], MODULE, "moda1")?;
        let leaky = &node.children[4];
        tag(leaky, MODULE, "moda3")?;
        if !leaky.children.is_empty() || leaky.data.len() != 4
            || number(leaky.data, 0)?.to_bits() != 0.2_f32.to_bits()
        {
            return Err("Legacy shape network requires leaky ReLU 0.2".into());
        }
        Ok(Self {
            first: Centered::new(convolution(&node.children[0], 72, 256, 3, 1, 1)?),
            second: Centered::new(convolution(&node.children[2], 256, 128, 3, 1, 1)?),
            last: convolution(&node.children[5], 128, 1, 1, 0, 1)?,
        })
    }

    fn push(&mut self, ordinal: u64, row: &[f32]) -> Result<Option<(u64, f32)>> {
        let Some((ordinal, mut first)) = self.first.push(ordinal, row)? else { return Ok(None); };
        first.iter_mut().for_each(|x| *x = x.max(0.0));
        let Some((ordinal, mut second)) = self.second.push(ordinal, &first)? else { return Ok(None); };
        for value in &mut second {
            *value = value.tanh();
            if *value < 0.0 { *value *= 0.2; }
        }
        let value = pointwise(&self.last, &second)?[0];
        let rd = (0.1 + 2.9 / (1.0 + (-value).exp())).clamp(0.1, 3.0);
        if !rd.is_finite() { return Err("Legacy shape network produced a nonfinite result".into()); }
        Ok(Some((ordinal, rd)))
    }
}

struct MainBlock {
    gate: Centered,
    skip: NeuralConv,
    residual: NeuralConv,
    pending_residual: VecDeque<(u64, Vec<f32>)>,
}

impl MainBlock {
    fn prepare(gate: &ModelNode<'_>, skip: &ModelNode<'_>, residual: &ModelNode<'_>, input: usize) -> Result<Self> {
        tag(gate, MODULE, "_gnc1v0")?;
        children(gate, 1)?;
        if gate.data.len() != 12 || [word(gate.data, 0)?, word(gate.data, 4)?, word(gate.data, 8)?]
            != [input as u32, HIDDEN as u32, 0]
        {
            return Err("Legacy unconditioned gate dimensions differ".into());
        }
        Ok(Self {
            gate: Centered::new(convolution(&gate.children[0], input, 2 * HIDDEN, 3, 1, 1)?),
            skip: convolution(skip, HIDDEN, HIDDEN, 1, 0, 1)?,
            residual: convolution(residual, input, HIDDEN, 1, 0, 1)?,
            pending_residual: VecDeque::new(),
        })
    }

    fn push(&mut self, ordinal: u64, row: &[f32]) -> Result<Option<(u64, Vec<f32>, Vec<f32>)>> {
        let residual = pointwise(&self.residual, row)?;
        self.pending_residual.push_back((ordinal, residual));
        let Some((at, projected)) = self.gate.push(ordinal, row)? else { return Ok(None); };
        let (residual_at, mut next) = self.pending_residual.pop_front().ok_or("Missing legacy residual row")?;
        if at != residual_at { return Err("Legacy residual row ordinal differs".into()); }
        let gated: Vec<f32> = (0..HIDDEN).map(|i| {
            projected[i].tanh() / (1.0 + (-projected[HIDDEN + i]).exp())
        }).collect();
        let skip = pointwise(&self.skip, &gated)?;
        for (value, gate) in next.iter_mut().zip(gated) { *value += gate; }
        Ok(Some((at, next, skip)))
    }
}

struct Main {
    blocks: Vec<MainBlock>,
    sums: VecDeque<(u64, Vec<f32>)>,
    next_output: u64,
}

impl Main {
    fn prepare(node: &ModelNode<'_>) -> Result<Self> {
        tag(node, MODULE, "_ncwnv0")?;
        children(node, 3)?;
        if node.data.len() != 12 || [word(node.data, 0)?, word(node.data, 4)?, word(node.data, 8)?] != [72, 256, 0] {
            return Err("Legacy main network dimensions differ".into());
        }
        for group in &node.children {
            tag(group, MODULE, "cmpg1")?;
            children(group, 3)?;
            if !group.data.is_empty() { return Err("Legacy main group payload differs".into()); }
        }
        let mut blocks = Vec::with_capacity(3);
        for i in 0..3 {
            blocks.push(MainBlock::prepare(&node.children[0].children[i], &node.children[1].children[i],
                                           &node.children[2].children[i], if i == 0 { 72 } else { HIDDEN })?);
        }
        Ok(Self { blocks, sums: VecDeque::new(), next_output: 0 })
    }

    fn push(&mut self, ordinal: u64, input: &[f32]) -> Result<Option<(u64, Vec<f32>)>> {
        self.sums.push_back((ordinal, vec![0.0; HIDDEN]));
        let mut current = input.to_vec();
        let mut at = ordinal;
        for block in &mut self.blocks {
            let Some((produced_at, next, skip)) = block.push(at, &current)? else { return Ok(None); };
            let offset = usize::try_from(produced_at.checked_sub(self.next_output)
                .ok_or("Legacy main output was already retired")?).map_err(|_| "Legacy main ordinal overflow")?;
            let (stored_at, total) = self.sums.get_mut(offset).ok_or("Missing legacy main sum")?;
            if *stored_at != produced_at { return Err("Legacy main skip ordinal differs".into()); }
            for (value, add) in total.iter_mut().zip(skip) { *value += add; }
            at = produced_at;
            current = next;
        }
        let (stored_at, total) = self.sums.pop_front().ok_or("Missing completed legacy main row")?;
        if stored_at != at || at != self.next_output { return Err("Legacy main output ordinal differs".into()); }
        self.next_output = self.next_output.checked_add(1).ok_or("Legacy main ordinal overflow")?;
        Ok(Some((at, total)))
    }
}

struct Band {
    start: usize,
    end: usize,
    weights: Vec<f32>,
    centered: Centered,
    projection: NeuralConv,
}

impl Band {
    fn prepare(node: &ModelNode<'_>, start: usize, end: usize, left: usize, right: usize) -> Result<Self> {
        tag(node, MODULE, "modm0")?;
        children(node, 6)?;
        if !node.data.is_empty() || start >= end || end > OUTPUT_BINS || left == 1 || right == 1
            || left.checked_add(right).is_none_or(|v| v > end - start)
        {
            return Err("Legacy spectral band configuration differs".into());
        }
        empty_leaf(&node.children[0], MODULE, "moda5")?;
        empty_leaf(&node.children[1], MODULE, "moda7")?;
        empty_leaf(&node.children[3], MODULE, "moda7")?;
        empty_leaf(&node.children[5], MODULE, "moda5")?;
        let width = end - start;
        let weights = (0..width).map(|i| {
            if i < left { i as f32 / (left - 1) as f32 }
            else if i >= width - right { (width - 1 - i) as f32 / (right - 1) as f32 }
            else { 1.0 }
        }).collect();
        Ok(Self { start, end, weights,
            centered: Centered::new(convolution(&node.children[2], HIDDEN, HIDDEN, 3, 1, 1)?),
            projection: convolution(&node.children[4], HIDDEN, 3 * width, 1, 0, 1)?,
        })
    }
}

struct Spectra { bands: Vec<Band> }
impl Spectra {
    fn prepare(node: &ModelNode<'_>) -> Result<Self> {
        tag(node, MODULE, "_cepsg1")?;
        children(node, 1)?;
        if node.data.len() != 56 || word(node.data, 0)? != 3 || word(node.data, 4)? != 3 {
            return Err("Legacy spectral network requires three heads and three bands".into());
        }
        let list = &node.children[0];
        tag(list, MODULE, "cmpg1")?;
        children(list, 3)?;
        if !list.data.is_empty() { return Err("Legacy spectral group payload differs".into()); }
        let mut bands = Vec::with_capacity(3);
        for i in 0..3 {
            let p = 8 + 16 * i;
            bands.push(Band::prepare(&list.children[i], word(node.data, p)? as usize,
                word(node.data, p + 4)? as usize, word(node.data, p + 8)? as usize,
                word(node.data, p + 12)? as usize)?);
        }
        if bands.iter().map(|b| b.end).max() != Some(OUTPUT_BINS) {
            return Err("Legacy spectral output must cover 512 bins".into());
        }
        Ok(Self { bands })
    }

    fn push(&mut self, ordinal: u64, row: &[f32]) -> Result<Option<(u64, [Vec<f32>; 3])>> {
        let activated: Vec<f32> = row.iter().map(|&x| x / (1.0 + (-x).exp())).collect();
        let mut output: [Vec<f32>; 3] = std::array::from_fn(|_| vec![0.0; OUTPUT_BINS]);
        let mut ready = None;
        let mut availability = None;
        for band in &mut self.bands {
            let current = band.centered.push(ordinal, &activated)?;
            if availability.is_some_and(|previous| previous != current.is_some()) {
                return Err("Legacy spectral band availability differs".into());
            }
            availability = Some(current.is_some());
            let Some((at, mut hidden)) = current else { continue; };
            if ready.is_some_and(|previous| previous != at) { return Err("Legacy spectral ordinal differs".into()); }
            ready = Some(at);
            for x in &mut hidden { *x /= 1.0 + (-*x).exp(); }
            let projection = pointwise(&band.projection, &hidden)?;
            let width = band.end - band.start;
            for head in 0..3 {
                for i in 0..width {
                    output[head][band.start + i] += projection[head * width + i] * band.weights[i];
                }
            }
        }
        Ok(ready.map(|at| (at, output)))
    }
}

struct Linear { matrix: DenseMatrix, bias: Vec<f32> }
impl Linear {
    fn prepare(node: &ModelNode<'_>, input: usize, output: usize) -> Result<Self> {
        tag(node, MODULE, "modl0")?;
        children(node, 2)?;
        if !node.data.is_empty() { return Err("Legacy linear payload differs".into()); }
        tag(&node.children[0], COMPONENT, "prim4")?;
        let matrix = DenseMatrix::from_node(&node.children[0])?;
        if matrix.input_width != input || matrix.output_width != output {
            return Err("Legacy linear matrix dimensions differ".into());
        }
        Ok(Self { matrix, bias: vector(&node.children[1], output)? })
    }
    fn apply(&self, input: &[f32]) -> Result<Vec<f32>> {
        let mut output = vec![0.0; self.bias.len()];
        self.matrix.overwrite(input, &mut output)?;
        for (value, bias) in output.iter_mut().zip(&self.bias) { *value += bias; }
        Ok(output)
    }
}

struct Modulation { first: Linear, second: Linear }
impl Modulation {
    fn prepare(node: &ModelNode<'_>) -> Result<Self> {
        tag(node, MODULE, "modm0")?;
        children(node, 3)?;
        if !node.data.is_empty() { return Err("Legacy modulation network payload differs".into()); }
        empty_leaf(&node.children[1], MODULE, "moda7")?;
        Ok(Self { first: Linear::prepare(&node.children[0], HIDDEN, 64)?,
                  second: Linear::prepare(&node.children[2], 64, 10)? })
    }
    fn apply(&self, input: &[f32]) -> Result<[f32; 10]> {
        let mut hidden = self.first.apply(input)?;
        for x in &mut hidden { *x /= 1.0 + (-*x).exp(); }
        self.second.apply(&hidden)?.try_into().map_err(|_| "Legacy modulation width differs".into())
    }
}

pub(super) struct Model {
    normalizer: Normalizer,
    shape: Shape,
    main: Main,
    spectra: Spectra,
    modulation: Modulation,
    raw: VecDeque<(u64, [f32; CHANNELS], [f32; CHANNELS])>,
    shapes: VecDeque<(u64, f32)>,
    modulation_rows: VecDeque<(u64, [f32; 10])>,
    next_input: u64,
    next_output: u64,
    failure: Option<String>,
}

impl Model {
    pub(super) fn prepare(bytes: &[u8]) -> Result<Prepared> {
        let tree = ModelTree::parse(bytes)?;
        if tree.version != 1 { return Err("Select an unwrapped legacy DNNI v1 vocoder".into()); }
        let root = &tree.root;
        tag(root, MODULE, "_nhntv2")?;
        children(root, 13)?;
        if root.data.len() != 18 || word(root.data, 0)? != 8 || root.data[12] != 1 || root.data[13] > 1 {
            return Err("Legacy vocoder root configuration differs".into());
        }
        let log_gain = number(root.data, 4)?;
        let _noise_log_gain = number(root.data, 8)?;
        let modulation_bias = number(root.data, 14)?;
        let format = &root.children[0];
        tag(format, COMPONENT, "_vocfv1")?;
        children(format, 4)?;
        if format.data.len() != 48 || word(format.data, 0)? != 72 || word(format.data, 4)? != 1
            || word(format.data, 8)? != 48_000 || word(format.data, 12)? != 0.005_f32.to_bits()
            || (0..8).map(|i| word(format.data, 16 + 4 * i)).collect::<Result<Vec<_>>>()?
                != [72, 1, 1, 0, 1, 1, 1, 0]
        {
            return Err("Legacy vocoder must use the supported 72-channel 48 kHz/5 ms format".into());
        }
        let normalizer = Normalizer::prepare(&format.children[0], CHANNELS)?;
        let _sample_normalizer = Normalizer::prepare(&format.children[1], 30_720)?;
        empty_leaf(&format.children[2], COMPONENT, "cmpno")?;
        empty_leaf(&format.children[3], COMPONENT, "cmpno")?;
        for filter in &root.children[6..9] {
            tag(filter, MODULE, "_ltvfs1")?;
            if !filter.children.is_empty() || filter.data.len() != 13 || word(filter.data, 0)? != 2
                || filter.data[4] != 0 || word(filter.data, 5)? != 1024 || word(filter.data, 9)? != 512
            {
                return Err("Legacy vocoder filter configuration differs".into());
            }
        }
        tag(&root.children[9], MODULE, "_ppusv0")?;
        tag(&root.children[10], MODULE, "_ppdsv0")?;
        let up = SynthesisBank8::from_node(&root.children[9])?;
        let down = AnalysisBank8::from_node(&root.children[10])?;
        tag(&root.children[11], MODULE, "_ncwnv0")?;
        let residual = NcwnNetwork::from_node(&root.children[11])?;
        if (residual.input_width, residual.output_width, residual.condition_width) != (16, 64, 72) {
            return Err("Legacy residual network dimensions differ".into());
        }
        let (_, right) = residual.streaming_context()?;
        if right > 512 { return Err("Legacy residual network lookahead exceeds the supported bound".into()); }
        let post = SynthesisPost::from_node(&root.children[12])?;
        let ratios: [f32; 5] = vector(&root.children[4], 5)?.try_into().map_err(|_| "Legacy ratio width differs")?;
        let features = Self {
            normalizer,
            shape: Shape::prepare(&root.children[1])?,
            main: Main::prepare(&root.children[2])?,
            spectra: Spectra::prepare(&root.children[3])?,
            modulation: Modulation::prepare(&root.children[5])?,
            raw: VecDeque::new(), shapes: VecDeque::new(), modulation_rows: VecDeque::new(),
            next_input: 0, next_output: 0, failure: None,
        };
        Ok(Prepared { features, down, up, residual, post, log_gain, modulation_bias,
                      gate_noise: root.data[13] != 0, ratios })
    }

    pub(super) fn lookahead_rows(&self) -> usize { LOOKAHEAD }

    pub(super) fn process(&mut self, raw: &[f32], frames: &mut Vec<NeuralFrame>) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let result = self.advance(raw, frames);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, raw: &[f32], frames: &mut Vec<NeuralFrame>) -> Result<()> {
        let raw: [f32; CHANNELS] = raw.try_into().map_err(|_| "Legacy vocoder needs 72 input channels")?;
        let normalized = self.normalizer.apply(&raw)?;
        let ordinal = self.next_input;
        self.next_input = self.next_input.checked_add(1).ok_or("Legacy vocoder ordinal overflow")?;
        self.raw.push_back((ordinal, raw, normalized));
        if let Some(shape) = self.shape.push(ordinal, &normalized)? { self.shapes.push_back(shape); }
        let Some((at, main)) = self.main.push(ordinal, &normalized)? else { return Ok(()); };
        self.modulation_rows.push_back((at, self.modulation.apply(&main)?));
        let Some((at, spectra)) = self.spectra.push(at, &main)? else { return Ok(()); };
        let (raw_at, raw, normalized) = self.raw.pop_front().ok_or("Missing legacy physical row")?;
        let (shape_at, rd) = self.shapes.pop_front().ok_or("Missing legacy shape row")?;
        let (mod_at, modulation) = self.modulation_rows.pop_front().ok_or("Missing legacy modulation row")?;
        if at != self.next_output || raw_at != at || shape_at != at || mod_at != at {
            return Err("Legacy vocoder feature branch ordinals differ".into());
        }
        if spectra.iter().flatten().chain(modulation.iter()).any(|v| !v.is_finite()) {
            return Err("Legacy vocoder emitted nonfinite neural features".into());
        }
        frames.push(NeuralFrame { raw, normalized, rd, spectra, modulation });
        self.next_output = self.next_output.checked_add(1).ok_or("Legacy vocoder output ordinal overflow")?;
        Ok(())
    }
}
