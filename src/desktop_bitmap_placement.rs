use crate::source_scalar as math;
use eframe::egui::{Rect, pos2};
use math::shared_scalar_f32::{
    ScalarF32Architecture, ScalarF32Error, ScalarF32InstructionSet, ScalarF32Mxcsr,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, SourceScalarF32,
};
use math::finite_power::{ExplicitExceptionState, SourceF32CompareOpcode};

use super::canvas_primitives::{CompareFlags, F32Operation};
use super::canvas_placement64 as graph;

#[derive(Clone, Copy)]
enum Site { ImageDimension(usize), Placement(u32), DisplayCorner { corner: usize, axis: usize } }
enum ArithmeticError {
    State(math::PowerError), Instruction(ScalarF32Error),
    Divide(math::LogArithmeticError), Unsupported(F32Operation), Stopped,
    ReturnedState { before: u32, after: u32, bits: Option<u32> },
    Trap { mxcsr: u32, unmasked: u8 },
}
struct Arithmetic {
    backend: SourceScalarF32,
    state: ExplicitExceptionState,
    reported: Option<ScalarF32Mxcsr>,
    attempted: Option<(Site, F32Operation)>,
    scalar_attempted: Option<(Site, ScalarF32Operation)>,
    comparison: Option<(u32, u32, u32, ExplicitExceptionState, CompareFlags)>,
    last_result: Option<u32>,
    stopped: bool,
}
impl Arithmetic {
    fn bind(word: u32, errno: i32, architecture: ScalarF32Architecture) -> Result<Self, ArithmeticError> {
        let state = ExplicitExceptionState::from_explicit_parts(word, errno).map_err(ArithmeticError::State)?;
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
            ScalarF32Opcode::Cvtsi2ssZeroExtendedU32,
            ScalarF32Opcode::Mulss, ScalarF32Opcode::Subss, ScalarF32Opcode::Addss,
        ]).map_err(ArithmeticError::Instruction)?;
        let backend = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(ArithmeticError::Instruction)?;
        Ok(Self { backend, state, reported: None, attempted: None, scalar_attempted: None, comparison: None, last_result: None, stopped: false })
    }
    fn ready(&self) -> Result<(), ArithmeticError> {
        if self.stopped { Err(ArithmeticError::Stopped) } else { Ok(()) }
    }
    fn scalar(&mut self, site: Site, operation: ScalarF32Operation) -> Result<u32, ArithmeticError> {
        self.ready()?;
        self.scalar_attempted = Some((site, operation));
        let before = self.state.mxcsr();
        let state = ScalarF32Mxcsr::from_explicit_word(before).ok_or_else(|| {
            self.stopped = true;
            ArithmeticError::ReturnedState { before, after: before, bits: None }
        })?;
        match self.backend.execute(operation, state) {
            Err(error) => { self.stopped = true; Err(ArithmeticError::Instruction(error)) }
            Ok(ScalarF32Outcome::Trapped { state, unmasked }) => {
                self.reported = Some(state); self.stopped = true;
                Err(ArithmeticError::Trap { mxcsr: state.word(), unmasked: unmasked.bits() })
            }
            Ok(ScalarF32Outcome::Completed { bits, state }) => {
                self.reported = Some(state); self.last_result = Some(bits);
                let after = state.word();
                if before & !0x3f != after & !0x3f || before & 0x3f & !after != 0 {
                    self.stopped = true;
                    return Err(ArithmeticError::ReturnedState { before, after, bits: Some(bits) });
                }
                self.state = ExplicitExceptionState::from_explicit_parts(after, self.state.errno())
                    .map_err(|error| { self.stopped = true; ArithmeticError::State(error) })?;
                Ok(bits)
            }
        }
    }
    fn operation(&mut self, site: Site, operation: F32Operation) -> Result<u32, ArithmeticError> {
        self.ready()?;
        self.attempted = Some((site, operation));
        let scalar = match operation {
            F32Operation::Mul { destination, source } => ScalarF32Operation::Mulss { left: destination, right: source },
            F32Operation::Sub { destination, source } => ScalarF32Operation::Subss { left: destination, right: source },
            F32Operation::Add { destination, source } => ScalarF32Operation::Addss { left: destination, right: source },
            F32Operation::Div { destination, source } => {
                return match math::raw_divss_with_explicit_state(destination, source, self.state, self.backend.architecture()) {
                    Ok(result) => { self.state = result.exception_state; self.last_result = Some(result.bits); Ok(result.bits) }
                    Err(error) => {
                        self.state = match error {
                            math::LogArithmeticError::Dispatch(_) => self.state,
                            math::LogArithmeticError::ArithmeticRequired { advanced, .. }
                            | math::LogArithmeticError::ScalarServiceRequired { advanced, .. }
                            | math::LogArithmeticError::UnexpectedScalarState { advanced, .. }
                            | math::LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
                        };
                        self.stopped = true;
                        Err(ArithmeticError::Divide(error))
                    }
                };
            }
            _ => { self.stopped = true; return Err(ArithmeticError::Unsupported(operation)); }
        };
        self.scalar(site, scalar)
    }
    fn dimension(&mut self, axis: usize, value: u32) -> Result<u32, ArithmeticError> {
        self.scalar(Site::ImageDimension(axis), ScalarF32Operation::Cvtsi2ssZeroExtendedU32 { value })
    }
}
impl graph::values::Fp for Arithmetic {
    type Error = ArithmeticError;
    fn operation(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error> {
        self.operation(Site::Placement(pc), operation)
    }
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error> {
        self.ready()?;
        let before = self.state;
        let result = math::finite_power::compare_f32_with_explicit_state(SourceF32CompareOpcode::Ucomiss, left, right, self.state);
        self.state = result.exception_state;
        let flags = CompareFlags { carry: result.carry, zero: result.zero, parity: result.parity };
        self.comparison = Some((pc, left, right, before, flags));
        Ok(flags)
    }
}

#[derive(Clone, Copy)]
struct Input { size: [usize; 2], destination: [u32; 4], mxcsr: u32, errno: i32, architecture: ScalarF32Architecture }
struct Data {
    input: Input,
    arithmetic: Arithmetic,
    source: [u32; 4], destination: [u32; 4], flags: u32,
    matrix: [Option<u32>; 6], vertices: [[Option<u32>; 2]; 4],
    corner_steps: [[[Option<u32>; 4]; 2]; 4],
    bindings: graph::Bindings<graph::values::Region>, captures: graph::Captures,
}
enum PublicationError { MissingOutput, UnsupportedAxes, Nonfinite, Extent, Arithmetic(ArithmeticError) }
enum FailureKind {
    Binding { input: Input, reason: ArithmeticError },
    Dimensions { input: Input, arithmetic: Arithmetic, words: [Option<u32>; 2], reason: Option<ArithmeticError> },
    Graph { input: Input, owner: graph::Stopped<graph::values::Frame<Arithmetic>> },
    OtherFlags { input: Input, owner: graph::OtherFlagsRequired<graph::values::Frame<Arithmetic>> },
    Budget { input: Input, owner: graph::Prepared<graph::values::Frame<Arithmetic>> },
    Publication { data: Data, reason: PublicationError },
}
pub(super) struct Failure { kind: FailureKind }
pub(super) struct Placement { data: Data, rect: Rect }
impl Placement {
    pub fn rect(&self) -> Rect { self.rect }
}

pub(super) fn for_image(size: [usize; 2], destination: Rect, mxcsr: u32, errno: i32,
    architecture: ScalarF32Architecture) -> Result<Placement, Failure>
{
    let input = Input { size, destination: [destination.left(), destination.top(), destination.width(), destination.height()].map(f32::to_bits),
        mxcsr, errno, architecture };
    let mut arithmetic = Arithmetic::bind(mxcsr, errno, architecture)
        .map_err(|reason| Failure { kind: FailureKind::Binding { input, reason } })?;
    let mut words = [None; 2];
    for axis in 0..2 {
        let Some(value) = u32::try_from(size[axis]).ok().filter(|value| *value != 0) else {
            return Err(Failure { kind: FailureKind::Dimensions { input, arithmetic, words, reason: None } });
        };
        match arithmetic.dimension(axis, value) {
            Ok(bits) => words[axis] = Some(bits),
            Err(reason) => return Err(Failure { kind: FailureKind::Dimensions { input, arithmetic, words, reason: Some(reason) } }),
        }
    }
    let [Some(width), Some(height)] = words else {
        return Err(Failure { kind: FailureKind::Dimensions { input, arithmetic, words, reason: None } });
    };
    let pending = graph::values::prepare(arithmetic, 0x40, [0, 0, width, height], input.destination);
    let completed = match pending.poll(64) {
        graph::Poll::Complete(completed) => completed,
        graph::Poll::Failed(owner) => return Err(Failure { kind: FailureKind::Graph { input, owner } }),
        graph::Poll::OtherFlags(owner) => return Err(Failure { kind: FailureKind::OtherFlags { input, owner } }),
        graph::Poll::Pending(owner) => return Err(Failure { kind: FailureKind::Budget { input, owner } }),
    };
    let (frame, bindings, captures) = completed.into_completed_parts();
    let (arithmetic, flags, source, destination, matrix) = frame.into_owned_parts();
    let mut data = Data { input, arithmetic, source, destination, flags, matrix, bindings, captures,
        vertices: [[None; 2]; 4], corner_steps: [[[None; 4]; 2]; 4] };
    let rect = match data.display_rect() {
        Ok(rect) => rect,
        Err(reason) => return Err(Failure { kind: FailureKind::Publication { data, reason } }),
    };
    Ok(Placement { data, rect })
}
impl Data {
    fn display_rect(&mut self) -> Result<Rect, PublicationError> {
        let [Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)] = self.matrix else {
            return Err(PublicationError::MissingOutput);
        };
        if b & 0x7fff_ffff != 0 || d & 0x7fff_ffff != 0 { return Err(PublicationError::UnsupportedAxes); }

