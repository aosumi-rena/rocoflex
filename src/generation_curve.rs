#[path = "generation_fit_qr.rs"]
mod fit_qr;
#[path = "interpolation_remap.rs"]
mod remap;
#[path = "interpolation_remap_math.rs"]
mod remap_math;
#[path = "generation_packet_norm_math.rs"]
mod packet_norm_math;
#[path = "generation_packet_norm_prefix.rs"]
mod packet_norm_prefix;
pub use packet_norm_math::{SourceGenerationRsqrtpsRequest, SourceGenerationRsqrtpsCallReport,
    SourceGenerationRsqrtpsProjectionReport, SourceGenerationRsqrtpsService,
    SourceGenerationPacketNormConfiguration, SourceGenerationPacketNormBindingCause,
    SourceGenerationPacketNormBindingError, SourceGenerationPacketNormTag,
    SourceGenerationPacketNormStage, SourceGenerationPacketNormCurrent,
    SourceGenerationPacketNormPendingPacked, SourceGenerationPacketNormAttempt,
    SourceGenerationPacketNormMask, SourceGenerationPacketNormPacket,
    SourceGenerationPacketNormCallAttempt, SourceGenerationPacketNormProjectionAttempt,
    SourceGenerationPacketNormProgress, SourceGenerationPacketNormCause,
    SourceGenerationPacketNormFailure, SourceGenerationPacketNormStopped, SourceGenerationPacketNormMath};
pub use packet_norm_prefix::{evaluate_packet_prefix_with_source_norm_math,
    SourceGenerationPacketNormPrefixStage, SourceGenerationPacketNormPrefixProgress,
    SourceGenerationPacketNormPrefixCause, SourceGenerationPacketNormPrefixParts,
    SourceGenerationPacketNormPrefixCompleted, SourceGenerationPacketNormPrefixFailure};
pub use remap::{inverse_cumulative_with_source_math, SourceRemapStage, SourceRemapScanFlags,
    SourceRemapProgress, SourceRemapCause, SourceRemapParts, SourceRemapCompleted, SourceRemapFailure};
pub use remap_math::{SourceInterpolationRemapConfiguration, SourceInterpolationRemapBindingCause,
    SourceInterpolationRemapBindingFailure, SourceInterpolationRemapTag, SourceInterpolationRemapCurrent,
    SourceInterpolationRemapStage, SourceInterpolationRemapOperands, SourceInterpolationRemapReport,
    SourceInterpolationRemapAttempt, SourceInterpolationRemapWanted, SourceInterpolationRemapScan,
    SourceInterpolationRemapInterpolation, SourceInterpolationRemapProgress, SourceInterpolationRemapCause,
    SourceInterpolationRemapFailure, SourceInterpolationRemapStopped, SourceInterpolationRemapMath};

#[derive(Debug)]
pub struct GenerationCurve4 {
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub coefficients: [[f32; 4]; 4],
    pub normal_columns: [[[f32; 4]; 4]; 4],
    pub rhs: [[f32; 4]; 4],
}

pub fn unit_grid(count: usize) -> Result<Vec<f32>, &'static str> {
    if count < 2 || count > i32::MAX as usize {
        return Err("grid needs at least two parameters");
    }
    let step = 1.0_f32 / (count - 1) as f32;
    let mut values: Vec<_> = (0..count).map(|i| i as f32 * step).collect();
    values[count - 1] = 1.0;
    Ok(values)
}

fn normal_system(parameters: &[f32], values: &[f32], lambda: f32) -> ([[f32; 4]; 4], [f32; 4]) {
    let start = values[0];
    let end = values[values.len() - 1];
    let mut columns = [[0.0_f32; 4]; 4];
    let mut rhs = [0.0_f32; 4];
    for i in 0..4 {
        columns[i][i] = lambda;
    }
    for (&t, &y) in parameters.iter().zip(values) {
        for row in 0..4 {
            let factor = t.powf((row + 2) as f32) - t;
            let mut power = t;
            for col in 0..4 {
                power *= t;
                columns[col][row] = (power - t) * factor + columns[col][row];
            }
            let residual = -(end - start) * t + (y - start);
            rhs[row] = residual * factor + rhs[row];
        }
    }
    (columns, rhs)
}

