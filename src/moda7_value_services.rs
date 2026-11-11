use std::collections::{TryReserveError, VecDeque};
use crate::{conditional_gate::Batch, moda7_projection::{PublishRefusal, RequiredServices},
    modl0_value_services::{HistoryAt, LifecyclePolicy},
    value_call_owners as owners, value_row_allocation::{self as allocation, BackingPolicy},
    vector_f32::{self, Backing, ReciprocalPolicy}};

pub struct RowWorkspace {
    pub input: Backing,
    pub destination: Backing,
}
pub struct CallRecord {
    pub output: Batch,
    pub rows: Vec<RowWorkspace>,
}
#[derive(Clone, Copy)]
pub struct AllocationPolicy {
    pub input: BackingPolicy, pub destination: BackingPolicy,
    pub output_word: u32, pub initial_output_status: u8,
}

pub enum Calls { Supplied(VecDeque<CallRecord>), Allocate(AllocationPolicy) }
pub struct Parts {
    pub calls: Calls,
    pub reciprocal: ReciprocalPolicy,
    pub lifecycle: LifecyclePolicy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Acquired, Need152, Need112, NeedTruncate, AwaitPublish }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopKind { Protocol, Exhausted, Preparation, Shape, Slot152, Slot112, Truncate, Publication, Reset }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoppedAt {
    pub kind: StopKind,
    pub phase: Phase,
    pub call: Option<usize>,
    pub next_row: usize,
    pub removed_rows: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    pub issued: usize,
    pub call: Option<usize>,
    pub count: Option<usize>,
    pub width: Option<usize>,
    pub next_row: usize,
    pub removed_rows: usize,
    pub stopped: Option<StoppedAt>,
    pub reset_stopped: Option<StoppedAt>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackRole { Input, Output }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationPart { OutputSlots, WorkspaceSlots, Input, Destination, LogicalOutput }
pub enum Error {
    PriorFailure,
    PriorResetFailure,
    Phase { expected: Phase, actual: Phase },
    Exhausted { issued: usize },
    CounterOverflow,
    AllocationState,
    Allocation { row: Option<usize>, part: AllocationPart, cause: allocation::Error },
    MissingActive,
    NonemptyPrepareRequired,
    MissingPreparedCount,
    MissingPreparedWidth,
    OutputCount { expected: usize, actual: usize },
    OutputWidth { row: usize, expected: usize, actual: usize },
    WorkspaceCount { expected: usize, actual: usize },
    InputWorkspaceWidth { row: usize, expected: usize, actual: usize },
    DestinationWidth { row: usize, expected: usize, actual: usize },
    MissingWorkspace { row: usize },
    RowOrdinal { row: usize, count: usize },
    CallbackWidth { row: usize, role: CallbackRole, expected: usize, actual: usize },
    Sigmoid { row: usize, error: vector_f32::Error },
    Multiply { row: usize, error: vector_f32::Error },
    TruncateCount { expected: usize, actual: usize },
    RemovedHistoryCapacity { ordinal: usize, error: TryReserveError },
    HistoryCapacity { at: HistoryAt, error: TryReserveError },
    ResetOwnerMismatch { active_call: bool, output: bool },
}

pub struct RemovedRow { ordinal: usize, values: Vec<f32> }
impl RemovedRow {
    pub fn ordinal(&self) -> usize { self.ordinal }
    pub fn values(&self) -> &[f32] { &self.values }
}
struct ActiveCall {
    ordinal: usize,
    rows: Vec<RowWorkspace>,
    removed: Vec<RemovedRow>,
    count: Option<usize>,
    width: Option<usize>,
    next_row: usize,
    pending_input: Option<Backing>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit { Published, Reset }

pub struct RetiredCall {
    ordinal: usize,
    exit: Exit,
    phase: Phase,
    count: Option<usize>,
    width: Option<usize>,
    next_row: usize,
    stopped: Option<StoppedAt>,
    rows: Vec<RowWorkspace>,
    removed: Vec<RemovedRow>,
    output: Option<Batch>,
    pending_input: Option<Backing>,
}
impl RetiredCall {
    pub fn ordinal(&self) -> usize { self.ordinal }
    pub fn exit(&self) -> Exit { self.exit }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn count(&self) -> Option<usize> { self.count }
    pub fn width(&self) -> Option<usize> { self.width }
    pub fn next_row(&self) -> usize { self.next_row }
    pub fn stopped(&self) -> Option<StoppedAt> { self.stopped }
    pub fn workspaces(&self) -> &[RowWorkspace] { &self.rows }
    pub fn removed_rows(&self) -> &[RemovedRow] { &self.removed }
    pub fn output(&self) -> Option<&Batch> { self.output.as_ref() }
    pub fn pending_input(&self) -> Option<&Backing> { self.pending_input.as_ref() }
}
pub struct ReadyParts {
    pub calls: Calls,
    pub reciprocal: ReciprocalPolicy,
    pub lifecycle: LifecyclePolicy,
    pub issued: usize,
    pub retired: Vec<RetiredCall>,
}
pub struct ValueServices {
    calls: Calls,
    reciprocal: ReciprocalPolicy,
    lifecycle: LifecyclePolicy,
    issued: usize,
    active: Option<ActiveCall>,
    retired: Vec<RetiredCall>,
    phase: Phase,
    stopped: Option<StoppedAt>,
    reset_stopped: Option<StoppedAt>,
}
impl ValueServices {

    pub fn from_explicit_parts(parts: Parts) -> Self {
        Self { calls: parts.calls, reciprocal: parts.reciprocal, lifecycle: parts.lifecycle,
            issued: 0, active: None, retired: Vec::new(), phase: Phase::Idle,
            stopped: None, reset_stopped: None }
    }
    pub fn progress(&self) -> Progress {
        Progress { phase: self.phase, issued: self.issued,
            call: self.active.as_ref().map(|a| a.ordinal),
            count: self.active.as_ref().and_then(|a| a.count),
            width: self.active.as_ref().and_then(|a| a.width),
            next_row: self.active.as_ref().map(|a| a.next_row).unwrap_or(0),
            removed_rows: self.active.as_ref().map(|a| a.removed.len()).unwrap_or(0),
            stopped: self.stopped, reset_stopped: self.reset_stopped }
    }
    pub fn reciprocal_policy(&self) -> ReciprocalPolicy { self.reciprocal }
    pub fn unissued_calls(&self) -> Option<&VecDeque<CallRecord>> {
        match &self.calls { Calls::Supplied(calls) => Some(calls), Calls::Allocate(_) => None }
    }
    pub fn allocation_policy(&self) -> Option<AllocationPolicy> {
        match &self.calls { Calls::Allocate(policy) => Some(*policy), Calls::Supplied(_) => None }
    }
    pub fn active_workspaces(&self) -> Option<&[RowWorkspace]> {
        self.active.as_ref().map(|a| a.rows.as_slice())
    }
    pub fn active_pending_input(&self) -> Option<&Backing> {
        self.active.as_ref().and_then(|a| a.pending_input.as_ref())
    }
    pub fn removed_rows(&self) -> Option<&[RemovedRow]> {
        self.active.as_ref().map(|a| a.removed.as_slice())
    }
    pub fn retired_calls(&self) -> &[RetiredCall] { &self.retired }
    pub fn into_ready_parts(self) -> Result<ReadyParts, Self> {
        if self.phase != Phase::Idle || self.active.is_some()
            || self.stopped.is_some() || self.reset_stopped.is_some() { return Err(self); }
        Ok(ReadyParts { calls: self.calls, reciprocal: self.reciprocal, lifecycle: self.lifecycle,
            issued: self.issued, retired: self.retired })
    }
    fn stopped_at(&self, kind: StopKind) -> StoppedAt {
        StoppedAt { kind, phase: self.phase, call: self.active.as_ref().map(|a| a.ordinal),
            next_row: self.active.as_ref().map(|a| a.next_row).unwrap_or(0),
            removed_rows: self.active.as_ref().map(|a| a.removed.len()).unwrap_or(0) }
    }
    fn fail(&mut self, kind: StopKind, error: Error) -> Error {
        if self.stopped.is_none() { self.stopped = Some(self.stopped_at(kind)); }
        error
    }
    fn fail_reset(&mut self, error: Error) -> Error {
        if self.reset_stopped.is_none() { self.reset_stopped = Some(self.stopped_at(StopKind::Reset)); }
        error
    }
    fn running(&self) -> Result<(), Error> {
        if self.reset_stopped.is_some() { return Err(Error::PriorResetFailure); }
        if self.stopped.is_some() { return Err(Error::PriorFailure); }
        Ok(())
    }
    fn require_phase(&mut self, expected: Phase) -> Result<(), Error> {
        self.running()?;
        if self.phase != expected {
            return Err(self.fail(StopKind::Protocol, Error::Phase { expected, actual: self.phase }));
        }
        if expected != Phase::Idle && self.active.is_none() {
            return Err(self.fail(StopKind::Protocol, Error::MissingActive));
        }
        Ok(())
    }
    fn current_count(&mut self) -> Result<usize, Error> {
        match self.active.as_ref().and_then(|a| a.count) {
            Some(count) => Ok(count),
            None => Err(self.fail(StopKind::Protocol, Error::MissingPreparedCount)),
        }
    }
    fn current_width(&mut self) -> Result<usize, Error> {
        match self.active.as_ref().and_then(|a| a.width) {
            Some(width) => Ok(width),
            None => Err(self.fail(StopKind::Protocol, Error::MissingPreparedWidth)),
        }
    }
    fn current_row(&mut self, input: &[f32], output: &[f32]) -> Result<(usize, usize), Error> {
        let count = self.current_count()?;
        let width = self.current_width()?;
        let row = self.active.as_ref().expect("phase checked active row").next_row;
        if row >= count { return Err(self.fail(StopKind::Protocol, Error::RowOrdinal { row, count })); }
        if input.len() != width {
            return Err(self.fail(StopKind::Shape, Error::CallbackWidth {
                row, role: CallbackRole::Input, expected: width, actual: input.len() }));
        }
        if output.len() != width {
            return Err(self.fail(StopKind::Shape, Error::CallbackWidth {
                row, role: CallbackRole::Output, expected: width, actual: output.len() }));
        }
        Ok((row, width))
    }
    fn prepare_allocated(&mut self, output: &mut Batch, count: usize, width: usize, policy: AllocationPolicy)
        -> Result<(), Error>
    {
        let active = self.active.as_ref().expect("phase checked allocated call");
        if !output.rows.is_empty() || !active.rows.is_empty() || active.pending_input.is_some() {
            return Err(self.fail(StopKind::Protocol, Error::AllocationState));
        }
        output.rows.try_reserve_exact(count).map_err(|cause| self.fail(StopKind::Preparation,
            Error::Allocation { row: None, part: AllocationPart::OutputSlots, cause: allocation::Error::Capacity(cause) }))?;
        self.active.as_mut().expect("retained allocated call").rows.try_reserve_exact(count)
            .map_err(|cause| self.fail(StopKind::Preparation, Error::Allocation {
                row: None, part: AllocationPart::WorkspaceSlots, cause: allocation::Error::Capacity(cause) }))?;
        for row in 0..count {
            let input = allocation::allocate_backing(width, policy.input).map_err(|cause|
                self.fail(StopKind::Preparation, Error::Allocation { row: Some(row), part: AllocationPart::Input, cause }))?;

            self.active.as_mut().expect("retained allocated call").pending_input = Some(input);
            let destination = allocation::allocate_backing(width, policy.destination).map_err(|cause|
                self.fail(StopKind::Preparation, Error::Allocation { row: Some(row), part: AllocationPart::Destination, cause }))?;
            let active = self.active.as_mut().expect("retained allocated call");
            let input = active.pending_input.take().expect("same allocated input backing");
            active.rows.push(RowWorkspace { input, destination });
            let values = allocation::allocate_logical_row(width, policy.output_word).map_err(|cause|
                self.fail(StopKind::Preparation, Error::Allocation { row: Some(row), part: AllocationPart::LogicalOutput, cause: cause.into() }))?;
            output.rows.push(values);
        }
        Ok(())
    }
    fn prefix_shape(output: &Batch, count: usize, width: usize) -> Result<(), Error> {
        if output.rows.len() < count {
            return Err(Error::OutputCount { expected: count, actual: output.rows.len() });
        }
        for (row, values) in output.rows[..count].iter().enumerate() {
            if values.len() != width {
                return Err(Error::OutputWidth { row, expected: width, actual: values.len() });
            }
        }
        Ok(())
    }

    fn archive(active: ActiveCall, output: Option<Batch>, exit: owners::Exit,
        phase: Phase, stopped: Option<StoppedAt>) -> RetiredCall {
        RetiredCall { ordinal: active.ordinal,
            exit: match exit { owners::Exit::Published => Exit::Published, owners::Exit::Reset => Exit::Reset },
            phase, count: active.count, width: active.width, next_row: active.next_row,
            stopped, rows: active.rows, removed: active.removed, output, pending_input: active.pending_input }
    }
}

impl RequiredServices for ValueServices {
    type Error = Error;
    fn acquire_output(&mut self) -> Result<Batch, Error> {
        self.require_phase(Phase::Idle)?;
        let next = self.issued.checked_add(1)
            .ok_or_else(|| self.fail(StopKind::Protocol, Error::CounterOverflow))?;
        let record = match &mut self.calls {
            Calls::Supplied(calls) => calls.pop_front(),
            Calls::Allocate(policy) => Some(CallRecord {
                output: Batch { rows: Vec::new(), status: policy.initial_output_status }, rows: Vec::new(),
            }),
        };
        let Some(CallRecord { output, rows }) = record else {
            return Err(self.fail(StopKind::Exhausted, Error::Exhausted { issued: self.issued }));
        };
        let ordinal = self.issued;
        self.issued = next;
        self.active = Some(ActiveCall { ordinal, rows, removed: Vec::new(), count: None,
            width: None, next_row: 0, pending_input: None });
        self.phase = Phase::Acquired;
        Ok(output)
    }
    fn prepare_output(&mut self, output: &mut Batch, count: usize, width: usize) -> Result<(), Error> {
        self.require_phase(Phase::Acquired)?;
        if count == 0 { return Err(self.fail(StopKind::Protocol, Error::NonemptyPrepareRequired)); }
        let active = self.active.as_mut().expect("phase checked actual call");
        active.count = Some(count);
        active.width = Some(width);
        if let Some(policy) = self.allocation_policy() { self.prepare_allocated(output, count, width, policy)?; }
        if output.rows.len() != count {
            return Err(self.fail(StopKind::Shape, Error::OutputCount { expected: count, actual: output.rows.len() }));
        }
        if let Err(error) = Self::prefix_shape(output, count, width) { return Err(self.fail(StopKind::Shape, error)); }
        let rows = &self.active.as_ref().expect("phase checked actual call").rows;
        if rows.len() != count {
            let actual = rows.len();
            return Err(self.fail(StopKind::Shape, Error::WorkspaceCount { expected: count, actual }));
        }
        let wrong_width = rows.iter().enumerate().find_map(|(row, workspace)| {
            if workspace.input.logical_width() != width {
                Some(Error::InputWorkspaceWidth { row, expected: width, actual: workspace.input.logical_width() })
            } else if workspace.destination.logical_width() != width {
                Some(Error::DestinationWidth { row, expected: width, actual: workspace.destination.logical_width() })
            } else { None }
        });
        if let Some(error) = wrong_width { return Err(self.fail(StopKind::Shape, error)); }

        self.phase = Phase::Need152;
        Ok(())
    }
    fn input_slot152_to_destination(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), Error> {
        self.require_phase(Phase::Need152)?;
        let (row, width) = self.current_row(input, output)?;
        let reciprocal = self.reciprocal;
        let result = match self.active.as_mut().expect("phase checked active row").rows.get_mut(row) {
            Some(workspace) => {

                for (word, value) in workspace.input.full_bits_mut()[..width].iter_mut().zip(input) {
                    *word = value.to_bits();
                }
                workspace.input.sigmoid_into(&mut workspace.destination, reciprocal)
                    .map_err(|error| Error::Sigmoid { row, error })
            }
            None => Err(Error::MissingWorkspace { row }),
        };
        if let Err(error) = result { return Err(self.fail(StopKind::Slot152, error)); }
        let workspace = &self.active.as_ref().expect("retained successful sigmoid row").rows[row];
        for (value, &bits) in output.iter_mut().zip(workspace.destination.logical_bits()) { *value = f32::from_bits(bits); }
        self.phase = Phase::Need112;
        Ok(())
    }
    fn input_slot112_to_destination(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), Error> {
        self.require_phase(Phase::Need112)?;

        let (row, _) = self.current_row(input, output)?;
        let count = self.current_count()?;
        let result = match self.active.as_mut().expect("phase checked active row").rows.get_mut(row) {
            Some(workspace) => workspace.input.multiply_into(&mut workspace.destination)
                .map_err(|error| Error::Multiply { row, error }),
            None => Err(Error::MissingWorkspace { row }),
        };
        if let Err(error) = result { return Err(self.fail(StopKind::Slot112, error)); }
        let active = self.active.as_mut().expect("retained successful multiplied row");
        for (value, &bits) in output.iter_mut().zip(active.rows[row].destination.logical_bits()) { *value = f32::from_bits(bits); }
        active.next_row += 1;
        self.phase = if active.next_row == count { Phase::NeedTruncate } else { Phase::Need152 };
        Ok(())
    }
    fn truncate_output(&mut self, output: &mut Batch, count: usize) -> Result<(), Error> {
        if count == 0 {

            self.require_phase(Phase::Acquired)?;
            self.active.as_mut().expect("phase checked acquired call").count = Some(0);
        } else {
            self.require_phase(Phase::NeedTruncate)?;
            let expected = self.current_count()?;
            if count != expected {
                return Err(self.fail(StopKind::Protocol, Error::TruncateCount { expected, actual: count }));
            }
            let width = self.current_width()?;

            if let Err(error) = Self::prefix_shape(output, count, width) { return Err(self.fail(StopKind::Shape, error)); }
        }
        while output.rows.len() > count {
            let ordinal = output.rows.len() - 1;
            let reserved = self.active.as_mut().expect("retained truncating call").removed.try_reserve(1);
            if let Err(error) = reserved {
                return Err(self.fail(StopKind::Truncate, Error::RemovedHistoryCapacity { ordinal, error }));
            }
            let values = output.rows.pop().expect("current back exists after successful reserve");
            self.active.as_mut().expect("retained truncating call").removed.push(RemovedRow { ordinal, values });
        }

        self.phase = Phase::AwaitPublish;
        Ok(())
    }
    fn publish_output(&mut self, output: Batch) -> Result<Batch, PublishRefusal<Error>> {
        if let Err(reason) = self.require_phase(Phase::AwaitPublish) { return Err(PublishRefusal { reason, output }); }
        let count = match self.current_count() { Ok(count) => count, Err(reason) => return Err(PublishRefusal { reason, output }) };
        if output.rows.len() != count {
            let reason = self.fail(StopKind::Shape, Error::OutputCount { expected: count, actual: output.rows.len() });
            return Err(PublishRefusal { reason, output });
        }
        if count != 0 {
            let width = match self.current_width() { Ok(width) => width, Err(reason) => return Err(PublishRefusal { reason, output }) };
            if let Err(error) = Self::prefix_shape(&output, count, width) {
                let reason = self.fail(StopKind::Shape, error);
                return Err(PublishRefusal { reason, output });
            }
        }
        let phase = self.phase;
        let stopped = self.stopped;
        let result = owners::archive_current(&mut self.active, &mut self.retired, owners::ArchiveOutput::Published,
            move |active, output, exit| Self::archive(active, output, exit, phase, stopped));
        if let Err(error) = result {
            let reason = match error {
                owners::ArchiveError::Capacity(error) => self.fail(StopKind::Publication, Error::HistoryCapacity { at: HistoryAt::Publication, error }),
                owners::ArchiveError::MissingActive => self.fail(StopKind::Protocol, Error::MissingActive),
                owners::ArchiveError::OwnerMismatch { active_call, output } => self.fail(StopKind::Protocol, Error::ResetOwnerMismatch { active_call, output }),
            };
            return Err(PublishRefusal { reason, output });
        }
        self.phase = Phase::Idle;
        Ok(output)
    }
    fn reset(&mut self, active_output: &mut Option<Batch>) -> Result<(), Error> {
        if self.reset_stopped.is_some() { return Err(Error::PriorResetFailure); }
        match self.lifecycle { LifecyclePolicy::ArchiveIssuedCalls => {} }
        let phase = self.phase;
        let stopped = self.stopped;
        let result = owners::archive_current(&mut self.active, &mut self.retired, owners::ArchiveOutput::Reset(active_output),
            move |active, output, exit| Self::archive(active, output, exit, phase, stopped));
        if let Err(error) = result {
            let error = match error {
                owners::ArchiveError::Capacity(error) => Error::HistoryCapacity { at: HistoryAt::Reset, error },
                owners::ArchiveError::MissingActive => Error::MissingActive,
                owners::ArchiveError::OwnerMismatch { active_call, output } => Error::ResetOwnerMismatch { active_call, output },
            };
            return Err(self.fail_reset(error));
        }

        self.stopped = None;
        self.phase = Phase::Idle;
        Ok(())
    }
}
