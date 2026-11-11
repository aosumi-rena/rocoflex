use crate::Result;
use crate::frontend_gain_count_math as count_math;
use crate::{frontend_gain_log_rows as log_rows, frontend_gain_statistics as statistics};
use std::convert::Infallible;

#[path = "frontend_gain_application.rs"]
mod application;
pub(crate) use application::apply_gain_windows_in_place_with_source_bits;
pub use application::{SourceGainPowerAssociation, SourceGainPowerServices, SourceGainPowerAttempt,
    SourceGainApplicationConfiguration, SourceGainApplicationBindingCause, SourceGainApplicationBindingFailure,
    SourceGainApplicationAttempt, SourceGainApplicationMathCause, SourceGainApplicationMathFailure, SourceGainApplicationMath,
    SourceGainApplicationRequest, SourceGainApplicationInterpolation, SourceGainApplicationStage, SourceGainApplicationCurrent,
    SourceGainApplicationRecord, SourceGainApplicationContext, SourceGainApplicationParts, SourceGainApplicationCompleted,
    SourceGainApplicationCause, SourceGainApplicationFailure, apply_gain_windows_with_source_services,
    SourceGainInPlaceApplicationRequest, SourceGainInPlaceApplicationParts, SourceGainInPlaceApplicationCompleted,
    SourceGainInPlaceApplicationFailure, apply_gain_windows_in_place_with_source_services};

#[path = "frontend_peak_math.rs"]
mod peak_math;
pub(crate) use peak_math::peak_normalize09_in_place_with_source_bits;
pub use peak_math::{SourcePeakConfiguration, SourcePeakBindingCause, SourcePeakBindingFailure,
    SourcePeakStage, SourcePeakAttempt, SourcePeakMathCause, SourcePeakMathFailure, SourcePeakMath,
    SourcePeakProgress, SourcePeakFailureCause, SourcePeakFailure, peak_normalize09_with_source_math,
    SourcePeakInPlaceProgress, SourcePeakInPlaceFailure, peak_normalize09_in_place_with_source_math};

#[path = "frontend_filter_math.rs"]
mod filter_math;
pub use filter_math::{SourceFilterConfiguration, SourceFilterBindingCause, SourceFilterBindingFailure,
    SourceFilterRole, SourceFilterOperands, SourceFilterAttempt, SourceFilterMathCause, SourceFilterMathFailure,
    SourceFilterMath, SourceFilterOutputs, SourceFilterProgress, SourceFilterGainCause, SourceFilterGainFailure,
    frontend_gain_table_with_source_filters};

#[path = "frontend_slew_math.rs"]
mod slew_math;
pub use slew_math::{SourceSlewConfiguration, SourceSlewBindingCause, SourceSlewBindingFailure,
    SourceSlewOperation, SourceSlewDirection, SourceSlewAttempt, SourceSlewMathCause, SourceSlewMathFailure,
    SourceSlewMath, SourceSlewProgress, SourceSlewCause, SourceSlewFailure, SourceSlewGainFailure,
    gain_slew_from_log_with_source_math, frontend_gain_table_with_source_slew,
    SourceSlewFinalGainError, SourceSlewFinalGainFailure, gain_slew_from_log_with_source_final_gain};

#[path = "frontend_final_gain_math.rs"]
mod final_gain_math;
pub use final_gain_math::{SourceFinalGainConfiguration, SourceFinalGainBindingCause, SourceFinalGainBindingFailure,
    SourceFinalGainOperation, SourceFinalGainAttempt, SourceFinalGainMathCause, SourceFinalGainMathFailure,
    SourceFinalGainMath, SourceFinalGainStage, SourceFinalGainProgress, SourceFinalGainCause, SourceFinalGainFailure};

#[path = "frontend_energy_reduction_math.rs"]
mod energy_reduction_math;
pub use energy_reduction_math::{SourceEnergyConfiguration, SourceEnergyBindingCause, SourceEnergyBindingFailure,
    SourceEnergyOperation, SourceEnergyStage, SourceEnergyAttempt, SourceEnergyMathCause, SourceEnergyMathFailure,
    SourceEnergyMath, SourceEnergyProgress, SourceEnergyFailureCause, SourceEnergyFailure, SourceEnergyCompleted};
#[path = "frontend_energy_windows.rs"]
mod energy_windows;
pub use energy_windows::{SourceEnergyWindow, SourceEnergyWindows, SourceEnergyRow, SourceEnergyRowsCompleted,
    SourceEnergyStorageSlot, SourceEnergyRowsCause, SourceEnergyRowsFailure, SourceWindowReceipt,
    SourceWindowEnergyRow, SourceWindowEnergyRowsCompleted, SourceWindowEnergyRowsError, SourceWindowEnergyRowsFailure,
    SourceWindowStorage, SourceEnergyArrayInitializationStage, SourceEnergyArrayInitializationCause,
    SourceEnergyArrayInitializationFailure, SourceInitializedWindowEnergyRowsError, SourceInitializedWindowEnergyRowsFailure};

pub struct FrontendGainStages {
    pub first_filter: Vec<f32>,
    pub second_filter: Vec<f32>,
    pub energy: Vec<f32>,
    pub metric_log: Vec<f32>,
    pub slew_rounds: Vec<Vec<f32>>,
    pub gain_db: Vec<f32>,
}

pub fn gain_log_from_energy(energy: &[f32]) -> Result<Vec<f32>> {
    if energy.is_empty() || energy.iter().any(|&x| !x.is_finite() || x < 0.0) {
        return Err("gain energy requires nonnegative finite values".into());
    }
    Ok(energy
        .iter()
        .map(|&x| {
            let logf = ((x as f64).log10()) as f32;
            ((logf as f64) * 10.0_f64 + -0.6909999999999999_f64) as f32
        })
        .collect())
}

pub fn gain_slew_from_log(log: &[f32], target_db: f32) -> Result<(Vec<Vec<f32>>, Vec<f32>)> {
    validate_slew_input(log, target_db)?;
    let (rounds, envelope) = slew_math::host_slew(log);
    Ok((rounds, host_gain_from_envelope(&envelope, target_db)))
}
fn validate_slew_input(log: &[f32], target_db: f32) -> Result<()> {
    if log.is_empty()
        || !target_db.is_finite()
        || log.iter().any(|&x| x.is_nan() || x == f32::INFINITY)
    {
        return Err("invalid gain envelope/target".into());
    }
    Ok(())
}
fn host_gain_from_envelope(envelope: &[f32], target_db: f32) -> Vec<f32> {
    final_gain_math::host_final_gain(envelope, target_db)
}

pub fn frontend_gain_table(src: &[f32], target_db: f32) -> Result<FrontendGainStages> {
    validate_gain_input(src)?;
    let (first_filter, second_filter) = host_gain_filters(src);
    gain_table_suffix(src.len(), target_db, first_filter, second_filter).map_err(|failure| failure.error)
}

enum FilterSlewGainOwner {
    Filter { failure: SourceFilterGainFailure, slew_math: SourceSlewMath },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        failure: GainSuffixChoiceFailure<SourceSlewMath, SourceSlewFailure> },
}
pub enum SourceFilterSlewGainError<'a> {
    Filter(&'a SourceFilterGainFailure), BeforeSlew(&'a str), Slew(&'a SourceSlewFailure),
}
pub struct SourceFilterSlewGainFailure { owner: FilterSlewGainOwner }
impl SourceFilterSlewGainFailure {
    pub fn error(&self) -> SourceFilterSlewGainError<'_> {
        match &self.owner {
            FilterSlewGainOwner::Filter { failure, .. } => SourceFilterSlewGainError::Filter(failure),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } =>
                SourceFilterSlewGainError::BeforeSlew(&failure.error),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } =>
                SourceFilterSlewGainError::Slew(failure),
        }
    }
    pub fn input(&self) -> &[f32] {
        match &self.owner { FilterSlewGainOwner::Filter { failure, .. } => failure.input(),
            FilterSlewGainOwner::Suffix { input, .. } => input }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner { FilterSlewGainOwner::Filter { failure, .. } => failure.target_db(),
            FilterSlewGainOwner::Suffix { target_db, .. } => *target_db }
    }
    pub fn filter_math(&self) -> &SourceFilterMath {
        match &self.owner { FilterSlewGainOwner::Filter { failure, .. } => failure.math(),
            FilterSlewGainOwner::Suffix { filter_math, .. } => filter_math }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner {
            FilterSlewGainOwner::Filter { slew_math, .. } => slew_math,
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => owner,
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.math(),
        }
    }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> {
        match &self.owner { FilterSlewGainOwner::Filter { failure, .. } => failure.progress(),
            FilterSlewGainOwner::Suffix { filter_progress, .. } => Some(filter_progress) }
    }
    pub fn first_filter(&self) -> &[f32] {
        match &self.owner {
            FilterSlewGainOwner::Filter { failure, .. } => failure.first_filter(),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.first_filter,
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { first_filter, .. }, .. } => first_filter,
        }
    }
    pub fn second_filter(&self) -> &[f32] {
        match &self.owner {
            FilterSlewGainOwner::Filter { failure, .. } => failure.second_filter(),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.second_filter,
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { second_filter, .. }, .. } => second_filter,
        }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterSlewGainOwner::Filter { failure, .. } => failure.energy(),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => Some(&failure.energy),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { energy, .. }, .. } => Some(energy),
        }
    }
    pub fn metric_log(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterSlewGainOwner::Filter { failure, .. } => failure.metric_log(),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => failure.metric_log.as_deref(),
            FilterSlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        }
    }
}