impl GenerationCurve4 {
    pub fn from_rows(
        parameters: &[f32],
        rows: &[[f32; 4]],
        lambda: f32,
    ) -> Result<Self, &'static str> {
        if rows.len() < 2
            || rows.len() != parameters.len()
            || !lambda.is_finite()
            || lambda <= 0.0
            || parameters
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
            || rows.iter().flatten().any(|v| !v.is_finite())
            || parameters.windows(2).any(|v| v[0] > v[1])
        {
            return Err("finite explicit rows, parameters and positive regularization required");
        }
        let mut coefficients = [[0.0_f32; 4]; 4];
        let mut normal_columns = [[[0.0_f32; 4]; 4]; 4];
        let mut rhs = [[0.0_f32; 4]; 4];
        for axis in 0..4 {
            let values: Vec<_> = rows.iter().map(|r| r[axis]).collect();
            (normal_columns[axis], rhs[axis]) = normal_system(parameters, &values, lambda);
            coefficients[axis] = fit_qr::solve(normal_columns[axis], rhs[axis]);
        }
        Ok(Self {
            start: rows[0],
            end: rows[rows.len() - 1],
            coefficients,
            normal_columns,
            rhs,
        })
    }

    pub fn evaluate(&self, t: f32) -> [f32; 4] {
        let mut out = self.start;
        for axis in 0..4 {
            let c = self.coefficients[axis];
            let sum = (c[1] + c[3]) + (c[0] + c[2]);
            let slope = (self.end[axis] - self.start[axis]) - sum;
            out[axis] += slope * t;
            let mut power = t;
            for coefficient in c {
                power = t * power;
                out[axis] = coefficient * power + out[axis];
            }
        }
        out
    }
}

