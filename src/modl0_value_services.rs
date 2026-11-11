use std::collections::{TryReserveError, VecDeque};
use crate::{conditional_gate::Batch, logical_read_rows::ReadRows,
    modl0_matrix_value::{BatchError, Matrix},
    modl0_projection::{PublishRefusal, RequiredServices}, vector_f32::{self, Backing},
    value_call_owners as owners, value_row_allocation::{self as allocation, BackingPolicy}};

pub struct CallRecord {
    pub output: Batch,
    pub destinations: Vec<Backing>,
}
#[derive(Clone, Copy)]
pub struct AllocationPolicy {
    pub destination: BackingPolicy, pub output_word: u32, pub initial_output_status: u8,
}

pub enum Calls { Supplied(VecDeque<CallRecord>), Allocate(AllocationPolicy) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecyclePolicy { ArchiveIssuedCalls }

pub struct Parts {
    pub matrix: Matrix,
    pub calls: Calls,

    pub additive_post: Option<Backing>,
    pub lifecycle: LifecyclePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Acquired, WidthRead, Prepared, MatrixComplete, PostRows, AwaitPublish }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopKind { Protocol, Exhausted, Preparation, Shape, Matrix, Post, Publication, Reset }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoppedAt {
    pub kind: StopKind,
    pub phase: Phase,
    pub call: Option<usize>,
    pub next_post: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,

    pub issued: usize,
    pub call: Option<usize>,
    pub prepared_count: Option<usize>,
    pub selected_post: Option<bool>,
    pub next_post: usize,
    pub stopped: Option<StoppedAt>,
    pub reset_stopped: Option<StoppedAt>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryAt { Publication, Reset }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationPart { OutputSlots, BackingSlots, Destination, LogicalOutput }
pub enum Error {
    PriorFailure,
    PriorResetFailure,
    Phase { expected: Phase, actual: Phase },
    Exhausted { issued: usize },
    CounterOverflow,
    AllocationState,
    Allocation { row: Option<usize>, part: AllocationPart, cause: allocation::Error },
    MissingActive,
    MissingPreparedCount,
    WidthArgument { expected: usize, actual: usize },
    OutputCount { expected: usize, actual: usize },
    OutputWidth { row: usize, expected: usize, actual: usize },
    BackingCount { expected: usize, actual: usize },
    BackingWidth { row: usize, expected: usize, actual: usize },
    PostOrdinal { expected: usize, actual: usize },
    MissingPost,
    MissingBacking { row: usize },
    Matrix(BatchError),
    Post { row: usize, error: vector_f32::Error },
    HistoryCapacity { at: HistoryAt, error: TryReserveError },
    ResetOwnerMismatch { active_call: bool, output: bool },
}

struct ActiveCall {
    ordinal: usize,
    destinations: Vec<Backing>,
    count: Option<usize>,
    selected_post: Option<bool>,
    next_post: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit { Published, Reset }

pub struct RetiredCall {
    ordinal: usize,
    exit: Exit,
    phase: Phase,
    count: Option<usize>,
    selected_post: Option<bool>,
    next_post: usize,
    stopped: Option<StoppedAt>,
    destinations: Vec<Backing>,
    output: Option<Batch>,
}
impl RetiredCall {
    pub fn ordinal(&self) -> usize { self.ordinal }
    pub fn exit(&self) -> Exit { self.exit }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn prepared_count(&self) -> Option<usize> { self.count }
    pub fn selected_post(&self) -> Option<bool> { self.selected_post }
    pub fn next_post(&self) -> usize { self.next_post }
    pub fn stopped(&self) -> Option<StoppedAt> { self.stopped }
    pub fn destinations(&self) -> &[Backing] { &self.destinations }
    pub fn output(&self) -> Option<&Batch> { self.output.as_ref() }
}
pub struct ReadyParts {
    pub matrix: Matrix,
    pub calls: Calls,
    pub additive_post: Option<Backing>,
    pub lifecycle: LifecyclePolicy,
    pub issued: usize,
    pub retired: Vec<RetiredCall>,
}
pub struct ValueServices {
    matrix: Matrix,
    calls: Calls,
    additive_post: Option<Backing>,
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
        Self { matrix: parts.matrix, calls: parts.calls, additive_post: parts.additive_post,
            lifecycle: parts.lifecycle, issued: 0, active: None, retired: Vec::new(),
            phase: Phase::Idle, stopped: None, reset_stopped: None }
    }
    pub fn progress(&self) -> Progress {
        Progress { phase: self.phase, issued: self.issued,
            call: self.active.as_ref().map(|a| a.ordinal),
            prepared_count: self.active.as_ref().and_then(|a| a.count),
            selected_post: self.active.as_ref().and_then(|a| a.selected_post),
            next_post: self.active.as_ref().map(|a| a.next_post).unwrap_or(0),
            stopped: self.stopped, reset_stopped: self.reset_stopped }
    }
    pub fn matrix(&self) -> &Matrix { &self.matrix }
    pub fn additive_post(&self) -> Option<&Backing> { self.additive_post.as_ref() }
    pub fn unissued_calls(&self) -> Option<&VecDeque<CallRecord>> {
        match &self.calls { Calls::Supplied(calls) => Some(calls), Calls::Allocate(_) => None }
    }
    pub fn allocation_policy(&self) -> Option<AllocationPolicy> {
        match &self.calls { Calls::Allocate(policy) => Some(*policy), Calls::Supplied(_) => None }
    }
    pub fn active_destinations(&self) -> Option<&[Backing]> {
        self.active.as_ref().map(|a| a.destinations.as_slice())
    }
    pub fn retired_calls(&self) -> &[RetiredCall] { &self.retired }
    pub fn into_ready_parts(self) -> Result<ReadyParts, Self> {
        if self.phase != Phase::Idle || self.active.is_some()
            || self.stopped.is_some() || self.reset_stopped.is_some() { return Err(self); }
        Ok(ReadyParts { matrix: self.matrix, calls: self.calls, additive_post: self.additive_post,
            lifecycle: self.lifecycle, issued: self.issued, retired: self.retired })
    }
    fn stopped_at(&self, kind: StopKind) -> StoppedAt {
        StoppedAt { kind, phase: self.phase, call: self.active.as_ref().map(|a| a.ordinal),
            next_post: self.active.as_ref().map(|a| a.next_post).unwrap_or(0) }
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
    fn prepare_allocated(&mut self, output: &mut Batch, count: usize, width: usize, policy: AllocationPolicy)
        -> Result<(), Error>
    {
        if !output.rows.is_empty() || !self.active.as_ref().expect("phase checked allocated call").destinations.is_empty() {
            return Err(self.fail(StopKind::Protocol, Error::AllocationState));
        }
        output.rows.try_reserve_exact(count).map_err(|cause| self.fail(StopKind::Preparation,
            Error::Allocation { row: None, part: AllocationPart::OutputSlots, cause: allocation::Error::Capacity(cause) }))?;
        self.active.as_mut().expect("retained allocated call").destinations.try_reserve_exact(count)
            .map_err(|cause| self.fail(StopKind::Preparation, Error::Allocation {
                row: None, part: AllocationPart::BackingSlots, cause: allocation::Error::Capacity(cause) }))?;
        for row in 0..count {
            let destination = allocation::allocate_backing(width, policy.destination).map_err(|cause|
                self.fail(StopKind::Preparation, Error::Allocation { row: Some(row), part: AllocationPart::Destination, cause }))?;

            self.active.as_mut().expect("retained allocated call").destinations.push(destination);
            let values = allocation::allocate_logical_row(width, policy.output_word).map_err(|cause|
                self.fail(StopKind::Preparation, Error::Allocation { row: Some(row), part: AllocationPart::LogicalOutput, cause: cause.into() }))?;
            output.rows.push(values);
        }
        Ok(())
    }
    fn output_shape(output: &Batch, count: usize, width: usize) -> Result<(), Error> {
        if output.rows.len() != count {
            return Err(Error::OutputCount { expected: count, actual: output.rows.len() });
        }
        for (row, values) in output.rows.iter().enumerate() {
            if values.len() != width {
                return Err(Error::OutputWidth { row, expected: width, actual: values.len() });
            }
        }
        Ok(())
    }
    fn archive(active: ActiveCall, exit: Exit, phase: Phase, stopped: Option<StoppedAt>,
        output: Option<Batch>) -> RetiredCall {
        RetiredCall { ordinal: active.ordinal, exit, phase, count: active.count,
            selected_post: active.selected_post, next_post: active.next_post,
            stopped, destinations: active.destinations, output }
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
            Calls::Allocate(policy) => Some(CallRecord { output: Batch {
                rows: Vec::new(), status: policy.initial_output_status }, destinations: Vec::new() }),
        };
        let Some(CallRecord { output, destinations }) = record else {
            return Err(self.fail(StopKind::Exhausted, Error::Exhausted { issued: self.issued }));
        };
        let ordinal = self.issued;
        self.issued = next;
        self.active = Some(ActiveCall { ordinal, destinations, count: None,
            selected_post: None, next_post: 0 });
        self.phase = Phase::Acquired;
        Ok(output)
    }
    fn matrix_output_width(&mut self) -> Result<usize, Error> {
        self.require_phase(Phase::Acquired)?;
        let width = self.matrix.output_width();
        self.phase = Phase::WidthRead;
        Ok(width)
    }
    fn prepare_output(&mut self, output: &mut Batch, count: usize, width: usize) -> Result<(), Error> {
        self.require_phase(Phase::WidthRead)?;
        self.active.as_mut().expect("phase checked actual call").count = Some(count);
        let expected = self.matrix.output_width();
        if width != expected {
            return Err(self.fail(StopKind::Shape, Error::WidthArgument { expected, actual: width }));
        }
        if let Some(policy) = self.allocation_policy() { self.prepare_allocated(output, count, width, policy)?; }
        if let Err(error) = Self::output_shape(output, count, width) {
            return Err(self.fail(StopKind::Shape, error));
        }
        let destinations = &self.active.as_ref().expect("phase checked actual call").destinations;
        if destinations.len() != count {
            let actual = destinations.len();
            return Err(self.fail(StopKind::Shape, Error::BackingCount { expected: count, actual }));
        }
        let wrong_width = destinations.iter().enumerate().find_map(|(row, backing)| {
            (backing.logical_width() != width).then_some(Error::BackingWidth {
                row, expected: width, actual: backing.logical_width() })
        });
        if let Some(error) = wrong_width { return Err(self.fail(StopKind::Shape, error)); }

        self.phase = Phase::Prepared;
        Ok(())
    }
    fn matrix_all_rows(&mut self, input: &dyn ReadRows, output: &mut Batch) -> Result<(), Error> {
        self.require_phase(Phase::Prepared)?;
        let count = self.current_count()?;
        let width = self.matrix.output_width();
        if let Err(error) = Self::output_shape(output, count, width) {
            return Err(self.fail(StopKind::Shape, error));
        }
        let result = {
            let active = self.active.as_mut().expect("phase checked actual call");
            self.matrix.execute_read_backings(input, &mut active.destinations)
        };
        if let Err(error) = result { return Err(self.fail(StopKind::Matrix, Error::Matrix(error))); }

        for (row, backing) in output.rows.iter_mut().zip(
            &self.active.as_ref().expect("retained complete matrix call").destinations) {
            for (value, &bits) in row.iter_mut().zip(backing.logical_bits()) { *value = f32::from_bits(bits); }
        }
        self.phase = Phase::MatrixComplete;
        Ok(())
    }
    fn select_postprocessor(&mut self) -> Result<bool, Error> {
        self.require_phase(Phase::MatrixComplete)?;
        let count = self.current_count()?;
        let selected = self.additive_post.is_some();
        self.active.as_mut().expect("phase checked actual call").selected_post = Some(selected);
        self.phase = if selected && count != 0 { Phase::PostRows } else { Phase::AwaitPublish };
        Ok(selected)
    }
    fn postprocess_row(&mut self, ordinal: usize, row: &mut [f32]) -> Result<(), Error> {
        self.require_phase(Phase::PostRows)?;
        let count = self.current_count()?;
        let expected = self.active.as_ref().expect("phase checked actual call").next_post;
        if ordinal != expected || ordinal >= count {
            return Err(self.fail(StopKind::Protocol, Error::PostOrdinal { expected, actual: ordinal }));
        }
        let width = self.matrix.output_width();
        if row.len() != width {
            return Err(self.fail(StopKind::Shape, Error::OutputWidth { row: ordinal, expected: width, actual: row.len() }));
        }

        let result = match (self.additive_post.as_ref(), self.active.as_mut()) {
            (Some(receiver), Some(active)) => match active.destinations.get_mut(ordinal) {
                Some(destination) => receiver.add_into(destination).map_err(|error| Error::Post { row: ordinal, error }),
                None => Err(Error::MissingBacking { row: ordinal }),
            },
            (None, _) => Err(Error::MissingPost),
            (_, None) => Err(Error::MissingActive),
        };
        if let Err(error) = result { return Err(self.fail(StopKind::Post, error)); }
        let active = self.active.as_mut().expect("retained successful post call");
        let backing = &active.destinations[ordinal];
        for (value, &bits) in row.iter_mut().zip(backing.logical_bits()) { *value = f32::from_bits(bits); }
        active.next_post += 1;
        if active.next_post == count { self.phase = Phase::AwaitPublish; }
        Ok(())
    }
    fn publish_output(&mut self, output: Batch) -> Result<Batch, PublishRefusal<Error>> {
        if let Err(reason) = self.require_phase(Phase::AwaitPublish) {
            return Err(PublishRefusal { reason, output });
        }
        let count = match self.current_count() {
            Ok(count) => count,
            Err(reason) => return Err(PublishRefusal { reason, output }),
        };
        if let Err(error) = Self::output_shape(&output, count, self.matrix.output_width()) {
            let reason = self.fail(StopKind::Shape, error);
            return Err(PublishRefusal { reason, output });
        }
        let phase = self.phase;
        let stopped = self.stopped;
        let result = owners::archive_current(&mut self.active, &mut self.retired,
            owners::ArchiveOutput::Published, move |active, output, exit| {
                let exit = match exit { owners::Exit::Published => Exit::Published, owners::Exit::Reset => Exit::Reset };
                Self::archive(active, exit, phase, stopped, output)
            });
        if let Err(error) = result {
            let reason = match error {
                owners::ArchiveError::Capacity(error) => self.fail(StopKind::Publication,
                    Error::HistoryCapacity { at: HistoryAt::Publication, error }),
                owners::ArchiveError::MissingActive => self.fail(StopKind::Protocol, Error::MissingActive),
                owners::ArchiveError::OwnerMismatch { active_call, output } => self.fail(StopKind::Protocol,
                    Error::ResetOwnerMismatch { active_call, output }),
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
        let result = owners::archive_current(&mut self.active, &mut self.retired,
            owners::ArchiveOutput::Reset(active_output), move |active, output, exit| {
                let exit = match exit { owners::Exit::Published => Exit::Published, owners::Exit::Reset => Exit::Reset };
                Self::archive(active, exit, phase, stopped, output)
            });
        if let Err(error) = result {
            let error = match error {
                owners::ArchiveError::Capacity(error) => Error::HistoryCapacity { at: HistoryAt::Reset, error },
                owners::ArchiveError::OwnerMismatch { active_call, output } => Error::ResetOwnerMismatch { active_call, output },
                owners::ArchiveError::MissingActive => Error::MissingActive,
            };
            return Err(self.fail_reset(error));
        }

        self.stopped = None;
        self.phase = Phase::Idle;
        Ok(())
    }
}
