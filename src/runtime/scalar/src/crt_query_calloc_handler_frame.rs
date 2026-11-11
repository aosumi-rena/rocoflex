#[path = "crt_query_calloc_handler_continuation.rs"]
pub mod continuation;
#[path = "crt_query_calloc_handler_transfer.rs"]
pub mod transfer;

use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocProgress,
    SourceCrtQueryCallocRequest, SourceCrtCallocCallKind, SourceCrtCallocCall,
    SourceCrtQueryCallocReceipt, SourceCrtQueryCallocError};
use super::crt_query_context::SourceCrtQueryStackWord;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetStackRead,
    SourceCrtNativeRetExpectation, SourceCrtNativeRetObservation, SourceCrtNativeRetired,
    SourceCrtNativeRetError, SourceCrtNativeReturnFailure, accept_source_crt_native_ret_receipt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtHandlerActualPrefix { pub frame: SourceCrtNativeRetFrame, pub rbx: u64, pub rdi: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtHandlerActualEntry {
    pub entry: SourceCrtHandlerActualPrefix, pub actual_rcx: u64,

    pub saved: [SourceCrtQueryStackWord; 2], pub body: SourceCrtHandlerActualPrefix,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtHandlerActualEpilogue {
    pub before: SourceCrtHandlerActualPrefix,
    pub reads: [SourceCrtNativeRetStackRead; 2], pub after: SourceCrtHandlerActualPrefix,
}
pub struct SourceCrtHandlerFrameBindings<D, P, S, A> {
    pub source_bytes: fn(&D) -> u64,
    pub completed_eax: fn(&D) -> Option<u32>,
    pub actual_entry: fn(&A) -> SourceCrtHandlerActualEntry,
    pub owned_entry: fn(&P, &S) -> Option<SourceCrtHandlerActualEntry>,
    pub current_prefix: fn(&P, &S) -> SourceCrtHandlerActualPrefix,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtHandlerFramePhase { EpilogueRequired, EpilogueContinuationRequired, RetRequired, RetContinuationRequired, Complete }
#[must_use = "retain original stopped calloc, SAME handler D/full frame/FP/native/lock resources"]
pub struct SourceCrtCallocHandlerFrame<D, P, S, A> {
    calloc: SourceCrtQueryCalloc58f074,
    pub handler: D, pub prefix: P, pub services: S, pub actual_entry: A,
    bindings: SourceCrtHandlerFrameBindings<D, P, S, A>, entry: SourceCrtHandlerActualEntry,
    request: SourceCrtCallocCall, eax: u32, phase: SourceCrtHandlerFramePhase,
}
#[must_use = "retain all rejected bind inputs; no handler/calloc replay"]
pub struct SourceCrtHandlerFrameBindFailure<D, P, S, A> {
    pub calloc: SourceCrtQueryCalloc58f074, pub handler: D, pub prefix: P, pub services: S, pub actual_entry: A,
    pub bindings: SourceCrtHandlerFrameBindings<D, P, S, A>,
    pub observed: SourceCrtHandlerActualEntry, pub owned: Option<SourceCrtHandlerActualEntry>,
    pub current: SourceCrtHandlerActualPrefix, pub source_bytes: u64, pub completed_eax: Option<u32>,
}
pub enum SourceCrtHandlerEpilogueError<R, E> {
    Phase { actual: R },
    Adapter { actual: R, observed: SourceCrtHandlerActualEpilogue, cause: E },
    Binding { actual: R, observed: SourceCrtHandlerActualEpilogue, current: SourceCrtHandlerActualPrefix },
}
pub enum SourceCrtHandlerFrameReturnError<N, E> {
    Phase { actual: N }, Native(SourceCrtNativeRetError<N, E>),
    Caller { actual: N, retired: SourceCrtNativeRetired, current: SourceCrtHandlerActualPrefix,
        source: Option<SourceCrtQueryCallocError> },
}
#[must_use = "retain completed SAME handler/entry/epilogue/RET and original calloc source"]
pub struct SourceCrtHandlerFrameReturned<D, P, S, A, R, N> {
    pub owner: SourceCrtCallocHandlerFrame<D, P, S, A>, pub actual_return: N,
    pub actual_epilogue: R,
    pub retired: SourceCrtNativeRetired, pub source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain SAME handler/entry/epilogue/RET/frame/FP/native resources with continuing calloc"]
pub struct SourceCrtHandlerFrameRetained<D, P, S, A, R, N> {
    pub handler: D, pub prefix: P, pub services: S, pub actual_entry: A,
    pub actual_epilogue: R, pub actual_return: N, pub retired: SourceCrtNativeRetired,
    pub bindings: SourceCrtHandlerFrameBindings<D, P, S, A>,
    pub entry: SourceCrtHandlerActualEntry, pub call: SourceCrtCallocCall, pub normalized_eax: u32,
}
impl<D, P, S, A, R, N> SourceCrtHandlerFrameReturned<D, P, S, A, R, N> {

    pub fn into_source_and_retained(self)
        -> Result<(SourceCrtQueryCalloc58f074, SourceCrtHandlerFrameRetained<D, P, S, A, R, N>), Self> {
        if self.owner.phase != SourceCrtHandlerFramePhase::Complete || self.owner.calloc.progress() != self.source
            || !self.owner.same_binding()
            || (self.owner.bindings.current_prefix)(&self.owner.prefix, &self.owner.services).frame != self.retired.after {
            return Err(self);
        }
        let SourceCrtHandlerFrameReturned { owner, actual_return, actual_epilogue, retired, .. } = self;
        let SourceCrtCallocHandlerFrame { calloc, handler, prefix, services, actual_entry, bindings, entry, request, eax, .. } = owner;
        Ok((calloc, SourceCrtHandlerFrameRetained { handler, prefix, services, actual_entry, actual_epilogue,
            actual_return, retired, bindings, entry, call: request, normalized_eax: eax }))
    }
}
struct SourceCrtHandlerFrameRawReturned<D, P, S, A, N> {
    owner: SourceCrtCallocHandlerFrame<D, P, S, A>, actual_return: N,
    retired: SourceCrtNativeRetired, source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain actual completed epilogue and SAME owner before actual RET"]
pub struct SourceCrtHandlerEpilogueAccepted<D, P, S, A, R> {
    owner: SourceCrtCallocHandlerFrame<D, P, S, A>, pub actual_epilogue: R,
}
impl<D, P, S, A, R> SourceCrtHandlerEpilogueAccepted<D, P, S, A, R> {
    pub fn owner(&self) -> &SourceCrtCallocHandlerFrame<D, P, S, A> { &self.owner }

    pub fn accept_actual_ret<N, E>(self, actual: N, project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N) -> Result<(), E>)
        -> Result<SourceCrtHandlerFrameReturned<D, P, S, A, R, N>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerFrameReturnError<N, E>>> {
        let Self { owner, actual_epilogue } = self;
        match owner.accept_actual_ret(actual, project, apply) {
            Ok(value) => Ok(SourceCrtHandlerFrameReturned { owner: value.owner, actual_epilogue,
                actual_return: value.actual_return, retired: value.retired, source: value.source }),
            Err(failure) => Err(SourceCrtNativeReturnFailure {
                owner: Self { owner: failure.owner, actual_epilogue }, cause: failure.cause }),
        }
    }
}
impl<D, P, S, A> SourceCrtCallocHandlerFrame<D, P, S, A> {

    pub fn bind_completed_body(mut calloc: SourceCrtQueryCalloc58f074, handler: D, prefix: P, services: S,
        actual_entry: A, bindings: SourceCrtHandlerFrameBindings<D, P, S, A>)
        -> Result<Self, SourceCrtHandlerFrameBindFailure<D, P, S, A>> {
        calloc.stop();
        let observed = (bindings.actual_entry)(&actual_entry);
        let owned = (bindings.owned_entry)(&prefix, &services);
        let current = (bindings.current_prefix)(&prefix, &services);
        let source_bytes = (bindings.source_bytes)(&handler);
        let completed_eax = (bindings.completed_eax)(&handler);
        let request = match calloc.awaiting_actual_completion() {
            Some(SourceCrtQueryCallocRequest::RequiredCall(call)) if call.kind == SourceCrtCallocCallKind::Handler5765dc => Some(call), _ => None,
        };
        let base = calloc.entry().actual_module_base;
        let valid = match (request, completed_eax) {
            (Some(call), Some(eax)) if eax <= 1 => {
                let rsp = call.actual_caller_rsp.wrapping_sub(8);
                observed.entry.frame.actual_rip == call.actual_target && observed.entry.frame.actual_rsp == rsp
                    && observed.entry.rbx == call.required_bytes_rbx && observed.actual_rcx == source_bytes
                    && Some(source_bytes) == call.source_rcx && source_bytes == call.required_bytes_rbx
                    && observed.saved == [SourceCrtQueryStackWord { actual_address: rsp.wrapping_add(8), actual_word: observed.entry.rbx },
                        SourceCrtQueryStackWord { actual_address: rsp.wrapping_sub(8), actual_word: observed.entry.rdi }]
                    && observed.body == (SourceCrtHandlerActualPrefix { frame: SourceCrtNativeRetFrame {
                        actual_rip: base.wrapping_add(0x5765e9), actual_rsp: rsp.wrapping_sub(0x28), actual_rax: observed.entry.frame.actual_rax },
                        rbx: observed.entry.rbx, rdi: source_bytes })
                    && owned == Some(observed)

                    && current == (SourceCrtHandlerActualPrefix { frame: SourceCrtNativeRetFrame {
                        actual_rip: base.wrapping_add(0x57660f), actual_rsp: rsp.wrapping_sub(0x28), actual_rax: u64::from(eax) },
                        rbx: u64::from(eax), rdi: source_bytes })
            }
            _ => false,
        };
        if !valid {
            return Err(SourceCrtHandlerFrameBindFailure { calloc, handler, prefix, services, actual_entry, bindings,
                observed, owned, current, source_bytes, completed_eax });
        }
        let (Some(request), Some(eax)) = (request, completed_eax) else {
            return Err(SourceCrtHandlerFrameBindFailure { calloc, handler, prefix, services, actual_entry, bindings,
                observed, owned, current, source_bytes, completed_eax });
        };
        Ok(Self { calloc, handler, prefix, services, actual_entry, bindings, entry: observed,
            request, eax, phase: SourceCrtHandlerFramePhase::EpilogueRequired })
    }
    pub fn phase(&self) -> SourceCrtHandlerFramePhase { self.phase }
    pub fn source(&self) -> &SourceCrtQueryCalloc58f074 { &self.calloc }
    pub fn accept_actual_epilogue<R, E>(mut self, mut actual: R,
        project: fn(&R) -> SourceCrtHandlerActualEpilogue,
        apply: fn(&mut P, &mut S, &mut R) -> Result<(), E>)
        -> Result<SourceCrtHandlerEpilogueAccepted<D, P, S, A, R>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerEpilogueError<R, E>>> {
        if !matches!(self.phase, SourceCrtHandlerFramePhase::EpilogueRequired | SourceCrtHandlerFramePhase::EpilogueContinuationRequired) {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerEpilogueError::Phase { actual } });
        }
        self.phase = SourceCrtHandlerFramePhase::EpilogueContinuationRequired;
        let observed = project(&actual);
        if let Err(cause) = apply(&mut self.prefix, &mut self.services, &mut actual) {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerEpilogueError::Adapter { actual, observed, cause } });
        }
        let current = (self.bindings.current_prefix)(&self.prefix, &self.services);
        let base = self.calloc.entry().actual_module_base;
        let expected_before = SourceCrtHandlerActualPrefix { frame: SourceCrtNativeRetFrame {
            actual_rip: base.wrapping_add(0x57660f), actual_rsp: self.entry.body.frame.actual_rsp, actual_rax: u64::from(self.eax) },
            rbx: u64::from(self.eax), rdi: self.request.required_bytes_rbx };
        let expected_after = self.ret_prefix();
        let reads = [SourceCrtNativeRetStackRead { actual_address: self.entry.saved[0].actual_address, actual_word: self.entry.saved[0].actual_word },
            SourceCrtNativeRetStackRead { actual_address: self.entry.saved[1].actual_address, actual_word: self.entry.saved[1].actual_word }];
        if project(&actual) != observed || observed.before != expected_before || observed.reads != reads
            || observed.after != expected_after || current != expected_after || !self.same_binding() {
            return Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtHandlerEpilogueError::Binding { actual, observed, current } });
        }
        self.phase = SourceCrtHandlerFramePhase::RetRequired;
        Ok(SourceCrtHandlerEpilogueAccepted { owner: self, actual_epilogue: actual })
    }
    fn ret_prefix(&self) -> SourceCrtHandlerActualPrefix {
        SourceCrtHandlerActualPrefix { frame: SourceCrtNativeRetFrame {
            actual_rip: self.calloc.entry().actual_module_base.wrapping_add(0x576619),
            actual_rsp: self.entry.entry.frame.actual_rsp, actual_rax: u64::from(self.eax) },
            rbx: self.entry.entry.rbx, rdi: self.entry.entry.rdi }
    }
    fn same_binding(&self) -> bool {
        (self.bindings.actual_entry)(&self.actual_entry) == self.entry
            && (self.bindings.owned_entry)(&self.prefix, &self.services) == Some(self.entry)
            && (self.bindings.source_bytes)(&self.handler) == self.request.required_bytes_rbx
            && (self.bindings.completed_eax)(&self.handler) == Some(self.eax)
    }
    fn accept_actual_ret<N, E>(mut self, actual: N, project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N) -> Result<(), E>)
        -> Result<SourceCrtHandlerFrameRawReturned<D, P, S, A, N>,
            SourceCrtNativeReturnFailure<Self, SourceCrtHandlerFrameReturnError<N, E>>> {
        if !matches!(self.phase, SourceCrtHandlerFramePhase::RetRequired | SourceCrtHandlerFramePhase::RetContinuationRequired) {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerFrameReturnError::Phase { actual } });
        }
        self.phase = SourceCrtHandlerFramePhase::RetContinuationRequired;
        let expected = SourceCrtNativeRetExpectation { before: self.ret_prefix().frame,
            actual_required_target: self.request.required_actual_resume };
        let (actual, retired) = match accept_source_crt_native_ret_receipt(&mut self, expected, actual, project,
            |owner, actual| apply(&mut owner.prefix, &mut owner.services, actual),
            |owner| (owner.bindings.current_prefix)(&owner.prefix, &owner.services).frame) {
            Ok(value) => value,
            Err(cause) => return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerFrameReturnError::Native(cause) }),
        };
        let current = (self.bindings.current_prefix)(&self.prefix, &self.services);
        if current.rbx != self.request.required_bytes_rbx || current.rdi != self.entry.entry.rdi || !self.same_binding() {
            return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerFrameReturnError::Caller {
                actual, retired, current, source: None } });
        }
        let source = match self.calloc.accept_actual_completion(SourceCrtQueryCallocReceipt::CallReturned {
            request: self.request, actual_frame: retired.after, actual_rbx: current.rbx }) {
            Ok(value) => value,
            Err(source) => return Err(SourceCrtNativeReturnFailure { owner: self, cause: SourceCrtHandlerFrameReturnError::Caller {
                actual, retired, current, source: Some(source) } }),
        };
        self.phase = SourceCrtHandlerFramePhase::Complete;
        Ok(SourceCrtHandlerFrameRawReturned { owner: self, actual_return: actual, retired, source })
    }
}
