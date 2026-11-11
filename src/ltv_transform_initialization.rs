use std::cell::Cell;
use crate::{ltv_analysis_window::SourceLtvWindowCosine,
    ltv_coefficient_math as coefficient,
    ltv_first_call::{FirstCallBindingError, SourceTransformMathProfile, SourceTransformSelection,
        SourceTransformSelectionMath, SourceTransformSelector},
    ltv_render::{ConfiguredFft, FftBindingFailure, SourceFftBindingFailure, LtvArithmeticPolicy},
    ltv_value_binding::Identity, ltv_value_initialization as value,
    model2_operator_binding::{Ltvfs1Config13, OperatorBindingError, SourceLtvfs1Binding},
    userdata::{self, Role}};
use crate::source_scalar::{ExplicitExceptionState, SourceF64ValueMath, SourceFiniteCosineWordZero,
    SourceFiniteCosineWordZeroError, shared_scalar_f32::ScalarF32Architecture};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Runtime { pub owner_rcx: u64, pub linked50: u64, pub word_e8: i32, pub word_f0: i32 }

pub struct Inputs<M, F> {
    pub identity: Identity, pub frame: i32, pub arithmetic: LtvArithmeticPolicy,
    pub storage: value::StorageWords, pub outputs: value::OutputPolicy,
    pub history: value::History, pub math: M, pub fft: F,
}
impl<M, F> Inputs<M, F> {
    fn into_initializer(self, transform_length: usize) -> value::Inputs<M, F> {
        let Self { identity, frame, arithmetic, storage, outputs, history, math, fft } = self;
        value::Inputs { identity, frame, transform_length, arithmetic, storage, outputs, history, math, fft }
    }
}

pub struct ConfiguredInputs<M> {
    pub identity: Identity, pub frame: i32, pub arithmetic: LtvArithmeticPolicy,
    pub storage: value::StorageWords, pub outputs: value::OutputPolicy,
    pub history: value::History, pub math: M,
}
impl<M> ConfiguredInputs<M> {
    fn into_initializer(self, transform_length: usize, fft: ConfiguredFft) -> value::Inputs<M, ConfiguredFft> {
        let Self { identity, frame, arithmetic, storage, outputs, history, math } = self;
        value::Inputs { identity, frame, transform_length, arithmetic, storage, outputs, history, math, fft }
    }
}

