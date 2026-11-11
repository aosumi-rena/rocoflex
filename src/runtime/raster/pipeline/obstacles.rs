include!("../obstacle/producer.rs");

#[derive(Debug)]
pub enum MathCause { Source(Error), RequiredAngle, RequiredNorm([u32; 2]) }
#[derive(Clone, Copy, Debug)]
pub enum MathPhase {
    Inputs, AngleA([f32; 2]), AngleB([f32; 2]), DistanceA, DistanceB,
    Wrap, WrapDistance, Append, Complete,
}
#[must_use = "retain completed records and failing source stage; no replay"]
pub struct MathFailure {
    pub cause: MathCause, pub partial: Output, pub segment: usize, pub phase: MathPhase,
}
impl std::fmt::Debug for MathFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MathFailure").field("cause", &self.cause)
            .field("segment", &self.segment).field("phase", &self.phase)
            .field("completed_records", &self.partial.records.len()).finish()
    }
}

fn required_angle(out: &mut Output, pair: [f32; 2], math: &mut impl super::geometry::MathServices)
    -> Result<f32, MathCause>
{
    let value = math.atan2(pair[0], pair[1]).map_err(|_| MathCause::RequiredAngle)?;
    out.math.push([0, pair[0].to_bits(), pair[1].to_bits(), value.to_bits()]);
    if value.is_finite() { Ok(value) } else { Err(MathCause::Source(Error::NonFinite)) }
}

fn required_norm(out: &mut Output, pair: [f32; 2], math: &mut impl super::geometry::MathServices)
    -> Result<f32, MathCause>
{
    let value = math.norm(pair[0], pair[1]).map_err(|_| MathCause::RequiredNorm(pair.map(f32::to_bits)))?;
    out.math.push([1, pair[0].to_bits(), pair[1].to_bits(), value.to_bits()]);
    if value.is_finite() { Ok(value) } else { Err(MathCause::Source(Error::NonFinite)) }
}

pub fn produce_with_math(origin: [f32; 2], segments: &[[f32; 4]], capacity: usize,
    math: &mut impl super::geometry::MathServices) -> Result<Output, MathFailure>
{
    let mut out = Output::default();
    let mut segment = 0;
    let mut phase = MathPhase::Inputs;
    let result = (|| -> Result<(), MathCause> {
        if capacity > i32::MAX as usize || segments.len() > i32::MAX as usize / 2 {
            return Err(MathCause::Source(Error::Size));
        }
        if origin.iter().chain(segments.iter().flatten()).any(|v| !v.is_finite()) {
            return Err(MathCause::Source(Error::NonFinite));
        }
        let far = [origin[0] + 0.0, origin[1] + 100000.0];
        out.ray = [origin[0].to_bits(), origin[1].to_bits(), far[0].to_bits(), far[1].to_bits()];
        out.capacity = capacity;
        for (index, edge) in segments.iter().enumerate() {
            segment = index;
            let a = [edge[0], edge[1]]; let b = [edge[2], edge[3]];

            let pair_a = [a[0] - origin[0], origin[1] - a[1]];
            phase = MathPhase::AngleA(pair_a);
            let aa = required_angle(&mut out, pair_a, math)?;
            let pair_b = [b[0] - origin[0], origin[1] - b[1]];
            phase = MathPhase::AngleB(pair_b);
            let ab = required_angle(&mut out, pair_b, math)?;
            phase = MathPhase::DistanceA;
            let da = required_norm(&mut out, [origin[0] - a[0], pair_a[1]], math)?;
            phase = MathPhase::DistanceB;
            let db = required_norm(&mut out, [origin[0] - b[0], pair_b[1]], math)?;
            let (start, end, ds, de) = if aa > ab { (ab, aa, db, da) } else { (aa, ab, da, db) };
            let pi = f32::from_bits(0x40490fdb);
            if end - start > pi {
                phase = MathPhase::Wrap;
                let (point, hit) = wrap_vertical(origin, far, a, b).map_err(MathCause::Source)?;
                out.wraps.push([origin[0].to_bits(), origin[1].to_bits(), far[0].to_bits(), far[1].to_bits(),
                    a[0].to_bits(), a[1].to_bits(), b[0].to_bits(), b[1].to_bits(),
                    point[0].to_bits(), point[1].to_bits(), hit as u32]);
                phase = MathPhase::WrapDistance;
                let d = required_norm(&mut out, [origin[0] - point[0], origin[1] - point[1]], math)?;
                phase = MathPhase::Append;
                append(&mut out, [-pi, start, d, ds]);
                append(&mut out, [end, pi, de, d]);
            } else {
                phase = MathPhase::Append;
                append(&mut out, [start, end, ds, de]);
            }
        }
        phase = MathPhase::Complete;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(out),
        Err(cause) => Err(MathFailure { cause, partial: out, segment, phase }),
    }
}