pub fn frontend_gain_table_with_source_filters_and_slew(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, slew_math: SourceSlewMath)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, SourceSlewMath, Vec<f32>), SourceFilterSlewGainFailure>
{
    let prepared = match filter_math::prepare_source_filters(input, target_db, filter_math, outputs) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterSlewGainFailure { owner: FilterSlewGainOwner::Filter { failure, slew_math } }),
    };
    let filter_math::PreparedSourceFilters { input, target_db, math: filter_math, outputs, progress: filter_progress } = prepared;
    match gain_table_suffix_with(input.len(), target_db, outputs.first, outputs.second, slew_math, |metric_log, target, owner| {
        gain_slew_from_log_with_source_math(metric_log, target, owner)
            .map(|((rounds, gain_db), owner, metric_log)| SlewCompleted { rounds, gain_db, owner, metric_log })
    }) {
        Ok((stages, slew_math)) => Ok((stages, filter_math, slew_math, input)),
        Err(failure) => Err(SourceFilterSlewGainFailure { owner: FilterSlewGainOwner::Suffix {
            input, target_db, filter_math, filter_progress, failure } }),
    }
}

enum FilterEnergySlewGainOwner {
    Filter { failure: SourceFilterGainFailure, energy_math: SourceEnergyMath, windows: SourceEnergyWindows, slew_math: SourceSlewMath },
    Energy { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
        filter_progress: SourceFilterProgress, failure: SourceEnergyRowsFailure, slew_math: SourceSlewMath },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        rows: SourceEnergyRowsCompleted, failure: GainSuffixChoiceFailure<SourceSlewMath, SourceSlewFailure> },
}
pub enum SourceFilterEnergySlewGainError<'a> {
    Filter(&'a SourceFilterGainFailure), Energy(&'a SourceEnergyRowsFailure), BeforeSlew(&'a str), Slew(&'a SourceSlewFailure),
}
pub struct SourceFilterEnergySlewGainFailure { owner: FilterEnergySlewGainOwner }
impl SourceFilterEnergySlewGainFailure {
    pub fn error(&self) -> SourceFilterEnergySlewGainError<'_> {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => SourceFilterEnergySlewGainError::Filter(failure),
            FilterEnergySlewGainOwner::Energy { failure, .. } => SourceFilterEnergySlewGainError::Energy(failure),
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } =>
                SourceFilterEnergySlewGainError::BeforeSlew(&failure.error),
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } =>
                SourceFilterEnergySlewGainError::Slew(failure),
        }
    }
    pub fn input(&self) -> &[f32] {
        match &self.owner { FilterEnergySlewGainOwner::Filter { failure, .. } => failure.input(),
            FilterEnergySlewGainOwner::Energy { input, .. } | FilterEnergySlewGainOwner::Suffix { input, .. } => input }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner { FilterEnergySlewGainOwner::Filter { failure, .. } => failure.target_db(),
            FilterEnergySlewGainOwner::Energy { target_db, .. } | FilterEnergySlewGainOwner::Suffix { target_db, .. } => *target_db }
    }
    pub fn filter_math(&self) -> &SourceFilterMath {
        match &self.owner { FilterEnergySlewGainOwner::Filter { failure, .. } => failure.math(),
            FilterEnergySlewGainOwner::Energy { filter_math, .. } | FilterEnergySlewGainOwner::Suffix { filter_math, .. } => filter_math }
    }
    pub fn energy_math(&self) -> &SourceEnergyMath {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { energy_math, .. } => energy_math,
            FilterEnergySlewGainOwner::Energy { failure, .. } => failure.math(),
            FilterEnergySlewGainOwner::Suffix { rows, .. } => rows.math(),
        }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { slew_math, .. } | FilterEnergySlewGainOwner::Energy { slew_math, .. } => slew_math,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => owner,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.math(),
        }
    }
    pub fn unused_energy_windows(&self) -> Option<&SourceEnergyWindows> {
        match &self.owner { FilterEnergySlewGainOwner::Filter { windows, .. } => Some(windows), _ => None }
    }
    pub fn energy_rows(&self) -> Option<&SourceEnergyRowsCompleted> {
        match &self.owner { FilterEnergySlewGainOwner::Suffix { rows, .. } => Some(rows), _ => None }
    }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => failure.progress(),
            FilterEnergySlewGainOwner::Energy { filter_progress, .. } | FilterEnergySlewGainOwner::Suffix { filter_progress, .. } => Some(filter_progress),
        }
    }
    pub fn first_filter(&self) -> &[f32] {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => failure.first_filter(),
            FilterEnergySlewGainOwner::Energy { outputs, .. } => &outputs.first,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.first_filter,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { first_filter, .. }, .. } => first_filter,
        }
    }
    pub fn second_filter(&self) -> &[f32] {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => failure.second_filter(),
            FilterEnergySlewGainOwner::Energy { outputs, .. } => &outputs.second,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.second_filter,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { second_filter, .. }, .. } => second_filter,
        }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => failure.energy(),
            FilterEnergySlewGainOwner::Energy { failure, .. } => Some(failure.energy()),
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => Some(&failure.energy),
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { energy, .. }, .. } => Some(energy),
        }
    }
    pub fn metric_log(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterEnergySlewGainOwner::Filter { failure, .. } => failure.metric_log(),
            FilterEnergySlewGainOwner::Energy { .. } => None,
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => failure.metric_log.as_deref(),
            FilterEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        }
    }
}

pub fn frontend_gain_table_with_source_filters_energy_and_slew(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, energy_math: SourceEnergyMath,
    windows: SourceEnergyWindows, slew_math: SourceSlewMath)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, SourceEnergyRowsCompleted, SourceSlewMath, Vec<f32>), SourceFilterEnergySlewGainFailure>
{
    let prepared = match filter_math::prepare_source_filters(input, target_db, filter_math, outputs) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(SourceFilterEnergySlewGainFailure { owner: FilterEnergySlewGainOwner::Filter {
            failure, energy_math, windows, slew_math } }),
    };
    let filter_math::PreparedSourceFilters { input, target_db, math: filter_math, outputs, progress: filter_progress } = prepared;
    let expected = gain_energy_count(input.len());
    let (energy, rows) = match energy_windows::reduce_windows(windows, expected, energy_math) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourceFilterEnergySlewGainFailure { owner: FilterEnergySlewGainOwner::Energy {
            input, target_db, filter_math, outputs, filter_progress, failure, slew_math } }),
    };
    match gain_table_after_energy(target_db, outputs.first, outputs.second, energy, slew_math, |metric_log, target, owner| {
        gain_slew_from_log_with_source_math(metric_log, target, owner)
            .map(|((rounds, gain_db), owner, metric_log)| SlewCompleted { rounds, gain_db, owner, metric_log })
    }) {
        Ok((stages, slew_math)) => Ok((stages, filter_math, rows, slew_math, input)),
        Err(failure) => Err(SourceFilterEnergySlewGainFailure { owner: FilterEnergySlewGainOwner::Suffix {
            input, target_db, filter_math, filter_progress, rows, failure } }),
    }
}
enum FilterWindowEnergySlewGainOwner<P = SourceSlewMath, E = SourceSlewFailure> {
    Filter { failure: SourceFilterGainFailure, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: SourceEnergyWindows, slew_math: P },
    Windows { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, first_filter: Vec<f32>,
        filter_progress: SourceFilterProgress, failure: SourceWindowEnergyRowsFailure, slew_math: P },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        rows: SourceWindowEnergyRowsCompleted, failure: GainSuffixChoiceFailure<P, E> },
}
pub enum SourceFilterWindowEnergySlewGainError<'a> {
    Filter(&'a SourceFilterGainFailure), Windows(&'a SourceWindowEnergyRowsFailure),
    BeforeSlew(&'a str), Slew(&'a SourceSlewFailure),
}
pub struct SourceFilterWindowEnergySlewGainFailure { owner: FilterWindowEnergySlewGainOwner }
impl SourceFilterWindowEnergySlewGainFailure {
    pub fn error(&self) -> SourceFilterWindowEnergySlewGainError<'_> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => SourceFilterWindowEnergySlewGainError::Filter(failure),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => SourceFilterWindowEnergySlewGainError::Windows(failure),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } =>
                SourceFilterWindowEnergySlewGainError::BeforeSlew(&failure.error),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } =>
                SourceFilterWindowEnergySlewGainError::Slew(failure),
        }
    }
    pub fn input(&self) -> &[f32] {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.input(),
            FilterWindowEnergySlewGainOwner::Windows { input, .. } | FilterWindowEnergySlewGainOwner::Suffix { input, .. } => input,
        }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.target_db(),
            FilterWindowEnergySlewGainOwner::Windows { target_db, .. } | FilterWindowEnergySlewGainOwner::Suffix { target_db, .. } => *target_db,
        }
    }
    pub fn filter_math(&self) -> &SourceFilterMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.math(),
            FilterWindowEnergySlewGainOwner::Windows { filter_math, .. } | FilterWindowEnergySlewGainOwner::Suffix { filter_math, .. } => filter_math,
        }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { window_math, .. } => window_math,
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.window_math(),
            FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => rows.window_math(),
        }
    }
    pub fn energy_math(&self) -> &SourceEnergyMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { energy_math, .. } => energy_math,
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.math(),
            FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => rows.math(),
        }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { slew_math, .. } | FilterWindowEnergySlewGainOwner::Windows { slew_math, .. } => slew_math,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => owner,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.math(),
        }
    }
    pub fn unused_window_storage(&self) -> Option<&SourceEnergyWindows> {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { storage, .. } => Some(storage), _ => None }
    }
    pub fn window_energy_rows(&self) -> Option<&SourceWindowEnergyRowsCompleted> {
        match &self.owner { FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => Some(rows), _ => None }
    }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.progress(),
            FilterWindowEnergySlewGainOwner::Windows { filter_progress, .. }
            | FilterWindowEnergySlewGainOwner::Suffix { filter_progress, .. } => Some(filter_progress),
        }
    }
    pub fn first_filter(&self) -> &[f32] {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.first_filter(),
            FilterWindowEnergySlewGainOwner::Windows { first_filter, .. } => first_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.first_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { first_filter, .. }, .. } => first_filter,
        }
    }
    pub fn second_filter(&self) -> &[f32] {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.second_filter(),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.source(),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.second_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { second_filter, .. }, .. } => second_filter,
        }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.energy(),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => Some(failure.energy()),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => Some(&failure.energy),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { energy, .. }, .. } => Some(energy),
        }
    }
    pub fn metric_log(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.metric_log(),
            FilterWindowEnergySlewGainOwner::Windows { .. } => None,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => failure.metric_log.as_deref(),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        }
    }
}

