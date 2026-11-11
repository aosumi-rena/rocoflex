use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocProgress,
    SourceCrtQueryCallocRequest, SourceCrtCallocCallKind, SourceCrtQueryCallocReceipt,
    SourceCrtQueryCallocError};
use super::crt_query_owned::{SourceCrtQueryOwned, SourceCrtQueryOwnedBindings, SourceCrtQueryCursor};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetExpectation,
    SourceCrtNativeRetObservation, SourceCrtNativeRetired, validate_source_crt_native_ret};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocAccessorEntry {
    pub pc: u32, pub actual_module_base: u64, pub rsp: u64,
    pub subtract_before_after: Option<(u64, u64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocAccessorReturn {
    pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCallocAccessorDescription {
    pub body_pc: u32, pub actual_module_base: u64, pub body_work_rsp: u64,
    pub entry: Option<SourceCrtCallocAccessorEntry>,
    pub boundary_pc: u32, pub boundary_rsp: u64, pub boundary_rax: u64,
    pub adjustment_before_after: Option<(u64, u64)>,
    pub returned: SourceCrtCallocAccessorReturn,
}

pub struct SourceCrtCallocAccessorBindings<D, P, S, N> {
    pub describe: fn(&D) -> SourceCrtCallocAccessorDescription,
    pub current_frame: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub current_rbx: fn(&P, &S) -> u64,
    pub actual_ret: fn(&N) -> SourceCrtNativeRetObservation,
    pub owned_ret: fn(&D, &P, &S) -> Option<SourceCrtNativeRetObservation>,
}
#[must_use = "retain SAME accessor provider/context/storage/rawRET/frame/FP/native owners"]
pub struct SourceCrtCallocAccessorCapsule<D, P, S, N> {
    pub accessor: D, pub prefix: P, pub services: S, pub actual_ret: N,
    pub bindings: SourceCrtCallocAccessorBindings<D, P, S, N>,
}
#[must_use = "retain stopped original calloc and rejected actual accessor resources"]
pub struct SourceCrtCallocAccessorFailure<D, P, S, N> {
    pub calloc: SourceCrtQueryCalloc58f074, pub actual: SourceCrtCallocAccessorCapsule<D, P, S, N>,
    pub description: SourceCrtCallocAccessorDescription,
    pub observed_ret: SourceCrtNativeRetObservation, pub owned_ret: Option<SourceCrtNativeRetObservation>,
    pub actual_frame: SourceCrtNativeRetFrame, pub actual_rbx: u64, pub source: Option<SourceCrtQueryCallocError>,
}
#[must_use = "retain original advanced calloc and whole accessor capsule through actual DWORD12 write"]
pub struct SourceCrtCallocAccessorAccepted<D, P, S, N> {
    calloc: SourceCrtQueryCalloc58f074,
    pub actual: SourceCrtCallocAccessorCapsule<D, P, S, N>,
    description: SourceCrtCallocAccessorDescription,
    retired: SourceCrtNativeRetired,
    source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain actual accessor/context/native/rawRET owners with continuing query P/S"]
pub struct SourceCrtCallocAccessorHistory<D, P, S, N> {
    pub accessor: D, pub actual_ret: N, pub description: SourceCrtCallocAccessorDescription,
    pub retired: SourceCrtNativeRetired, pub bindings: SourceCrtCallocAccessorBindings<D, P, S, N>,
}
impl<D, P, S, N> SourceCrtCallocAccessorAccepted<D, P, S, N> {

    pub fn accept_same_return(mut calloc: SourceCrtQueryCalloc58f074,
        actual: SourceCrtCallocAccessorCapsule<D, P, S, N>)
        -> Result<Self, SourceCrtCallocAccessorFailure<D, P, S, N>> {
        calloc.stop();
        let description = (actual.bindings.describe)(&actual.accessor);
        let observed_ret = (actual.bindings.actual_ret)(&actual.actual_ret);
        let owned_ret = (actual.bindings.owned_ret)(&actual.accessor, &actual.prefix, &actual.services);
        let actual_frame = (actual.bindings.current_frame)(&actual.prefix, &actual.services);
        let actual_rbx = (actual.bindings.current_rbx)(&actual.prefix, &actual.services);
        let expected = match calloc.awaiting_actual_completion() {
            Some(SourceCrtQueryCallocRequest::RequiredCall(call)) if call.kind == SourceCrtCallocCallKind::ErrnoAccessor578eb0 => Some(call), _ => None,
        };
        let retired = match observed_ret { SourceCrtNativeRetObservation::Retired(value) => Some(value), _ => None };
        let base = calloc.entry().actual_module_base;
        let matches = match (expected, description.entry, retired) {
            (Some(expected), Some(entry), Some(retired)) => {
                let entry_rsp = expected.actual_caller_rsp.wrapping_sub(8);
                let body_rsp = entry_rsp.wrapping_sub(0x28);
                description.body_pc == 0x578eb4 && description.actual_module_base == base && description.body_work_rsp == body_rsp
                    && entry.pc == 0x578eb0 && entry.actual_module_base == base && entry.rsp == entry_rsp
                    && entry.subtract_before_after == Some((entry_rsp, body_rsp))
                    && description.boundary_pc == 0x578ecb && description.boundary_rsp == body_rsp
                    && description.boundary_rax == description.returned.rax
                    && description.adjustment_before_after == Some((body_rsp, entry_rsp))
                    && description.returned.before_rsp == entry_rsp && description.returned.after_rsp == expected.actual_caller_rsp
                    && description.returned.target == expected.required_actual_resume
                    && actual_rbx == expected.required_bytes_rbx && owned_ret == Some(observed_ret)
                    && validate_source_crt_native_ret(SourceCrtNativeRetExpectation {
                        before: SourceCrtNativeRetFrame { actual_rip: base.wrapping_add(0x578ecf), actual_rsp: entry_rsp,
                            actual_rax: description.returned.rax }, actual_required_target: expected.required_actual_resume }, retired).is_ok()
                    && actual_frame == retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtCallocAccessorFailure { calloc, actual, description, observed_ret, owned_ret, actual_frame, actual_rbx, source: None });
        }
        let (Some(request), Some(retired)) = (expected, retired) else {
            return Err(SourceCrtCallocAccessorFailure { calloc, actual, description, observed_ret, owned_ret, actual_frame, actual_rbx, source: None });
        };
        let source = match calloc.accept_actual_completion(SourceCrtQueryCallocReceipt::CallReturned {
            request, actual_frame, actual_rbx }) {
            Ok(source) => source,
            Err(source) => return Err(SourceCrtCallocAccessorFailure { calloc, actual, description, observed_ret, owned_ret,
                actual_frame, actual_rbx, source: Some(source) }),
        };
        Ok(Self { calloc, actual, description, retired, source })
    }
    pub fn source(&self) -> &SourceCrtQueryCalloc58f074 { &self.calloc }

    pub fn into_owned_query(self, bindings: SourceCrtQueryOwnedBindings<P, S>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtCallocAccessorHistory<D, P, S, N>>, Self> {
        if self.calloc.is_stopped() || self.calloc.progress() != self.source
            || (self.actual.bindings.describe)(&self.actual.accessor) != self.description
            || (self.actual.bindings.actual_ret)(&self.actual.actual_ret) != SourceCrtNativeRetObservation::Retired(self.retired)
            || (self.actual.bindings.owned_ret)(&self.actual.accessor, &self.actual.prefix, &self.actual.services)
                != Some(SourceCrtNativeRetObservation::Retired(self.retired))
            || (self.actual.bindings.current_frame)(&self.actual.prefix, &self.actual.services) != self.retired.after
            || (self.actual.bindings.current_rbx)(&self.actual.prefix, &self.actual.services) != self.calloc.entry().arguments.source_bytes()
            || (bindings.actual_frame)(&self.actual.prefix, &self.actual.services) != self.retired.after {
            return Err(self);
        }
        let Self { calloc, actual, description, retired, .. } = self;
        let SourceCrtCallocAccessorCapsule { accessor, prefix, services, actual_ret, bindings: accessor_bindings } = actual;
        Ok(SourceCrtQueryOwned::from_same_cursor_and_history(SourceCrtQueryCursor::Calloc(calloc), prefix, services,
            SourceCrtCallocAccessorHistory { accessor, actual_ret, description, retired, bindings: accessor_bindings }, bindings))
    }
}
