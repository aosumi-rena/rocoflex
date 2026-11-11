use std::collections::TryReserveError;
use crate::source_scalar::{self as scalar, ExplicitExceptionState, PowerError,
    SourceCvttMxcsrCapabilities, SourcePackedF32ToF64WithState, SourceScalarConversionError,
    SourceRawCmpltpdOutcome, SourceRawCmpltpdError, SourceRawCvtss2sdOutcome,
    SourceRawCvtss2sdError, SourceStrictUcomisdOutcome, SourceStrictUcomisdError,
    SourceRawUcomisdOutcome};
use scalar::shared_scalar_f32::ScalarF32Architecture;
use super::{AcceptedWindow, GenerationSchedule, WindowCounter, SelectionCause, SelectionFailure,
    select_windows, no_consumer, ConsumerStop, ConsumerPhase, SourceWindowVisit};

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationWindowConfiguration {
    pub packed_threshold_bits: [u64; 2],
    pub scalar_threshold_bits: u64,
    pub architecture: ScalarF32Architecture,
    pub capabilities: SourceCvttMxcsrCapabilities,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationWindowBindingCause { Architecture, Controls }
pub struct SourceGenerationWindowBindingFailure {
    configuration: SourceGenerationWindowConfiguration,
    entry: ExplicitExceptionState,
    cause: SourceGenerationWindowBindingCause,
}
impl SourceGenerationWindowBindingFailure {
    pub fn configuration(&self) -> SourceGenerationWindowConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGenerationWindowBindingCause { self.cause }
}

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationWindow {
    pub ordinal: usize,
    pub window: AcceptedWindow,
    pub count: i64,
    pub low: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationWindowStage {
    Window, GroupLoads, PackedWiden, PackedCompare, FirstHalfCommit,
    SecondHalfCommit, Collapse, TailLoad, TailWiden, TailCompare, TailCount, Complete,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationWindowOperands {
    PackedF32([u32; 2]), PackedF64 { left: [u64; 2], right: [u64; 2] },
    ScalarF32(u32), ScalarF64 { left: u64, right: u64 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationWindowReport {
    PackedWiden(Result<SourcePackedF32ToF64WithState, SourceScalarConversionError>),
    PackedCompare(Result<SourceRawCmpltpdOutcome, SourceRawCmpltpdError>),
    TailWiden(Result<SourceRawCvtss2sdOutcome, SourceRawCvtss2sdError>),
    TailCompare(Result<SourceStrictUcomisdOutcome, SourceStrictUcomisdError>),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationWindowAttempt {
    pub pc: u32,
    pub entry: ExplicitExceptionState,
    pub operands: SourceGenerationWindowOperands,
    pub reported: SourceGenerationWindowReport,
    pub raw_admission: Option<Result<ExplicitExceptionState, PowerError>>,
    pub known_advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationWindowGroup {
    pub window_ordinal: usize,
    pub start: usize,
    pub source_bits: [u32; 8],
    pub widened_bits: [Option<[u64; 2]>; 4],
    pub masks: [Option<[u64; 2]>; 4],

    pub attempts: [Option<SourceGenerationWindowAttempt>; 8],
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationWindowTail {
    pub window_ordinal: usize,
    pub index: usize,
    pub source_bits: u32,
    pub widened_bits: Option<u64>,
    pub attempts: [Option<SourceGenerationWindowAttempt>; 2],
}

pub struct SourceGenerationWindowProgress {
    current: SourceGenerationWindow,
    stage: SourceGenerationWindowStage,
    next_index: usize,
    accumulator_a: [u32; 4], accumulator_b: [u32; 4],
    collapsed: Option<u32>, low: Option<u32>,
    group: Option<SourceGenerationWindowGroup>, tail: Option<SourceGenerationWindowTail>,
    group_reached: bool, tail_reached: bool,
}
impl SourceGenerationWindowProgress {
    pub fn current(&self) -> SourceGenerationWindow { self.current }
    pub fn stage(&self) -> SourceGenerationWindowStage { self.stage }
    pub fn next_index(&self) -> usize { self.next_index }
    pub fn accumulator_a(&self) -> [u32; 4] { self.accumulator_a }
    pub fn accumulator_b(&self) -> [u32; 4] { self.accumulator_b }
    pub fn collapsed(&self) -> Option<u32> { self.collapsed }
    pub fn low(&self) -> Option<u32> { self.low }

    pub fn group(&self) -> Option<&SourceGenerationWindowGroup> { self.group.as_ref() }

    pub fn tail(&self) -> Option<&SourceGenerationWindowTail> { self.tail.as_ref() }

    pub fn group_reached(&self) -> bool { self.group_reached }

    pub fn tail_reached(&self) -> bool { self.tail_reached }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationWindowMathCause {
    PackedWiden, PackedCompareRequired, PackedCompareTrapped,
    TailWidenRequired, TailWidenTrapped, TailCompareRequired, TailCompareTrapped, RawStateAdmission,
}
pub struct SourceGenerationWindowMathFailure { cause: SourceGenerationWindowMathCause, pc: u32 }
impl SourceGenerationWindowMathFailure {
    pub fn cause(&self) -> SourceGenerationWindowMathCause { self.cause }
    pub fn pc(&self) -> u32 { self.pc }
}

pub struct SourceGenerationWindowMath {
    configuration: SourceGenerationWindowConfiguration,
    state: ExplicitExceptionState,
    progress: Option<SourceGenerationWindowProgress>,
    failure: Option<SourceGenerationWindowMathFailure>,
}
impl SourceGenerationWindowMath {
    pub fn from_explicit_parts(configuration: SourceGenerationWindowConfiguration,
        entry: ExplicitExceptionState) -> Result<Self, SourceGenerationWindowBindingFailure>
    {
        let cause = if configuration.architecture != ScalarF32Architecture::IntelSdm {
            Some(SourceGenerationWindowBindingCause::Architecture)
        } else if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            Some(SourceGenerationWindowBindingCause::Controls)
        } else { None };
        if let Some(cause) = cause {
            return Err(SourceGenerationWindowBindingFailure { configuration, entry, cause });
        }

        Ok(Self { configuration, state: entry, progress: None, failure: None })
    }
    pub fn configuration(&self) -> SourceGenerationWindowConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn progress(&self) -> Option<&SourceGenerationWindowProgress> { self.progress.as_ref() }
    pub fn failure(&self) -> Option<&SourceGenerationWindowMathFailure> { self.failure.as_ref() }

    fn stop(&mut self, cause: SourceGenerationWindowMathCause, pc: u32) -> Stopped {
        if self.failure.is_none() { self.failure = Some(SourceGenerationWindowMathFailure { cause, pc }); }
        Stopped
    }
    fn begin(&mut self, current: SourceGenerationWindow) {
        let (group, tail) = match self.progress.as_ref() {
            Some(prior) => (prior.group, prior.tail), None => (None, None),
        };
        self.progress = Some(SourceGenerationWindowProgress { current,
            stage: SourceGenerationWindowStage::Window, next_index: current.window.record_start,
            accumulator_a: [0; 4], accumulator_b: [0; 4], collapsed: None, low: None, group, tail,
            group_reached: false, tail_reached: false });
    }
    fn attempt_mut(&mut self, slot: usize, tail: bool) -> &mut SourceGenerationWindowAttempt {
        let progress = self.progress.as_mut().unwrap();
        if tail { progress.tail.as_mut().unwrap().attempts[slot].as_mut().unwrap() }
        else { progress.group.as_mut().unwrap().attempts[slot].as_mut().unwrap() }
    }
    fn save(&mut self, slot: usize, tail: bool, pc: u32, operands: SourceGenerationWindowOperands,
        reported: SourceGenerationWindowReport)
    {
        let attempt = SourceGenerationWindowAttempt { pc, entry: self.state, operands, reported,
            raw_admission: None, known_advanced: self.state };
        let progress = self.progress.as_mut().unwrap();
        if tail { progress.tail.as_mut().unwrap().attempts[slot] = Some(attempt); }
        else { progress.group.as_mut().unwrap().attempts[slot] = Some(attempt); }
    }
    fn adopt_raw(&mut self, word: u32, slot: usize, tail: bool, pc: u32) -> Result<(), Stopped> {
        let admission = ExplicitExceptionState::from_explicit_parts(word, self.state.errno());
        self.attempt_mut(slot, tail).raw_admission = Some(admission);
        match admission {
            Ok(advanced) => {
                self.state = advanced;
                self.attempt_mut(slot, tail).known_advanced = advanced;
                Ok(())
            }
            Err(_) => Err(self.stop(SourceGenerationWindowMathCause::RawStateAdmission, pc)),
        }
    }
    fn packed_widen(&mut self, pair: usize, source: [u32; 2], pc: u32) -> Result<[u64; 2], Stopped> {
        self.progress.as_mut().unwrap().stage = SourceGenerationWindowStage::PackedWiden;
        let reported = scalar::cvtps2pd_with_explicit_state(source, self.state, self.configuration.architecture);
        self.save(pair, false, pc, SourceGenerationWindowOperands::PackedF32(source),
            SourceGenerationWindowReport::PackedWiden(reported));
        match reported {
            Ok(value) => {
                self.state = value.exception_state;
                self.attempt_mut(pair, false).known_advanced = self.state;
                Ok(value.bits)
            }
            Err(error) => {
                self.state = error.known_advanced_state(self.state);
                self.attempt_mut(pair, false).known_advanced = self.state;
                Err(self.stop(SourceGenerationWindowMathCause::PackedWiden, pc))
            }
        }
    }
    fn packed_compare(&mut self, slot: usize, left: [u64; 2], pc: u32) -> Result<[u64; 2], Stopped> {
        self.progress.as_mut().unwrap().stage = SourceGenerationWindowStage::PackedCompare;
        let right = self.configuration.packed_threshold_bits;
        let reported = scalar::cmpltpd_with_raw_mxcsr(left, right, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, false, pc, SourceGenerationWindowOperands::PackedF64 { left, right },
            SourceGenerationWindowReport::PackedCompare(reported));
        let word = match reported {
            Ok(SourceRawCmpltpdOutcome::Completed { state, .. })
            | Ok(SourceRawCmpltpdOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, false, pc)?;
        match reported {
            Ok(SourceRawCmpltpdOutcome::Completed { masks, .. }) => Ok(masks),
            Ok(SourceRawCmpltpdOutcome::Trapped { .. }) =>
                Err(self.stop(SourceGenerationWindowMathCause::PackedCompareTrapped, pc)),
            Err(_) => Err(self.stop(SourceGenerationWindowMathCause::PackedCompareRequired, pc)),
        }
    }
    fn tail_widen(&mut self, source: u32) -> Result<u64, Stopped> {
        let pc = 0x2c97a5;
        self.progress.as_mut().unwrap().stage = SourceGenerationWindowStage::TailWiden;
        let reported = scalar::cvtss2sd_with_raw_mxcsr(source, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(0, true, pc, SourceGenerationWindowOperands::ScalarF32(source),
            SourceGenerationWindowReport::TailWiden(reported));
        let word = match reported {
            Ok(SourceRawCvtss2sdOutcome::Completed { state, .. })
            | Ok(SourceRawCvtss2sdOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, 0, true, pc)?;
        match reported {
            Ok(SourceRawCvtss2sdOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawCvtss2sdOutcome::Trapped { .. }) =>
                Err(self.stop(SourceGenerationWindowMathCause::TailWidenTrapped, pc)),
            Err(_) => Err(self.stop(SourceGenerationWindowMathCause::TailWidenRequired, pc)),
        }
    }
    fn tail_compare(&mut self, right: u64) -> Result<bool, Stopped> {
        let pc = 0x2c97ac;
        self.progress.as_mut().unwrap().stage = SourceGenerationWindowStage::TailCompare;
        let left = self.configuration.scalar_threshold_bits;
        let reported = scalar::ucomisd_f64_with_explicit_state(left, right, self.state,
            self.configuration.architecture);
        self.save(1, true, pc, SourceGenerationWindowOperands::ScalarF64 { left, right },
            SourceGenerationWindowReport::TailCompare(reported));
        match reported {
            Ok(value) => {
                self.state = value.exception_state;
                self.attempt_mut(1, true).known_advanced = self.state;
                match value.reported {
                    SourceRawUcomisdOutcome::Completed { flags, .. } => Ok(!flags.carry && !flags.zero),
                    SourceRawUcomisdOutcome::Trapped { .. } =>
                        Err(self.stop(SourceGenerationWindowMathCause::TailCompareTrapped, pc)),
                }
            }
            Err(error) => {
                self.state = error.known_advanced_state(self.state);
                self.attempt_mut(1, true).known_advanced = self.state;
                Err(self.stop(SourceGenerationWindowMathCause::TailCompareRequired, pc))
            }
        }
    }
    fn count_window(&mut self, energies: &[f32], current: SourceGenerationWindow) -> Result<i64, Stopped> {
        if self.failure.is_some() { return Err(Stopped); }
        self.begin(current);
        let mut index = current.window.record_start;
        let end = current.window.record_end;

        while end - index > 8 {
            let mut source_bits = [0; 8];
            self.progress.as_mut().unwrap().stage = SourceGenerationWindowStage::GroupLoads;
            for lane in [0, 1, 2, 3, 7, 6, 5, 4] { source_bits[lane] = energies[index + lane].to_bits(); }
            self.progress.as_mut().unwrap().group = Some(SourceGenerationWindowGroup {
                window_ordinal: current.ordinal, start: index, source_bits,
                widened_bits: [None; 4], masks: [None; 4], attempts: [None; 8] });
            self.progress.as_mut().unwrap().group_reached = true;
            let mut widened = [[0; 2]; 4];
            for (pair, pc) in [0x2c9723, 0x2c9726, 0x2c9729, 0x2c972d].iter().copied().enumerate() {
                let bits = self.packed_widen(pair, [source_bits[2 * pair], source_bits[2 * pair + 1]], pc)?;
                widened[pair] = bits;
                self.progress.as_mut().unwrap().group.as_mut().unwrap().widened_bits[pair] = Some(bits);
            }
            let masks23 = self.packed_compare(4, widened[1], 0x2c9730)?;
            self.progress.as_mut().unwrap().group.as_mut().unwrap().masks[1] = Some(masks23);
            let masks01 = self.packed_compare(5, widened[0], 0x2c9736)?;
            self.progress.as_mut().unwrap().group.as_mut().unwrap().masks[0] = Some(masks01);
            let progress = self.progress.as_mut().unwrap();
            progress.stage = SourceGenerationWindowStage::FirstHalfCommit;
            for (lane, mask) in [masks01[0], masks01[1], masks23[0], masks23[1]].iter().copied().enumerate() {
                progress.accumulator_a[lane] = progress.accumulator_a[lane].wrapping_sub(mask as u32);
            }
            let masks45 = self.packed_compare(6, widened[2], 0x2c9744)?;
            self.progress.as_mut().unwrap().group.as_mut().unwrap().masks[2] = Some(masks45);
            let masks67 = self.packed_compare(7, widened[3], 0x2c974a)?;
            self.progress.as_mut().unwrap().group.as_mut().unwrap().masks[3] = Some(masks67);
            let progress = self.progress.as_mut().unwrap();
            progress.stage = SourceGenerationWindowStage::SecondHalfCommit;
            for (lane, mask) in [masks45[0], masks45[1], masks67[0], masks67[1]].iter().copied().enumerate() {
                progress.accumulator_b[lane] = progress.accumulator_b[lane].wrapping_sub(mask as u32);
            }
            index += 8;
            progress.next_index = index;
        }
        let progress = self.progress.as_mut().unwrap();
        progress.stage = SourceGenerationWindowStage::Collapse;
        let mut sum = [0u32; 4];
        for lane in 0..4 { sum[lane] = progress.accumulator_b[lane].wrapping_add(progress.accumulator_a[lane]); }
        let t0 = sum[2].wrapping_add(sum[0]);
        let t1 = sum[3].wrapping_add(sum[1]);
        let mut low = t1.wrapping_add(t0);
        progress.collapsed = Some(low);
        progress.low = Some(low);
        while index < end {
            let progress = self.progress.as_mut().unwrap();
            progress.stage = SourceGenerationWindowStage::TailLoad;
            let source_bits = energies[index].to_bits();
            progress.tail = Some(SourceGenerationWindowTail { window_ordinal: current.ordinal,
                index, source_bits, widened_bits: None, attempts: [None; 2] });
            progress.tail_reached = true;
            let widened = self.tail_widen(source_bits)?;
            self.progress.as_mut().unwrap().tail.as_mut().unwrap().widened_bits = Some(widened);
            let below = self.tail_compare(widened)?;
            let progress = self.progress.as_mut().unwrap();
            progress.stage = SourceGenerationWindowStage::TailCount;
            low = low.wrapping_add(u32::from(below));
            progress.low = Some(low);
            index += 1;
            progress.next_index = index;
        }
        let progress = self.progress.as_mut().unwrap();
        progress.stage = SourceGenerationWindowStage::Complete;
        progress.current.low = Some(i64::from(low));
        Ok(i64::from(low))
    }
}

struct Stopped;
struct SourceCount { math: SourceGenerationWindowMath }
enum CounterError { Storage(TryReserveError), Math }
impl WindowCounter for SourceCount {
    type Error = CounterError;
    fn values_admitted(&self, _: &[f32]) -> bool { true }
    fn reserve(&mut self, windows: &mut Vec<AcceptedWindow>) -> Result<(), CounterError> {
        windows.try_reserve(1).map_err(CounterError::Storage)
    }
    fn count(&mut self, energies: &[f32], current: SourceGenerationWindow) -> Result<i64, CounterError> {
        self.math.count_window(energies, current).map_err(|_| CounterError::Math)
    }
}

pub struct SourceGenerationWindowsParts {
    pub energies: Vec<f32>, pub math: SourceGenerationWindowMath,
    pub windows: Vec<AcceptedWindow>, pub schedule: GenerationSchedule, pub bank_factor: usize,
}
pub struct SourceGenerationWindowsCompleted { parts: SourceGenerationWindowsParts }
impl SourceGenerationWindowsCompleted {
    pub fn energies(&self) -> &[f32] { &self.parts.energies }
    pub fn math(&self) -> &SourceGenerationWindowMath { &self.parts.math }
    pub fn windows(&self) -> &[AcceptedWindow] { &self.parts.windows }
    pub fn schedule(&self) -> GenerationSchedule { self.parts.schedule }
    pub fn bank_factor(&self) -> usize { self.parts.bank_factor }
    pub fn into_parts(self) -> SourceGenerationWindowsParts { self.parts }
}
pub enum SourceGenerationWindowsCause { Input, Extent, Start, End, Storage(TryReserveError), Math }
pub struct SourceGenerationWindowsFailure {
    parts: SourceGenerationWindowsParts, current: Option<SourceGenerationWindow>,
    cause: SourceGenerationWindowsCause,
}
impl SourceGenerationWindowsFailure {
    pub fn energies(&self) -> &[f32] { &self.parts.energies }
    pub fn math(&self) -> &SourceGenerationWindowMath { &self.parts.math }
    pub fn windows(&self) -> &[AcceptedWindow] { &self.parts.windows }
    pub fn schedule(&self) -> GenerationSchedule { self.parts.schedule }
    pub fn bank_factor(&self) -> usize { self.parts.bank_factor }
    pub fn current(&self) -> Option<SourceGenerationWindow> { self.current }
    pub fn cause(&self) -> &SourceGenerationWindowsCause { &self.cause }
}

impl GenerationSchedule {

    pub fn accepted_windows_with_source_math(&self, energies: Vec<f32>, bank_factor: usize,
        math: SourceGenerationWindowMath)
        -> Result<SourceGenerationWindowsCompleted, SourceGenerationWindowsFailure>
    {
        match select_windows(*self, &energies, bank_factor, SourceCount { math }, (), no_consumer, no_consumer) {
            Ok((windows, counter, ())) => Ok(SourceGenerationWindowsCompleted { parts: SourceGenerationWindowsParts {
                energies, math: counter.math, windows, schedule: *self, bank_factor } }),
            Err(failure) => {
                let cause = match failure.cause {
                    SelectionCause::Input(()) => SourceGenerationWindowsCause::Input,
                    SelectionCause::Extent(()) => SourceGenerationWindowsCause::Extent,
                    SelectionCause::Start(()) => SourceGenerationWindowsCause::Start,
                    SelectionCause::End(()) => SourceGenerationWindowsCause::End,
                    SelectionCause::Counter { error: CounterError::Storage(error), .. } => SourceGenerationWindowsCause::Storage(error),
                    SelectionCause::Counter { error: CounterError::Math, .. } => SourceGenerationWindowsCause::Math,
                    SelectionCause::Prepare(stop) | SelectionCause::Accept(stop) => match stop.error {},
                };
                Err(SourceGenerationWindowsFailure { parts: SourceGenerationWindowsParts {
                    energies, math: failure.counter.math, windows: failure.windows,
                    schedule: *self, bank_factor }, current: failure.current, cause })
            }
        }
    }
}

pub(crate) struct ConsumedSourceWindows<K> {
    pub(crate) selection: SourceGenerationWindowsParts,
    pub(crate) owner: K,
}
pub(crate) enum SourceWindowConsumptionCause<'a, E> {
    Input, Extent, Start, End, WindowStorage(&'a TryReserveError), Math,
    Consumer { phase: ConsumerPhase, error: &'a E },
}

pub(crate) struct InterruptedSourceWindows<K, E> {
    energies: Vec<f32>, schedule: GenerationSchedule, bank_factor: usize,
    core: SelectionFailure<SourceCount, K, E>,
}
impl<K, E> InterruptedSourceWindows<K, E> {
    pub(crate) fn energies(&self) -> &[f32] { &self.energies }
    pub(crate) fn math(&self) -> &SourceGenerationWindowMath { &self.core.counter.math }
    pub(crate) fn windows(&self) -> &[AcceptedWindow] { &self.core.windows }
    pub(crate) fn schedule(&self) -> GenerationSchedule { self.schedule }
    pub(crate) fn bank_factor(&self) -> usize { self.bank_factor }
    pub(crate) fn current(&self) -> Option<SourceGenerationWindow> { self.core.current }
    pub(crate) fn selected_ordinal(&self) -> Option<usize> { self.core.selected_ordinal }
    pub(crate) fn accepted_current(&self) -> bool { self.core.accepted_current }
    pub(crate) fn owner(&self) -> &K {
        match &self.core.cause {
            SelectionCause::Input(owner) | SelectionCause::Extent(owner)
            | SelectionCause::Start(owner) | SelectionCause::End(owner)
            | SelectionCause::Counter { owner, .. } => owner,
            SelectionCause::Prepare(stop) | SelectionCause::Accept(stop) => &stop.owner,
        }
    }
    pub(crate) fn cause(&self) -> SourceWindowConsumptionCause<'_, E> {
        match &self.core.cause {
            SelectionCause::Input(_) => SourceWindowConsumptionCause::Input,
            SelectionCause::Extent(_) => SourceWindowConsumptionCause::Extent,
            SelectionCause::Start(_) => SourceWindowConsumptionCause::Start,
            SelectionCause::End(_) => SourceWindowConsumptionCause::End,
            SelectionCause::Counter { error: CounterError::Storage(error), .. } => SourceWindowConsumptionCause::WindowStorage(error),
            SelectionCause::Counter { error: CounterError::Math, .. } => SourceWindowConsumptionCause::Math,
            SelectionCause::Prepare(stop) => SourceWindowConsumptionCause::Consumer { phase: ConsumerPhase::Prepare, error: &stop.error },
            SelectionCause::Accept(stop) => SourceWindowConsumptionCause::Consumer { phase: ConsumerPhase::Accept, error: &stop.error },
        }
    }
}

pub(crate) fn consume_source_windows<K, E, P, A>(schedule: GenerationSchedule, energies: Vec<f32>,
    bank_factor: usize, math: SourceGenerationWindowMath, owner: K, prepare: P, accept: A)
    -> Result<ConsumedSourceWindows<K>, InterruptedSourceWindows<K, E>>
where P: FnMut(K, &[f32], SourceWindowVisit) -> Result<K, ConsumerStop<K, E>>,
    A: FnMut(K, &[f32], SourceWindowVisit) -> Result<K, ConsumerStop<K, E>>,
{
    match select_windows(schedule, &energies, bank_factor, SourceCount { math }, owner, prepare, accept) {
        Ok((windows, counter, owner)) => Ok(ConsumedSourceWindows {
            selection: SourceGenerationWindowsParts { energies, math: counter.math, windows, schedule, bank_factor }, owner }),
        Err(core) => Err(InterruptedSourceWindows { energies, schedule, bank_factor, core }),
    }
}