pub fn frontend_gain_table_with_source_filters_window_energy_and_slew(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, slew_math: SourceSlewMath)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, SourceWindowEnergyRowsCompleted, SourceSlewMath, Vec<f32>),
        SourceFilterWindowEnergySlewGainFailure>
{
    constructed_gain_with(input, target_db, filter_math, outputs, window_math, energy_math, storage, slew_math,
        |metric_log, target, owner| {
            gain_slew_from_log_with_source_math(metric_log, target, owner)
                .map(|((rounds, gain_db), owner, metric_log)| SlewCompleted { rounds, gain_db, owner, metric_log })
        }).map_err(|owner| SourceFilterWindowEnergySlewGainFailure { owner })
}
fn constructed_gain_with<P, E>(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, slew_math: P,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, SourceWindowEnergyRowsCompleted, P, Vec<f32>),
        FilterWindowEnergySlewGainOwner<P, E>>
{
    match constructed_gain_with_count(input, target_db, filter_math, outputs, HostCount, window_math, energy_math, storage, slew_math, slew) {
        Ok((stages, filter, (), rows, owner, input)) => Ok((stages, filter, rows, owner, input)),
        Err(owner) => Err(match owner {
            CountedGainOwner::Filter { failure, count: HostCount, window_math, energy_math, storage, slew_math } =>
                FilterWindowEnergySlewGainOwner::Filter { failure, window_math, energy_math, storage, slew_math },
            CountedGainOwner::Count { failure, .. } => match failure {},
            CountedGainOwner::Windows { input, target_db, filter_math, first_filter, filter_progress, count: (), failure, slew_math } =>
                FilterWindowEnergySlewGainOwner::Windows { input, target_db, filter_math, first_filter, filter_progress, failure, slew_math },
            CountedGainOwner::Suffix { input, target_db, filter_math, filter_progress, count: (), rows, failure } =>
                FilterWindowEnergySlewGainOwner::Suffix { input, target_db, filter_math, filter_progress, rows, failure },
        }),
    }
}
trait GainCountChoice {
    type Receipt;
    type Error;
    fn select(self, source_count: usize) -> std::result::Result<(usize, Self::Receipt), Self::Error>;
}
struct HostCount;
impl GainCountChoice for HostCount {
    type Receipt = ();
    type Error = Infallible;
    fn select(self, source_count: usize) -> std::result::Result<(usize, ()), Infallible> {
        Ok((gain_energy_count(source_count), ()))
    }
}
struct SourceCount(count_math::SourceGainCountMath);
impl GainCountChoice for SourceCount {
    type Receipt = count_math::SourceGainCountParts;
    type Error = count_math::SourceGainCountFailure;
    fn select(self, source_count: usize) -> std::result::Result<(usize, Self::Receipt), Self::Error> {

        let request = count_math::SourceGainCountRequest { source_count_dword: source_count as u32 };
        let parts = count_math::count_source_windows(request, self.0)?.into_parts();

        let expected = parts.observation.selected_count as usize;
        Ok((expected, parts))
    }
}
enum CountedGainOwner<K: GainCountChoice, P, E> {
    Filter { failure: SourceFilterGainFailure, count: K, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: SourceEnergyWindows, slew_math: P },
    Count { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
        filter_progress: SourceFilterProgress, failure: K::Error, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: SourceEnergyWindows, slew_math: P },
    Windows { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, first_filter: Vec<f32>,
        filter_progress: SourceFilterProgress, count: K::Receipt, failure: SourceWindowEnergyRowsFailure, slew_math: P },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        count: K::Receipt, rows: SourceWindowEnergyRowsCompleted, failure: GainSuffixChoiceFailure<P, E> },
}
fn constructed_gain_with_count<K: GainCountChoice, P, E>(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, count: K,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, slew_math: P,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, K::Receipt, SourceWindowEnergyRowsCompleted, P, Vec<f32>),
        CountedGainOwner<K, P, E>>
{
    match constructed_gain_with_log(input, target_db, filter_math, outputs, count, window_math,
        energy_math, storage, (), slew_math, host_gain_log, slew) {
        Ok((stages, filter, count, rows, (), owner, input)) => Ok((stages, filter, count, rows, owner, input)),
        Err(owner) => Err(match owner {
            CountedLogGainOwner::Filter { failure, count, window_math, energy_math, storage, log: (), slew_math } =>
                CountedGainOwner::Filter { failure, count, window_math, energy_math, storage, slew_math },
            CountedLogGainOwner::Count { input, target_db, filter_math, outputs, filter_progress, failure,
                window_math, energy_math, storage, log: (), slew_math } => CountedGainOwner::Count {
                    input, target_db, filter_math, outputs, filter_progress, failure, window_math, energy_math, storage, slew_math },
            CountedLogGainOwner::Windows { input, target_db, filter_math, first_filter, filter_progress, count,
                failure, log: (), slew_math } => CountedGainOwner::Windows {
                    input, target_db, filter_math, first_filter, filter_progress, count, failure, slew_math },
            CountedLogGainOwner::Suffix { input, target_db, filter_math, filter_progress, count, rows, failure } =>
                CountedGainOwner::Suffix { input, target_db, filter_math, filter_progress, count, rows,
                    failure: host_log_suffix_failure(failure) },
        }),
    }
}
enum CountedLogGainOwner<K: GainCountChoice, LI, LO, LE, P, E, BS = SourceEnergyWindows, BE = SourceWindowEnergyRowsFailure> {
    Filter { failure: SourceFilterGainFailure, count: K, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: BS, log: LI, slew_math: P },
    Count { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
        filter_progress: SourceFilterProgress, failure: K::Error, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: BS, log: LI, slew_math: P },
    Windows { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, first_filter: Vec<f32>,
        filter_progress: SourceFilterProgress, count: K::Receipt, failure: BE, log: LI, slew_math: P },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        count: K::Receipt, rows: SourceWindowEnergyRowsCompleted, failure: LogSuffixFailure<LO, LE, P, E> },
}
fn constructed_gain_with_log<K: GainCountChoice, LI, LO, LE, P, E>(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, count: K,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, log: LI, slew_math: P,
    logarithm: impl FnOnce(Vec<f32>, LI) -> std::result::Result<LogCompleted<LO>, LE>,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, K::Receipt, SourceWindowEnergyRowsCompleted, LO, P, Vec<f32>),
        CountedLogGainOwner<K, LI, LO, LE, P, E>>
{
    match constructed_gain_with_batch(input, target_db, filter_math, outputs, count, window_math, energy_math, storage, log, slew_math,
        |source, source_count, storage, expected, window, energy, log| {
            match energy_windows::construct_and_reduce_windows(source, source_count, storage, expected, window, energy) {
                Ok(energy_windows::ConstructedRows { energy, source, rows }) => Ok(GainBatchCompleted { energy, source, rows, normal: log }),
                Err(failure) => Err(PreservedBatchFailure { failure, log }),
            }
        }, logarithm, slew) {
        Ok(completed) => Ok(completed),
        Err(owner) => Err(match owner {
            BatchedGainOwner::Filter { failure, count, window_math, energy_math, storage, log, slew_math } =>
                CountedLogGainOwner::Filter { failure, count, window_math, energy_math, storage, log, slew_math },
            BatchedGainOwner::Count { input, target_db, filter_math, outputs, filter_progress, failure,
                window_math, energy_math, storage, log, slew_math } => CountedLogGainOwner::Count {
                    input, target_db, filter_math, outputs, filter_progress, failure, window_math, energy_math, storage, log, slew_math },
            BatchedGainOwner::Windows { input, target_db, filter_math, first_filter, filter_progress, count,
                failure: PreservedBatchFailure { failure, log }, slew_math } => CountedLogGainOwner::Windows {
                    input, target_db, filter_math, first_filter, filter_progress, count, failure, log, slew_math },
            BatchedGainOwner::Suffix { input, target_db, filter_math, filter_progress, count, rows, failure } =>
                CountedLogGainOwner::Suffix { input, target_db, filter_math, filter_progress, count, rows, failure },
        }),
    }
}
struct PreservedBatchFailure<LI> { failure: SourceWindowEnergyRowsFailure, log: LI }
struct GainBatchCompleted<M> { energy: Vec<f32>, source: Vec<f32>, rows: SourceWindowEnergyRowsCompleted, normal: M }
enum BatchedGainOwner<K: GainCountChoice, LI, LO, LE, P, E, BS, BE> {
    Filter { failure: SourceFilterGainFailure, count: K, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: BS, log: LI, slew_math: P },
    Count { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
        filter_progress: SourceFilterProgress, failure: K::Error, window_math: crate::frontend_energy_window_math::SourceWindowMath,
        energy_math: SourceEnergyMath, storage: BS, log: LI, slew_math: P },
    Windows { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, first_filter: Vec<f32>,
        filter_progress: SourceFilterProgress, count: K::Receipt, failure: BE, slew_math: P },
    Suffix { input: Vec<f32>, target_db: f32, filter_math: SourceFilterMath, filter_progress: SourceFilterProgress,
        count: K::Receipt, rows: SourceWindowEnergyRowsCompleted, failure: LogSuffixFailure<LO, LE, P, E> },
}
fn constructed_gain_with_batch<K: GainCountChoice, LI, LP, LO, LE, P, E, BS, BE>(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, count: K,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: BS, log: LI, slew_math: P,
    batch: impl FnOnce(Vec<f32>, usize, BS, usize, crate::frontend_energy_window_math::SourceWindowMath, SourceEnergyMath, LI)
        -> std::result::Result<GainBatchCompleted<LP>, BE>,
    logarithm: impl FnOnce(Vec<f32>, LP) -> std::result::Result<LogCompleted<LO>, LE>,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, K::Receipt, SourceWindowEnergyRowsCompleted, LO, P, Vec<f32>),
        BatchedGainOwner<K, LI, LO, LE, P, E, BS, BE>>
{
    constructed_gain_with_batch_and_admission(input, target_db, filter_math, outputs, count, window_math, energy_math,
        storage, log, slew_math, batch, logarithm, slew, filter_math::FilterInputAdmission::Legacy)
}
fn constructed_gain_with_batch_and_admission<K: GainCountChoice, LI, LP, LO, LE, P, E, BS, BE>(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, count: K,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: BS, log: LI, slew_math: P,
    batch: impl FnOnce(Vec<f32>, usize, BS, usize, crate::frontend_energy_window_math::SourceWindowMath, SourceEnergyMath, LI)
        -> std::result::Result<GainBatchCompleted<LP>, BE>,
    logarithm: impl FnOnce(Vec<f32>, LP) -> std::result::Result<LogCompleted<LO>, LE>,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>, admission: filter_math::FilterInputAdmission)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, K::Receipt, SourceWindowEnergyRowsCompleted, LO, P, Vec<f32>),
        BatchedGainOwner<K, LI, LO, LE, P, E, BS, BE>>
{
    let prepared = match filter_math::prepare_source_filters_with_admission(input, target_db, filter_math, outputs, admission) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(BatchedGainOwner::Filter {
            failure, count, window_math, energy_math, storage, log, slew_math }),
    };
    let filter_math::PreparedSourceFilters { input, target_db, math: filter_math, outputs, progress: filter_progress } = prepared;
    let (expected, count) = match count.select(input.len()) {
        Ok(completed) => completed,
        Err(failure) => return Err(BatchedGainOwner::Count { input, target_db, filter_math, outputs,
            filter_progress, failure, window_math, energy_math, storage, log, slew_math }),
    };
    let completed = match batch(outputs.second, input.len(), storage, expected, window_math, energy_math, log) {
        Ok(completed) => completed,
        Err(failure) => return Err(BatchedGainOwner::Windows {
            input, target_db, filter_math, first_filter: outputs.first, filter_progress, count, failure, slew_math }),
    };
    let GainBatchCompleted { energy, source: second_filter, rows, normal } = completed;
    match gain_table_after_energy_with_log(target_db, outputs.first, second_filter, energy, normal, slew_math, logarithm, slew) {
        Ok((stages, log, slew_math)) => Ok((stages, filter_math, count, rows, log, slew_math, input)),
        Err(failure) => Err(BatchedGainOwner::Suffix {
            input, target_db, filter_math, filter_progress, count, rows, failure }),
    }
}
pub enum SourceFilterWindowEnergySlewFinalGainError<'a> {
    Filter(&'a SourceFilterGainFailure), Windows(&'a SourceWindowEnergyRowsFailure),
    BeforeSlew(&'a str), Finish(&'a SourceSlewFinalGainFailure),
}
pub struct SourceFilterWindowEnergySlewFinalGainFailure {
    owner: FilterWindowEnergySlewGainOwner<(SourceSlewMath, SourceFinalGainMath), SourceSlewFinalGainFailure>,
}
impl SourceFilterWindowEnergySlewFinalGainFailure {
    pub fn error(&self) -> SourceFilterWindowEnergySlewFinalGainError<'_> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => SourceFilterWindowEnergySlewFinalGainError::Filter(failure),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => SourceFilterWindowEnergySlewFinalGainError::Windows(failure),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } =>
                SourceFilterWindowEnergySlewFinalGainError::BeforeSlew(&failure.error),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } =>
                SourceFilterWindowEnergySlewFinalGainError::Finish(failure),
        }
    }
    pub fn input(&self) -> &[f32] {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.input(),
            FilterWindowEnergySlewGainOwner::Windows { input, .. } | FilterWindowEnergySlewGainOwner::Suffix { input, .. } => input }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.target_db(),
            FilterWindowEnergySlewGainOwner::Windows { target_db, .. } | FilterWindowEnergySlewGainOwner::Suffix { target_db, .. } => *target_db }
    }
    pub fn filter_math(&self) -> &SourceFilterMath {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.math(),
            FilterWindowEnergySlewGainOwner::Windows { filter_math, .. } | FilterWindowEnergySlewGainOwner::Suffix { filter_math, .. } => filter_math }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { window_math, .. } => window_math,
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.window_math(),
            FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => rows.window_math() }
    }
    pub fn energy_math(&self) -> &SourceEnergyMath {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { energy_math, .. } => energy_math,
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.math(),
            FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => rows.math() }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { slew_math, .. } | FilterWindowEnergySlewGainOwner::Windows { slew_math, .. } => &slew_math.0,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => &owner.0,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.slew_math(),
        }
    }
    pub fn final_gain_math(&self) -> &SourceFinalGainMath {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { slew_math, .. } | FilterWindowEnergySlewGainOwner::Windows { slew_math, .. } => &slew_math.1,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => &owner.1,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.final_gain_math(),
        }
    }
    pub fn unused_window_storage(&self) -> Option<&SourceEnergyWindows> {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { storage, .. } => Some(storage), _ => None }
    }
    pub fn window_energy_rows(&self) -> Option<&SourceWindowEnergyRowsCompleted> {
        match &self.owner { FilterWindowEnergySlewGainOwner::Suffix { rows, .. } => Some(rows), _ => None }
    }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> {
        match &self.owner { FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.progress(),
            FilterWindowEnergySlewGainOwner::Windows { filter_progress, .. }
            | FilterWindowEnergySlewGainOwner::Suffix { filter_progress, .. } => Some(filter_progress) }
    }
    pub fn first_filter(&self) -> &[f32] {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.first_filter(),
            FilterWindowEnergySlewGainOwner::Windows { first_filter, .. } => first_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.first_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { first_filter, .. }, .. } => first_filter,
        }
    }
    pub fn second_filter(&self) -> &[f32] {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.second_filter(),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => failure.source(),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.second_filter,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { second_filter, .. }, .. } => second_filter,
        }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.energy(),
            FilterWindowEnergySlewGainOwner::Windows { failure, .. } => Some(failure.energy()),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => Some(&failure.energy),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { energy, .. }, .. } => Some(energy),
        }
    }
    pub fn metric_log(&self) -> Option<&[f32]> {
        match &self.owner {
            FilterWindowEnergySlewGainOwner::Filter { failure, .. } => failure.metric_log(),
            FilterWindowEnergySlewGainOwner::Windows { .. } => None,
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => failure.metric_log.as_deref(),
            FilterWindowEnergySlewGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        }
    }
}

