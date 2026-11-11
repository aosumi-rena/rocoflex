#[path = "desktop_probe_parameter_publication.rs"]
mod publication;
use super::empty_background::literal::public_numeric as camera;
use super::source_menu::literal as center;
use crate::source_scalar::{ExplicitExceptionState, PowerError,
    ScalarF32Architecture, SourceF64ValueError, SourceF64ValueMath};

impl center::CenterMath for SourceF64ValueMath {
    type Error = SourceF64ValueError;
    fn widen_f32(&mut self, bits: u32) -> Result<u64, Self::Error> { self.widen_f32_bits(bits) }
    fn add_f64(&mut self, left: u64, right: u64) -> Result<u64, Self::Error> { self.add_bits(left, right) }
}

#[derive(Clone, Copy)]
struct Input { center: [u32; 2], current: [u32; 2], mxcsr: u32, errno: i32, architecture: ScalarF32Architecture }
#[derive(Clone, Copy)]
enum Phase { Rounding, InitialPrefix, Normalize(usize), InitialFinish, IterationZoom(usize), Divide { iteration: usize, axis: usize }, Parameter(usize), Done }
#[derive(Clone, Copy)]
struct Iteration {
    seed: Option<[u64; 2]>, zoom: Option<u64>, result: [Option<u64>; 2],
    before: Option<ExplicitExceptionState>, after: Option<ExplicitExceptionState>,
}
impl Iteration {
    const EMPTY: Self = Self { seed: None, zoom: None, result: [None; 2], before: None, after: None };
}
struct Pending {
    input: Input, math: SourceF64ValueMath, phase: Phase,
    rounding: Option<center::CenterCompleted>, pixel: Option<[i32; 2]>,
    initial_prefix: Option<camera::Prefix>, initial_progress: Option<camera::Progress>,
    normalization: [Option<u64>; 6], normalized: [Option<u64>; 2],
    iterations: [Iteration; 11], inverse: Option<[u64; 2]>,
    parameters: [Option<publication::Completed>;2], position: [Option<u32>; 2],
}
enum Cause {
    State(PowerError), Binding(SourceF64ValueError), Operation(SourceF64ValueError),
    Rounding(center::CenterError<SourceF64ValueError>),
    Camera(camera::Failure<SourceF64ValueError>), Parameter(Box<publication::Failed>), MissingValue, Nonfinite,
}
enum FailureKind { Binding { input: Input, cause: Cause }, Stopped { pending: Pending, cause: Cause } }
pub(super) struct Failure { kind: FailureKind }
pub(super) struct Completed { pending: Pending, position: [u32; 2] }
impl Completed {
    pub fn position(&self) -> [f32; 2] { self.position.map(f32::from_bits) }
    pub fn normalized(&self) -> Option<[f64; 2]> {
        let [Some(x), Some(y)] = self.pending.normalized else { return None; };
        Some([f64::from_bits(x), f64::from_bits(y)])
    }
    pub fn inverse(&self) -> Option<[f64; 2]> { self.pending.inverse.map(|p| p.map(f64::from_bits)) }
    pub fn rounded(&self) -> Option<[i32; 2]> { self.pending.rounding.as_ref().map(|r| r.rounded) }
    pub fn clamped(&self) -> Option<[i32; 2]> { self.pending.pixel }
}