pub struct SourceWorkspace {
    pub identity: Identity, pub frame: i32, pub arithmetic: LtvArithmeticPolicy,
    pub storage: value::StorageWords, pub outputs: value::OutputPolicy,
    pub history: value::History,
}
impl SourceWorkspace {
    fn with_math<M>(self, math: M) -> ConfiguredInputs<M> {
        let Self { identity, frame, arithmetic, storage, outputs, history } = self;
        ConfiguredInputs { identity, frame, arithmetic, storage, outputs, history, math }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceWindowParts {
    pub source_word: u32, pub architecture: ScalarF32Architecture, pub state: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientEntry {
    pub configuration: coefficient::Configuration, pub state: ExplicitExceptionState,
}
pub struct SourceCoefficientParts<U> { pub entry: SourceCoefficientEntry, pub upper: U }

pub struct SourceConfiguredInputs<U> {
    pub workspace: SourceWorkspace, pub window: SourceWindowParts,
    pub coefficient: SourceCoefficientParts<U>,
}

pub struct InitialSourceWorkspace {
    pub identity: Identity, pub frame: i32, pub arithmetic: LtvArithmeticPolicy,
    pub outputs: value::OutputPolicy,
}
pub struct InitialSourceConfiguredInputs<U> {
    pub workspace: InitialSourceWorkspace, pub window: SourceWindowParts,
    pub coefficient: SourceCoefficientParts<U>,
}

pub struct InitialConfiguredInputs<M> {
    pub workspace: InitialSourceWorkspace, pub math: M,
}
impl<M> InitialConfiguredInputs<M> {
    fn into_initializer(self, fft: ConfiguredFft) -> value::InitialInputs<M, ConfiguredFft> {
        let Self { workspace: InitialSourceWorkspace { identity, frame, arithmetic, outputs }, math } = self;
        value::InitialInputs { identity, frame, arithmetic, outputs, math, fft }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { Log582ff8(u64), Ceil57dc64(u64), TruncateI32(u64) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultBits { Float(u64), Signed32(i32) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call { pub operation: Operation, pub result: Option<ResultBits> }
pub enum SelectionError<E> { Math(E), TraceCapacity }

pub struct MathTrace<T> {
    owner: T, profile: Cell<Option<SourceTransformMathProfile>>,
    calls: [Option<Call>; 3], attempted: usize,
}
impl<T> MathTrace<T> {
    fn new(owner: T) -> Self {
        Self { owner, profile: Cell::new(None), calls: [None, None, None], attempted: 0 }
    }
    pub fn owner(&self) -> &T { &self.owner }
    pub fn profile(&self) -> Option<SourceTransformMathProfile> { self.profile.get() }
    pub fn calls(&self) -> &[Option<Call>; 3] { &self.calls }
    pub fn attempted(&self) -> usize { self.attempted }
    fn begin<E>(&mut self, operation: Operation) -> Result<usize, SelectionError<E>> {

        if self.attempted == self.calls.len() { return Err(SelectionError::TraceCapacity); }
        let slot = self.attempted;
        self.calls[slot] = Some(Call { operation, result: None });
        self.attempted += 1;
        Ok(slot)
    }
}
impl<T: SourceTransformSelectionMath> SourceTransformSelectionMath for MathTrace<T> {
    type Error = SelectionError<T::Error>;
    fn explicit_profile(&self) -> SourceTransformMathProfile {
        let profile = self.owner.explicit_profile();
        self.profile.set(Some(profile));
        profile
    }
    fn helper582ff8(&mut self, input: f64) -> Result<f64, Self::Error> {
        let operation = Operation::Log582ff8(input.to_bits());
        let slot = self.begin::<T::Error>(operation)?;
        let result = self.owner.helper582ff8(input).map_err(SelectionError::Math)?;
        self.calls[slot] = Some(Call { operation, result: Some(ResultBits::Float(result.to_bits())) });
        Ok(result)
    }
    fn helper57dc64(&mut self, input: f64) -> Result<f64, Self::Error> {
        let operation = Operation::Ceil57dc64(input.to_bits());
        let slot = self.begin::<T::Error>(operation)?;
        let result = self.owner.helper57dc64(input).map_err(SelectionError::Math)?;
        self.calls[slot] = Some(Call { operation, result: Some(ResultBits::Float(result.to_bits())) });
        Ok(result)
    }
    fn cvttsd2si_i32(&mut self, input: f64) -> Result<i32, Self::Error> {
        let operation = Operation::TruncateI32(input.to_bits());
        let slot = self.begin::<T::Error>(operation)?;
        let result = self.owner.cvttsd2si_i32(input).map_err(SelectionError::Math)?;
        self.calls[slot] = Some(Call { operation, result: Some(ResultBits::Signed32(result)) });
        Ok(result)
    }
}
pub struct Selection<T> {
    pub role: Role, pub runtime: Runtime, pub binding: SourceLtvfs1Binding,
    pub actual: SourceTransformSelection, pub math: MathTrace<T>,
}
pub struct Bound<C, T, M, F> { pub selection: Selection<T>, pub initialized: value::Bound<C, M, F> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError { Role(Role), MissingConfiguration(Role), Identity(OperatorBindingError), Selector(FirstCallBindingError) }
pub enum Failure<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M, F> {
    Admission { role: Role, runtime: Runtime, configuration: Option<Ltvfs1Config13>, cause: AdmissionError,
        inputs: Inputs<M, F>, cosine: C, math: T },
    Selection { role: Role, runtime: Runtime, binding: SourceLtvfs1Binding, selector: SourceTransformSelector,
        inputs: Inputs<M, F>, cosine: C, math: MathTrace<T>, cause: SelectionError<T::Error> },
    TransformWord { selection: Selection<T>, inputs: Inputs<M, F>, cosine: C },
    Initialization { selection: Selection<T>, failure: value::Failure<C, M, F> },
}

pub enum SelectionFailure<T: SourceTransformSelectionMath> {
    Admission { role: Role, runtime: Runtime, configuration: Option<Ltvfs1Config13>, cause: AdmissionError, math: T },
    Selection { role: Role, runtime: Runtime, binding: SourceLtvfs1Binding, selector: SourceTransformSelector,
        math: MathTrace<T>, cause: SelectionError<T::Error> },
    TransformWord { selection: Selection<T> },
}
impl<T: SourceTransformSelectionMath> SelectionFailure<T> {
    fn with_explicit_inputs<C: SourceLtvWindowCosine, M, F>(self, inputs: Inputs<M, F>, cosine: C)
        -> Failure<C, T, M, F>
    {
        match self {
            Self::Admission { role, runtime, configuration, cause, math } =>
                Failure::Admission { role, runtime, configuration, cause, inputs, cosine, math },
            Self::Selection { role, runtime, binding, selector, math, cause } =>
                Failure::Selection { role, runtime, binding, selector, inputs, cosine, math, cause },
            Self::TransformWord { selection } => Failure::TransformWord { selection, inputs, cosine },
        }
    }
}
pub enum ConfiguredFailure<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M> {
    Selection { failure: SelectionFailure<T>, inputs: ConfiguredInputs<M>, cosine: C },
    Shape { selection: Selection<T>, inputs: ConfiguredInputs<M>, cosine: C, cause: value::ShapeError },
    Fft { selection: Selection<T>, inputs: ConfiguredInputs<M>, cosine: C, failure: FftBindingFailure },
    Initialization { selection: Selection<T>, failure: value::Failure<C, M, ConfiguredFft> },
}
pub enum InitialConfiguredFailure<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M> {
    Selection { failure: SelectionFailure<T>, inputs: InitialConfiguredInputs<M>, cosine: C },
    Shape { selection: Selection<T>, inputs: InitialConfiguredInputs<M>, cosine: C, cause: value::ShapeError },
    Fft { selection: Selection<T>, inputs: InitialConfiguredInputs<M>, cosine: C, failure: FftBindingFailure },
    Initialization { selection: Selection<T>, failure: value::InitialFailure<C, M, ConfiguredFft> },
}
pub enum SourceFftConfiguredCause<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    WindowBinding { role: Role, runtime: Runtime, inputs: SourceConfiguredInputs<U>,
        math: T, fft_math: SourceF64ValueMath, fft_source_word: u32, cause: SourceFiniteCosineWordZeroError },
    CoefficientBinding { role: Role, runtime: Runtime, workspace: SourceWorkspace,
        cosine: SourceFiniteCosineWordZero, math: T, fft_math: SourceF64ValueMath,
        fft_source_word: u32, failure: coefficient::BindingFailure<U> },
    Selection { failure: SelectionFailure<T>, inputs: ConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, fft_math: SourceF64ValueMath, fft_source_word: u32 },
    Shape { selection: Selection<T>, inputs: ConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, fft_math: SourceF64ValueMath, fft_source_word: u32,
        cause: value::ShapeError },
    Fft { selection: Selection<T>, inputs: ConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, failure: SourceFftBindingFailure },
    Initialization { selection: Selection<T>,
        failure: value::Failure<SourceFiniteCosineWordZero, coefficient::Provider<U>, ConfiguredFft> },
}

pub struct SourceFftConfiguredFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
    cause: SourceFftConfiguredCause<T, U>,
}
impl<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> SourceFftConfiguredFailure<T, U> {
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }
    pub fn cause(&self) -> &SourceFftConfiguredCause<T, U> { &self.cause }
}
pub struct SourceConfiguredBound<T, U: coefficient::ScalarUpper> {
    initialized: Bound<SourceFiniteCosineWordZero, T, coefficient::Provider<U>, ConfiguredFft>,
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
}

pub struct SourceInitializationHistory<T> {
    selection: Selection<T>, window: value::WindowHistory<SourceFiniteCosineWordZero>,
    role: Role, configuration: Ltvfs1Config13,
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
}
impl<T> SourceInitializationHistory<T> {
    pub fn selection(&self) -> &Selection<T> { &self.selection }
    pub fn window(&self) -> &value::WindowHistory<SourceFiniteCosineWordZero> { &self.window }
    pub fn role(&self) -> Role { self.role }
    pub fn configuration(&self) -> Ltvfs1Config13 { self.configuration }
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }
    pub fn into_parts(self) -> (Selection<T>, value::WindowHistory<SourceFiniteCosineWordZero>,
        Role, Ltvfs1Config13, SourceWindowParts, SourceCoefficientEntry)
    {
        (self.selection, self.window, self.role, self.configuration, self.window_entry, self.coefficient_entry)
    }
}
impl<T, U: coefficient::ScalarUpper> SourceConfiguredBound<T, U> {
    pub fn initialized(&self) -> &Bound<SourceFiniteCosineWordZero, T, coefficient::Provider<U>, ConfiguredFft> {
        &self.initialized
    }
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }

    pub fn into_parts(self) -> (Bound<SourceFiniteCosineWordZero, T, coefficient::Provider<U>, ConfiguredFft>,
        SourceWindowParts, SourceCoefficientEntry)
    {
        (self.initialized, self.window_entry, self.coefficient_entry)
    }

    pub fn into_state_and_history(self) -> (
        crate::ltv_filter::State<coefficient::Provider<U>, ConfiguredFft,
            crate::ltv_stream::Kernel<crate::ltv_output_values::ValueOutputs>>,
        SourceInitializationHistory<T>)
    {
        let Self { initialized: Bound { selection, initialized: value::Bound {
            initialized: crate::ltv_value_binding::Bound { role, configuration, state }, window } },
            window_entry, coefficient_entry } = self;
        (state, SourceInitializationHistory { selection, window, role, configuration,
            window_entry, coefficient_entry })
    }
}
pub enum SourceConfiguredFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    WindowBinding { role: Role, runtime: Runtime, inputs: SourceConfiguredInputs<U>,
        math: T, cause: SourceFiniteCosineWordZeroError },
    CoefficientBinding { role: Role, runtime: Runtime, workspace: SourceWorkspace,
        window_entry: SourceWindowParts, cosine: SourceFiniteCosineWordZero,
        math: T, failure: coefficient::BindingFailure<U> },
    Initialization(SourceConfiguredInitializationFailure<T, U>),
}

pub struct SourceConfiguredInitializationFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
    failure: ConfiguredFailure<SourceFiniteCosineWordZero, T, coefficient::Provider<U>>,
}
impl<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> SourceConfiguredInitializationFailure<T, U> {
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }
    pub fn failure(&self) -> &ConfiguredFailure<SourceFiniteCosineWordZero, T, coefficient::Provider<U>> {
        &self.failure
    }
}