pub fn frontend_gain_table_with_source_filters_window_energy_slew_and_final_gain(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, slew_math: SourceSlewMath, final_math: SourceFinalGainMath)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, SourceWindowEnergyRowsCompleted, SourceSlewMath, SourceFinalGainMath, Vec<f32>),
        SourceFilterWindowEnergySlewFinalGainFailure>
{
    constructed_gain_with(input, target_db, filter_math, outputs, window_math, energy_math, storage, (slew_math, final_math),
        |metric_log, target, (slew, final_math)| {
            gain_slew_from_log_with_source_final_gain(metric_log, target, slew, final_math)
                .map(|((rounds, gain_db), slew, final_math, metric_log)| SlewCompleted { rounds, gain_db, owner: (slew, final_math), metric_log })
        }).map(|(stages, filter, rows, (slew, final_math), input)| (stages, filter, rows, slew, final_math, input))
        .map_err(|owner| SourceFilterWindowEnergySlewFinalGainFailure { owner })
}
pub enum SourceCountFilterWindowEnergySlewFinalGainError<'a> {
    Filter(&'a SourceFilterGainFailure), Count(&'a count_math::SourceGainCountFailure),
    Windows(&'a SourceWindowEnergyRowsFailure), BeforeSlew(&'a str), Finish(&'a SourceSlewFinalGainFailure),
}
pub struct SourceCountFilterWindowEnergySlewFinalGainFailure {
    owner: CountedGainOwner<SourceCount, (SourceSlewMath, SourceFinalGainMath), SourceSlewFinalGainFailure>,
}
impl SourceCountFilterWindowEnergySlewFinalGainFailure {
    pub fn error(&self) -> SourceCountFilterWindowEnergySlewFinalGainError<'_> {
        match &self.owner {
            CountedGainOwner::Filter { failure, .. } => SourceCountFilterWindowEnergySlewFinalGainError::Filter(failure),
            CountedGainOwner::Count { failure, .. } => SourceCountFilterWindowEnergySlewFinalGainError::Count(failure),
            CountedGainOwner::Windows { failure, .. } => SourceCountFilterWindowEnergySlewFinalGainError::Windows(failure),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } =>
                SourceCountFilterWindowEnergySlewFinalGainError::BeforeSlew(&failure.error),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } =>
                SourceCountFilterWindowEnergySlewFinalGainError::Finish(failure),
        }
    }
    pub fn input(&self) -> &[f32] {
        match &self.owner { CountedGainOwner::Filter { failure, .. } => failure.input(),
            CountedGainOwner::Count { input, .. } | CountedGainOwner::Windows { input, .. } | CountedGainOwner::Suffix { input, .. } => input }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner { CountedGainOwner::Filter { failure, .. } => failure.target_db(),
            CountedGainOwner::Count { target_db, .. } | CountedGainOwner::Windows { target_db, .. }
            | CountedGainOwner::Suffix { target_db, .. } => *target_db }
    }
    pub fn filter_math(&self) -> &SourceFilterMath {
        match &self.owner { CountedGainOwner::Filter { failure, .. } => failure.math(),
            CountedGainOwner::Count { filter_math, .. } | CountedGainOwner::Windows { filter_math, .. }
            | CountedGainOwner::Suffix { filter_math, .. } => filter_math }
    }
    pub fn count_math(&self) -> &count_math::SourceGainCountMath {
        match &self.owner {
            CountedGainOwner::Filter { count, .. } => &count.0,
            CountedGainOwner::Count { failure, .. } => failure.math(),
            CountedGainOwner::Windows { count, .. } | CountedGainOwner::Suffix { count, .. } => &count.math,
        }
    }
    pub fn count_parts(&self) -> Option<&count_math::SourceGainCountParts> {
        match &self.owner {
            CountedGainOwner::Windows { count, .. } | CountedGainOwner::Suffix { count, .. } => Some(count), _ => None,
        }
    }
    pub fn count_failure(&self) -> Option<&count_math::SourceGainCountFailure> {
        match &self.owner { CountedGainOwner::Count { failure, .. } => Some(failure), _ => None }
    }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath {
        match &self.owner {
            CountedGainOwner::Filter { window_math, .. } | CountedGainOwner::Count { window_math, .. } => window_math,
            CountedGainOwner::Windows { failure, .. } => failure.window_math(),
            CountedGainOwner::Suffix { rows, .. } => rows.window_math(),
        }
    }
    pub fn energy_math(&self) -> &SourceEnergyMath {
        match &self.owner {
            CountedGainOwner::Filter { energy_math, .. } | CountedGainOwner::Count { energy_math, .. } => energy_math,
            CountedGainOwner::Windows { failure, .. } => failure.math(),
            CountedGainOwner::Suffix { rows, .. } => rows.math(),
        }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner {
            CountedGainOwner::Filter { slew_math, .. } | CountedGainOwner::Count { slew_math, .. }
            | CountedGainOwner::Windows { slew_math, .. } => &slew_math.0,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => &owner.0,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.slew_math(),
        }
    }
    pub fn final_gain_math(&self) -> &SourceFinalGainMath {
        match &self.owner {
            CountedGainOwner::Filter { slew_math, .. } | CountedGainOwner::Count { slew_math, .. }
            | CountedGainOwner::Windows { slew_math, .. } => &slew_math.1,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { owner, .. }, .. } => &owner.1,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => failure.final_gain_math(),
        }
    }
    pub fn unused_window_storage(&self) -> Option<&SourceEnergyWindows> {
        match &self.owner { CountedGainOwner::Filter { storage, .. } | CountedGainOwner::Count { storage, .. } => Some(storage), _ => None }
    }
    pub fn window_energy_rows(&self) -> Option<&SourceWindowEnergyRowsCompleted> {
        match &self.owner { CountedGainOwner::Suffix { rows, .. } => Some(rows), _ => None }
    }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> {
        match &self.owner { CountedGainOwner::Filter { failure, .. } => failure.progress(),
            CountedGainOwner::Count { filter_progress, .. } | CountedGainOwner::Windows { filter_progress, .. }
            | CountedGainOwner::Suffix { filter_progress, .. } => Some(filter_progress) }
    }
    pub fn first_filter(&self) -> &[f32] {
        match &self.owner {
            CountedGainOwner::Filter { failure, .. } => failure.first_filter(),
            CountedGainOwner::Count { outputs, .. } => &outputs.first,
            CountedGainOwner::Windows { first_filter, .. } => first_filter,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.first_filter,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { first_filter, .. }, .. } => first_filter,
        }
    }
    pub fn second_filter(&self) -> &[f32] {
        match &self.owner {
            CountedGainOwner::Filter { failure, .. } => failure.second_filter(),
            CountedGainOwner::Count { outputs, .. } => &outputs.second,
            CountedGainOwner::Windows { failure, .. } => failure.source(),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => &failure.second_filter,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { second_filter, .. }, .. } => second_filter,
        }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner {
            CountedGainOwner::Filter { failure, .. } => failure.energy(), CountedGainOwner::Count { .. } => None,
            CountedGainOwner::Windows { failure, .. } => Some(failure.energy()),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => Some(&failure.energy),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { energy, .. }, .. } => Some(energy),
        }
    }
    pub fn metric_log(&self) -> Option<&[f32]> {
        match &self.owner {
            CountedGainOwner::Filter { failure, .. } => failure.metric_log(),
            CountedGainOwner::Count { .. } | CountedGainOwner::Windows { .. } => None,
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Before { failure, .. }, .. } => failure.metric_log.as_deref(),
            CountedGainOwner::Suffix { failure: GainSuffixChoiceFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        }
    }
}