pub struct GenerationArcStages {
    pub parameters: Vec<f32>,
    pub evaluated: Vec<[f32; 4]>,
    pub squared: Vec<f32>,
    pub distances: Vec<f32>,
    pub cumulative: Vec<f32>,
    pub table: Vec<f32>,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationArcPreparationStage { Parameters, PacketNorm }

pub struct SourceGenerationArcPreparation {
    cumulative: Option<Vec<f32>>,
    distances: Option<Vec<f32>>,
    squared: Option<Vec<f32>>,
    evaluated: Option<Vec<[f32; 4]>>,
    parameters: Option<Vec<f32>>,
    stage: SourceGenerationArcPreparationStage,
}
impl SourceGenerationArcPreparation {
    pub fn stage(&self) -> SourceGenerationArcPreparationStage { self.stage }
    pub fn parameters(&self) -> Option<&[f32]> { self.parameters.as_deref() }
    pub fn evaluated(&self) -> Option<&[[f32; 4]]> { self.evaluated.as_deref() }
    pub fn squared(&self) -> Option<&[f32]> { self.squared.as_deref() }
    pub fn distances(&self) -> Option<&[f32]> { self.distances.as_deref() }
    pub fn cumulative(&self) -> Option<&[f32]> { self.cumulative.as_deref() }
}
struct PreparedArc {
    cumulative: Vec<f32>, distances: Vec<f32>, squared: Vec<f32>,
    evaluated: Vec<[f32; 4]>, parameters: Vec<f32>,
}
struct ArcPrepareFailure { preparation: SourceGenerationArcPreparation, error: &'static str }
pub struct SourceGenerationArcContext {
    pub queries: Vec<f32>,
    pub math: SourceInterpolationRemapMath,
    pub remap_progress: SourceRemapProgress,
}
pub struct SourceGenerationArcParts {
    pub stages: GenerationArcStages,
    pub context: SourceGenerationArcContext,
}
pub struct SourceGenerationArcCompleted { parts: SourceGenerationArcParts }
impl SourceGenerationArcCompleted {
    pub fn stages(&self) -> &GenerationArcStages { &self.parts.stages }
    pub fn context(&self) -> &SourceGenerationArcContext { &self.parts.context }
    pub fn into_parts(self) -> SourceGenerationArcParts { self.parts }
}
enum SourceGenerationArcFailureOwner {
    Prepare { preparation: SourceGenerationArcPreparation, error: &'static str,
        queries: Vec<f32>, math: SourceInterpolationRemapMath },
    Remap { failure: SourceRemapFailure, distances: Vec<f32>, squared: Vec<f32>, evaluated: Vec<[f32; 4]> },
}

pub struct SourceGenerationArcFailure { owner: SourceGenerationArcFailureOwner }
impl SourceGenerationArcFailure {
    pub fn preparation(&self) -> Option<&SourceGenerationArcPreparation> {
        match &self.owner { SourceGenerationArcFailureOwner::Prepare { preparation, .. } => Some(preparation), _ => None }
    }
    pub fn preparation_error(&self) -> Option<&'static str> {
        match &self.owner { SourceGenerationArcFailureOwner::Prepare { error, .. } => Some(*error), _ => None }
    }
    pub fn remap_failure(&self) -> Option<&SourceRemapFailure> {
        match &self.owner { SourceGenerationArcFailureOwner::Remap { failure, .. } => Some(failure), _ => None }
    }
    pub fn queries(&self) -> &[f32] {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { queries, .. } => queries,
            SourceGenerationArcFailureOwner::Remap { failure, .. } => failure.queries(),
        }
    }
    pub fn math(&self) -> &SourceInterpolationRemapMath {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { math, .. } => math,
            SourceGenerationArcFailureOwner::Remap { failure, .. } => failure.math(),
        }
    }
    pub fn parameters(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { preparation, .. } => preparation.parameters(),
            SourceGenerationArcFailureOwner::Remap { failure, .. } => Some(failure.sampled_positions()),
        }
    }
    pub fn cumulative(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { preparation, .. } => preparation.cumulative(),
            SourceGenerationArcFailureOwner::Remap { failure, .. } => Some(failure.cumulative()),
        }
    }
    pub fn evaluated(&self) -> Option<&[[f32; 4]]> {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { preparation, .. } => preparation.evaluated(),
            SourceGenerationArcFailureOwner::Remap { evaluated, .. } => Some(evaluated),
        }
    }
    pub fn squared(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { preparation, .. } => preparation.squared(),
            SourceGenerationArcFailureOwner::Remap { squared, .. } => Some(squared),
        }
    }
    pub fn distances(&self) -> Option<&[f32]> {
        match &self.owner {
            SourceGenerationArcFailureOwner::Prepare { preparation, .. } => preparation.distances(),
            SourceGenerationArcFailureOwner::Remap { distances, .. } => Some(distances),
        }
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn packet_sqrt_source(squared: &[f32], distances: &mut [f32]) {
    use std::arch::x86_64::*;
    unsafe {
        for i in (0..496).step_by(4) {
            let x = _mm_loadu_ps(squared.as_ptr().add(i));
            let abs = _mm_and_ps(x, _mm_castsi128_ps(_mm_set1_epi32(0x7fffffff)));
            let abs_i = _mm_castps_si128(abs);
            let sign = _mm_set1_epi32(i32::MIN);
            let negative = _mm_cmpgt_epi32(_mm_setzero_si128(), _mm_castps_si128(x));
            let first = _mm_andnot_si128(
                _mm_cmpgt_epi32(
                    _mm_xor_si128(_mm_add_epi32(abs_i, _mm_set1_epi32(-1)), sign),
                    _mm_set1_epi32(0x807ffffe_u32 as i32),
                ),
                negative,
            );
            let infinite = _mm_or_si128(
                _mm_cmpgt_epi32(abs_i, _mm_set1_epi32(0x7f800000)),
                _mm_cmpeq_epi32(abs_i, _mm_set1_epi32(0x7f800000)),
            );
            let normal = _mm_cmpgt_epi32(
                _mm_set1_epi32(0xff000000_u32 as i32),
                _mm_xor_si128(_mm_sub_epi32(abs_i, _mm_set1_epi32(0x00800000)), sign),
            );
            let mask = _mm_or_si128(_mm_or_si128(normal, infinite), first);
            let q = _mm_rsqrt_ps(x);
            let negative_half_x = _mm_mul_ps(x, _mm_set1_ps(-0.5));
            let factor = _mm_add_ps(
                _mm_mul_ps(_mm_mul_ps(q, q), negative_half_x),
                _mm_set1_ps(1.5),
            );
            let answer = _mm_and_ps(_mm_mul_ps(_mm_mul_ps(factor, q), x), _mm_castsi128_ps(mask));
            _mm_storeu_ps(distances.as_mut_ptr().add(i), answer);
        }
    }
}

pub fn arc_stages(
    curve: &GenerationCurve4,
    queries: &[f32],
) -> Result<GenerationArcStages, &'static str> {
    let prepared = prepare_arc(curve).map_err(|failure| failure.error)?;
    let table = remap::inverse_cumulative_source_order(&prepared.cumulative, &prepared.parameters, queries)?;
    Ok(GenerationArcStages {
        parameters: prepared.parameters, evaluated: prepared.evaluated,
        squared: prepared.squared, distances: prepared.distances, cumulative: prepared.cumulative, table,
    })
}

