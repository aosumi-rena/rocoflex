use core::num::NonZeroU64;
use super::crt_module_publish::{SourceCrtModulePublish59be59, SourceCrtModulePublishCompletion,
    SourceCrtModulePublishError, SourceCrtModulePublishProgress, SourceCrtModulePublishReceipt,
    SourceCrtModulePublishRequest};

pub struct SourceCrtModulePublicationCallbacks<M, R, P, S, E> {
    pub source_module_word: fn(&M) -> NonZeroU64,
    pub actual_receipt: fn(&P) -> Option<SourceCrtModulePublishReceipt>,
    pub resource_receipt: fn(&R) -> SourceCrtModulePublishReceipt,
    pub invoke: fn(SourceCrtModulePublishRequest, &mut M, &mut [Option<R>; 2], &mut P, &mut S)
        -> Result<SourceCrtModulePublicationActual<R>, E>,
}
pub struct SourceCrtModulePublicationActual<R> {
    pub receipt: SourceCrtModulePublishReceipt,
    pub resources: R,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModulePublicationPhase { Ready, ActualCompletionRequired, Complete }
#[must_use = "retain source cursor and all actual module/frame/FP/native resources"]
pub struct SourceCrtModulePublicationBindFailure<M, P, S> {
    pub source: SourceCrtModulePublish59be59,
    pub module_owner: M,
    pub actual_module_word: NonZeroU64,
    pub prefix: P,
    pub services: S,
}
pub enum SourceCrtModulePublicationError<E> {
    PhaseRequired { actual: SourceCrtModulePublicationPhase },
    ModuleWordRequired { expected: NonZeroU64, actual: NonZeroU64 },
    Primitive(E),
    UnadoptedResourcesRequired,
    ResourceAlreadyReported { request: SourceCrtModulePublishRequest },
    ResourcePrefixRequired { request: SourceCrtModulePublishRequest, actual: SourceCrtModulePublishReceipt },
    ResourceSlotOccupied { request: SourceCrtModulePublishRequest, actual: SourceCrtModulePublishReceipt },
    ResourceReceiptRequired { actual: SourceCrtModulePublishReceipt, observed: SourceCrtModulePublishReceipt },
    ActualPrefixRequired { actual: SourceCrtModulePublishReceipt, observed: Option<SourceCrtModulePublishReceipt> },
    Source(SourceCrtModulePublishError),
}
impl<E> SourceCrtModulePublicationError<E> {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::PhaseRequired { .. } => "owned module publication requires its current phase; no implicit native/atomic retry",
            Self::ModuleWordRequired { .. } => "module resource must retain the same historical successful-load source word",
            Self::Primitive(_) => "module publication primitive failed with actual module/cache/frame/FP resources retained",
            Self::UnadoptedResourcesRequired => "additional actual module resources require explicit ownership continuation before another invocation",
            Self::ResourceAlreadyReported { .. } => "module operation already has a resource prefix; actual continuation is required before any invocation",
            Self::ResourcePrefixRequired { .. } => "actual module publication completion requires its owned resource prefix",
            Self::ResourceSlotOccupied { .. } => "module publication service returned an additional resource for an occupied slot; both owners retained",
            Self::ResourceReceiptRequired { .. } => "module publication receipt differs from its actual resource capsule projection",
            Self::ActualPrefixRequired { .. } => "module publication actual receipt differs from the same owned frame prefix",
            Self::Source(error) => error.reason(),
        }
    }
}
#[must_use = "retain the entire module/cache/native/frame/FP continuation"]
pub struct SourceCrtModulePublicationFailure<T, E> { pub owner: T, pub cause: E }
pub enum SourceCrtModulePublicationResumeError<A, E> {
    NotPending { actual: A },
    Adapter { actual: A, cause: E },
    Validation(SourceCrtModulePublicationError<E>),
}
#[must_use = "retain loaded-module/cache/native resources and actual frame/FP prefix"]
pub struct SourceCrtModulePublicationCompletion<M, R, P, S> {
    pub source: SourceCrtModulePublishCompletion,
    pub module_owner: M,
    pub actual_module_word: NonZeroU64,
    pub actual_receipts: [Option<SourceCrtModulePublishReceipt>; 2],
    pub resources: [Option<R>; 2],
    pub unadopted_resources: Option<R>,
    pub prefix: P,
    pub services: S,
}
pub struct SourceCrtModulePublicationParts<M, R, P, S, E> {
    pub source: SourceCrtModulePublish59be59,
    pub module_owner: M,
    pub actual_module_word: NonZeroU64,
    pub phase: SourceCrtModulePublicationPhase,
    pub pending: Option<SourceCrtModulePublishRequest>,
    pub actual_receipts: [Option<SourceCrtModulePublishReceipt>; 2],
    pub resources: [Option<R>; 2],
    pub unadopted_resources: Option<R>,
    pub prefix: P,
    pub services: S,
    pub callbacks: SourceCrtModulePublicationCallbacks<M, R, P, S, E>,
}