pub fn frontend_gain_table_with_source_filters_count_window_energy_slew_and_final_gain(input: Vec<f32>, target_db: f32,
    filter_math: SourceFilterMath, outputs: SourceFilterOutputs, count: count_math::SourceGainCountMath,
    window_math: crate::frontend_energy_window_math::SourceWindowMath, energy_math: SourceEnergyMath,
    storage: SourceEnergyWindows, slew_math: SourceSlewMath, final_math: SourceFinalGainMath)
    -> std::result::Result<(FrontendGainStages, SourceFilterMath, count_math::SourceGainCountParts, SourceWindowEnergyRowsCompleted,
        SourceSlewMath, SourceFinalGainMath, Vec<f32>), SourceCountFilterWindowEnergySlewFinalGainFailure>
{
    constructed_gain_with_count(input, target_db, filter_math, outputs, SourceCount(count), window_math, energy_math, storage, (slew_math, final_math),
        |metric_log, target, (slew, final_math)| {
            gain_slew_from_log_with_source_final_gain(metric_log, target, slew, final_math)
                .map(|((rounds, gain_db), slew, final_math, metric_log)| SlewCompleted { rounds, gain_db, owner: (slew, final_math), metric_log })
        }).map(|(stages, filter, count, rows, (slew, final_math), input)| (stages, filter, count, rows, slew, final_math, input))
        .map_err(|owner| SourceCountFilterWindowEnergySlewFinalGainFailure { owner })
}

pub struct SourceGainLogRequest<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub metric: Vec<f32>, pub statistics: statistics::SourceGainStatisticsRequest,
    pub gated_math: crate::frontend_gated_energy_math::SourceGatedEnergyMath,
    pub suffix_math: crate::source_scalar::SourceF64ValueMath, pub services: L,
}

pub struct SourceGainLogContext<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub request: statistics::SourceGainStatisticsRequest,
    pub gated_math: crate::frontend_gated_energy_math::SourceGatedEnergyMath,
    pub suffix_math: crate::source_scalar::SourceF64ValueMath, pub services: L,
    pub rows: Vec<log_rows::SourceGainLogRowReceipt<L>>,
    pub first: statistics::SourceGainStatisticsPassReceipt<L>, pub second: statistics::SourceGainStatisticsPassReceipt<L>,

    pub result_bits: u32,
}
pub struct SourceLoggedGainRequest<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub filter_math: SourceFilterMath, pub outputs: SourceFilterOutputs, pub count_math: count_math::SourceGainCountMath,
    pub window_math: crate::frontend_energy_window_math::SourceWindowMath,
    pub energy_math: SourceEnergyMath, pub windows: SourceEnergyWindows,
    pub log: SourceGainLogRequest<L>, pub slew_math: SourceSlewMath, pub final_gain_math: SourceFinalGainMath,
}
pub struct SourceLoggedGainContext<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub filter_math: SourceFilterMath, pub count: count_math::SourceGainCountParts,
    pub rows: SourceWindowEnergyRowsCompleted, pub log: SourceGainLogContext<L>,
    pub slew_math: SourceSlewMath, pub final_gain_math: SourceFinalGainMath,
}
pub struct SourceLoggedGainParts<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub input: Vec<f32>, pub stages: FrontendGainStages, pub context: SourceLoggedGainContext<L>,
}
pub struct SourceLoggedGainCompleted<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    parts: SourceLoggedGainParts<L>,
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceLoggedGainCompleted<L> {
    pub fn input(&self) -> &[f32] { &self.parts.input }
    pub fn stages(&self) -> &FrontendGainStages { &self.parts.stages }
    pub fn context(&self) -> &SourceLoggedGainContext<L> { &self.parts.context }
    pub fn into_parts(self) -> SourceLoggedGainParts<L> { self.parts }
}
type LoggedGainOwner<L> = CountedLogGainOwner<SourceCount, SourceGainLogRequest<L>, SourceGainLogContext<L>,
    log_rows::SourceGainLogRowsFailure<L>, (SourceSlewMath, SourceFinalGainMath), SourceSlewFinalGainFailure>;