pub enum InitialSourceConfiguredFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    WindowBinding { role: Role, runtime: Runtime, inputs: InitialSourceConfiguredInputs<U>,
        math: T, cause: SourceFiniteCosineWordZeroError },
    CoefficientBinding { role: Role, runtime: Runtime, workspace: InitialSourceWorkspace,
        window_entry: SourceWindowParts, cosine: SourceFiniteCosineWordZero,
        math: T, failure: coefficient::BindingFailure<U> },
    Initialization(InitialSourceInitializationFailure<T, U>),
}

pub struct InitialSourceInitializationFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
    failure: InitialConfiguredFailure<SourceFiniteCosineWordZero, T, coefficient::Provider<U>>,
}
impl<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> InitialSourceInitializationFailure<T, U> {
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }
    pub fn failure(&self) -> &InitialConfiguredFailure<SourceFiniteCosineWordZero, T, coefficient::Provider<U>> {
        &self.failure
    }
}

pub enum InitialSourceFftConfiguredCause<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    WindowBinding { role: Role, runtime: Runtime, inputs: InitialSourceConfiguredInputs<U>,
        math: T, fft_math: SourceF64ValueMath, fft_source_word: u32, cause: SourceFiniteCosineWordZeroError },
    CoefficientBinding { role: Role, runtime: Runtime, workspace: InitialSourceWorkspace,
        cosine: SourceFiniteCosineWordZero, math: T, fft_math: SourceF64ValueMath,
        fft_source_word: u32, failure: coefficient::BindingFailure<U> },
    Selection { failure: SelectionFailure<T>, inputs: InitialConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, fft_math: SourceF64ValueMath, fft_source_word: u32 },
    Shape { selection: Selection<T>, inputs: InitialConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, fft_math: SourceF64ValueMath, fft_source_word: u32,
        cause: value::ShapeError },
    Fft { selection: Selection<T>, inputs: InitialConfiguredInputs<coefficient::Provider<U>>,
        cosine: SourceFiniteCosineWordZero, failure: SourceFftBindingFailure },
    Initialization { selection: Selection<T>,
        failure: value::InitialFailure<SourceFiniteCosineWordZero, coefficient::Provider<U>, ConfiguredFft> },
}

