use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocProgress,
    SourceCrtQueryCallocRequest, SourceCrtCallocCallKind, SourceCrtQueryCallocReceipt,
    SourceCrtQueryCallocError};
use super::crt_storage_native_return::SourceCrtNativeRetFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocActualHandlerEntry {
    pub actual_frame: SourceCrtNativeRetFrame,
    pub actual_rcx: u64,
    pub actual_rbx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocActualHandlerReturn {

    pub entry: SourceCrtCallocActualHandlerEntry,

    pub returned: SourceCrtQueryCallocReceipt,
}

pub struct SourceCrtCallocHandlerBindings<D, P, S, R> {
    pub source_bytes: fn(&D) -> u64,
    pub completed_eax: fn(&D) -> Option<u32>,
    pub actual_frame: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub actual_rbx: fn(&P, &S) -> u64,
    pub owned_return: fn(&P, &S) -> Option<SourceCrtCallocActualHandlerReturn>,
    pub actual_return: fn(&R) -> SourceCrtCallocActualHandlerReturn,
}
#[must_use = "retain SAME handler/lock/native/frame/FP/whole-return resource capsule"]
pub struct SourceCrtCallocHandlerCapsule<D, P, S, R> {
    pub handler: D,
    pub prefix: P,
    pub services: S,
    pub actual_return: R,
    pub bindings: SourceCrtCallocHandlerBindings<D, P, S, R>,
}
#[must_use = "retain full handler capsule through next actual heap read/attempt"]
pub struct SourceCrtCallocHandlerAccepted<D, P, S, R> {
    pub actual: SourceCrtCallocHandlerCapsule<D, P, S, R>,
    pub source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain rejected handler capsule with stopped original calloc; no implicit retry"]
pub struct SourceCrtCallocHandlerError<D, P, S, R> {
    pub actual: SourceCrtCallocHandlerCapsule<D, P, S, R>,
    pub pending: SourceCrtQueryCallocProgress,
    pub source_bytes: u64,
    pub completed_eax: Option<u32>,
    pub observed: SourceCrtCallocActualHandlerReturn,
    pub owned_observed: Option<SourceCrtCallocActualHandlerReturn>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub actual_rbx: u64,
    pub source: Option<SourceCrtQueryCallocError>,
}
impl SourceCrtQueryCalloc58f074 {

    pub fn accept_owned_handler_return<D, P, S, R>(&mut self,
        actual: SourceCrtCallocHandlerCapsule<D, P, S, R>)
        -> Result<SourceCrtCallocHandlerAccepted<D, P, S, R>, SourceCrtCallocHandlerError<D, P, S, R>> {
        self.stop();
        let pending = self.progress();
        let source_bytes = (actual.bindings.source_bytes)(&actual.handler);
        let completed_eax = (actual.bindings.completed_eax)(&actual.handler);
        let observed = (actual.bindings.actual_return)(&actual.actual_return);
        let owned_observed = (actual.bindings.owned_return)(&actual.prefix, &actual.services);
        let actual_frame = (actual.bindings.actual_frame)(&actual.prefix, &actual.services);
        let actual_rbx = (actual.bindings.actual_rbx)(&actual.prefix, &actual.services);
        let expected = match pending {
            SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredCall(call))
                if call.kind == SourceCrtCallocCallKind::Handler5765dc => Some(call), _ => None,
        };
        let matches = match (expected, observed.returned, completed_eax) {
            (Some(expected), SourceCrtQueryCallocReceipt::CallReturned { request, actual_frame: returned, actual_rbx: returned_rbx }, Some(eax)) => {
                request == expected && source_bytes == expected.required_bytes_rbx
                    && expected.source_rcx == Some(source_bytes) && eax <= 1
                    && observed.entry.actual_frame.actual_rip == expected.actual_target
                    && observed.entry.actual_frame.actual_rsp == expected.actual_caller_rsp.wrapping_sub(8)
                    && observed.entry.actual_rcx == source_bytes
                    && observed.entry.actual_rbx == expected.required_bytes_rbx
                    && owned_observed == Some(observed) && returned == actual_frame
                    && actual_rbx == returned_rbx && actual_rbx == expected.required_bytes_rbx
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && actual_frame.actual_rax as u32 == eax
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtCallocHandlerError { actual, pending, source_bytes, completed_eax,
                observed, owned_observed, actual_frame, actual_rbx, source: None });
        }
        match self.accept_actual_completion(observed.returned) {
            Ok(source) => Ok(SourceCrtCallocHandlerAccepted { actual, source }),
            Err(source) => Err(SourceCrtCallocHandlerError { actual, pending, source_bytes, completed_eax,
                observed, owned_observed, actual_frame, actual_rbx, source: Some(source) }),
        }
    }
}