pub enum SourceLoggedGainError<'a, L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    Filter(&'a SourceFilterGainFailure), Count(&'a count_math::SourceGainCountFailure),
    Windows(&'a SourceWindowEnergyRowsFailure), Log(&'a log_rows::SourceGainLogRowsFailure<L>), Finish(&'a SourceSlewFinalGainFailure),
}
pub struct SourceLoggedGainFailure<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    owner: LoggedGainOwner<L>,
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceLoggedGainFailure<L> {
    pub fn error(&self) -> SourceLoggedGainError<'_, L> { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => SourceLoggedGainError::Filter(failure),
        CountedLogGainOwner::Count { failure, .. } => SourceLoggedGainError::Count(failure),
        CountedLogGainOwner::Windows { failure, .. } => SourceLoggedGainError::Windows(failure),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => SourceLoggedGainError::Log(failure),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => SourceLoggedGainError::Finish(failure),
    } }
    pub fn input(&self) -> &[f32] { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.input(),
        CountedLogGainOwner::Count { input, .. } | CountedLogGainOwner::Windows { input, .. }
        | CountedLogGainOwner::Suffix { input, .. } => input,
    } }
    pub fn target_db(&self) -> f32 { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.target_db(),
        CountedLogGainOwner::Count { target_db, .. } | CountedLogGainOwner::Windows { target_db, .. }
        | CountedLogGainOwner::Suffix { target_db, .. } => *target_db,
    } }
    pub fn filter_math(&self) -> &SourceFilterMath { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.math(),
        CountedLogGainOwner::Count { filter_math, .. } | CountedLogGainOwner::Windows { filter_math, .. }
        | CountedLogGainOwner::Suffix { filter_math, .. } => filter_math,
    } }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.progress(),
        CountedLogGainOwner::Count { filter_progress, .. } | CountedLogGainOwner::Windows { filter_progress, .. }
        | CountedLogGainOwner::Suffix { filter_progress, .. } => Some(filter_progress),
    } }
    pub fn count_math(&self) -> &count_math::SourceGainCountMath { match &self.owner {
        CountedLogGainOwner::Filter { count, .. } => &count.0,
        CountedLogGainOwner::Count { failure, .. } => failure.math(),
        CountedLogGainOwner::Windows { count, .. } | CountedLogGainOwner::Suffix { count, .. } => &count.math,
    } }
    pub fn count_parts(&self) -> Option<&count_math::SourceGainCountParts> { match &self.owner {
        CountedLogGainOwner::Windows { count, .. } | CountedLogGainOwner::Suffix { count, .. } => Some(count), _ => None,
    } }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath { match &self.owner {
        CountedLogGainOwner::Filter { window_math, .. } | CountedLogGainOwner::Count { window_math, .. } => window_math,
        CountedLogGainOwner::Windows { failure, .. } => failure.window_math(),
        CountedLogGainOwner::Suffix { rows, .. } => rows.window_math(),
    } }
    pub fn energy_math(&self) -> &SourceEnergyMath { match &self.owner {
        CountedLogGainOwner::Filter { energy_math, .. } | CountedLogGainOwner::Count { energy_math, .. } => energy_math,
        CountedLogGainOwner::Windows { failure, .. } => failure.math(),
        CountedLogGainOwner::Suffix { rows, .. } => rows.math(),
    } }
    pub fn unused_window_storage(&self) -> Option<&SourceEnergyWindows> { match &self.owner {
        CountedLogGainOwner::Filter { storage, .. } | CountedLogGainOwner::Count { storage, .. } => Some(storage), _ => None,
    } }
    pub fn window_energy_rows(&self) -> Option<&SourceWindowEnergyRowsCompleted> { match &self.owner {
        CountedLogGainOwner::Suffix { rows, .. } => Some(rows), _ => None,
    } }
    pub fn unattempted_log(&self) -> Option<&SourceGainLogRequest<L>> { match &self.owner {
        CountedLogGainOwner::Filter { log, .. } | CountedLogGainOwner::Count { log, .. }
        | CountedLogGainOwner::Windows { log, .. } => Some(log), _ => None,
    } }
    pub fn log_failure(&self) -> Option<&log_rows::SourceGainLogRowsFailure<L>> { match &self.owner {
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure), _ => None,
    } }
    pub fn log_context(&self) -> Option<&SourceGainLogContext<L>> { match &self.owner {
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { log, .. }, .. } => Some(log), _ => None,
    } }
    pub fn slew_math(&self) -> &SourceSlewMath { match &self.owner {
        CountedLogGainOwner::Filter { slew_math, .. } | CountedLogGainOwner::Count { slew_math, .. }
        | CountedLogGainOwner::Windows { slew_math, .. } => &slew_math.0,
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { owner, .. }, .. } => &owner.0,
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => failure.slew_math(),
    } }
    pub fn final_gain_math(&self) -> &SourceFinalGainMath { match &self.owner {
        CountedLogGainOwner::Filter { slew_math, .. } | CountedLogGainOwner::Count { slew_math, .. }
        | CountedLogGainOwner::Windows { slew_math, .. } => &slew_math.1,
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { owner, .. }, .. } => &owner.1,
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => failure.final_gain_math(),
    } }
    pub fn first_filter(&self) -> &[f32] { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.first_filter(),
        CountedLogGainOwner::Count { outputs, .. } => &outputs.first,
        CountedLogGainOwner::Windows { first_filter, .. } => first_filter,
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { first_filter, .. }
            | LogSuffixFailure::Slew { first_filter, .. }, .. } => first_filter,
    } }
    pub fn second_filter(&self) -> &[f32] { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.second_filter(),
        CountedLogGainOwner::Count { outputs, .. } => &outputs.second,
        CountedLogGainOwner::Windows { failure, .. } => failure.source(),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { second_filter, .. }
            | LogSuffixFailure::Slew { second_filter, .. }, .. } => second_filter,
    } }
    pub fn energy(&self) -> Option<&[f32]> { match &self.owner {
        CountedLogGainOwner::Filter { failure, .. } => failure.energy(), CountedLogGainOwner::Count { .. } => None,
        CountedLogGainOwner::Windows { failure, .. } => Some(failure.energy()),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure.energy()),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { energy, .. }, .. } => Some(energy),
    } }
    pub fn metric_log(&self) -> Option<&[f32]> { match &self.owner {
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure.metric()),
        CountedLogGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()),
        _ => self.unattempted_log().map(|request| request.metric.as_slice()),
    } }
}
fn source_gain_log<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(
    energy: Vec<f32>, initial: SourceGainLogRequest<L>) -> std::result::Result<LogCompleted<SourceGainLogContext<L>>, log_rows::SourceGainLogRowsFailure<L>>
{
    let SourceGainLogRequest { metric, statistics: request, gated_math, suffix_math, services } = initial;
    let log_rows::SourceGainLogRowsParts { statistics, rows } =
        log_rows::run_gain_log_rows(energy, metric, request, gated_math, suffix_math, services)?.into_parts();
    Ok(completed_gain_log(statistics, rows))
}
fn completed_gain_log<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(
    parts: statistics::SourceGainStatisticsParts<L>, rows: Vec<log_rows::SourceGainLogRowReceipt<L>>) -> LogCompleted<SourceGainLogContext<L>>
{
    let statistics::SourceGainStatisticsParts { energy, metric_log, request, gated_math, suffix_math,
        log_services: services, first, second, result_bits } = parts;
    LogCompleted { energy, metric_log, owner: SourceGainLogContext {
        request, gated_math, suffix_math, services, rows, first, second, result_bits } }
}

pub fn frontend_gain_with_source_logs<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(
    input: Vec<f32>, target_db: f32, request: SourceLoggedGainRequest<L>)
    -> std::result::Result<SourceLoggedGainCompleted<L>, SourceLoggedGainFailure<L>>
{
    let SourceLoggedGainRequest { filter_math, outputs, count_math, window_math, energy_math, windows,
        log, slew_math, final_gain_math } = request;
    constructed_gain_with_log(input, target_db, filter_math, outputs, SourceCount(count_math), window_math, energy_math,
        windows, log, (slew_math, final_gain_math), source_gain_log,
        |metric_log, target, (slew, final_math)| gain_slew_from_log_with_source_final_gain(metric_log, target, slew, final_math)
            .map(|((rounds, gain_db), slew, final_math, metric_log)| SlewCompleted { rounds, gain_db, owner: (slew, final_math), metric_log }))
        .map(|(stages, filter_math, count, rows, log, (slew_math, final_gain_math), input)| SourceLoggedGainCompleted {
            parts: SourceLoggedGainParts { input, stages, context: SourceLoggedGainContext {
                filter_math, count, rows, log, slew_math, final_gain_math } } })
        .map_err(|owner| SourceLoggedGainFailure { owner })
}