pub struct InitialSourceFftConfiguredFailure<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> {
    window_entry: SourceWindowParts, coefficient_entry: SourceCoefficientEntry,
    cause: InitialSourceFftConfiguredCause<T, U>,
}
impl<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper> InitialSourceFftConfiguredFailure<T, U> {
    pub fn window_entry(&self) -> SourceWindowParts { self.window_entry }
    pub fn coefficient_entry(&self) -> SourceCoefficientEntry { self.coefficient_entry }
    pub fn cause(&self) -> &InitialSourceFftConfiguredCause<T, U> { &self.cause }
}

pub fn initialize_source_configured_userdata<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: SourceConfiguredInputs<U>, math: T)
    -> Result<SourceConfiguredBound<T, U>, SourceConfiguredFailure<T, U>>
{
    let entry = inputs.window;
    let cosine = match SourceFiniteCosineWordZero::bind(entry.source_word, entry.architecture, entry.state) {
        Ok(cosine) => cosine,
        Err(cause) => return Err(SourceConfiguredFailure::WindowBinding { role, runtime, inputs, math, cause }),
    };
    let SourceConfiguredInputs { workspace, window: window_entry,
        coefficient: SourceCoefficientParts { entry: coefficient_entry, upper } } = inputs;
    let provider = match coefficient::Provider::from_explicit_parts(coefficient_entry.configuration,
        coefficient_entry.state, upper)
    {
        Ok(provider) => provider,
        Err(failure) => return Err(SourceConfiguredFailure::CoefficientBinding {
            role, runtime, workspace, window_entry, cosine, math, failure }),
    };
    match initialize_configured_userdata(userdata, role, runtime, workspace.with_math(provider), cosine, math) {
        Ok(initialized) => Ok(SourceConfiguredBound { initialized, window_entry, coefficient_entry }),
        Err(failure) => Err(SourceConfiguredFailure::Initialization(
            SourceConfiguredInitializationFailure { window_entry, coefficient_entry, failure })),
    }
}

