use std::collections::{TryReserveError, VecDeque};
use crate::{conditional_gate::{Batch, GateError, Projection},
    conditional_ncwn::ConditionedTransform,
    ltv_coefficient_math::{Provider, ScalarUpper}, paired_adopt::pipeline::call,
    paired_condition as condition, paired_condition_worker::NcwnWorker,
    paired_constructor_reset_values as graph, paired_features as features,
    paired_input as input, paired_interpolation as interpolation,
    paired_post::Module, paired_post_join as post,
    paired_projection_module::ProjectionModule, paired_scalar_packing as packing};

pub trait ConstructorReset {
    fn reset_for_constructor(&mut self) -> Result<(), GateError>;
}
impl<P: Projection> ConstructorReset for ProjectionModule<P> {
    fn reset_for_constructor(&mut self) -> Result<(), GateError> {
        ProjectionModule::reset_for_constructor(self)
    }
}
impl<G: ConditionedTransform, S: Projection, R: Projection, H: Projection>
    ConstructorReset for NcwnWorker<G, S, R, H>
{
    fn reset_for_constructor(&mut self) -> Result<(), GateError> {
        NcwnWorker::reset_for_constructor(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub hop: i32, pub groups: i32, pub enabled: bool,

    pub disabled_cursor512: i32,
}
pub struct Modules<M, A, B, W> {
    pub module48: M, pub module56: A, pub module40: B, pub worker64: W,
}
pub struct Providers<R, O, S, U: ScalarUpper, P> {
    pub input_rows: R, pub current_rows: O, pub interpolation_storage: S,
    pub math: Provider<U>, pub frames: P, pub gaussian: features::StandardGaussian,
}

pub struct Scratch { pub input176_indices: Vec<Option<usize>>, pub condition208: Batch }
pub struct Parts<M, R, W, A, B, O, S, U: ScalarUpper, P> {
    pub configuration: Configuration, pub modules: Modules<M, A, B, W>,
    pub providers: Providers<R, O, S, U, P>, pub scratch: Scratch,

    pub packing_output: Batch, pub continuation: call::Continuation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Rate, Groups, ScratchHeader, GaussianWord }
pub enum Cause<M, R, W, A, B, O: packing::CurrentRows, S, U: ScalarUpper, P> {
    Reset(graph::ResetError<GateError>), Acquire(O::Error),

    ResetUnwind(Box<dyn std::any::Any + Send>),
    Scalar(input::BindingFailure<M, R>), Condition(condition::BindingFailure<W>),
    Engine(post::EngineFailure<A, B>), Packing(packing::BindingFailure<O>),
    Interpolation(interpolation::BindingFailure<S, U>),
    Features(features::BindingFailure<P, features::StandardGaussian>),
    Call(call::BindingFailure<M, R, W, A, B, O, S, U, P, features::StandardGaussian>),
}
pub enum Failure<M, R, W, A, B, O: packing::CurrentRows, S, U: ScalarUpper, P> {
    Admission { cause: Domain, parts: Parts<M, R, W, A, B, O, S, U, P> },
    CursorAllocation { cause: TryReserveError, parts: Parts<M, R, W, A, B, O, S, U, P> },
    Stopped(Stopped<M, R, W, A, B, O, S, U, P>),
}
pub struct Stopped<M, R, W, A, B, O: packing::CurrentRows, S, U: ScalarUpper, P> {
    cause: Cause<M, R, W, A, B, O, S, U, P>, owner: Owner<M, R, W, A, B, O, S, U, P>,
}
impl<M, R, W, A, B, O: packing::CurrentRows, S, U: ScalarUpper, P>
    Stopped<M, R, W, A, B, O, S, U, P>
{
    pub fn cause(&self) -> &Cause<M, R, W, A, B, O, S, U, P> { &self.cause }
    pub fn reset_progress(&self) -> Option<graph::ResetProgress> { self.owner.reset_work.as_ref().map(graph::ResetWork::progress) }
    pub fn configuration(&self) -> Configuration { self.owner.configuration }
    pub fn current(&self) -> Option<&[f32]> { self.owner.current.as_deref() }
    pub fn cursor_words(&self) -> &[u32] { &self.owner.workspace.cursors }
    pub fn pending_modules(&self) -> (Option<&B>, Option<&M>, Option<&A>, Option<&W>) {
        (self.owner.module40.as_ref(), self.owner.module48.as_ref(),
            self.owner.module56.as_ref(), self.owner.worker64.as_ref())
    }
    pub fn scalar(&self) -> Option<&input::State<M, R>> { self.owner.scalar.as_ref() }
    pub fn condition(&self) -> Option<&condition::State<W>> { self.owner.condition.as_ref() }
    pub fn engine(&self) -> Option<&post::Engine<A, B>> { self.owner.engine.as_ref() }
    pub fn packing(&self) -> Option<&packing::State<O>> { self.owner.packing.as_ref() }
    pub fn interpolation(&self) -> Option<&interpolation::State<S, U>> { self.owner.interpolation.as_ref() }
    pub fn features(&self) -> Option<&features::State<P, features::StandardGaussian>> { self.owner.features.as_ref() }
}

pub struct Bound<M, R, W, A, B, O, S, U: ScalarUpper, P> {
    call: call::Bound<M, R, W, A, B, O, S, U, P, features::StandardGaussian>,
    reset: graph::ResetComplete, configuration: Configuration,
}
impl<M, R, W, A, B, O, S, U: ScalarUpper, P> Bound<M, R, W, A, B, O, S, U, P> {
    pub fn call(&self) -> &call::Bound<M, R, W, A, B, O, S, U, P, features::StandardGaussian> { &self.call }
    pub fn reset_completion(&self) -> &graph::ResetComplete { &self.reset }
    pub fn configuration(&self) -> Configuration { self.configuration }
    pub fn into_parts(self) -> (call::Bound<M, R, W, A, B, O, S, U, P, features::StandardGaussian>, graph::ResetComplete) {
        (self.call, self.reset)
    }
}

fn source_empty_header() -> Batch { Batch { rows: Vec::new(), status: 0 } }
struct Workspace {
    input112: Option<Batch>, feature144: Option<Batch>, condition208: Option<Batch>,
    retained240: Option<Batch>, stage272: Option<Batch>, final304: Option<Batch>,
    output336: Option<Batch>, indices: Option<Vec<Option<usize>>>, cursors: Vec<u32>,
    seen16: u64, width28: i32, ended32: u8,
}
struct Owner<M, R, W, A, B, O, S, U: ScalarUpper, P> {
    configuration: Configuration, workspace: Workspace,
    module48: Option<M>, module56: Option<A>, module40: Option<B>, worker64: Option<W>,
    input_rows: Option<R>, current_rows: Option<O>, storage: Option<S>, math: Option<Provider<U>>,
    frames: Option<P>, gaussian: Option<features::StandardGaussian>, current: Option<Vec<f32>>,
    packing_output: Option<Batch>, continuation: Option<call::Continuation>,
    reset_work: Option<graph::ResetWork>, reset_complete: Option<graph::ResetComplete>,
    scalar: Option<input::State<M, R>>, condition: Option<condition::State<W>>,
    engine: Option<post::Engine<A, B>>, packing: Option<packing::State<O>>,
    interpolation: Option<interpolation::State<S, U>>,
    features: Option<features::State<P, features::StandardGaussian>>,
}
impl<M: ConstructorReset, R, W: ConstructorReset, A: ConstructorReset, B: ConstructorReset,
    O, S, U: ScalarUpper, P> graph::ResetServices for Owner<M, R, W, A, B, O, S, U, P>
{
    type Error = GateError;
    fn reset_module40(&mut self) -> Result<(), GateError> { self.module40.as_mut().expect("retained module40").reset_for_constructor() }
    fn reset_module48(&mut self) -> Result<(), GateError> { self.module48.as_mut().expect("retained module48").reset_for_constructor() }
    fn reset_module56(&mut self) -> Result<(), GateError> { self.module56.as_mut().expect("retained module56").reset_for_constructor() }
    fn reset_module64(&mut self) -> Result<(), GateError> { self.worker64.as_mut().expect("retained worker64").reset_for_constructor() }
    fn finish_empty_owners(&mut self) -> Result<(), GateError> {

        self.workspace.seen16 = 0; self.workspace.width28 = -1; self.workspace.ended32 = 0;
        self.workspace.cursors.fill(0);
        Ok(())
    }
}

pub fn initialize<M, R, W, A, B, O, S, U, P>(parts: Parts<M, R, W, A, B, O, S, U, P>)
    -> Result<Bound<M, R, W, A, B, O, S, U, P>, Failure<M, R, W, A, B, O, S, U, P>>
where M: Module + ConstructorReset, A: Module + ConstructorReset, B: Module + ConstructorReset,
    W: condition::Worker + ConstructorReset, R: input::Rows, O: packing::CurrentRows,
    S: interpolation::Storage, U: ScalarUpper, P: features::Frames
{
    let c = parts.configuration;
    let cause = if c.hop <= 0 || c.groups <= 0 || c.hop % c.groups != 0 { Some(Domain::Rate) }
        else if c.groups < 2 || c.groups & 1 != 0 || c.groups > i32::MAX / 2 { Some(Domain::Groups) }
        else if !parts.scratch.condition208.rows.is_empty() || parts.scratch.condition208.status != 0 { Some(Domain::ScratchHeader) }
        else if parts.providers.gaussian.uniform_word() != 1 { Some(Domain::GaussianWord) }
        else { None };
    if let Some(cause) = cause { return Err(Failure::Admission { cause, parts }); }
    let mut cursors = Vec::new();
    let count = if c.enabled { 2 } else { 1 };
    if let Err(cause) = cursors.try_reserve_exact(count) { return Err(Failure::CursorAllocation { cause, parts }); }
    cursors.resize(count, 0);
    let Parts { configuration, modules, providers, scratch, packing_output, continuation } = parts;
    let Modules { module48, module56, module40, worker64 } = modules;
    let Providers { input_rows, current_rows, interpolation_storage, math, frames, gaussian } = providers;
    let mut owner = Owner { configuration, workspace: Workspace {
        input112: Some(source_empty_header()), feature144: Some(source_empty_header()),
        condition208: Some(scratch.condition208), retained240: Some(source_empty_header()),
        stage272: Some(source_empty_header()), final304: Some(source_empty_header()), output336: Some(source_empty_header()),
        indices: Some(scratch.input176_indices), cursors, seen16: 0, width28: -1, ended32: 0 },
        module48: Some(module48), module56: Some(module56), module40: Some(module40), worker64: Some(worker64),
        input_rows: Some(input_rows), current_rows: Some(current_rows), storage: Some(interpolation_storage), math: Some(math),
        frames: Some(frames), gaussian: Some(gaussian), current: None, packing_output: Some(packing_output), continuation: Some(continuation),
        reset_work: None, reset_complete: None,
        scalar: None, condition: None, engine: None, packing: None, interpolation: None, features: None };

    let mut work = graph::ResetWork::after_admission();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
        graph::advance(&mut work, &mut owner, usize::MAX)));
    owner.reset_work = Some(work);
    match result {
        Ok(Ok(graph::ResetAdvance::Complete(marker))) => owner.reset_complete = Some(marker),
        Ok(Err(error)) => return Err(Failure::Stopped(Stopped { cause: Cause::Reset(error), owner })),
        Err(payload) => return Err(Failure::Stopped(Stopped { cause: Cause::ResetUnwind(payload), owner })),
        Ok(Ok(graph::ResetAdvance::Paused)) => unreachable!("five callbacks fit the synchronous budget"),
    }
    match owner.current_rows.as_mut().expect("retained required current provider").acquire_current() {
        Ok(current) => owner.current = Some(current),
        Err(error) => return Err(Failure::Stopped(Stopped { cause: Cause::Acquire(error), owner })),
    }
    match assemble(&mut owner) {
        Ok(call) => Ok(Bound { call, reset: owner.reset_complete.take().expect("same completed reset"), configuration }),
        Err(cause) => Err(Failure::Stopped(Stopped { cause, owner })),
    }
}

