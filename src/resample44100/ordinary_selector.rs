use super::halfband_up2::ProfileVariant;

#[derive(Clone, Copy)]
pub enum StagePlan {
    Fir { up: u32, down: u32, cutoff: f64, gain: f64, transition: f64 },
    Up2 { index: u32, variant: ProfileVariant },
    Down2 { index: u32, variant: ProfileVariant },
    Fractional { source: f64, target: f64, variant: ProfileVariant },
}

fn fir(up: u32, down: u32, gain: f64) -> StagePlan {
    StagePlan::Fir {
        up, down, cutoff: 1.0 / f64::from(up.max(down)), gain, transition: 2.0,
    }
}

fn exact_up_chain(up: u32, count: u32, variant: ProfileVariant) -> Vec<StagePlan> {
    let mut plan = vec![fir(up, 1, f64::from(up))];
    for index in 0..count { plan.push(StagePlan::Up2 { index, variant }); }
    plan
}

pub(super) fn source_rational(source: f64, target: f64) -> Option<(usize, usize)> {
    if !source.is_finite() || !target.is_finite() || source <= 0.0 || target <= 0.0 {
        return None;
    }
    let mut a = source;
    let mut b = target;
    for _ in 0..49 {
        let difference = (a - b).abs();
        if difference == 0.0 {
            if b < 1.0 { return None; }
            let numerator = source / b;
            let denominator = target / b;
            let valid = numerator >= 0.0 && numerator < 2147483648.0
                && denominator >= 0.0 && denominator < 1501.0
                && numerator == numerator.trunc() && denominator == denominator.trunc();
            return if valid { Some((numerator as usize, denominator as usize)) } else { None };
        }
        a = b;
        b = difference;
    }
    None
}

pub fn ordinary_plan(source_rate: u32) -> Result<Vec<StagePlan>, &'static str> {
    if source_rate == 0 { return Err("positive source sample rate required"); }
    let source = f64::from(source_rate);
    let target = 48000.0;
    if source == target { return Ok(Vec::new()); }
    if target * 2.0 == source { return Ok(vec![fir(1, 2, 1.0)]); }
    if target * 3.0 == source { return Ok(vec![fir(1, 3, 1.0)]); }
    if source * 2.0 == target * 3.0 { return Ok(vec![fir(2, 3, 2.0)]); }
    if source * 3.0 == target * 2.0 { return Ok(vec![fir(3, 2, 3.0)]); }
    if source * 3.0 == target * 4.0 { return Ok(vec![fir(3, 4, 3.0)]); }
    if source * 2.0 == target {
        return Ok(exact_up_chain(2, 0, ProfileVariant::False));
    }

    let mut rate = source * 2.0;
    for index in 0..30 {
        if rate > target { break; }
        rate = f64::from(4u32 << index) * source;
        if rate == target {
            return Ok(exact_up_chain(2, index + 1, ProfileVariant::False));
        }
    }
    rate = source * 3.0;
    if rate == target { return Ok(exact_up_chain(3, 0, ProfileVariant::True)); }
    for index in 0..29 {
        if rate > target { break; }
        rate = f64::from(6u32 << index) * source;
        if rate == target {
            return Ok(exact_up_chain(3, index + 1, ProfileVariant::True));
        }
    }

    if target * 2.0 > source {
        let cutoff = if target > source { 0.5 } else { (0.5 * target) / source };
        let mut plan = vec![StagePlan::Fir {
            up: 2, down: 1, cutoff, gain: 2.0, transition: 2.0,
        }];
        let adjusted = source / (2.0 * f64::from_bits(0xbf91eb851eb851ec) + 1.0);
        let mut power = 1u32;
        let mut level = 0u32;
        loop {
            let doubled = power.checked_mul(2).ok_or("source up-stage count overflow")?;
            if f64::from(doubled) * adjusted > target { break; }
            power = doubled;
            level += 1;
        }
        let triple = power.checked_mul(3).ok_or("source up-stage count overflow")?;
        let triple_fits = adjusted * f64::from(triple) <= target;
        let candidate = if triple_fits { triple } else { 1 };
        if level == 0 || (level == 1 && source_rational(source * 2.0, target).is_some()) {
            plan.push(StagePlan::Fractional {
                source: source * 2.0, target, variant: ProfileVariant::False,
            });
            return Ok(plan);
        }
        let ternary = candidate > power && triple_fits;
        let (factor, stages) = if ternary { (candidate, 1) } else { (power, level) };
        let mut fractional_source = source * 2.0;
        fractional_source *= f64::from(factor);
        plan.push(StagePlan::Fractional {
            source: fractional_source, target, variant: ProfileVariant::False,
        });
        let up = if ternary { 3 } else { 2 };
        let mut relative = f64::from(factor);
        relative *= source;
        relative /= target;
        let mut transition = 1.0 - relative;
        transition /= f64::from_bits(0x3f91eb851eb851ec);
        transition = f64::from_bits(0x4046800000000000).min(transition);
        plan.push(StagePlan::Fir {
            up, down: 1, cutoff: 1.0 / f64::from(up),
            gain: f64::from(up), transition,
        });
        let variant = if ternary { ProfileVariant::True } else { ProfileVariant::False };
        for index in 1..stages {
            plan.push(StagePlan::Up2 { index: index - 1, variant });
        }
        return Ok(plan);
    }

    let mut count = 0u32;
    let mut boundary = target * 4.0;
    let mut gain = 1.0;
    if source >= boundary {
        loop {
            count += 1;
            boundary += boundary;
            gain *= 0.5;
            if source < boundary { break; }
        }
    }
    let intermediate = f64::from(1u32 << count) * target;
    let (down, cutoff, variant, fractional) = if intermediate * 2.0 == source {
        (2, 0.5, ProfileVariant::False, false)
    } else if intermediate * 3.0 == source {
        (3, 1.0 / 3.0, ProfileVariant::True, false)
    } else {
        let cutoff = intermediate / source;
        let variant = if 1.0 >= 3.0 * cutoff { ProfileVariant::True }
            else { ProfileVariant::False };
        (1, cutoff, variant, true)
    };
    let mut plan = Vec::new();
    for index in (0..count).rev() { plan.push(StagePlan::Down2 { index, variant }); }
    plan.push(StagePlan::Fir { up: 1, down, cutoff, gain, transition: 2.0 });
    if fractional {
        plan.push(StagePlan::Fractional { source, target: intermediate, variant });
    }
    Ok(plan)
}