pub fn initialize_source_configured_fft_userdata<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: SourceConfiguredInputs<U>, fft_math: SourceF64ValueMath,
    fft_source_word: u32, math: T)
    -> Result<SourceConfiguredBound<T, U>, SourceFftConfiguredFailure<T, U>>
{
    let window_entry = inputs.window;
    let coefficient_entry = inputs.coefficient.entry;
    let refused = |cause| SourceFftConfiguredFailure { window_entry, coefficient_entry, cause };
    let cosine = match SourceFiniteCosineWordZero::bind(window_entry.source_word,
        window_entry.architecture, window_entry.state)
    {
        Ok(cosine) => cosine,
        Err(cause) => return Err(refused(SourceFftConfiguredCause::WindowBinding {
            role, runtime, inputs, math, fft_math, fft_source_word, cause })),
    };
    let SourceConfiguredInputs { workspace, coefficient: SourceCoefficientParts { entry, upper }, .. } = inputs;
    let provider = match coefficient::Provider::from_explicit_parts(entry.configuration, entry.state, upper) {
        Ok(provider) => provider,
        Err(failure) => return Err(refused(SourceFftConfiguredCause::CoefficientBinding {
            role, runtime, workspace, cosine, math, fft_math, fft_source_word, failure })),
    };
    let inputs = workspace.with_math(provider);
    let identity = inputs.identity;
    let frame = inputs.frame;
    let (selection, inputs, cosine, config, fft) = match admit_configured(
        userdata, role, runtime, identity, frame, inputs, cosine, math,
        SourceFftRequest { math: fft_math, source_word: fft_source_word })
    {
        Ok(admitted) => admitted,
        Err(ConfiguredAdmissionFailure::Selection { failure, inputs, cosine,
            fft: SourceFftRequest { math: fft_math, source_word: fft_source_word } }) =>
            return Err(refused(SourceFftConfiguredCause::Selection {
                failure, inputs, cosine, fft_math, fft_source_word })),
        Err(ConfiguredAdmissionFailure::Shape { selection, inputs, cosine, cause,
            fft: SourceFftRequest { math: fft_math, source_word: fft_source_word } }) =>
            return Err(refused(SourceFftConfiguredCause::Shape {
                selection, inputs, cosine, fft_math, fft_source_word, cause })),
        Err(ConfiguredAdmissionFailure::Fft { selection, inputs, cosine, failure }) =>
            return Err(refused(SourceFftConfiguredCause::Fft { selection, inputs, cosine, failure })),
    };
    match value::initialize_shaped(role, selection.binding,
        inputs.into_initializer(config.transform_length(), fft), config, cosine)
    {
        Ok(initialized) => Ok(SourceConfiguredBound {
            initialized: Bound { selection, initialized }, window_entry, coefficient_entry }),
        Err(failure) => Err(refused(SourceFftConfiguredCause::Initialization { selection, failure })),
    }
}

