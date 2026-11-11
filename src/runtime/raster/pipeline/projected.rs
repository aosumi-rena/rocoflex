include!("../angular_projection/projection.rs");

#[derive(Debug)]
pub enum MathCause { Source(Error), RequiredTrig }
#[derive(Clone, Copy, Debug)]
pub enum MathPhase { Inputs, EndSin(u32), EndCos(u32), StartSin(u32), StartCos(u32), Store(usize), Complete }
#[derive(Debug)]
#[must_use = "retain the partial planar buffer, trace and caller math owner"]
pub struct MathFailure {
    pub cause: MathCause, pub partial: Output, pub record: usize, pub phase: MathPhase,
}

fn required_trig(out: &mut Output, arg: f32, cosine: bool,
    math: &mut impl super::geometry::MathServices) -> Result<f32, MathCause>
{
    let value = if cosine { math.projection_cos(arg) } else { math.projection_sin(arg) }
        .map_err(|_| MathCause::RequiredTrig)?;
    out.trig.push([cosine as u32, arg.to_bits(), value.to_bits()]);
    if value.is_finite() { Ok(value) } else { Err(MathCause::Source(Error::NonFinite)) }
}

pub fn project_with_math(records: &[Record], origin: [f32; 2], rect: [i32; 4], zoom: f32,
    math: &mut impl super::geometry::MathServices) -> Result<Output, MathFailure>
{
    let mut out = Output { planar: Vec::new(), trig: Vec::new(), stores: Vec::new() };
    let mut record = 0;
    let mut phase = MathPhase::Inputs;
    let result = (|| -> Result<(), MathCause> {
        if rect[2] <= 0 || rect[3] <= 0 { return Err(MathCause::Source(Error::InvalidRectangle)); }
        if !zoom.is_finite() || zoom <= 0.0 { return Err(MathCause::Source(Error::InvalidZoom)); }
        if origin.iter().any(|v| !v.is_finite()) || records.iter().any(|r|
            [r.start, r.end, r.start_distance, r.end_distance].iter().any(|v| !v.is_finite()))
        { return Err(MathCause::Source(Error::NonFinite)); }
        if records.len() > u32::MAX as usize / 4 { return Err(MathCause::Source(Error::TooManyRecords)); }
        let count = records.len() * 4;

        out.planar.try_reserve_exact(count).map_err(|_| MathCause::Source(Error::Allocation))?;
        out.planar.resize(count, 0.0);
        out.trig.try_reserve_exact(count).map_err(|_| MathCause::Source(Error::Allocation))?;
        out.stores.try_reserve_exact(count).map_err(|_| MathCause::Source(Error::Allocation))?;
        let half = f32::from_bits(0x3f000000);
        let shift = f32::from_bits(0xbfc90fdb);
        let m = (rect[2].min(rect[3]) as f32) * half;
        let cx = (rect[2] as f32) * half + rect[0] as f32;
        let cy = (rect[3] as f32) * half + rect[1] as f32;
        for (i, r) in records.iter().enumerate() {
            record = i;
            let ea = r.end + shift;
            phase = MathPhase::EndSin(ea.to_bits());
            let ec = required_trig(&mut out, ea, false, math)?;
            let ea2 = r.end + shift;
            phase = MathPhase::EndCos(ea2.to_bits());
            let es = required_trig(&mut out, ea2, true, math)?;
            let sa = r.start + shift;
            phase = MathPhase::StartSin(sa.to_bits());
            let sc = required_trig(&mut out, sa, false, math)?;
            let sa2 = r.start + shift;
            phase = MathPhase::StartCos(sa2.to_bits());
            let ss = required_trig(&mut out, sa2, true, math)?;
            let ey = ec * r.end_distance + origin[1];
            let ex = es * r.end_distance + origin[0];
            let sy = sc * r.start_distance + origin[1];
            let sx = ss * r.start_distance + origin[0];
            let px = ((sx * m) * zoom + cx) + (-half);
            let py = (cy - (sy * m) * zoom) + (-half);
            let qx = ((ex * m) * zoom + cx) + (-half);
            let qy = (cy - (ey * m) * zoom) + (-half);
            let n = 2 * records.len();
            for (index, value) in [(2*i, px), (n+2*i, py), (2*i+1, qx), (n+2*i+1, qy)] {
                phase = MathPhase::Store(index);
                if !value.is_finite() { return Err(MathCause::Source(Error::NonFinite)); }
                out.planar[index] = value;
                out.stores.push([index as u32, value.to_bits()]);
            }
        }
        phase = MathPhase::Complete;
        Ok(())
    })();
    match result { Ok(()) => Ok(out), Err(cause) => Err(MathFailure { cause, partial: out, record, phase }) }
}
