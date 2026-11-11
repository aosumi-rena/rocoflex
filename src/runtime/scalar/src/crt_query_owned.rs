#[path = "crt_query_context_allocation.rs"]
pub mod context_allocation;

#[path = "crt_query_context_lifecycle.rs"]
pub mod context_lifecycle;

#[path = "crt_query_owned_fresh_read.rs"]
pub mod fresh_read;

#[path = "crt_query_owned_helpers.rs"]
pub mod helpers;

#[path = "crt_query_owned_slot_store.rs"]
pub mod slot_store;

#[path = "crt_query_owned_epilogue.rs"]
pub mod epilogue;

#[path = "crt_query_replace_counter.rs"]
pub mod replace_counter;

#[path = "crt_query_replace_retirement.rs"]
pub mod replace_retirement;

use super::crt_query_context::{SourceCrtQueryContext58db7c, SourceCrtQueryContextRequest,
    SourceCrtQueryContextReceipt, SourceCrtQueryContextProgress, SourceCrtQueryContextCompletion,
    SourceCrtQueryContextError};
use super::crt_query_refresh::{SourceCrtQueryRefresh58e238, SourceCrtQueryRefreshRequest,
    SourceCrtQueryRefreshReceipt, SourceCrtQueryRefreshProgress, SourceCrtQueryRefreshCompletion,
    SourceCrtQueryRefreshError};
use super::crt_query_resolve::{SourceCrtQueryResolve5941fc, SourceCrtQueryResolveRequest,
    SourceCrtQueryResolveReceipt, SourceCrtQueryResolveProgress, SourceCrtQueryResolveCompletion,
    SourceCrtQueryResolveError};
use super::crt_query_replace::{SourceCrtQueryReplace59426c, SourceCrtQueryReplaceRequest,
    SourceCrtQueryReplaceReceipt, SourceCrtQueryReplaceProgress, SourceCrtQueryReplaceCompletion,
    SourceCrtQueryReplaceError};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeReturnFailure};
use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocRequest,
    SourceCrtQueryCallocReceipt, SourceCrtQueryCallocProgress, SourceCrtQueryCallocCompletion,
    SourceCrtQueryCallocError};
use super::crt_query_mode::{SourceCrtQueryMode59bfc8, SourceCrtQueryModeRequest,
    SourceCrtQueryModeReceipt, SourceCrtQueryModeProgress, SourceCrtQueryModeCompletion,
    SourceCrtQueryModeError};
use super::crt_query_initialization::{SourceCrtQueryInitialization58df48, SourceCrtQueryInitializationRequest,
    SourceCrtQueryInitializationReceipt, SourceCrtQueryInitializationProgress, SourceCrtQueryInitializationCompletion,
    SourceCrtQueryInitializationError};
use super::crt_query_null_release::{SourceCrtQueryNullRelease58d910, SourceCrtQueryNullReleaseRequest,
    SourceCrtQueryNullReleaseReceipt, SourceCrtQueryNullReleaseProgress, SourceCrtQueryNullReleaseCompletion,
    SourceCrtQueryNullReleaseError};

