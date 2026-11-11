#[derive(Clone, Copy, Debug)]
pub struct Arithmetic { pub current_thread_mxcsr: u32, pub separate_operations: bool }

pub trait Services {
    type RootSpan;
    type Root;
    type Samples;
    type Error;
    fn arithmetic(&mut self) -> Result<Arithmetic, Self::Error>;

    fn capture_roots(&mut self, held: &mut Option<Self::RootSpan>) -> Result<usize, Self::Error>;
    fn root_at(&mut self, span: &Self::RootSpan, index: usize) -> Result<Self::Root, Self::Error>;

    fn project_samples(&mut self, root: &Self::Root, held: &mut Option<Self::Samples>)
        -> Result<usize, Self::Error>;
    fn sample_at(&mut self, samples: &Self::Samples, index: usize) -> Result<[f32; 2], Self::Error>;

    fn retire_samples(&mut self, held: &mut Option<Self::Samples>) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Admission, CaptureRoots, ProjectRoot, Sample, RetireSamples, Candidate, Done }
#[derive(Debug)]
pub enum Fault<E> {
    Service(E), FloatingPointContract, NonfiniteInput, NonfiniteArithmetic,
    MissingOwner, UnretiredSamples, AlreadyStopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiniteFault { NonfiniteInput, NonfiniteArithmetic }

impl<E> From<FiniteFault> for Fault<E> {
    fn from(value: FiniteFault) -> Self {
        match value {
            FiniteFault::NonfiniteInput => Self::NonfiniteInput,
            FiniteFault::NonfiniteArithmetic => Self::NonfiniteArithmetic,
        }
    }
}

pub fn candidate_centers(hit: [f32; 2]) -> Result<[[f32; 2]; 8], FiniteFault> {
    if hit.iter().any(|v| !v.is_finite()) { return Err(FiniteFault::NonfiniteInput); }
    let [x, y] = hit;

    let candidates = [
        [x + 30.0, y + 0.0], [0.0 + x, y + 30.0],
        [x + (-30.0), 0.0 + y], [0.0 + x, y + (-30.0)],
        [30.0 + x, y + 30.0], [x + 30.0, y + (-30.0)],
        [x + (-30.0), y + 30.0], [(-30.0) + x, y + (-30.0)],
    ];
    if candidates.iter().flatten().any(|v| !v.is_finite()) {
        Err(FiniteFault::NonfiniteArithmetic)
    } else { Ok(candidates) }
}

pub fn finite_sample_distance(sample: [f32; 2], center: [f32; 2]) -> Result<f32, FiniteFault> {
    if sample.iter().chain(&center).any(|v| !v.is_finite()) {
        return Err(FiniteFault::NonfiniteInput);
    }
    let dy = sample[1] - center[1];
    let dx = sample[0] - center[0];

    let yy = (dy as f64) * (dy as f64);
    let xx = (dx as f64) * (dx as f64);
    let squared = yy + xx;
    let distance = squared.sqrt() as f32;
    if distance.is_finite() { Ok(distance) } else { Err(FiniteFault::NonfiniteArithmetic) }
}

pub fn improving_offset(center: [f32; 2], projected_root: [f32; 2])
    -> Result<[f32; 2], FiniteFault>
{
    if center.iter().chain(&projected_root).any(|v| !v.is_finite()) {
        return Err(FiniteFault::NonfiniteInput);
    }
    let dy = center[1] - projected_root[1];
    let dx = center[0] - projected_root[0];
    if dx.is_finite() && dy.is_finite() { Ok([dx, dy]) }
    else { Err(FiniteFault::NonfiniteArithmetic) }
}

pub struct Prepared<S: Services> {
    services: S,
    hit: [f32; 2],
    root_position: [f32; 2],
    offset: [f32; 2],
    candidates: [[f32; 2]; 8],
    candidate: usize,
    roots: Option<S::RootSpan>,
    root: Option<S::Root>,
    samples: Option<S::Samples>,
    root_count: usize,
    root_index: usize,
    sample_count: usize,
    sample_index: usize,
    nearest: f32,
    best: f32,
    best_candidate: Option<usize>,
    phase: Phase,
    attempted: Option<Phase>,
    halted: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub struct Finished<S: Services> {
    pub services: S,
    pub offset: [f32; 2],

    pub selected_candidate: Option<usize>,
    pub score: f32,
}

impl<S: Services> Prepared<S> {

