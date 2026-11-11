use crate::Result;
use std::convert::Infallible;

#[path = "generation_window_math.rs"]
mod generation_window_math;
pub use generation_window_math::*;
pub(crate) use generation_window_math::{consume_source_windows, InterruptedSourceWindows,
    SourceWindowConsumptionCause};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationSchedule {
    pub width: usize,
    pub stride: usize,
    pub requested_points: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedWindow {
    pub record_start: usize,
    pub record_end: usize,
    pub bank_start: usize,
    pub bank_end: usize,
}

impl GenerationSchedule {

    pub fn from_normalized_sample_count(samples: usize) -> Result<Self> {
        if samples == 0 || samples > i32::MAX as usize {
            return Err("generation requires a positive signed-i32 sample count".into());
        }
        Ok(Self {
            width: (samples / 720).max(1000),
            stride: (samples / 12000).max(100),
            requested_points: (2 * (samples / 48000) / 3).clamp(8, 16),
        })
    }

    pub fn accepted_windows(
        &self,
        energies: &[f32],
        bank_factor: usize,
    ) -> Result<Vec<AcceptedWindow>> {
        match select_windows(*self, energies, bank_factor, HostCount, (), no_consumer, no_consumer) {
            Ok((windows, _, ())) => Ok(windows),
            Err(failure) => match failure.cause {
                SelectionCause::Input(()) => Err(INPUT_ERROR.into()),
                SelectionCause::Extent(()) => Err("window iterator overflow".into()),
                SelectionCause::Start(()) => Err("window start overflow".into()),
                SelectionCause::End(()) => Err("window end overflow".into()),
                SelectionCause::Counter { error: never, .. } => match never {},
                SelectionCause::Prepare(stop) | SelectionCause::Accept(stop) => match stop.error {},
            },
        }
    }
}

const INPUT_ERROR: &str = "window selection requires finite energies and a positive model factor";

trait WindowCounter {
    type Error;
    fn values_admitted(&self, energies: &[f32]) -> bool;
    fn reserve(&mut self, windows: &mut Vec<AcceptedWindow>) -> std::result::Result<(), Self::Error>;
    fn count(&mut self, energies: &[f32], current: SourceGenerationWindow)
        -> std::result::Result<i64, Self::Error>;
}

struct HostCount;
impl WindowCounter for HostCount {
    type Error = Infallible;
    fn values_admitted(&self, energies: &[f32]) -> bool {
        !energies.iter().any(|v| !v.is_finite())
    }
    fn reserve(&mut self, _: &mut Vec<AcceptedWindow>) -> std::result::Result<(), Infallible> { Ok(()) }
    fn count(&mut self, energies: &[f32], current: SourceGenerationWindow)
        -> std::result::Result<i64, Infallible>
    {
        Ok(energies[current.window.record_start..current.window.record_end]
            .iter().filter(|&&v| f64::from(v) < 0.1_f64).count() as i64)
    }
}

pub(crate) struct ConsumerStop<K, E> { pub(crate) owner: K, pub(crate) error: E }
#[derive(Clone, Copy, Debug)]
pub(crate) enum ConsumerPhase { Prepare, Accept }
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceWindowVisit {
    pub(crate) current: SourceGenerationWindow,

    pub(crate) selected_ordinal: usize,
}
fn no_consumer(_: (), _: &[f32], _: SourceWindowVisit)
    -> std::result::Result<(), ConsumerStop<(), Infallible>> { Ok(()) }

enum SelectionCause<E, K, U> {
    Input(K), Extent(K), Start(K), End(K), Counter { error: E, owner: K },
    Prepare(ConsumerStop<K, U>), Accept(ConsumerStop<K, U>),
}
struct SelectionFailure<C: WindowCounter, K, U> {
    counter: C, windows: Vec<AcceptedWindow>, current: Option<SourceGenerationWindow>,
    selected_ordinal: Option<usize>, accepted_current: bool,
    cause: SelectionCause<C::Error, K, U>,
}

fn select_windows<C, K, U, P, A>(schedule: GenerationSchedule, energies: &[f32],
    bank_factor: usize, mut counter: C, mut owner: K, mut prepare: P, mut accept: A)
    -> std::result::Result<(Vec<AcceptedWindow>, C, K), SelectionFailure<C, K, U>>
where C: WindowCounter,
    P: FnMut(K, &[f32], SourceWindowVisit) -> std::result::Result<K, ConsumerStop<K, U>>,
    A: FnMut(K, &[f32], SourceWindowVisit) -> std::result::Result<K, ConsumerStop<K, U>>,
{
    let mut windows = Vec::new();
    let mut current = None;
    let mut selected_ordinal = None;
    let mut accepted_current = false;
    let result = (|| {
        if energies.is_empty() || energies.len() > i32::MAX as usize
            || !counter.values_admitted(energies) || bank_factor == 0
            || schedule.width == 0 || schedule.stride == 0
        { return Err(SelectionCause::Input(owner)); }
        let extent = match energies.len().checked_add(schedule.width - 1).filter(|v| *v <= i32::MAX as usize) {
            Some(extent) => extent, None => return Err(SelectionCause::Extent(owner)),
        };
        for ordinal in 0..extent / schedule.stride {
            current = None;
            selected_ordinal = None;
            accepted_current = false;
            let start = match ordinal.checked_mul(schedule.stride) {
                Some(start) => start, None => return Err(SelectionCause::Start(owner)),
            };
            let end = match start.checked_add(schedule.width) {
                Some(end) => end.min(energies.len()), None => return Err(SelectionCause::End(owner)),
            };

            let count = (end as i64) - (start as i64);
            current = Some(SourceGenerationWindow { ordinal, window: AcceptedWindow {
                record_start: start, record_end: end,
                bank_start: start / bank_factor, bank_end: end / bank_factor,
            }, count, low: None });
            selected_ordinal = Some(windows.len());
            if let Err(error) = counter.reserve(&mut windows) {
                return Err(SelectionCause::Counter { error, owner });
            }
            let visit = SourceWindowVisit { current: current.unwrap(), selected_ordinal: windows.len() };
            owner = match prepare(owner, energies, visit) {
                Ok(owner) => owner,
                Err(stop) => return Err(SelectionCause::Prepare(stop)),
            };
            let low = if start < end {
                match counter.count(energies, current.unwrap()) {
                    Ok(low) => low, Err(error) => return Err(SelectionCause::Counter { error, owner }),
                }
            } else { 0 };
            current.as_mut().unwrap().low = Some(low);
            if low > count / 2 && count - low < 1000 { continue; }
            windows.push(current.unwrap().window);
            accepted_current = true;
            let visit = SourceWindowVisit { current: current.unwrap(), selected_ordinal: windows.len() - 1 };
            owner = match accept(owner, energies, visit) {
                Ok(owner) => owner,
                Err(stop) => return Err(SelectionCause::Accept(stop)),
            };
        }
        Ok(owner)
    })();
    match result {
        Ok(owner) => Ok((windows, counter, owner)),
        Err(cause) => Err(SelectionFailure { counter, windows, current, selected_ordinal, accepted_current, cause }),
    }
}