pub fn arc_stages_with_source_remap_math(curve: &GenerationCurve4, queries: Vec<f32>,
    math: SourceInterpolationRemapMath) -> Result<SourceGenerationArcCompleted, SourceGenerationArcFailure>
{
    let prepared = match prepare_arc(curve) {
        Ok(prepared) => prepared,
        Err(ArcPrepareFailure { preparation, error }) => return Err(SourceGenerationArcFailure {
            owner: SourceGenerationArcFailureOwner::Prepare { preparation, error, queries, math } }),
    };
    let PreparedArc { cumulative, distances, squared, evaluated, parameters } = prepared;
    let remap = match inverse_cumulative_with_source_math(cumulative, parameters, queries, math) {
        Ok(completed) => completed.into_parts(),
        Err(failure) => return Err(SourceGenerationArcFailure {
            owner: SourceGenerationArcFailureOwner::Remap { failure, distances, squared, evaluated } }),
    };
    let SourceRemapParts { cumulative, sampled_positions: parameters, queries, output: table, math, progress } = remap;
    Ok(SourceGenerationArcCompleted { parts: SourceGenerationArcParts {
        stages: GenerationArcStages { parameters, evaluated, squared, distances, cumulative, table },
        context: SourceGenerationArcContext { queries, math, remap_progress: progress },
    } })
}

fn prepare_arc(curve: &GenerationCurve4) -> Result<PreparedArc, ArcPrepareFailure> {
    let parameters = match unit_grid(500) {
        Ok(parameters) => parameters,
        Err(error) => return Err(ArcPrepareFailure {
            preparation: SourceGenerationArcPreparation { cumulative: None, distances: None, squared: None,
                evaluated: None, parameters: None, stage: SourceGenerationArcPreparationStage::Parameters }, error }),
    };
    let evaluated: Vec<_> = parameters.iter().map(|&t| curve.evaluate(t)).collect();
    let mut squared = Vec::with_capacity(499);
    for pair in evaluated.windows(2) {
        let delta: [f32; 4] = std::array::from_fn(|axis| pair[1][axis] - pair[0][axis]);
        let mut sum = delta[0] * delta[0];
        for axis in 1..4 {
            sum += delta[axis] * delta[axis];
        }
        squared.push(sum);
    }
    let mut distances = vec![0.0_f32; 499];
    #[cfg(target_arch = "x86_64")]
    unsafe {
        packet_sqrt_source(&squared, &mut distances);
    }
    #[cfg(not(target_arch = "x86_64"))]
    return Err(ArcPrepareFailure {
        preparation: SourceGenerationArcPreparation { cumulative: None, distances: Some(distances),
            squared: Some(squared), evaluated: Some(evaluated), parameters: Some(parameters),
            stage: SourceGenerationArcPreparationStage::PacketNorm },
        error: "source packet norm requires x86_64 RSQRTPS",
    });
    for i in 496..499 {
        distances[i] = squared[i].sqrt();
    }
    let mut cumulative = vec![0.0_f32; 500];
    for i in 1..500 {
        cumulative[i] = cumulative[i - 1] + distances[i - 1];
    }
    Ok(PreparedArc { cumulative, distances, squared, evaluated, parameters })
}
pub fn interpolate_final_rows(
    post: &[f32],
    width: usize,
    table: &[f32],
) -> Result<Vec<f32>, &'static str> {
    if width == 0
        || post.len() % width != 0
        || post.len() / width < 2
        || post.iter().any(|v| !v.is_finite())
        || table.is_empty()
        || table.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
    {
        return Err("invalid explicit postdecim rows/table");
    }
    let rows = post.len() / width;
    let mut out = Vec::with_capacity(table.len() * width);
    for &t in table {
        let pos = t * (rows - 1) as f32;
        let lo = (pos.floor() as usize).min(rows - 1);
        let hi = (lo + 1).min(rows - 1);
        let frac = pos - lo as f32;
        for axis in 0..width {
            let delta = post[hi * width + axis] - post[lo * width + axis];
            let scaled = delta * frac;
            out.push(post[lo * width + axis] + scaled);
        }
    }
    Ok(out)
}
