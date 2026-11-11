use std::collections::TryReserveError;
use crate::conditional_gate::Batch;

pub(crate) enum ArchiveOutput<'a> {
    Published,
    Reset(&'a mut Option<Batch>),
}
pub(crate) enum Exit { Published, Reset }
pub(crate) enum ArchiveError {
    MissingActive,
    OwnerMismatch { active_call: bool, output: bool },
    Capacity(TryReserveError),
}

pub(crate) fn archive_current<D, R>(active: &mut Option<D>, retired: &mut Vec<R>,
    output: ArchiveOutput<'_>, into_retired: impl FnOnce(D, Option<Batch>, Exit) -> R)
    -> Result<(), ArchiveError>
{
    match &output {
        ArchiveOutput::Published => {
            if active.is_none() { return Err(ArchiveError::MissingActive); }
        }
        ArchiveOutput::Reset(output) => {
            let active_call = active.is_some();
            let output = output.is_some();
            if active_call != output { return Err(ArchiveError::OwnerMismatch { active_call, output }); }
            if !active_call { return Ok(()); }
        }
    }
    retired.try_reserve(1).map_err(ArchiveError::Capacity)?;
    let active = active.take().expect("validated active owner after successful reserve");
    let (output, exit) = match output {
        ArchiveOutput::Published => (None, Exit::Published),
        ArchiveOutput::Reset(output) => (output.take(), Exit::Reset),
    };
    retired.push(into_retired(active, output, exit));
    Ok(())
}