pub fn initialize_initial_source_configured_userdata<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: InitialSourceConfiguredInputs<U>, math: T)
    -> Result<SourceConfiguredBound<T, U>, InitialSourceConfiguredFailure<T, U>>
{
    let entry = inputs.window;
    let cosine = match SourceFiniteCosineWordZero::bind(entry.source_word, entry.architecture, entry.state) {
        Ok(cosine) => cosine,
        Err(cause) => return Err(InitialSourceConfiguredFailure::WindowBinding { role, runtime, inputs, math, cause }),
    };
    let InitialSourceConfiguredInputs { workspace, window: window_entry,
        coefficient: SourceCoefficientParts { entry: coefficient_entry, upper } } = inputs;
    let provider = match coefficient::Provider::from_explicit_parts(coefficient_entry.configuration,
        coefficient_entry.state, upper)
    {
        Ok(provider) => provider,
        Err(failure) => return Err(InitialSourceConfiguredFailure::CoefficientBinding {
            role, runtime, workspace, window_entry, cosine, math, failure }),
    };
    let inputs = InitialConfiguredInputs { workspace, math: provider };
    match initialize_initial_configured(userdata, role, runtime, inputs, cosine, math) {
        Ok(initialized) => Ok(SourceConfiguredBound { initialized, window_entry, coefficient_entry }),
        Err(failure) => Err(InitialSourceConfiguredFailure::Initialization(
            InitialSourceInitializationFailure { window_entry, coefficient_entry, failure })),
    }
}

pub fn initialize_initial_source_configured_fft_userdata<T: SourceTransformSelectionMath, U: coefficient::ScalarUpper>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: InitialSourceConfiguredInputs<U>, fft_math: SourceF64ValueMath,
    fft_source_word: u32, math: T)
    -> Result<SourceConfiguredBound<T, U>, InitialSourceFftConfiguredFailure<T, U>>
{
    let window_entry = inputs.window;
    let coefficient_entry = inputs.coefficient.entry;
    let refused = |cause| InitialSourceFftConfiguredFailure { window_entry, coefficient_entry, cause };
    let cosine = match SourceFiniteCosineWordZero::bind(window_entry.source_word,
        window_entry.architecture, window_entry.state)
    {
        Ok(cosine) => cosine,
        Err(cause) => return Err(refused(InitialSourceFftConfiguredCause::WindowBinding {
            role, runtime, inputs, math, fft_math, fft_source_word, cause })),
    };
    let InitialSourceConfiguredInputs { workspace,
        coefficient: SourceCoefficientParts { entry, upper }, .. } = inputs;
    let provider = match coefficient::Provider::from_explicit_parts(entry.configuration, entry.state, upper) {
        Ok(provider) => provider,
        Err(failure) => return Err(refused(InitialSourceFftConfiguredCause::CoefficientBinding {
            role, runtime, workspace, cosine, math, fft_math, fft_source_word, failure })),
    };
    match initialize_initial_configured_request(userdata, role, runtime,
        InitialConfiguredInputs { workspace, math: provider }, cosine, math,
        SourceFftRequest { math: fft_math, source_word: fft_source_word })
    {
        Ok(initialized) => Ok(SourceConfiguredBound { initialized, window_entry, coefficient_entry }),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Selection {
            failure, inputs, cosine, fft: SourceFftRequest { math: fft_math, source_word: fft_source_word } })) =>
            Err(refused(InitialSourceFftConfiguredCause::Selection {
                failure, inputs, cosine, fft_math, fft_source_word })),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Shape {
            selection, inputs, cosine, cause,
            fft: SourceFftRequest { math: fft_math, source_word: fft_source_word } })) =>
            Err(refused(InitialSourceFftConfiguredCause::Shape {
                selection, inputs, cosine, fft_math, fft_source_word, cause })),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Fft {
            selection, inputs, cosine, failure })) =>
            Err(refused(InitialSourceFftConfiguredCause::Fft { selection, inputs, cosine, failure })),
        Err(InitialRequestFailure::Initialization { selection, failure }) =>
            Err(refused(InitialSourceFftConfiguredCause::Initialization { selection, failure })),
    }
}