        for (index, point) in [[0, 0], [self.source[2], 0], [self.source[2], self.source[3]], [0, self.source[3]]].into_iter().enumerate() {
            for (axis, row) in [[a, b, c], [d, e, f]].into_iter().enumerate() {
                let site = Site::DisplayCorner { corner: index, axis };
                let first = self.arithmetic.operation(site, F32Operation::Mul { destination: row[0], source: point[0] }).map_err(PublicationError::Arithmetic)?;
                self.corner_steps[index][axis][0] = Some(first);
                let second = self.arithmetic.operation(site, F32Operation::Mul { destination: row[1], source: point[1] }).map_err(PublicationError::Arithmetic)?;
                self.corner_steps[index][axis][1] = Some(second);
                let sum = self.arithmetic.operation(site, F32Operation::Add { destination: first, source: second }).map_err(PublicationError::Arithmetic)?;
                self.corner_steps[index][axis][2] = Some(sum);
                self.vertices[index][axis] = Some(self.arithmetic.operation(site, F32Operation::Add { destination: sum, source: row[2] }).map_err(PublicationError::Arithmetic)?);
                self.corner_steps[index][axis][3] = self.vertices[index][axis];
            }
        }
        let [[Some(x), Some(y)], _, [Some(right), Some(bottom)], _] = self.vertices else { return Err(PublicationError::MissingOutput) };
        let rect = Rect::from_min_max(pos2(f32::from_bits(x), f32::from_bits(y)), pos2(f32::from_bits(right), f32::from_bits(bottom)));
        if !rect.is_finite() { return Err(PublicationError::Nonfinite); }
        if rect.width() <= 0.0 || rect.height() <= 0.0 { return Err(PublicationError::Extent); }
        Ok(rect)
    }
}