#[must_use = "retain actual module/cache/native/frame/FP owners until explicit continuation"]
pub struct SourceCrtOwnedModulePublication<M, R, P, S, E> {
    source: SourceCrtModulePublish59be59,
    module_owner: M,
    actual_module_word: NonZeroU64,
    phase: SourceCrtModulePublicationPhase,
    pending: Option<SourceCrtModulePublishRequest>,
    actual_receipts: [Option<SourceCrtModulePublishReceipt>; 2],
    resources: [Option<R>; 2],
    unadopted_resources: Option<R>,
    prefix: P,
    services: S,
    callbacks: SourceCrtModulePublicationCallbacks<M, R, P, S, E>,
}
impl<M, R, P, S, E> SourceCrtOwnedModulePublication<M, R, P, S, E> {
    pub fn bind_actual_module(source: SourceCrtModulePublish59be59, module_owner: M, prefix: P, services: S,
        callbacks: SourceCrtModulePublicationCallbacks<M, R, P, S, E>)
        -> Result<Self, SourceCrtModulePublicationBindFailure<M, P, S>> {
        let actual_module_word = (callbacks.source_module_word)(&module_owner);
        let admitted = match source.request() {
            Some(SourceCrtModulePublishRequest::ExchangeModuleCache { desired_word, .. }) =>
                desired_word == actual_module_word.get() && source.actual_module_word() == actual_module_word.get()
                    && source.actual_exchanged_old_word().is_none(),
            _ => false,
        };
        if !admitted { return Err(SourceCrtModulePublicationBindFailure {
            source, module_owner, actual_module_word, prefix, services }); }
        Ok(Self { source, module_owner, actual_module_word, phase: SourceCrtModulePublicationPhase::Ready,
            pending: None, actual_receipts: [None; 2], resources: [None, None], unadopted_resources: None,
            prefix, services, callbacks })
    }
    pub fn phase(&self) -> SourceCrtModulePublicationPhase { self.phase }
    pub fn request(&self) -> Option<SourceCrtModulePublishRequest> {
        if self.phase == SourceCrtModulePublicationPhase::Ready { self.source.request() } else { None }
    }
    pub fn prefix(&self) -> &P { &self.prefix }
    pub fn module_owner(&self) -> &M { &self.module_owner }
    pub fn resources(&self) -> &[Option<R>; 2] { &self.resources }
    pub fn step(mut self) -> Result<Self, SourceCrtModulePublicationFailure<Self, SourceCrtModulePublicationError<E>>> {
        let Some(request) = self.request() else {
            let actual = self.phase;
            return Err(self.fail(SourceCrtModulePublicationError::PhaseRequired { actual }));
        };
        if self.unadopted_resources.is_some() {
            return Err(self.fail(SourceCrtModulePublicationError::UnadoptedResourcesRequired));
        }
        if self.resources[Self::slot(request)].is_some() {
            return Err(self.fail(SourceCrtModulePublicationError::ResourceAlreadyReported { request }));
        }
        let actual = (self.callbacks.source_module_word)(&self.module_owner);
        if actual != self.actual_module_word {
            let expected = self.actual_module_word;
            return Err(self.fail(SourceCrtModulePublicationError::ModuleWordRequired { expected, actual }));
        }
        self.phase = SourceCrtModulePublicationPhase::ActualCompletionRequired;
        self.pending = Some(request);
        self.source.suspend_actual_prefix();
        let result = (self.callbacks.invoke)(request, &mut self.module_owner, &mut self.resources,
            &mut self.prefix, &mut self.services);
        match result {
            Err(error) => Err(self.fail(SourceCrtModulePublicationError::Primitive(error))),
            Ok(actual) => {
                let slot = Self::slot(request);
                self.unadopted_resources = Some(actual.resources);
                if self.resources[slot].is_some() {
                    self.actual_receipts[slot] = Some(actual.receipt);
                    return Err(self.fail(SourceCrtModulePublicationError::ResourceSlotOccupied { request, actual: actual.receipt }));
                }
                self.resources[slot] = self.unadopted_resources.take();
                self.validate_completion(request, actual.receipt)
            }
        }
    }

    pub fn accept_actual_completion<A>(mut self, actual: A,
        apply: fn(SourceCrtModulePublishRequest, &mut M, &mut [Option<R>; 2], &mut P, &mut S, A)
            -> Result<SourceCrtModulePublishReceipt, (A, E)>)
        -> Result<Self, SourceCrtModulePublicationFailure<Self, SourceCrtModulePublicationResumeError<A, E>>> {
        if self.phase != SourceCrtModulePublicationPhase::ActualCompletionRequired {
            return Err(self.fail(SourceCrtModulePublicationResumeError::NotPending { actual }));
        }
        let Some(expected) = self.pending else {
            return Err(self.fail(SourceCrtModulePublicationResumeError::NotPending { actual }));
        };
        let receipt = match apply(expected, &mut self.module_owner, &mut self.resources, &mut self.prefix, &mut self.services, actual) {
            Err((actual, cause)) => return Err(self.fail(SourceCrtModulePublicationResumeError::Adapter { actual, cause })),
            Ok(receipt) => receipt,
        };
        self.validate_completion(expected, receipt).map_err(|failure| SourceCrtModulePublicationFailure {
            owner: failure.owner, cause: SourceCrtModulePublicationResumeError::Validation(failure.cause) })
    }