pub(super) fn calculate(canonical: [f32; 2], current: [f32; 2], mxcsr: u32,
    errno: i32, architecture: ScalarF32Architecture) -> Result<Completed, Failure>
{
    let input = Input { center: canonical.map(f32::to_bits), current: current.map(f32::to_bits), mxcsr, errno, architecture };
    let state = ExplicitExceptionState::from_explicit_parts(mxcsr, errno)
        .map_err(|error| Failure { kind: FailureKind::Binding { input, cause: Cause::State(error) } })?;
    let math = SourceF64ValueMath::bind(architecture, state)
        .map_err(|error| Failure { kind: FailureKind::Binding { input, cause: Cause::Binding(error) } })?;
    let mut pending = Pending { input, math, phase: Phase::Rounding, rounding: None, pixel: None, initial_prefix: None,
        initial_progress: None, normalization: [None; 6],
        normalized: [None; 2], iterations: [Iteration::EMPTY; 11], inverse: None, parameters: [None,None], position: [None; 2] };
    match pending.run() {
        Ok(position) => Ok(Completed { pending, position }),
        Err(cause) => Err(Failure { kind: FailureKind::Stopped { pending, cause } }),
    }
}
impl Pending {
    fn run(&mut self) -> Result<[u32; 2], Cause> {
        const HALF_HEIGHT: u64 = 0x4071_8000_0000_0000;
        const NEGATIVE_HALF_WIDTH: u64 = 0xc07a_e000_0000_0000;
        self.phase = Phase::Rounding;
        let rounded = center::round_requested_center_with_math(self.input.center.map(f32::from_bits),
            center::Arithmetic { current_thread_mxcsr: self.input.mxcsr, separate_operations: true }, &mut self.math)
            .map_err(Cause::Rounding)?;
        let pixel = [rounded.rounded[0].clamp(0, 860), rounded.rounded[1].clamp(0, 560)];
        self.rounding = Some(rounded); self.pixel = Some(pixel);

        self.phase = Phase::InitialPrefix;
        self.iterations[0].before = Some(self.math.exception_state());
        let prefix = camera::source_zoom_prefix_with_math(self.input.current.map(Some), &mut self.math).map_err(Cause::Camera)?;
        self.initial_progress = Some(*prefix.progress());
        self.initial_prefix = Some(prefix);

        self.phase = Phase::Normalize(0);
        let y = self.math.signed_i32_to_f64_bits(pixel[1]).map_err(Cause::Operation)?;
        self.normalization[0] = Some(y);
        self.phase = Phase::Normalize(1);
        let y = self.math.subtract_bits(HALF_HEIGHT, y).map_err(Cause::Operation)?;
        self.normalization[1] = Some(y);
        self.phase = Phase::Normalize(2);
        let y = self.math.divide_bits(y, HALF_HEIGHT).map_err(Cause::Operation)?;
        self.normalization[2] = Some(y); self.normalized[1] = Some(y);
        self.phase = Phase::Normalize(3);
        let x = self.math.signed_i32_to_f64_bits(pixel[0]).map_err(Cause::Operation)?;
        self.normalization[3] = Some(x);
        self.phase = Phase::Normalize(4);
        let x = self.math.add_bits(x, NEGATIVE_HALF_WIDTH).map_err(Cause::Operation)?;
        self.normalization[4] = Some(x);
        self.phase = Phase::Normalize(5);
        let x = self.math.divide_bits(x, HALF_HEIGHT).map_err(Cause::Operation)?;
        self.normalization[5] = Some(x); self.normalized[0] = Some(x);

        self.phase = Phase::InitialFinish;
        let prefix = self.initial_prefix.take().ok_or(Cause::MissingValue)?;
        let zoom = prefix.finish_with_math(&mut self.math).map_err(Cause::Camera)?;
        let mut guess = self.divide(0, [x, y], zoom)?;

        for iteration in 1..=10 {
            self.phase = Phase::IterationZoom(iteration);
            self.iterations[iteration].seed = Some(guess);
            self.iterations[iteration].before = Some(self.math.exception_state());
            let zoom = camera::source_zoom_from_unclamped_pair_with_math(guess, &mut self.math).map_err(Cause::Camera)?;
            guess = self.divide(iteration, [x, y], zoom)?;
        }
        self.inverse = Some(guess);
        if guess.iter().any(|word| word & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000) {
            return Err(Cause::Nonfinite);
        }

        for axis in 0..2 {
            self.phase = Phase::Parameter(axis);
            let entry=if axis==0 {self.math.exception_state()} else {
                self.parameters[0].as_ref().ok_or(Cause::MissingValue)?.state()
            };
            let completed=publication::calculate(guess[axis],entry,self.input.architecture)
                .map_err(|failure|Cause::Parameter(Box::new(failure)))?;
            let bits=completed.bits();
            self.parameters[axis]=Some(completed);
            self.position[axis] = Some(bits);
            if bits & 0x7f80_0000 == 0x7f80_0000 { return Err(Cause::Nonfinite); }
        }
        let [Some(x), Some(y)] = self.position else { return Err(Cause::MissingValue); };
        self.phase = Phase::Done;
        Ok([x, y])
    }
    fn divide(&mut self, iteration: usize, input: [u64; 2], zoom: u64) -> Result<[u64; 2], Cause> {
        self.iterations[iteration].zoom = Some(zoom);
        self.phase = Phase::Divide { iteration, axis: 1 };
        let y = self.math.divide_bits(input[1], zoom).map_err(Cause::Operation)?;
        self.iterations[iteration].result[1] = Some(y);
        self.phase = Phase::Divide { iteration, axis: 0 };
        let x = self.math.divide_bits(input[0], zoom).map_err(Cause::Operation)?;
        self.iterations[iteration].result[0] = Some(x);
        self.iterations[iteration].after = Some(self.math.exception_state());
        Ok([x, y])
    }
}