pub fn initialize_userdata<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M, F>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: Inputs<M, F>, cosine: C, math: T) -> Result<Bound<C, T, M, F>, Failure<C, T, M, F>>
{
    let selection = match select(userdata, role, runtime, inputs.identity, inputs.frame, math) {
        Ok(selection) => selection,
        Err(failure) => return Err(failure.with_explicit_inputs(inputs, cosine)),
    };
    let transform_length = selection.actual.raw_eax() as i32 as usize;
    match value::initialize_admitted(role, selection.binding, inputs.into_initializer(transform_length), cosine) {
        Ok(initialized) => Ok(Bound { selection, initialized }),
        Err(failure) => Err(Failure::Initialization { selection, failure }),
    }
}

pub fn initialize_configured_userdata<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: ConfiguredInputs<M>, cosine: C, math: T)
    -> Result<Bound<C, T, M, ConfiguredFft>, ConfiguredFailure<C, T, M>>
{
    let identity = inputs.identity;
    let frame = inputs.frame;
    let (selection, inputs, cosine, config, fft) = match admit_configured(
        userdata, role, runtime, identity, frame, inputs, cosine, math, HostFftRequest)
    {
        Ok(admitted) => admitted,
        Err(ConfiguredAdmissionFailure::Selection { failure, inputs, cosine, fft: HostFftRequest }) =>
            return Err(ConfiguredFailure::Selection { failure, inputs, cosine }),
        Err(ConfiguredAdmissionFailure::Shape { selection, inputs, cosine, cause, fft: HostFftRequest }) =>
            return Err(ConfiguredFailure::Shape { selection, inputs, cosine, cause }),
        Err(ConfiguredAdmissionFailure::Fft { selection, inputs, cosine, failure }) =>
            return Err(ConfiguredFailure::Fft { selection, inputs, cosine, failure }),
    };
    let inputs = inputs.into_initializer(config.transform_length(), fft);
    match value::initialize_shaped(role, selection.binding, inputs, config, cosine) {
        Ok(initialized) => Ok(Bound { selection, initialized }),
        Err(failure) => Err(ConfiguredFailure::Initialization { selection, failure }),
    }
}

fn initialize_initial_configured<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: InitialConfiguredInputs<M>, cosine: C, math: T)
    -> Result<Bound<C, T, M, ConfiguredFft>, InitialConfiguredFailure<C, T, M>>
{
    match initialize_initial_configured_request(userdata, role, runtime, inputs, cosine, math, HostFftRequest) {
        Ok(bound) => Ok(bound),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Selection {
            failure, inputs, cosine, fft: HostFftRequest })) =>
            Err(InitialConfiguredFailure::Selection { failure, inputs, cosine }),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Shape {
            selection, inputs, cosine, cause, fft: HostFftRequest })) =>
            Err(InitialConfiguredFailure::Shape { selection, inputs, cosine, cause }),
        Err(InitialRequestFailure::Admission(ConfiguredAdmissionFailure::Fft {
            selection, inputs, cosine, failure })) =>
            Err(InitialConfiguredFailure::Fft { selection, inputs, cosine, failure }),
        Err(InitialRequestFailure::Initialization { selection, failure }) =>
            Err(InitialConfiguredFailure::Initialization { selection, failure }),
    }
}

enum InitialRequestFailure<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M, Q: FftRequest> {
    Admission(ConfiguredAdmissionFailure<C, T, InitialConfiguredInputs<M>, Q>),
    Initialization { selection: Selection<T>, failure: value::InitialFailure<C, M, ConfiguredFft> },
}

fn initialize_initial_configured_request<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, M, Q: FftRequest>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime,
    inputs: InitialConfiguredInputs<M>, cosine: C, math: T, fft: Q)
    -> Result<Bound<C, T, M, ConfiguredFft>, InitialRequestFailure<C, T, M, Q>>
{
    let identity = inputs.workspace.identity;
    let frame = inputs.workspace.frame;
    let (selection, inputs, cosine, config, fft) = admit_configured(
        userdata, role, runtime, identity, frame, inputs, cosine, math, fft)
        .map_err(InitialRequestFailure::Admission)?;
    match value::initialize_initial_shaped(role, selection.binding, inputs.into_initializer(fft), config, cosine) {
        Ok(initialized) => Ok(Bound { selection, initialized }),
        Err(failure) => Err(InitialRequestFailure::Initialization { selection, failure }),
    }
}