    pub fn reconcile_actual_resources<A>(mut self, actual: A,
        apply: fn(SourceCrtModulePublishRequest, &mut M, &mut [Option<R>; 2], &mut Option<R>, &mut P, &mut S, A)
            -> Result<(), (A, E)>)
        -> Result<Self, SourceCrtModulePublicationFailure<Self, SourceCrtModulePublicationResumeError<A, E>>> {
        if self.phase != SourceCrtModulePublicationPhase::ActualCompletionRequired {
            return Err(self.fail(SourceCrtModulePublicationResumeError::NotPending { actual }));
        }
        let Some(expected) = self.pending else {
            return Err(self.fail(SourceCrtModulePublicationResumeError::NotPending { actual }));
        };
        match apply(expected, &mut self.module_owner, &mut self.resources, &mut self.unadopted_resources,
            &mut self.prefix, &mut self.services, actual) {
            Ok(()) => Ok(self),
            Err((actual, cause)) => Err(self.fail(SourceCrtModulePublicationResumeError::Adapter { actual, cause })),
        }
    }
    fn validate_completion(mut self, expected: SourceCrtModulePublishRequest, actual: SourceCrtModulePublishReceipt)
        -> Result<Self, SourceCrtModulePublicationFailure<Self, SourceCrtModulePublicationError<E>>> {
        let slot = Self::slot(expected);
        self.actual_receipts[slot] = Some(actual);
        let Some(resource) = self.resources[slot].as_ref() else {
            return Err(self.fail(SourceCrtModulePublicationError::ResourcePrefixRequired { request: expected, actual }));
        };
        let observed = (self.callbacks.resource_receipt)(resource);
        if observed != actual {
            return Err(self.fail(SourceCrtModulePublicationError::ResourceReceiptRequired { actual, observed }));
        }
        let word = (self.callbacks.source_module_word)(&self.module_owner);
        if word != self.actual_module_word {
            let expected = self.actual_module_word;
            return Err(self.fail(SourceCrtModulePublicationError::ModuleWordRequired { expected, actual: word }));
        }
        let observed = (self.callbacks.actual_receipt)(&self.prefix);
        if observed != Some(actual) {
            return Err(self.fail(SourceCrtModulePublicationError::ActualPrefixRequired { actual, observed }));
        }
        match self.source.accept_actual_completion(actual) {
            Err(error) => Err(self.fail(SourceCrtModulePublicationError::Source(error))),
            Ok(progress) => {
                self.pending = None;
                self.phase = match progress {
                    SourceCrtModulePublishProgress::Request(_) => SourceCrtModulePublicationPhase::Ready,
                    SourceCrtModulePublishProgress::Complete(_) => SourceCrtModulePublicationPhase::Complete,
                };
                Ok(self)
            }
        }
    }
    pub fn into_completed_publication(self) -> Result<SourceCrtModulePublicationCompletion<M, R, P, S>, Self> {
        if self.phase != SourceCrtModulePublicationPhase::Complete { return Err(self); }
        let SourceCrtModulePublishProgress::Complete(source) = self.source.progress() else { return Err(self); };
        Ok(SourceCrtModulePublicationCompletion { source, module_owner: self.module_owner,
            actual_module_word: self.actual_module_word, actual_receipts: self.actual_receipts,
            resources: self.resources, unadopted_resources: self.unadopted_resources,
            prefix: self.prefix, services: self.services })
    }
    pub fn into_owned_parts(self) -> SourceCrtModulePublicationParts<M, R, P, S, E> {
        SourceCrtModulePublicationParts { source: self.source, module_owner: self.module_owner,
            actual_module_word: self.actual_module_word, phase: self.phase, pending: self.pending,
            actual_receipts: self.actual_receipts, resources: self.resources, prefix: self.prefix,
            unadopted_resources: self.unadopted_resources, services: self.services, callbacks: self.callbacks }
    }
    fn slot(request: SourceCrtModulePublishRequest) -> usize {
        match request { SourceCrtModulePublishRequest::ExchangeModuleCache { .. } => 0,
            SourceCrtModulePublishRequest::CleanupAt59be73 { .. } => 1 }
    }
    fn fail<T>(self, cause: T) -> SourceCrtModulePublicationFailure<Self, T> {
        SourceCrtModulePublicationFailure { owner: self, cause }
    }
}
