include!("../angular_insertion/insertion.rs");

#[derive(Clone, Copy, Debug)]
pub struct TrigAttempt { pub kind: u32, pub argument: u32, pub trace_index: usize }
#[derive(Debug)]
pub enum MathCause { Source(Error), RequiredTrig(TrigAttempt), ArithmeticMode(u32), MissingCarried }
#[derive(Clone, Copy, Debug)]
pub enum MathPhase { Inputs, State, Find, Trig(TrigAttempt), Ratio(u32),
    Insert(usize), Remove(usize), Complete }
#[derive(Debug)]
#[must_use = "retain advanced insertion records and required math owner; no replay"]
pub struct MathFailure {
    pub cause: MathCause,
    pub partial: Output,
    pub incoming_index: usize,
    pub original: Option<Record>,
    pub carried: Option<Record>,
    pub envelope_index: Option<usize>,
    pub phase: MathPhase,
    pub attempted_trig: Option<TrigAttempt>,
}

fn current_mode() -> Result<(), MathCause> {
    let mut mxcsr = 0u32;
    unsafe {
        std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr,
            options(nostack, preserves_flags));
    }
    if mxcsr & 0xffc0 == 0x1f80 { Ok(()) }
    else { Err(MathCause::ArithmeticMode(mxcsr)) }
}

fn required_trig(o: &mut Output, kind: u32, argument: f32,
    math: &mut impl super::geometry::MathServices,
    phase: &mut MathPhase, attempted: &mut Option<TrigAttempt>) -> Result<f32, MathCause>
{
    let call = TrigAttempt { kind, argument: argument.to_bits(), trace_index: o.trig.len() };
    *phase = MathPhase::Trig(call);
    *attempted = Some(call);
    current_mode()?;
    let value = if kind == 0 { math.projection_sin(argument) }
        else { math.projection_cos(argument) }.map_err(|_| MathCause::RequiredTrig(call))?;
    o.trig.push([kind, argument.to_bits(), value.to_bits()]);

    current_mode()?;
    if value.is_finite() { Ok(value) } else { Err(MathCause::Source(Error::NonFinite)) }
}

fn required_ratio(o: &mut Output, kind: u32, numerator: f32, denominator: f32,
    phase: &mut MathPhase) -> Result<f32, MathCause>
{
    *phase = MathPhase::Ratio(kind);
    current_mode()?;
    ratio(o, kind, numerator, denominator).map_err(MathCause::Source)
}