pub enum SourceCrtQueryCursor {
    Context(SourceCrtQueryContext58db7c), Refresh(SourceCrtQueryRefresh58e238),
    Resolve(SourceCrtQueryResolve5941fc), Replace(SourceCrtQueryReplace59426c),
    Calloc(SourceCrtQueryCalloc58f074), Mode(SourceCrtQueryMode59bfc8),
    Initialization(SourceCrtQueryInitialization58df48),
    NullRelease(SourceCrtQueryNullRelease58d910),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryRequest {
    Context(SourceCrtQueryContextRequest), Refresh(SourceCrtQueryRefreshRequest),
    Resolve(SourceCrtQueryResolveRequest), Replace(SourceCrtQueryReplaceRequest),
    Calloc(SourceCrtQueryCallocRequest), Mode(SourceCrtQueryModeRequest),
    Initialization(SourceCrtQueryInitializationRequest),
    NullRelease(SourceCrtQueryNullReleaseRequest),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReceipt {
    Context(SourceCrtQueryContextReceipt), Refresh(SourceCrtQueryRefreshReceipt),
    Resolve(SourceCrtQueryResolveReceipt), Replace(SourceCrtQueryReplaceReceipt),
    Calloc(SourceCrtQueryCallocReceipt), Mode(SourceCrtQueryModeReceipt),
    Initialization(SourceCrtQueryInitializationReceipt),
    NullRelease(SourceCrtQueryNullReleaseReceipt),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryCompletion {
    Context(SourceCrtQueryContextCompletion), Refresh(SourceCrtQueryRefreshCompletion),
    Resolve(SourceCrtQueryResolveCompletion), Replace(SourceCrtQueryReplaceCompletion),
    Calloc(SourceCrtQueryCallocCompletion), Mode(SourceCrtQueryModeCompletion),
    Initialization(SourceCrtQueryInitializationCompletion),
    NullRelease(SourceCrtQueryNullReleaseCompletion),
}
impl SourceCrtQueryCompletion {
    pub fn actual_ret_frame(self) -> SourceCrtNativeRetFrame {
        match self { Self::Context(value) => value.actual_ret_frame(), Self::Refresh(value) => value.actual_ret_frame(),
            Self::Resolve(value) => value.actual_ret_frame(), Self::Replace(value) => value.actual_ret_frame(),
            Self::Calloc(value) => value.actual_ret_frame(), Self::Mode(value) => value.actual_ret_frame(),
            Self::Initialization(value) => value.actual_ret_frame(), Self::NullRelease(value) => value.actual_ret_frame() }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQuerySourceError {
    Context(SourceCrtQueryContextError), Refresh(SourceCrtQueryRefreshError),
    Resolve(SourceCrtQueryResolveError), Replace(SourceCrtQueryReplaceError),
    Calloc(SourceCrtQueryCallocError), Mode(SourceCrtQueryModeError),
    Initialization(SourceCrtQueryInitializationError), NullRelease(SourceCrtQueryNullReleaseError), WrongCursor,
}
impl SourceCrtQueryCursor {
    fn is_stopped(&self) -> bool {
        match self { Self::Context(source) => source.is_stopped(), Self::Refresh(source) => source.is_stopped(),
            Self::Resolve(source) => source.is_stopped(), Self::Replace(source) => source.is_stopped(),
            Self::Calloc(source) => source.is_stopped(), Self::Mode(source) => source.is_stopped(),
            Self::Initialization(source) => source.is_stopped(), Self::NullRelease(source) => source.is_stopped() }
    }
    fn stop(&mut self) { match self { Self::Context(source) => source.stop(), Self::Refresh(source) => source.stop(),
        Self::Resolve(source) => source.stop(), Self::Replace(source) => source.stop(),
        Self::Calloc(source) => source.stop(), Self::Mode(source) => source.stop(),
        Self::Initialization(source) => source.stop(), Self::NullRelease(source) => source.stop() } }
    fn pending(&self) -> Option<SourceCrtQueryRequest> {
        match self { Self::Context(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Context),
            Self::Refresh(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Refresh),
            Self::Resolve(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Resolve),
            Self::Replace(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Replace),
            Self::Calloc(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Calloc),
            Self::Mode(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Mode),
            Self::Initialization(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::Initialization),
            Self::NullRelease(source) => source.awaiting_actual_completion().map(SourceCrtQueryRequest::NullRelease) }
    }
    fn before_ret(&self) -> Option<SourceCrtQueryCompletion> {
        match self {
            Self::Context(source) => match source.progress() {
                SourceCrtQueryContextProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Context(value)), _ => None },
            Self::Refresh(source) => match source.progress() {
                SourceCrtQueryRefreshProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Refresh(value)), _ => None },
            Self::Resolve(source) => match source.progress() {
                SourceCrtQueryResolveProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Resolve(value)), _ => None },
            Self::Replace(source) => match source.progress() {
                SourceCrtQueryReplaceProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Replace(value)), _ => None },
            Self::Calloc(source) => match source.progress() {
                SourceCrtQueryCallocProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Calloc(value)), _ => None },
            Self::Mode(source) => match source.progress() {
                SourceCrtQueryModeProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Mode(value)), _ => None },
            Self::Initialization(source) => match source.progress() {
                SourceCrtQueryInitializationProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::Initialization(value)), _ => None },
            Self::NullRelease(source) => match source.progress() {
                SourceCrtQueryNullReleaseProgress::BeforeRet(value) => Some(SourceCrtQueryCompletion::NullRelease(value)), _ => None },
        }
    }
    fn accept(&mut self, receipt: SourceCrtQueryReceipt) -> Result<(), SourceCrtQuerySourceError> {
        match (self, receipt) {
            (Self::Context(source), SourceCrtQueryReceipt::Context(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Context),
            (Self::Refresh(source), SourceCrtQueryReceipt::Refresh(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Refresh),
            (Self::Resolve(source), SourceCrtQueryReceipt::Resolve(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Resolve),
            (Self::Replace(source), SourceCrtQueryReceipt::Replace(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Replace),
            (Self::Calloc(source), SourceCrtQueryReceipt::Calloc(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Calloc),
            (Self::Mode(source), SourceCrtQueryReceipt::Mode(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Mode),
            (Self::Initialization(source), SourceCrtQueryReceipt::Initialization(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::Initialization),
            (Self::NullRelease(source), SourceCrtQueryReceipt::NullRelease(actual)) => source.accept_actual_completion(actual)
                .map(|_| ()).map_err(SourceCrtQuerySourceError::NullRelease),
            _ => Err(SourceCrtQuerySourceError::WrongCursor),
        }
    }
}
impl SourceCrtQueryReceipt {
    fn supplied_frame(self) -> Option<SourceCrtNativeRetFrame> {
        match self {
            Self::Context(SourceCrtQueryContextReceipt::EntryContinued { actual_frame, .. }
                | SourceCrtQueryContextReceipt::CallContinued { actual_frame, .. }
                | SourceCrtQueryContextReceipt::FailureContinued { actual_frame }) => Some(actual_frame),
            Self::Context(SourceCrtQueryContextReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::Refresh(SourceCrtQueryRefreshReceipt::Prologue { actual_body, .. }) => Some(actual_body),
            Self::Refresh(SourceCrtQueryRefreshReceipt::Returned5941fc { actual_frame, .. }) => Some(actual_frame),
            Self::Refresh(SourceCrtQueryRefreshReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::Resolve(SourceCrtQueryResolveReceipt::Prologue { actual_body, .. }) => Some(actual_body),
            Self::Resolve(SourceCrtQueryResolveReceipt::CallReturned { actual_frame, .. }) => Some(actual_frame),
            Self::Resolve(SourceCrtQueryResolveReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::Replace(SourceCrtQueryReplaceReceipt::Prologue { actual_body, .. }) => Some(actual_body),
            Self::Replace(SourceCrtQueryReplaceReceipt::CallReturned { actual_frame, .. }) => Some(actual_frame),
            Self::Replace(SourceCrtQueryReplaceReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::Calloc(SourceCrtQueryCallocReceipt::EntryPrefix { actual_body, .. }) => Some(actual_body),
            Self::Calloc(SourceCrtQueryCallocReceipt::ProtectedContinued { actual_frame, .. }
                | SourceCrtQueryCallocReceipt::CallReturned { actual_frame, .. }) => Some(actual_frame),
            Self::Calloc(SourceCrtQueryCallocReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::Mode(actual) => Some(actual.actual_ret_frame),
            Self::Initialization(SourceCrtQueryInitializationReceipt::Prologue { actual_frame, .. }
                | SourceCrtQueryInitializationReceipt::BorrowedBody { actual_frame, .. }) => Some(actual_frame),
            Self::Initialization(SourceCrtQueryInitializationReceipt::Epilogue { actual_ret_frame, .. }) => Some(actual_ret_frame),
            Self::NullRelease(actual) => Some(actual.actual_ret_frame),
            _ => None,
        }
    }
}

pub struct SourceCrtQueryOwnedBindings<P, S> {
    pub actual_frame: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub actual_receipt: fn(&P, &S) -> Option<SourceCrtQueryReceipt>,
}
#[must_use = "retain every actual query receipt and earlier native/frame/FP/resource owner"]
pub struct SourceCrtQueryHistory<H, R> {
    pub previous: H,
    pub actual: R,
    pub receipt: SourceCrtQueryReceipt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtQueryReceiptReason { ActualRecordRequired, ActualPrefixRequired, ActualFrameRequired, SourceContinuationRequired }
#[must_use = "retain raw rejected resource/exception receipt and SAME advanced owner"]
pub struct SourceCrtQueryReceiptError<R> {
    pub actual: R,
    pub supplied: SourceCrtQueryReceipt,
    pub observed: SourceCrtQueryReceipt,
    pub owned_observed: Option<SourceCrtQueryReceipt>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub reason: SourceCrtQueryReceiptReason,
    pub source: Option<SourceCrtQuerySourceError>,
}
#[must_use = "retain rejected raw resources and all earlier history without replay"]
pub struct SourceCrtQueryRejectedHistory<H, R> { pub previous: H, pub rejected: SourceCrtQueryReceiptError<R> }
#[must_use = "retain exact failed service resources/unknown prefix without a fabricated normal-return packet"]
pub struct SourceCrtQueryServiceFailureHistory<H, F> {
    pub previous: H,
    pub actual_failure: F,
    pub pending: Option<SourceCrtQueryRequest>,
}
#[must_use = "retain actual corrected pre-RET prefix and prior query resources"]
pub struct SourceCrtQueryRetPrefixHistory<H, R> {
    pub previous: H,
    pub actual: R,
    pub actual_frame: SourceCrtNativeRetFrame,
}
pub struct SourceCrtQueryRetPrefixError<R> {
    pub actual: R,
    pub observed: SourceCrtNativeRetFrame,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub required: Option<SourceCrtNativeRetFrame>,
}

#[must_use = "retain SAME query source cursor, frame/FP/protected/native/resources"]
pub struct SourceCrtQueryOwned<P, S, H = ()> {
    cursor: SourceCrtQueryCursor,
    pub prefix: P,
    pub services: S,
    history: H,
    bindings: SourceCrtQueryOwnedBindings<P, S>,
    stopped: bool,
}
impl<P, S> SourceCrtQueryOwned<P, S> {

    pub fn from_same_cursor(cursor: SourceCrtQueryCursor, prefix: P, services: S,
        bindings: SourceCrtQueryOwnedBindings<P, S>) -> Self {
        Self::from_same_cursor_and_history(cursor, prefix, services, (), bindings)
    }
}
impl<P, S, H> SourceCrtQueryOwned<P, S, H> {

    pub fn from_same_cursor_and_history(cursor: SourceCrtQueryCursor, prefix: P, services: S,
        history: H, bindings: SourceCrtQueryOwnedBindings<P, S>) -> Self {
        let stopped = cursor.is_stopped();
        Self { cursor, prefix, services, bindings, history, stopped }
    }

    pub fn with_retained_history<T>(self, actual: T) -> SourceCrtQueryOwned<P, S, (H, T)> {
        SourceCrtQueryOwned { cursor: self.cursor, prefix: self.prefix, services: self.services,
            bindings: self.bindings, stopped: self.stopped, history: (self.history, actual) }
    }
    pub fn request(&self) -> Option<SourceCrtQueryRequest> { if self.stopped { None } else { self.cursor.pending() } }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtQueryRequest> { self.cursor.pending() }
    pub fn history(&self) -> &H { &self.history }

    pub fn retain_actual_service_failure<F>(mut self, actual_failure: F)
        -> SourceCrtQueryOwned<P, S, SourceCrtQueryServiceFailureHistory<H, F>> {
        let pending = self.cursor.pending();
        self.cursor.stop();
        SourceCrtQueryOwned { cursor: self.cursor, prefix: self.prefix, services: self.services,
            bindings: self.bindings, stopped: true,
            history: SourceCrtQueryServiceFailureHistory { previous: self.history, actual_failure, pending } }
    }

    pub fn accept_actual_receipt<R>(mut self, actual: R, supplied: SourceCrtQueryReceipt,
        project: fn(&R) -> SourceCrtQueryReceipt)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryHistory<H, R>>,
            SourceCrtNativeReturnFailure<Self, SourceCrtQueryReceiptError<R>>> {
        self.stopped = true; self.cursor.stop();
        let observed = project(&actual);
        let owned_observed = (self.bindings.actual_receipt)(&self.prefix, &self.services);
        let actual_frame = (self.bindings.actual_frame)(&self.prefix, &self.services);
        let reason = if observed != supplied { Some(SourceCrtQueryReceiptReason::ActualRecordRequired) }
            else if owned_observed != Some(supplied) { Some(SourceCrtQueryReceiptReason::ActualPrefixRequired) }
            else if supplied.supplied_frame().is_some_and(|frame| frame != actual_frame) {
                Some(SourceCrtQueryReceiptReason::ActualFrameRequired)
            } else { None };
        if let Some(reason) = reason {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtQueryReceiptError {
                actual, supplied, observed, owned_observed, actual_frame, reason, source: None } });
        }
        if let Err(source) = self.cursor.accept(supplied) {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtQueryReceiptError {
                actual, supplied, observed, owned_observed, actual_frame,
                reason: SourceCrtQueryReceiptReason::SourceContinuationRequired, source: Some(source) } });
        }
        Ok(SourceCrtQueryOwned { cursor: self.cursor, prefix: self.prefix, services: self.services,
            bindings: self.bindings, stopped: false,
            history: SourceCrtQueryHistory { previous: self.history, actual, receipt: supplied } })
    }
    pub fn retain_rejected_receipt<R>(failure: SourceCrtNativeReturnFailure<Self, SourceCrtQueryReceiptError<R>>)
        -> SourceCrtQueryOwned<P, S, SourceCrtQueryRejectedHistory<H, R>> {
        let SourceCrtNativeReturnFailure { owner, cause } = failure;
        SourceCrtQueryOwned { cursor: owner.cursor, prefix: owner.prefix, services: owner.services,
            bindings: owner.bindings, stopped: true,
            history: SourceCrtQueryRejectedHistory { previous: owner.history, rejected: cause } }
    }
    pub fn into_before_ret(mut self) -> Result<SourceCrtQueryBeforeRet<P, S, H>, Self> {
        let was_stopped = self.stopped;
        self.stopped = true; self.cursor.stop();
        let Some(source) = self.cursor.before_ret() else { return Err(self); };
        if was_stopped || (self.bindings.actual_frame)(&self.prefix, &self.services) != source.actual_ret_frame() {
            return Err(self);
        }
        Ok(SourceCrtQueryBeforeRet { cursor: self.cursor, prefix: self.prefix, services: self.services,
            history: self.history, source, current_frame: self.bindings.actual_frame })
    }

    pub fn accept_actual_ret_entry_prefix<R>(mut self, actual: R,
        project: fn(&R) -> SourceCrtNativeRetFrame)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtQueryRetPrefixHistory<H, R>>,
            SourceCrtNativeReturnFailure<Self, SourceCrtQueryRetPrefixError<R>>> {
        self.stopped = true; self.cursor.stop();
        let required = self.cursor.before_ret().map(SourceCrtQueryCompletion::actual_ret_frame);
        let observed = project(&actual);
        let actual_frame = (self.bindings.actual_frame)(&self.prefix, &self.services);
        if required != Some(observed) || actual_frame != observed {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtQueryRetPrefixError { actual, observed, actual_frame, required } });
        }
        Ok(SourceCrtQueryOwned { cursor: self.cursor, prefix: self.prefix, services: self.services,
            bindings: self.bindings, stopped: false,
            history: SourceCrtQueryRetPrefixHistory { previous: self.history, actual, actual_frame } })
    }
}
#[must_use = "retain stopped original query cursor and ALL before-RET owners"]
pub struct SourceCrtQueryBeforeRet<P, S, H> {
    pub(super) cursor: SourceCrtQueryCursor,
    pub prefix: P,
    pub services: S,
    pub history: H,
    pub(super) source: SourceCrtQueryCompletion,
    pub(super) current_frame: fn(&P, &S) -> SourceCrtNativeRetFrame,
}
impl<P, S, H> SourceCrtQueryBeforeRet<P, S, H> {
    pub fn source(&self) -> SourceCrtQueryCompletion { self.source }
}