pub struct SourceInitializedGainLogRequest<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub statistics: statistics::SourceGainStatisticsRequest,
    pub gated_math: crate::frontend_gated_energy_math::SourceGatedEnergyMath,
    pub suffix_math: crate::source_scalar::SourceF64ValueMath, pub services: L,
}
pub struct SourceInitializedLoggedGainRequest<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub filter_math: SourceFilterMath, pub outputs: SourceFilterOutputs, pub count_math: count_math::SourceGainCountMath,
    pub window_math: crate::frontend_energy_window_math::SourceWindowMath,
    pub energy_math: SourceEnergyMath, pub windows: SourceWindowStorage,
    pub log: SourceInitializedGainLogRequest<L>, pub slew_math: SourceSlewMath, pub final_gain_math: SourceFinalGainMath,
}
struct PreparedInitializedLog<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    metric: Vec<f32>, logs: energy_windows::InitializedRowLogs<L>,
}
pub struct SourceInitializedStatisticsFailure<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    failure: statistics::SourceGainStatisticsFailure<L>, rows: Vec<log_rows::SourceGainLogRowReceipt<L>>,
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceInitializedStatisticsFailure<L> {
    pub fn failure(&self) -> &statistics::SourceGainStatisticsFailure<L> { &self.failure }
    pub fn rows(&self) -> &[log_rows::SourceGainLogRowReceipt<L>] { &self.rows }
}
type InitializedGainOwner<L> = BatchedGainOwner<SourceCount, SourceInitializedGainLogRequest<L>, SourceGainLogContext<L>,
    SourceInitializedStatisticsFailure<L>, (SourceSlewMath, SourceFinalGainMath), SourceSlewFinalGainFailure,
    SourceWindowStorage, SourceInitializedWindowEnergyRowsFailure<L>>;
pub enum SourceInitializedLoggedGainError<'a, L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    Filter(&'a SourceFilterGainFailure), Count(&'a count_math::SourceGainCountFailure),
    Rows(&'a SourceInitializedWindowEnergyRowsFailure<L>), Statistics(&'a SourceInitializedStatisticsFailure<L>),
    Finish(&'a SourceSlewFinalGainFailure),
}
pub struct SourceInitializedLoggedGainFailure<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    owner: InitializedGainOwner<L>,
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceInitializedLoggedGainFailure<L> {
    pub fn error(&self) -> SourceInitializedLoggedGainError<'_, L> { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => SourceInitializedLoggedGainError::Filter(failure),
        BatchedGainOwner::Count { failure, .. } => SourceInitializedLoggedGainError::Count(failure),
        BatchedGainOwner::Windows { failure, .. } => SourceInitializedLoggedGainError::Rows(failure),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => SourceInitializedLoggedGainError::Statistics(failure),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => SourceInitializedLoggedGainError::Finish(failure),
    } }
    pub fn input(&self) -> &[f32] { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.input(),
        BatchedGainOwner::Count { input, .. } | BatchedGainOwner::Windows { input, .. } | BatchedGainOwner::Suffix { input, .. } => input,
    } }
    pub fn target_db(&self) -> f32 { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.target_db(),
        BatchedGainOwner::Count { target_db, .. } | BatchedGainOwner::Windows { target_db, .. }
        | BatchedGainOwner::Suffix { target_db, .. } => *target_db,
    } }
    pub fn filter_math(&self) -> &SourceFilterMath { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.math(),
        BatchedGainOwner::Count { filter_math, .. } | BatchedGainOwner::Windows { filter_math, .. }
        | BatchedGainOwner::Suffix { filter_math, .. } => filter_math,
    } }
    pub fn filter_progress(&self) -> Option<&SourceFilterProgress> { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.progress(),
        BatchedGainOwner::Count { filter_progress, .. } | BatchedGainOwner::Windows { filter_progress, .. }
        | BatchedGainOwner::Suffix { filter_progress, .. } => Some(filter_progress),
    } }
    pub fn count_math(&self) -> &count_math::SourceGainCountMath { match &self.owner {
        BatchedGainOwner::Filter { count, .. } => &count.0,
        BatchedGainOwner::Count { failure, .. } => failure.math(),
        BatchedGainOwner::Windows { count, .. } | BatchedGainOwner::Suffix { count, .. } => &count.math,
    } }
    pub fn count_parts(&self) -> Option<&count_math::SourceGainCountParts> { match &self.owner {
        BatchedGainOwner::Windows { count, .. } | BatchedGainOwner::Suffix { count, .. } => Some(count), _ => None,
    } }
    pub fn window_math(&self) -> &crate::frontend_energy_window_math::SourceWindowMath { match &self.owner {
        BatchedGainOwner::Filter { window_math, .. } | BatchedGainOwner::Count { window_math, .. } => window_math,
        BatchedGainOwner::Windows { failure, .. } => failure.window_math(), BatchedGainOwner::Suffix { rows, .. } => rows.window_math(),
    } }
    pub fn energy_math(&self) -> &SourceEnergyMath { match &self.owner {
        BatchedGainOwner::Filter { energy_math, .. } | BatchedGainOwner::Count { energy_math, .. } => energy_math,
        BatchedGainOwner::Windows { failure, .. } => failure.math(), BatchedGainOwner::Suffix { rows, .. } => rows.math(),
    } }
    pub fn unattempted_log(&self) -> Option<&SourceInitializedGainLogRequest<L>> { match &self.owner {
        BatchedGainOwner::Filter { log, .. } | BatchedGainOwner::Count { log, .. } => Some(log), _ => None,
    } }
    pub fn unused_window_storage(&self) -> Option<&SourceWindowStorage> { match &self.owner {
        BatchedGainOwner::Filter { storage, .. } | BatchedGainOwner::Count { storage, .. } => Some(storage), _ => None,
    } }
    pub fn batch_failure(&self) -> Option<&SourceInitializedWindowEnergyRowsFailure<L>> { match &self.owner {
        BatchedGainOwner::Windows { failure, .. } => Some(failure), _ => None,
    } }
    pub fn window_energy_rows(&self) -> Option<&SourceWindowEnergyRowsCompleted> { match &self.owner {
        BatchedGainOwner::Suffix { rows, .. } => Some(rows), _ => None,
    } }
    pub fn statistics_failure(&self) -> Option<&SourceInitializedStatisticsFailure<L>> { match &self.owner {
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure), _ => None,
    } }
    pub fn log_context(&self) -> Option<&SourceGainLogContext<L>> { match &self.owner {
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { log, .. }, .. } => Some(log), _ => None,
    } }
    pub fn slew_math(&self) -> &SourceSlewMath { match &self.owner {
        BatchedGainOwner::Filter { slew_math, .. } | BatchedGainOwner::Count { slew_math, .. }
        | BatchedGainOwner::Windows { slew_math, .. } => &slew_math.0,
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { owner, .. }, .. } => &owner.0,
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => failure.slew_math(),
    } }
    pub fn final_gain_math(&self) -> &SourceFinalGainMath { match &self.owner {
        BatchedGainOwner::Filter { slew_math, .. } | BatchedGainOwner::Count { slew_math, .. }
        | BatchedGainOwner::Windows { slew_math, .. } => &slew_math.1,
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { owner, .. }, .. } => &owner.1,
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => failure.final_gain_math(),
    } }
    pub fn first_filter(&self) -> &[f32] { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.first_filter(), BatchedGainOwner::Count { outputs, .. } => &outputs.first,
        BatchedGainOwner::Windows { first_filter, .. } => first_filter,
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { first_filter, .. } | LogSuffixFailure::Slew { first_filter, .. }, .. } => first_filter,
    } }
    pub fn second_filter(&self) -> &[f32] { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.second_filter(), BatchedGainOwner::Count { outputs, .. } => &outputs.second,
        BatchedGainOwner::Windows { failure, .. } => failure.source(),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { second_filter, .. } | LogSuffixFailure::Slew { second_filter, .. }, .. } => second_filter,
    } }
    pub fn energy(&self) -> Option<&[f32]> { match &self.owner {
        BatchedGainOwner::Filter { failure, .. } => failure.energy(), BatchedGainOwner::Count { .. } => None,
        BatchedGainOwner::Windows { failure, .. } => failure.energy(),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure.failure.energy()),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { energy, .. }, .. } => Some(energy),
    } }
    pub fn metric_log(&self) -> Option<&[f32]> { match &self.owner {
        BatchedGainOwner::Windows { failure, .. } => failure.metric(),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Log { failure, .. }, .. } => Some(failure.failure.metric_log()),
        BatchedGainOwner::Suffix { failure: LogSuffixFailure::Slew { failure, .. }, .. } => Some(failure.metric_log()), _ => None,
    } }
}