pub fn insert_with_math(envelope: &[Record], obstacles: &[Record], capacity: usize,
    math: &mut impl super::geometry::MathServices) -> Result<Output, MathFailure>
{
    let mut o = Output::default();
    let mut incoming_index = 0;
    let mut original = None;
    let mut carried = None;
    let mut envelope_index = None;
    let mut phase = MathPhase::Inputs;
    let mut attempted_trig = None;
    let result = (|| -> Result<(), MathCause> {
        current_mode()?;
        if capacity < envelope.len() { return Err(MathCause::Source(Error::InvalidCapacity)); }
        if capacity > i32::MAX as usize || envelope.len() > i32::MAX as usize
            || obstacles.len() > i32::MAX as usize { return Err(MathCause::Source(Error::Size)); }
        if envelope.iter().chain(obstacles).any(|r| r.0.iter().any(|v| !v.is_finite())) {
            return Err(MathCause::Source(Error::NonFinite));
        }
        o = Output { records: envelope.to_vec(), capacity, ..Output::default() };
        for (ordinal, &incoming) in obstacles.iter().enumerate() {
            incoming_index = ordinal;
            original = Some(incoming);
            carried = Some(incoming);
            let r = carried.as_mut().ok_or(MathCause::MissingCarried)?;
            let end_shift = r.0[1] + SHIFT;
            loop {
                phase = MathPhase::State;
                envelope_index = None;
                state(&mut o, *r);
                if r.0[1] <= r.0[0] { break; }
                phase = MathPhase::Find;
                let Some(index) = o.records.iter().rposition(|e|
                    r.0[0] >= e.0[0] && e.0[1] > r.0[0]) else { break; };
                envelope_index = Some(index);
                let e = o.records[index];
                if e.0[1] < r.0[1] {
                    let t = e.0[1] + SHIFT;
                    let ct = required_trig(&mut o, 1, t, math, &mut phase, &mut attempted_trig)?;
                    let st = required_trig(&mut o, 0, t, math, &mut phase, &mut attempted_trig)?;
                    let ye = required_trig(&mut o, 0, end_shift, math, &mut phase, &mut attempted_trig)? * r.0[3];
                    let xe = required_trig(&mut o, 1, end_shift, math, &mut phase, &mut attempted_trig)? * r.0[3];
                    let start_shift = r.0[0] + SHIFT;
                    let ys = required_trig(&mut o, 0, start_shift, math, &mut phase, &mut attempted_trig)? * r.0[2];
                    let xs = required_trig(&mut o, 1, start_shift, math, &mut phase, &mut attempted_trig)? * r.0[2];
                    let dx = xe - xs;
                    let dy = ye - ys;
                    let den = st * dx - ct * dy;
                    let num = dx * ys - dy * xs;
                    let d = required_ratio(&mut o, 0, num, den, &mut phase)?;
                    if unordered_or_le(e.0[3] + EPS, d) {
                        r.0[0] = e.0[1];
                        r.0[2] = d;
                    } else {
                        let ct = required_trig(&mut o, 1, start_shift, math, &mut phase, &mut attempted_trig)?;
                        let st = required_trig(&mut o, 0, start_shift, math, &mut phase, &mut attempted_trig)?;
                        let ye = required_trig(&mut o, 0, e.0[1] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[3];
                        let xe = required_trig(&mut o, 1, e.0[1] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[3];
                        let ys = required_trig(&mut o, 0, e.0[0] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[2];
                        let xs = required_trig(&mut o, 1, e.0[0] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[2];
                        let dx = xe - xs;
                        let dy = ye - ys;
                        let den = st * dx - ct * dy;
                        let num = dx * ys - dy * xs;
                        let ed = required_ratio(&mut o, 1, num, den, &mut phase)?;
                        let next = Record([r.0[0], e.0[1], r.0[2], d]);
                        r.0[0] = e.0[1];
                        r.0[2] = d;
                        o.records[index].0[1] = next.0[0];
                        o.records[index].0[3] = ed;
                        phase = MathPhase::Insert(index + 1);
                        insert(&mut o, index + 1, next).map_err(MathCause::Source)?;
                    }
                } else {
                    let ye = required_trig(&mut o, 0, e.0[1] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[3];
                    let xe = required_trig(&mut o, 1, e.0[1] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[3];
                    let ys = required_trig(&mut o, 0, e.0[0] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[2];
                    let xs = required_trig(&mut o, 1, e.0[0] + SHIFT, math, &mut phase, &mut attempted_trig)? * e.0[2];
                    let dx = xe - xs;
                    let dy = ye - ys;
                    let t = r.0[0] + SHIFT;
                    let ct = required_trig(&mut o, 1, t, math, &mut phase, &mut attempted_trig)?;
                    let st = required_trig(&mut o, 0, t, math, &mut phase, &mut attempted_trig)?;
                    let den = st * dx - ct * dy;
                    let num = ys * dx - xs * dy;
                    let d = required_ratio(&mut o, 2, num, den, &mut phase)?;
                    if unordered_or_le(d + EPS, r.0[2]) {
                        r.0[0] = r.0[1];
                    } else {
                        let ct = required_trig(&mut o, 1, end_shift, math, &mut phase, &mut attempted_trig)?;
                        let st = required_trig(&mut o, 0, end_shift, math, &mut phase, &mut attempted_trig)?;
                        let den = st * dx - ct * dy;
                        let end_distance = required_ratio(&mut o, 3, num, den, &mut phase)?;
                        o.records[index].0[1] = r.0[0];
                        o.records[index].0[3] = d;
                        if e.0[1] > r.0[1] {
                            phase = MathPhase::Insert(index + 1);
                            insert(&mut o, index + 1, Record([r.0[1], e.0[1], end_distance, e.0[3]]))
                                .map_err(MathCause::Source)?;
                        }
                        phase = MathPhase::Insert(index + 1);
                        insert(&mut o, index + 1, *r).map_err(MathCause::Source)?;
                        r.0[0] = r.0[1];
                    }
                }
                if o.records[index].0[0] >= o.records[index].0[1] {
                    phase = MathPhase::Remove(index);
                    o.records.remove(index);
                }
            }
        }
        phase = MathPhase::Complete;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(o),
        Err(cause) => Err(MathFailure { cause, partial: o, incoming_index, original,
            carried, envelope_index, phase, attempted_trig }),
    }
}
