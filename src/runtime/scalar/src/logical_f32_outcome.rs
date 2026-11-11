use super::finite_power::{ExplicitExceptionState, PowerError};
use super::shared_scalar_f32::{ScalarF32ComparisonFlags, ScalarF32ComparisonOutcome,
    ScalarF32Mxcsr, ScalarF32Outcome, XmmF32Outcome,
    ScalarF32SelectionOutcome, ScalarF32SelectionRequired};

#[derive(Clone, Copy, Debug)]
pub enum SourceScalarF32Projection {
    Completed { bits: u32, advanced: ExplicitExceptionState },
    Trapped { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
    ReportedStateRequired { reported: ScalarF32Outcome, reason: PowerError,
        known_advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub enum SourceXmmF32Projection {
    Completed { bits: [u32; 4], advanced: ExplicitExceptionState },
    Trapped { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
    ReportedStateRequired { reported: XmmF32Outcome, reason: PowerError,
        known_advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub enum SourceComparisonF32Projection {
    Completed { flags: ScalarF32ComparisonFlags, advanced: ExplicitExceptionState },

    Trapped { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
    ReportedStateRequired { reported: ScalarF32ComparisonOutcome, reason: PowerError,
        known_advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub enum SourceSelectionF32Projection {
    Completed { bits: u32, advanced: ExplicitExceptionState },
    Trapped { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },

    Required { required: ScalarF32SelectionRequired, advanced: ExplicitExceptionState },
    ReportedStateRequired { reported: ScalarF32SelectionOutcome, reason: PowerError,
        known_advanced: ExplicitExceptionState },
}

fn admit(entry: ExplicitExceptionState, reported: ScalarF32Mxcsr)
    -> Result<ExplicitExceptionState, PowerError>
{

    ExplicitExceptionState::from_explicit_parts(reported.word(), entry.errno())
}

pub fn project_scalar_f32_outcome(entry: ExplicitExceptionState, reported: ScalarF32Outcome)
    -> SourceScalarF32Projection
{
    let state = match reported {
        ScalarF32Outcome::Completed { state, .. } | ScalarF32Outcome::Trapped { state, .. } => state,
    };
    let advanced = match admit(entry, state) {
        Ok(advanced) => advanced,
        Err(reason) => return SourceScalarF32Projection::ReportedStateRequired {
            reported, reason, known_advanced: entry },
    };
    match reported {
        ScalarF32Outcome::Completed { bits, .. } => SourceScalarF32Projection::Completed { bits, advanced },
        ScalarF32Outcome::Trapped { unmasked, .. } => SourceScalarF32Projection::Trapped {
            reported_mxcsr: state.word(), unmasked: unmasked.bits(), advanced },
    }
}

pub fn project_xmm_f32_outcome(entry: ExplicitExceptionState, reported: XmmF32Outcome)
    -> SourceXmmF32Projection
{
    let state = match reported {
        XmmF32Outcome::Completed { state, .. } | XmmF32Outcome::Trapped { state, .. } => state,
    };
    let advanced = match admit(entry, state) {
        Ok(advanced) => advanced,
        Err(reason) => return SourceXmmF32Projection::ReportedStateRequired {
            reported, reason, known_advanced: entry },
    };
    match reported {
        XmmF32Outcome::Completed { bits, .. } => SourceXmmF32Projection::Completed { bits, advanced },
        XmmF32Outcome::Trapped { unmasked, .. } => SourceXmmF32Projection::Trapped {
            reported_mxcsr: state.word(), unmasked: unmasked.bits(), advanced },
    }
}

pub fn project_comparison_f32_outcome(entry: ExplicitExceptionState, reported: ScalarF32ComparisonOutcome)
    -> SourceComparisonF32Projection
{
    let state = match reported {
        ScalarF32ComparisonOutcome::Completed { state, .. }
            | ScalarF32ComparisonOutcome::Trapped { state, .. } => state,
    };
    let advanced = match admit(entry, state) {
        Ok(advanced) => advanced,
        Err(reason) => return SourceComparisonF32Projection::ReportedStateRequired {
            reported, reason, known_advanced: entry },
    };
    match reported {
        ScalarF32ComparisonOutcome::Completed { flags, .. } =>
            SourceComparisonF32Projection::Completed { flags, advanced },
        ScalarF32ComparisonOutcome::Trapped { unmasked, .. } => SourceComparisonF32Projection::Trapped {
            reported_mxcsr: state.word(), unmasked: unmasked.bits(), advanced },
    }
}

pub fn project_selection_f32_outcome(entry: ExplicitExceptionState, reported: ScalarF32SelectionOutcome)
    -> SourceSelectionF32Projection
{
    let state = match reported {
        ScalarF32SelectionOutcome::Completed { state, .. }
            | ScalarF32SelectionOutcome::Trapped { state, .. }
            | ScalarF32SelectionOutcome::Required { state, .. } => state,
    };
    let advanced = match admit(entry, state) {
        Ok(advanced) => advanced,
        Err(reason) => return SourceSelectionF32Projection::ReportedStateRequired {
            reported, reason, known_advanced: entry },
    };
    match reported {
        ScalarF32SelectionOutcome::Completed { bits, .. } => SourceSelectionF32Projection::Completed { bits, advanced },
        ScalarF32SelectionOutcome::Trapped { unmasked, .. } => SourceSelectionF32Projection::Trapped {
            reported_mxcsr: state.word(), unmasked: unmasked.bits(), advanced },
        ScalarF32SelectionOutcome::Required { required, .. } => SourceSelectionF32Projection::Required { required, advanced },
    }
}