fn assemble<M, R, W, A, B, O: packing::CurrentRows, S, U: ScalarUpper, P>(
    owner: &mut Owner<M, R, W, A, B, O, S, U, P>)
    -> Result<call::Bound<M, R, W, A, B, O, S, U, P, features::StandardGaussian>, Cause<M, R, W, A, B, O, S, U, P>>
{
    let c = owner.configuration; let hop = c.hop as usize; let groups = c.groups as usize;
    owner.scalar = Some(input::State::from_explicit_parts(input::Parts {
        module48: owner.module48.take().expect("same module48"), rows: owner.input_rows.take().expect("same preparation provider"),
        hop, groups, latency496: 0, ended517: 0,
        input112: owner.workspace.input112.take().expect("source input112"),
        feature144: owner.workspace.feature144.take().expect("source feature144"),
    }).map_err(Cause::Scalar)?);
    let cursors = std::mem::take(&mut owner.workspace.cursors);
    owner.condition = Some(condition::State::from_explicit_parts(condition::Parts {
        worker64: owner.worker64.take().expect("same worker64"), cursor_index508: 0, ended517: 0,
        cursor_count: cursors.len(), cursor_words: cursors, cursor_halted: false,
        input176_indices: owner.workspace.indices.take().expect("same initialized ordinal backing"), input176_len: 0, input176_status: 0,
        conditions432: VecDeque::new(), condition208: owner.workspace.condition208.take().expect("same empty208"),
        retained240: owner.workspace.retained240.take().expect("source retained240"),
    }).map_err(Cause::Condition)?);
    owner.engine = Some(post::Engine::from_explicit_parts(post::EngineParts {
        module56: owner.module56.take().expect("same module56"), module40: owner.module40.take().expect("same module40"),
        groups, enabled: c.enabled, ended: 0, cursor_index512: if c.enabled { 1 } else { c.disabled_cursor512 },
        stage272: owner.workspace.stage272.take().expect("source stage272"), final304: owner.workspace.final304.take().expect("source final304"),
    }).map_err(Cause::Engine)?);
    owner.packing = Some(packing::State::from_explicit_parts(packing::Parts {
        frame: hop, filled: 0, ended: 0, current: owner.current.take(),
        output: owner.packing_output.take().expect("same caller output header"), rows: owner.current_rows.take().expect("same advanced current provider"),
    }).map_err(Cause::Packing)?);
    owner.interpolation = Some(interpolation::State::from_explicit_parts(interpolation::Parts {
        storage: owner.storage.take().expect("same interpolation storage"), math: owner.math.take().expect("same scalar owner"),
        history8: None, seen16: owner.workspace.seen16, factor24: c.hop / c.groups,
        width28: owner.workspace.width28, ended32: owner.workspace.ended32,
        output336: owner.workspace.output336.take().expect("source output336"),
    }).map_err(Cause::Interpolation)?);
    owner.features = Some(features::State::from_explicit_parts(features::Parts {
        groups, frames: owner.frames.take().expect("same frame provider"), gaussian: owner.gaussian.take().expect("same explicit word1 Gaussian"),
        queue400: VecDeque::new(),
    }).map_err(Cause::Features)?);

    call::Bound::from_explicit_parts(call::Parts {
        continuation: owner.continuation.take().expect("same caller continuation"),
        scalar: owner.scalar.take().expect("same scalar state"), condition: owner.condition.take().expect("same condition state"),
        engine: owner.engine.take().expect("same post engine"), packing: owner.packing.take().expect("same packing state"),
        interpolation: owner.interpolation.take().expect("same interpolation state"), features: owner.features.take().expect("same feature state"),
    }).map_err(Cause::Call)
}