pub fn frontend_gain_with_initialized_source_logs<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(
    input: Vec<f32>, target_db: f32, request: SourceInitializedLoggedGainRequest<L>)
    -> std::result::Result<SourceLoggedGainCompleted<L>, SourceInitializedLoggedGainFailure<L>>
{
    let SourceInitializedLoggedGainRequest { filter_math, outputs, count_math, window_math, energy_math, windows,
        log, slew_math, final_gain_math } = request;
    constructed_gain_with_batch_and_admission(input, target_db, filter_math, outputs, SourceCount(count_math), window_math, energy_math,
        windows, log, (slew_math, final_gain_math),
        |source, source_count, windows, expected, window, energy, log| {
            let SourceInitializedGainLogRequest { statistics: request, gated_math, suffix_math: math, services } = log;
            let logs = energy_windows::InitializedRowLogs { request, gated_math, math, services, history: Vec::new() };
            energy_windows::initialize_construct_and_reduce_windows(source, source_count, windows, expected, window, energy, logs)
                .map(|energy_windows::InitializedConstructedRows { energy, metric, source, rows, logs }|
                    GainBatchCompleted { energy, source, rows, normal: PreparedInitializedLog { metric, logs } })
        }, |energy, PreparedInitializedLog { metric, logs }| {
            let energy_windows::InitializedRowLogs { request, gated_math, math, services, history } = logs;
            match statistics::run_gain_statistics(energy, metric, request, gated_math, math, services) {
                Ok(completed) => Ok(completed_gain_log(completed.into_parts(), history)),
                Err(failure) => Err(SourceInitializedStatisticsFailure { failure, rows: history }),
            }
        }, |metric_log, target, (slew, final_math)| slew_math::gain_slew_from_log_with_source_final_gain_bits(metric_log, target, slew, final_math)
            .map(|((rounds, gain_db), slew, final_math, metric_log)| SlewCompleted { rounds, gain_db, owner: (slew, final_math), metric_log }),
        filter_math::FilterInputAdmission::SourceFinite)
        .map(|(stages, filter_math, count, rows, log, (slew_math, final_gain_math), input)| SourceLoggedGainCompleted {
            parts: SourceLoggedGainParts { input, stages, context: SourceLoggedGainContext { filter_math, count, rows, log, slew_math, final_gain_math } } })
        .map_err(|owner| SourceInitializedLoggedGainFailure { owner })
}

fn host_gain_filters(src: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let [(b0, a0), (b1, a1)] = gain_filter_coefficients();
    let first = filter_math::host_biquad(src, b0, a0);
    let second = filter_math::host_biquad(&first, b1, a1);
    (first, second)
}

fn validate_gain_input(src: &[f32]) -> Result<()> {
    if src.len() <= 1920 || src.len() > i32::MAX as usize || src.iter().any(|x| !x.is_finite()) {
        return Err("gain input requires finite stream longer than1920 samples".into());
    }
    Ok(())
}
fn gain_filter_coefficients() -> [([f32; 3], [f32; 3]); 2] {
    let b0 = [
        f32::from_bits(0x3fc47ef9),
        f32::from_bits(0xc02c44c0),
        f32::from_bits(0x3f9964f0),
    ];
    let a0 = [1.0, f32::from_bits(0xbfd86786), f32::from_bits(0x3f3b83dc)];
    [(b0, a0), ([1.0, -2.0, 1.0], [1.0, f32::from_bits(0xbffeb9e0), f32::from_bits(0x3f7d7560)])]
}
struct GainSuffixFailure {
    error: String, first_filter: Vec<f32>, second_filter: Vec<f32>, energy: Vec<f32>, metric_log: Option<Vec<f32>>,
}
fn gain_table_suffix(sample_count: usize, target_db: f32, first_filter: Vec<f32>, second_filter: Vec<f32>)
    -> std::result::Result<FrontendGainStages, GainSuffixFailure>
{
    match gain_table_suffix_with(sample_count, target_db, first_filter, second_filter, (), |metric_log, target, ()| {
        match gain_slew_from_log(&metric_log, target) {
            Ok((rounds, gain_db)) => Ok(SlewCompleted { rounds, gain_db, owner: (), metric_log }),
            Err(error) => Err(HostSlewFailure { error, metric_log }),
        }
    }) {
        Ok((stages, ())) => Ok(stages),
        Err(GainSuffixChoiceFailure::Before { failure, owner: () }) => Err(failure),
        Err(GainSuffixChoiceFailure::Slew { first_filter, second_filter, energy, failure }) =>
            Err(GainSuffixFailure { error: failure.error, first_filter, second_filter, energy, metric_log: Some(failure.metric_log) }),
    }
}
struct HostSlewFailure { error: String, metric_log: Vec<f32> }
struct SlewCompleted<P> { rounds: Vec<Vec<f32>>, gain_db: Vec<f32>, owner: P, metric_log: Vec<f32> }
enum GainSuffixChoiceFailure<P, E> {
    Before { failure: GainSuffixFailure, owner: P },
    Slew { first_filter: Vec<f32>, second_filter: Vec<f32>, energy: Vec<f32>, failure: E },
}
fn gain_table_suffix_with<P, E>(sample_count: usize, target_db: f32, first_filter: Vec<f32>, second_filter: Vec<f32>, owner: P,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, P), GainSuffixChoiceFailure<P, E>>
{
    let energy = host_energy_rows(sample_count, &second_filter);
    gain_table_after_energy(target_db, first_filter, second_filter, energy, owner, slew)
}
fn gain_energy_count(sample_count: usize) -> usize {
    (((sample_count as i32 - 1920) as f32 / 480.0_f32) as i32).max(1) as usize
}
fn host_energy_rows(sample_count: usize, second_filter: &[f32]) -> Vec<f32> {
    let count = gain_energy_count(sample_count);
    let mut energy = Vec::with_capacity(count);
    for row in 0..count {
        let time = (row + 2) as f32 * 0.01_f32;
        let start = (time * 48000.0_f32) as i32 - 960;
        let mut window = [0.0_f32; 1920];
        for (j, value) in window.iter_mut().enumerate() {
            let index = start + j as i32;
            *value = if index >= 0 && (index as usize) < sample_count {
                second_filter.get(index as usize).copied().unwrap_or(0.0)
            } else {
                0.0
            };
        }

        let bits = match energy_reduction_math::reduce_host_sum_left_seed_zero_divisor1920(&window) {
            Ok(bits) => bits, Err(_) => unreachable!(),
        };
        energy.push(f32::from_bits(bits));
    }
    energy
}
fn gain_table_after_energy<P, E>(target_db: f32, first_filter: Vec<f32>, second_filter: Vec<f32>, energy: Vec<f32>, owner: P,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, P), GainSuffixChoiceFailure<P, E>>
{
    gain_table_after_energy_with_log(target_db, first_filter, second_filter, energy, (), owner, host_gain_log, slew)
        .map(|(stages, (), owner)| (stages, owner)).map_err(host_log_suffix_failure)
}
struct LogCompleted<O> { energy: Vec<f32>, metric_log: Vec<f32>, owner: O }
struct HostGainLogFailure { energy: Vec<f32>, error: String }
fn host_gain_log(energy: Vec<f32>, (): ()) -> std::result::Result<LogCompleted<()>, HostGainLogFailure> {
    match gain_log_from_energy(&energy) {
        Ok(metric_log) => Ok(LogCompleted { energy, metric_log, owner: () }),
        Err(error) => Err(HostGainLogFailure { energy, error }),
    }
}
enum LogSuffixFailure<LO, LE, P, E> {
    Log { first_filter: Vec<f32>, second_filter: Vec<f32>, failure: LE, owner: P },
    Slew { first_filter: Vec<f32>, second_filter: Vec<f32>, energy: Vec<f32>, log: LO, failure: E },
}
fn host_log_suffix_failure<P, E>(failure: LogSuffixFailure<(), HostGainLogFailure, P, E>) -> GainSuffixChoiceFailure<P, E> {
    match failure {
        LogSuffixFailure::Log { first_filter, second_filter, failure: HostGainLogFailure { energy, error }, owner } =>
            GainSuffixChoiceFailure::Before { failure: GainSuffixFailure { error, first_filter, second_filter, energy, metric_log: None }, owner },
        LogSuffixFailure::Slew { first_filter, second_filter, energy, log: (), failure } =>
            GainSuffixChoiceFailure::Slew { first_filter, second_filter, energy, failure },
    }
}
fn gain_table_after_energy_with_log<LI, LO, LE, P, E>(target_db: f32, first_filter: Vec<f32>, second_filter: Vec<f32>,
    energy: Vec<f32>, initial: LI, owner: P,
    logarithm: impl FnOnce(Vec<f32>, LI) -> std::result::Result<LogCompleted<LO>, LE>,
    slew: impl FnOnce(Vec<f32>, f32, P) -> std::result::Result<SlewCompleted<P>, E>)
    -> std::result::Result<(FrontendGainStages, LO, P), LogSuffixFailure<LO, LE, P, E>>
{
    let LogCompleted { energy, metric_log, owner: log } = match logarithm(energy, initial) {
        Ok(completed) => completed,
        Err(failure) => return Err(LogSuffixFailure::Log { first_filter, second_filter, failure, owner }),
    };
    let completed = match slew(metric_log, target_db, owner) {
        Ok(result) => result,
        Err(failure) => return Err(LogSuffixFailure::Slew { first_filter, second_filter, energy, log, failure }),
    };
    Ok((FrontendGainStages {
        first_filter,
        second_filter,
        energy,
        metric_log: completed.metric_log,
        slew_rounds: completed.rounds,
        gain_db: completed.gain_db,
    }, log, completed.owner))
}

pub fn apply_gain_windows(src: &[f32], gain: &[f32]) -> Result<Vec<f32>> {
    validate_gain_application(src, gain)?;
    Ok(application::host_application(src, gain))
}
fn validate_gain_application(src: &[f32], gain: &[f32]) -> Result<()> {
    if gain.is_empty()
        || src.len() > i32::MAX as usize
        || src.iter().chain(gain.iter()).any(|v| !v.is_finite())
    {
        return Err("gain application requires finite source/nonempty gain".into());
    }
    Ok(())
}

pub fn peak_normalize09(src: &[f32]) -> Result<Vec<f32>> {
    if src.iter().any(|v| !v.is_finite()) {
        return Err("peak normalization requires finite source".into());
    }
    Ok(peak_math::host_peak(src))
}