    pub fn newly_hit(services: S, hit_sample: [f32; 2], projected_root: [f32; 2],
        previous_offset: [f32; 2]) -> Self
    {
        Self { services, hit: hit_sample, root_position: projected_root, offset: previous_offset,
            candidates: [[0.0; 2]; 8], candidate: 0, roots: None, root: None, samples: None,
            root_count: 0, root_index: 0, sample_count: 0, sample_index: 0,
            nearest: f32::from_bits(0x5015_02f9), best: 0.0, best_candidate: None,
            phase: Phase::Admission, attempted: None, halted: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn services(&self) -> &S { &self.services }

    pub fn current_offset(&self) -> [f32; 2] { self.offset }
    pub fn selected_candidate(&self) -> Option<usize> { self.best_candidate }

    pub fn run(mut self) -> Result<Finished<S>, Stopped<S>> {
        if self.halted { return Err(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        while self.phase != Phase::Done {
            self.attempted = Some(self.phase);
            if let Err(fault) = self.step() {
                self.halted = true;
                return Err(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        Ok(Finished { services: self.services, offset: self.offset,
            selected_candidate: self.best_candidate, score: self.best })
    }
    fn controls(&mut self) -> Result<(), Fault<S::Error>> {
        let a = self.services.arithmetic().map_err(Fault::Service)?;
        if a.current_thread_mxcsr & 0xffc0 != 0x1f80 || !a.separate_operations {
            Err(Fault::FloatingPointContract)
        } else { Ok(()) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Admission => {
                self.controls()?;
                if self.hit.iter().chain(&self.root_position).chain(&self.offset).any(|v| !v.is_finite()) {
                    return Err(Fault::NonfiniteInput);
                }
                self.candidates = candidate_centers(self.hit).map_err(Fault::from)?;
                self.phase = Phase::CaptureRoots;
            }
            Phase::CaptureRoots => {
                self.nearest = f32::from_bits(0x5015_02f9);
                self.root_index = 0;
                self.root_count = self.services.capture_roots(&mut self.roots).map_err(Fault::Service)?;
                if self.roots.is_none() { return Err(Fault::MissingOwner); }
                self.phase = Phase::ProjectRoot;
            }
            Phase::ProjectRoot => {
                if self.root_index == self.root_count { self.phase = Phase::Candidate; return Ok(()); }
                let span = self.roots.as_ref().ok_or(Fault::MissingOwner)?;
                self.root = Some(self.services.root_at(span, self.root_index).map_err(Fault::Service)?);
                let root = self.root.as_ref().ok_or(Fault::MissingOwner)?;
                self.sample_count = self.services.project_samples(root, &mut self.samples).map_err(Fault::Service)?;
                if self.samples.is_none() { return Err(Fault::MissingOwner); }
                self.sample_index = 0;
                self.phase = Phase::Sample;
            }
            Phase::Sample => {
                if self.sample_index == self.sample_count { self.phase = Phase::RetireSamples; return Ok(()); }
                let samples = self.samples.as_ref().ok_or(Fault::MissingOwner)?;
                let point = self.services.sample_at(samples, self.sample_index).map_err(Fault::Service)?;
                self.controls()?;
                let center = self.candidates[self.candidate];
                let distance = finite_sample_distance(point, center).map_err(Fault::from)?;

                if distance < self.nearest { self.nearest = distance; }
                self.sample_index += 1;
            }
            Phase::RetireSamples => {
                self.services.retire_samples(&mut self.samples).map_err(Fault::Service)?;
                if self.samples.is_some() { return Err(Fault::UnretiredSamples); }
                self.root = None;
                self.root_index += 1;
                self.phase = Phase::ProjectRoot;
            }
            Phase::Candidate => {
                self.controls()?;
                if self.nearest > self.best {
                    self.offset = improving_offset(self.candidates[self.candidate], self.root_position)
                        .map_err(Fault::from)?;
                    self.best = self.nearest;
                    self.best_candidate = Some(self.candidate);
                }
                self.roots = None;
                self.candidate += 1;
                self.phase = if self.candidate == 8 { Phase::Done } else { Phase::CaptureRoots };
            }
            Phase::Done => {}
        }
        Ok(())
    }
}

pub fn current_center(projected_current_root: [f32; 2], cached_offset: [f32; 2]) -> [f32; 2] {
    [projected_current_root[0] + cached_offset[0], projected_current_root[1] + cached_offset[1]]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementFault { FloatingPointContract, Nonfinite, IntegerConversionBoundary }
#[derive(Clone, Copy, Debug)]
pub struct Bounds { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

pub trait CenterMath {
    type Error;
    fn widen_f32(&mut self, bits: u32) -> Result<u64, Self::Error>;
    fn add_f64(&mut self, left: u64, right: u64) -> Result<u64, Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub enum CenterStage { Admission, Widen(usize), Bias(usize), Done }
#[derive(Clone, Copy, Debug)]
pub struct CenterProgress {
    pub input: [u32; 2], pub stage: CenterStage,
    pub widened: [Option<u64>; 2], pub biased: [Option<u64>; 2],
    pub rounded: [Option<i32>; 2],
}
pub enum CenterError<E> {
    Admission { fault: PlacementFault, progress: CenterProgress },
    Arithmetic { error: E, progress: CenterProgress },
}
pub struct CenterCompleted { pub rounded: [i32; 2], pub progress: CenterProgress }

pub fn round_requested_center_with_math<M: CenterMath>(center: [f32; 2], arithmetic: Arithmetic,
    math: &mut M) -> Result<CenterCompleted, CenterError<M::Error>>
{
    let mut p = CenterProgress { input: center.map(f32::to_bits), stage: CenterStage::Admission,
        widened: [None; 2], biased: [None; 2], rounded: [None; 2] };
    placement_controls(arithmetic).map_err(|fault| CenterError::Admission { fault, progress: p })?;
    if center.iter().any(|v| !v.is_finite()) {
        return Err(CenterError::Admission { fault: PlacementFault::Nonfinite, progress: p });
    }

    if center.iter().any(|v| *v < -2147483648.0 || *v >= 2147483648.0) {
        return Err(CenterError::Admission { fault: PlacementFault::IntegerConversionBoundary, progress: p });
    }
    let mut rounded = [0_i32; 2];
    for axis in [1, 0] {
        p.stage = CenterStage::Widen(axis);
        let wide = math.widen_f32(p.input[axis]).map_err(|error| CenterError::Arithmetic { error, progress: p })?;
        p.widened[axis] = Some(wide);
        p.stage = CenterStage::Bias(axis);
        let bits = math.add_f64(wide, 0x4338_0000_0000_0000)
            .map_err(|error| CenterError::Arithmetic { error, progress: p })?;
        p.biased[axis] = Some(bits);
        rounded[axis] = bits as u32 as i32;
        p.rounded[axis] = Some(rounded[axis]);
    }
    p.stage = CenterStage::Done;
    Ok(CenterCompleted { rounded, progress: p })
}

struct ExistingCenterHostMath;
impl CenterMath for ExistingCenterHostMath {
    type Error = core::convert::Infallible;
    fn widen_f32(&mut self, bits: u32) -> Result<u64, Self::Error> { Ok((f32::from_bits(bits) as f64).to_bits()) }
    fn add_f64(&mut self, left: u64, right: u64) -> Result<u64, Self::Error> {
        Ok((f64::from_bits(left) + f64::from_bits(right)).to_bits())
    }
}

pub fn round_requested_center(center: [f32; 2], arithmetic: Arithmetic)
    -> Result<[i32; 2], PlacementFault>
{
    match round_requested_center_with_math(center, arithmetic, &mut ExistingCenterHostMath) {
        Ok(completed) => Ok(completed.rounded),
        Err(CenterError::Admission { fault, .. }) => Err(fault),
        Err(CenterError::Arithmetic { error, .. }) => match error {},
    }
}

pub fn place_with_supplied_inverse(requested_center: [i32; 2], inverse: [f32; 6],
    current_size: [i32; 2], arithmetic: Arithmetic) -> Result<Bounds, PlacementFault>
{
    placement_controls(arithmetic)?;
    if inverse.iter().any(|v| !v.is_finite()) { return Err(PlacementFault::Nonfinite); }
    let x = requested_center[0] as f32;
    let y = requested_center[1] as f32;
    let yy = inverse[4] * y;
    let yx = inverse[3] * x;
    let mapped_y = (yx + yy) + inverse[5];
    let xy = y * inverse[1];
    let xx = x * inverse[0];
    let mapped_x = (xx + xy) + inverse[2];
    let center_y = placement_cvtt(mapped_y)?;
    let center_x = placement_cvtt(mapped_x)?;

    let half_width = current_size[0] / 2;
    let half_height = current_size[1] / 2;
    Ok(Bounds { x: center_x.wrapping_sub(half_width),
        y: center_y.wrapping_sub(half_height), width: current_size[0], height: current_size[1] })
}
fn placement_controls(arithmetic: Arithmetic) -> Result<(), PlacementFault> {
    if arithmetic.current_thread_mxcsr & 0xffc0 != 0x1f80 || !arithmetic.separate_operations {
        Err(PlacementFault::FloatingPointContract)
    } else { Ok(()) }
}
fn placement_cvtt(value: f32) -> Result<i32, PlacementFault> {
    if !value.is_finite() || value < -2147483648.0 || value >= 2147483648.0 {
        Err(PlacementFault::IntegerConversionBoundary)
    } else { Ok(value as i32) }
}