trait FftRequest {
    type Failure;
    fn bind(self, config: crate::ltv_render::SourceLtvBlockConfig) -> Result<ConfiguredFft, Self::Failure>;
}
struct HostFftRequest;
impl FftRequest for HostFftRequest {
    type Failure = FftBindingFailure;
    fn bind(self, config: crate::ltv_render::SourceLtvBlockConfig) -> Result<ConfiguredFft, Self::Failure> {
        ConfiguredFft::from_explicit_config(config)
    }
}
struct SourceFftRequest { math: SourceF64ValueMath, source_word: u32 }
impl FftRequest for SourceFftRequest {
    type Failure = SourceFftBindingFailure;
    fn bind(self, config: crate::ltv_render::SourceLtvBlockConfig) -> Result<ConfiguredFft, Self::Failure> {
        ConfiguredFft::from_explicit_source_math(config, self.math, self.source_word)
    }
}
enum ConfiguredAdmissionFailure<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, R, Q: FftRequest> {
    Selection { failure: SelectionFailure<T>, inputs: R, cosine: C, fft: Q },
    Shape { selection: Selection<T>, inputs: R, cosine: C, cause: value::ShapeError, fft: Q },
    Fft { selection: Selection<T>, inputs: R, cosine: C, failure: Q::Failure },
}
type ConfiguredAdmission<C, T, R> = (Selection<T>, R, C,
    crate::ltv_render::SourceLtvBlockConfig, ConfiguredFft);

fn admit_configured<C: SourceLtvWindowCosine, T: SourceTransformSelectionMath, R, Q: FftRequest>(
    userdata: &userdata::Decoded<'_>, role: Role, runtime: Runtime, identity: Identity,
    frame: i32, inputs: R, cosine: C, math: T, fft: Q)
    -> Result<ConfiguredAdmission<C, T, R>, ConfiguredAdmissionFailure<C, T, R, Q>>
{
    let selection = match select(userdata, role, runtime, identity, frame, math) {
        Ok(selection) => selection,
        Err(failure) => return Err(ConfiguredAdmissionFailure::Selection { failure, inputs, cosine, fft }),
    };
    let config = match value::shape(selection.binding.config(), frame,
        selection.actual.raw_eax() as i32 as usize)
    {
        Ok(config) => config,
        Err(cause) => return Err(ConfiguredAdmissionFailure::Shape { selection, inputs, cosine, cause, fft }),
    };
    let fft = match fft.bind(config) {
        Ok(fft) => fft,
        Err(failure) => return Err(ConfiguredAdmissionFailure::Fft { selection, inputs, cosine, failure }),
    };
    Ok((selection, inputs, cosine, config, fft))
}

fn select<T: SourceTransformSelectionMath>(userdata: &userdata::Decoded<'_>, role: Role,
    runtime: Runtime, identity: Identity, frame: i32, math: T)
    -> Result<Selection<T>, SelectionFailure<T>>
{
    if !matches!(role, Role::Filter136 | Role::Filter144 | Role::ExtraFilter) {
        return Err(SelectionFailure::Admission { role, runtime, configuration: None,
            cause: AdmissionError::Role(role), math });
    }
    let configuration = match userdata.primary().filter(role) {
        Some(configuration) => configuration,
        None => return Err(SelectionFailure::Admission { role, runtime, configuration: None,
            cause: AdmissionError::MissingConfiguration(role), math }),
    };
    let binding = match SourceLtvfs1Binding::bind(identity.decoded_tag, identity.outer_class, identity.inner_class, configuration) {
        Ok(binding) => binding,
        Err(cause) => return Err(SelectionFailure::Admission { role, runtime, configuration: Some(configuration),
            cause: AdmissionError::Identity(cause), math }),
    };
    let selector = match SourceTransformSelector::bind(binding, runtime.owner_rcx,
        runtime.linked50, runtime.word_e8, runtime.word_f0)
    {
        Ok(selector) => selector,
        Err(cause) => return Err(SelectionFailure::Admission { role, runtime, configuration: Some(configuration),
            cause: AdmissionError::Selector(cause), math }),
    };
    let mut math = MathTrace::new(math);
    let actual = match selector.select(frame, &mut math) {
        Ok(actual) => actual,
        Err(cause) => return Err(SelectionFailure::Selection { role, runtime, binding, selector, math, cause }),
    };
    let signed_transform = actual.raw_eax() as i32;
    let selection = Selection { role, runtime, binding, actual, math };
    if signed_transform <= 0 { return Err(SelectionFailure::TransformWord { selection }); }
    Ok(selection)
}
